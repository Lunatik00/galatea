//! Encuadre 3:4 de la foto y recorte del resultado: funciones puras, sin ComfyUI.
use galatea::encuadre::{self, Marco};
use image::{DynamicImage, Rgb, RgbImage, imageops::FilterType};

const BLANCO: Rgb<u8> = Rgb([255, 255, 255]);
const CUADRANTES: [Rgb<u8>; 4] = [
    Rgb([200, 30, 30]),  // arriba a la izquierda
    Rgb([30, 160, 30]),  // arriba a la derecha
    Rgb([30, 30, 200]),  // abajo a la izquierda
    Rgb([200, 160, 30]), // abajo a la derecha
];

/// Una «foto» con un color distinto en cada cuadrante: si el recorte se corre, se nota.
fn cuadrantes(ancho: u32, alto: u32) -> DynamicImage {
    DynamicImage::ImageRgb8(RgbImage::from_fn(ancho, alto, |x, y| {
        let i = (x >= ancho / 2) as usize + 2 * (y >= alto / 2) as usize;
        CUADRANTES[i]
    }))
}

fn parecido(a: Rgb<u8>, b: Rgb<u8>) -> bool {
    a.0.iter().zip(b.0).all(|(x, y)| x.abs_diff(y) <= 12)
}

#[test]
fn una_foto_alta_900x1600_se_rellena_de_blanco_a_los_lados() {
    let (lienzo, marco) = encuadre::encuadrar(&cuadrantes(900, 1600));
    assert_eq!(
        marco,
        Marco {
            x: 150,
            y: 0,
            ancho: 900,
            alto: 1600,
            ancho_lienzo: 1200,
            alto_lienzo: 1600
        }
    );
    let l = lienzo.to_rgb8();
    assert_eq!(l.dimensions(), (1200, 1600));
    assert_eq!(*l.get_pixel(0, 0), BLANCO);
    assert_eq!(*l.get_pixel(149, 800), BLANCO);
    assert_eq!(*l.get_pixel(1050, 1599), BLANCO);
    assert_eq!(*l.get_pixel(150, 0), CUADRANTES[0]);
    assert_eq!(*l.get_pixel(1049, 1599), CUADRANTES[3]);
}

#[test]
fn una_foto_ancha_1600x900_se_rellena_arriba_y_abajo_redondeando_hacia_arriba() {
    // alto = ceil(1600 · 4/3) = ceil(2133.3) = 2134; y = (2134 − 900) / 2 = 617.
    let (lienzo, marco) = encuadre::encuadrar(&cuadrantes(1600, 900));
    assert_eq!(
        marco,
        Marco {
            x: 0,
            y: 617,
            ancho: 1600,
            alto: 900,
            ancho_lienzo: 1600,
            alto_lienzo: 2134
        }
    );
    let l = lienzo.to_rgb8();
    assert_eq!(l.dimensions(), (1600, 2134));
    assert_eq!(*l.get_pixel(800, 616), BLANCO);
    assert_eq!(*l.get_pixel(0, 617), CUADRANTES[0]);
    assert_eq!(*l.get_pixel(1599, 1516), CUADRANTES[3]);
    assert_eq!(*l.get_pixel(800, 1517), BLANCO);
}

#[test]
fn una_foto_ya_3x4_no_cambia() {
    for (ancho, alto) in [(768, 1024), (300, 401)] {
        let foto = cuadrantes(ancho, alto);
        let (lienzo, marco) = encuadre::encuadrar(&foto);
        assert_eq!(lienzo, foto, "{ancho}x{alto}");
        assert_eq!(
            marco,
            Marco {
                x: 0,
                y: 0,
                ancho,
                alto,
                ancho_lienzo: ancho,
                alto_lienzo: alto
            }
        );
    }
}

