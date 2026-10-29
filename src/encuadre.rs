//! Encuadre 3:4 de la foto de la persona antes de mandarla a ComfyUI, y recorte del
//! resultado de vuelta al tamaño original.
//!
//! Por qué: Leffa (y los modelos entrenados con VITON-HD) trabajan en un marco fijo de
//! 768×1024. `LeffaTryOn` mete cualquier foto en ese marco con relleno blanco y, con
//! `restore_size`, estira el marco rellenado al tamaño original: una foto de celular 9:16
//! sale un 25 % más flaca y con franjas blancas. Si la foto ya llega a 3:4, el nodo no
//! rellena ni estira nada; Galatea quita después el relleno que puso.
use image::{DynamicImage, ImageError, Rgb, RgbImage, codecs::jpeg::JpegEncoder, imageops};
use std::io::Cursor;

/// Proporción ancho:alto del marco de VITON-HD (768×1024).
const ANCHO_OBJETIVO: u64 = 3;
const ALTO_OBJETIVO: u64 = 4;
/// Una foto a menos del 1 % de 3:4 no se rellena: el nodo la escala sin deformarla a la vista
/// y rellenar uno o dos píxeles solo agregaría una línea blanca.
const TOLERANCIA_FOTO: f64 = 0.01;
/// El resultado puede volver a otro tamaño (768×1024, o redondeado a múltiplos de 8 o 16 por
/// el modelo): se acepta hasta un 2 % de diferencia de proporción con el lienzo. Más que eso
/// es un flujo que no respeta la proporción de la entrada, y recortar lo deformaría.
const TOLERANCIA_RESULTADO: f64 = 0.02;
/// El mismo blanco que usa Leffa para rellenar (`resize_and_center`).
const RELLENO: Rgb<u8> = Rgb([255, 255, 255]);
/// Calidad del JPEG que se sube a ComfyUI: la foto ya viene de un JPEG 90.
const CALIDAD_JPEG: u8 = 92;

/// Dónde quedó la foto original dentro del lienzo rellenado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marco {
    pub x: u32,
    pub y: u32,
    pub ancho: u32,
    pub alto: u32,
    pub ancho_lienzo: u32,
    pub alto_lienzo: u32,
}

impl Marco {
    /// La foto ocupa todo el lienzo: no hubo relleno.
    pub fn es_completo(&self) -> bool {
        self.ancho == self.ancho_lienzo && self.alto == self.alto_lienzo
    }
}

/// Diferencia relativa entre la proporción `ancho/alto` y `referencia`.
fn desvio(ancho: u32, alto: u32, referencia: f64) -> f64 {
    (ancho as f64 / alto as f64 / referencia - 1.0).abs()
}

/// Rellena la foto de blanco, centrada, hasta 3:4. Solo agrega píxeles, nunca recorta: el
/// lado que falta se redondea hacia arriba (1600×900 → 1600×2134). Una foto que ya es 3:4
/// (a menos del 1 %) vuelve tal cual.
pub fn encuadrar(foto: &DynamicImage) -> (DynamicImage, Marco) {
    let (ancho, alto) = (foto.width(), foto.height());
    let completo = Marco {
        x: 0,
        y: 0,
        ancho,
        alto,
        ancho_lienzo: ancho,
        alto_lienzo: alto,
    };
    let objetivo = ANCHO_OBJETIVO as f64 / ALTO_OBJETIVO as f64;
    if ancho == 0 || alto == 0 || desvio(ancho, alto, objetivo) <= TOLERANCIA_FOTO {
        return (foto.clone(), completo);
    }
    let arriba = |n: u64, d: u64| n.div_ceil(d) as u32;
    let ancho_lienzo = ancho.max(arriba(alto as u64 * ANCHO_OBJETIVO, ALTO_OBJETIVO));
    let alto_lienzo = alto.max(arriba(ancho as u64 * ALTO_OBJETIVO, ANCHO_OBJETIVO));
    let marco = Marco {
        x: (ancho_lienzo - ancho) / 2,
        y: (alto_lienzo - alto) / 2,
        ancho,
        alto,
        ancho_lienzo,
        alto_lienzo,
    };
    let mut lienzo = RgbImage::from_pixel(ancho_lienzo, alto_lienzo, RELLENO);
    imageops::replace(&mut lienzo, &foto.to_rgb8(), marco.x as i64, marco.y as i64);
    (DynamicImage::ImageRgb8(lienzo), marco)
}

/// Quita el relleno del resultado y lo devuelve al tamaño de la foto original. El resultado
/// puede venir a otra escala que el lienzo, pero con su misma proporción (±2 %); si no,
/// devuelve `None` y quien llama decide qué hacer con él.
pub fn desencuadrar(resultado: &DynamicImage, marco: &Marco) -> Option<DynamicImage> {
    let (ancho, alto) = (resultado.width(), resultado.height());
    if ancho == 0 || alto == 0 || marco.ancho_lienzo == 0 || marco.alto_lienzo == 0 {
        return None;
    }
    let proporcion_lienzo = marco.ancho_lienzo as f64 / marco.alto_lienzo as f64;
    if desvio(ancho, alto, proporcion_lienzo) > TOLERANCIA_RESULTADO {
        return None;
    }
    let (ex, ey) = (
        ancho as f64 / marco.ancho_lienzo as f64,
        alto as f64 / marco.alto_lienzo as f64,
    );
    let x = ((marco.x as f64 * ex).round() as u32).min(ancho - 1);
    let y = ((marco.y as f64 * ey).round() as u32).min(alto - 1);
    let w = ((marco.ancho as f64 * ex).round() as u32).clamp(1, ancho - x);
    let h = ((marco.alto as f64 * ey).round() as u32).clamp(1, alto - y);
    let recorte = resultado.crop_imm(x, y, w, h);
    if (w, h) == (marco.ancho, marco.alto) {
        Some(recorte)
    } else {
        Some(recorte.resize_exact(marco.ancho, marco.alto, imageops::FilterType::Lanczos3))
    }
}

/// JPEG de calidad 92, lo que se sube a ComfyUI como la foto de la persona.
pub fn a_jpeg(imagen: &DynamicImage) -> Result<Vec<u8>, ImageError> {
    let mut salida = Vec::new();
    JpegEncoder::new_with_quality(&mut salida, CALIDAD_JPEG).encode_image(&imagen.to_rgb8())?;
    Ok(salida)
}

pub fn a_png(imagen: &DynamicImage) -> Result<Vec<u8>, ImageError> {
    let mut salida = Cursor::new(Vec::new());
    imagen.write_to(&mut salida, image::ImageFormat::Png)?;
    Ok(salida.into_inner())
}
