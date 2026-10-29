use galatea::prendas::{self, ErrorPrendas, Prenda, Tipo};
use std::path::Path;

#[test]
fn carga_las_prendas_de_prueba() {
    let prendas = prendas::cargar(Path::new("pruebas/prendas")).unwrap();
    assert_eq!(prendas.len(), 2);
    assert_eq!(prendas[0].id, "roja");
    assert_eq!(prendas[0].tipo, Tipo::Superior);
    assert_eq!(prendas[1].tipo, Tipo::Vestido);
    assert!(prendas::buscar(&prendas, "azul").is_some());
    assert!(prendas::buscar(&prendas, "verde").is_none());
}

#[test]
fn rechaza_ids_repetidos_y_archivos_faltantes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy("pruebas/prendas/roja.png", dir.path().join("roja.png")).unwrap();
    std::fs::write(
        dir.path().join("prendas.toml"),
        r#"
[[prenda]]
id = "roja"
nombre = "Uno"
archivo = "roja.png"
tipo = "superior"
[[prenda]]
id = "roja"
nombre = "Dos"
archivo = "roja.png"
tipo = "superior"
"#,
    )
    .unwrap();
    assert!(
        matches!(prendas::cargar(dir.path()), Err(ErrorPrendas::IdRepetido(id)) if id == "roja")
    );

    std::fs::write(
        dir.path().join("prendas.toml"),
        r#"
[[prenda]]
id = "fantasma"
nombre = "No está"
archivo = "fantasma.png"
tipo = "inferior"
"#,
    )
    .unwrap();
    let e = prendas::cargar(dir.path()).unwrap_err();
    assert!(matches!(e, ErrorPrendas::ArchivoFaltante { .. }));
    assert!(
        e.to_string().contains("fantasma"),
        "el mensaje nombra el archivo: {e}"
    );
}

#[test]
fn un_toml_malformado_dice_donde() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("prendas.toml"), "[[prenda]\nid = 3").unwrap();
    let e = prendas::cargar(dir.path()).unwrap_err();
    assert!(matches!(e, ErrorPrendas::Formato { .. }));
    assert!(e.to_string().contains("prendas.toml"));
}

#[test]
fn genera_miniaturas_de_256() {
    let prendas = prendas::cargar(Path::new("pruebas/prendas")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    prendas::generar_miniaturas(Path::new("pruebas/prendas"), &prendas, dir.path()).unwrap();
    let im = image::open(dir.path().join("roja.png")).unwrap();
    assert_eq!(im.height(), 256);
    assert!(im.width() <= 256);
}

#[test]
fn sin_prendas_toml_dice_donde_falto() {
    let dir = tempfile::tempdir().unwrap();
    let e = prendas::cargar(dir.path()).unwrap_err();
    assert!(matches!(e, ErrorPrendas::Lectura { .. }));
    assert!(
        e.to_string().contains("prendas.toml"),
        "el mensaje nombra el archivo: {e}"
    );
}

#[test]
fn un_toml_sin_prendas_dice_que_falta_una_entrada() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("prendas.toml"), "").unwrap();
    let e = prendas::cargar(dir.path()).unwrap_err();
    assert!(matches!(e, ErrorPrendas::Vacio { .. }));
    assert!(
        e.to_string().contains("[[prenda]]"),
        "el mensaje dice cómo arreglarlo: {e}"
    );
}

#[test]
fn un_id_con_mayusculas_se_rechaza_y_se_nombra() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy("pruebas/prendas/roja.png", dir.path().join("roja.png")).unwrap();
    std::fs::write(
        dir.path().join("prendas.toml"),
        r#"
[[prenda]]
id = "Roja"
nombre = "Polera roja"
archivo = "roja.png"
tipo = "superior"
"#,
    )
    .unwrap();
    let e = prendas::cargar(dir.path()).unwrap_err();
    assert!(matches!(&e, ErrorPrendas::IdInvalido(id) if id == "Roja"));
    assert!(
        e.to_string().contains("Roja"),
        "el mensaje nombra el id: {e}"
    );
}

#[test]
fn una_imagen_ilegible_se_nombra_por_id() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("no-es-imagen.png"),
        b"esto no es una imagen",
    )
    .unwrap();
    let prendas = vec![Prenda {
        id: "rota".to_string(),
        nombre: "Prenda rota".to_string(),
        archivo: "no-es-imagen.png".to_string(),
        tipo: Tipo::Superior,
    }];
    let destino = tempfile::tempdir().unwrap();
    let e = prendas::generar_miniaturas(dir.path(), &prendas, destino.path()).unwrap_err();
    assert!(matches!(e, ErrorPrendas::Imagen { .. }));
    assert!(
        e.to_string().contains("rota"),
        "el mensaje nombra el id: {e}"
    );
}

#[test]
fn una_carpeta_de_destino_que_no_se_puede_crear_se_reporta() {
    let prendas = prendas::cargar(Path::new("pruebas/prendas")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let bloqueo = dir.path().join("bloqueo");
    std::fs::write(&bloqueo, b"soy un archivo, no una carpeta").unwrap();
    let destino = bloqueo.join("miniaturas");
    let e =
        prendas::generar_miniaturas(Path::new("pruebas/prendas"), &prendas, &destino).unwrap_err();
    assert!(matches!(e, ErrorPrendas::Carpeta { .. }));
}
