//! Marca de agua «imagen generada con IA» en la esquina inferior derecha.
use image::{DynamicImage, ImageFormat, imageops};
use std::io::Cursor;

pub const TEXTO: &str = "imagen generada con IA";
const SELLO: &[u8] = include_bytes!("../recursos/marca.png");

/// El sello ocupa el 45 % del ancho de la imagen, con un margen del 2 %.
pub fn aplicar(png: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    let mut base = image::load_from_memory(png)?.to_rgba8();
    let sello = image::load_from_memory(SELLO)?;
    let (ancho, alto) = base.dimensions();
    let ancho_sello = (ancho as f32 * 0.45).max(40.0) as u32;
    let alto_sello =
        (ancho_sello as f32 * sello.height() as f32 / sello.width() as f32).max(8.0) as u32;
    let sello = sello
        .resize_exact(ancho_sello, alto_sello, imageops::FilterType::Triangle)
        .to_rgba8();
    let margen = (ancho as f32 * 0.02) as i64;
    let x = ancho as i64 - ancho_sello as i64 - margen;
    let y = alto as i64 - alto_sello as i64 - margen;
    imageops::overlay(&mut base, &sello, x.max(0), y.max(0));
    let mut salida = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(base).write_to(&mut salida, ImageFormat::Png)?;
    Ok(salida.into_inner())
}