#[test]
fn desencuadrar_un_768x1024_devuelve_la_region_original_a_900x1600() {
    let (lienzo, marco) = encuadre::encuadrar(&cuadrantes(900, 1600));
    // Lo que haría Leffa: trabajar a 768×1024 y devolver esa proporción.
    let generado = lienzo.resize_exact(768, 1024, FilterType::Lanczos3);
    let final_ = encuadre::desencuadrar(&generado, &marco)
        .expect("proporción 3:4")
        .to_rgb8();
    assert_eq!(final_.dimensions(), (900, 1600));
    // Cada cuadrante sigue en su lugar, y los bordes no traen las franjas blancas.
    for (x, y, i) in [(225, 400, 0), (675, 400, 1), (225, 1200, 2), (675, 1200, 3)] {
        assert!(
            parecido(*final_.get_pixel(x, y), CUADRANTES[i]),
            "({x},{y})"
        );
    }
    for (x, y, i) in [(3, 400, 0), (896, 400, 1), (3, 1200, 2), (896, 1200, 3)] {
        assert!(
            parecido(*final_.get_pixel(x, y), CUADRANTES[i]),
            "borde ({x},{y}): {:?}",
            final_.get_pixel(x, y)
        );
    }
}

#[test]
fn desencuadrar_el_lienzo_del_mismo_tamano_devuelve_la_foto_exacta() {
    let foto = cuadrantes(1600, 900);
    let (lienzo, marco) = encuadre::encuadrar(&foto);
    let final_ = encuadre::desencuadrar(&lienzo, &marco).unwrap();
    assert_eq!(final_.to_rgb8(), foto.to_rgb8());
}

#[test]
fn desencuadrar_rechaza_un_resultado_de_otra_proporcion() {
    let (_, marco) = encuadre::encuadrar(&cuadrantes(900, 1600));
    let panoramico = cuadrantes(1024, 576);
    assert!(encuadre::desencuadrar(&panoramico, &marco).is_none());
}

#[test]
fn el_lienzo_codificado_en_jpeg_conserva_el_tamano() {
    let (lienzo, _) = encuadre::encuadrar(&cuadrantes(900, 1600));
    let bytes = encuadre::a_jpeg(&lienzo).unwrap();
    let vuelta = image::load_from_memory(&bytes).unwrap();
    assert_eq!(
        image::guess_format(&bytes).unwrap(),
        image::ImageFormat::Jpeg
    );
    assert_eq!((vuelta.width(), vuelta.height()), (1200, 1600));
    let png = encuadre::a_png(&vuelta).unwrap();
    assert_eq!(image::guess_format(&png).unwrap(), image::ImageFormat::Png);
    assert_eq!(image::load_from_memory(&png).unwrap().width(), 1200);
}

#[test]
fn un_relleno_impar_se_reparte_con_a_lo_sumo_un_pixel_de_diferencia() {
    // 901×1600 → lienzo 1200×1600: sobran 299 píxeles de ancho, 149 a un lado y 150 al otro.
    let (lienzo, marco) = encuadre::encuadrar(&cuadrantes(901, 1600));
    assert_eq!((marco.ancho_lienzo, marco.alto_lienzo), (1200, 1600));
    assert_eq!((lienzo.width(), lienzo.height()), (1200, 1600));
    let izquierda = marco.x;
    let derecha = marco.ancho_lienzo - marco.x - marco.ancho;
    assert_eq!(izquierda + marco.ancho + derecha, marco.ancho_lienzo);
    assert!(izquierda.abs_diff(derecha) <= 1, "{izquierda} y {derecha}");
    assert_eq!(marco.y, 0);
    let l = lienzo.to_rgb8();
    assert_eq!(*l.get_pixel(izquierda - 1, 800), BLANCO);
    assert_eq!(*l.get_pixel(izquierda, 800), CUADRANTES[2]);
    assert_eq!(
        *l.get_pixel(izquierda + marco.ancho - 1, 800),
        CUADRANTES[3]
    );
    assert_eq!(*l.get_pixel(izquierda + marco.ancho, 800), BLANCO);
}
