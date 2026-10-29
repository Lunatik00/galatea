use galatea::marca;
use image::{GenericImageView, Rgba, RgbaImage};
use std::io::Cursor;

fn png_gris(ancho: u32, alto: u32) -> Vec<u8> {
    let im = RgbaImage::from_pixel(ancho, alto, Rgba([128, 128, 128, 255]));
    let mut buf = Cursor::new(Vec::new());
    im.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

#[test]
fn deja_las_dimensiones_y_cambia_la_esquina_inferior_derecha() {
    let original = png_gris(800, 1200);
    let marcada = marca::aplicar(&original).unwrap();
    let im = image::load_from_memory(&marcada).unwrap();
    assert_eq!(im.dimensions(), (800, 1200));
    // El sello ocupa el 45 % del ancho (360 px) pegado a la esquina inferior derecha con
    // margen del 2 %: (600, 1160) cae dentro de la banda.
    let dentro = im.get_pixel(600, 1160);
    assert_ne!(
        dentro,
        Rgba([128, 128, 128, 255]),
        "la banda inferior derecha lleva el sello"
    );
    let centro = im.get_pixel(400, 300);
    assert_eq!(centro, Rgba([128, 128, 128, 255]), "el centro no se toca");
}

#[test]
fn una_imagen_chica_recibe_un_sello_proporcional() {
    let marcada = marca::aplicar(&png_gris(200, 300)).unwrap();
    let im = image::load_from_memory(&marcada).unwrap();
    assert_eq!(im.dimensions(), (200, 300));
    // sello de 90 px de ancho y ~11 de alto, margen 4: (150, 290) está dentro
    assert_ne!(im.get_pixel(150, 290), Rgba([128, 128, 128, 255]));
}

#[test]
fn bytes_que_no_son_imagen_dan_error() {
    assert!(marca::aplicar(b"hola").is_err());
}
