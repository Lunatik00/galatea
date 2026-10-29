//! Solo con el contenedor arriba:
//! `GALATEA_COMFY_URL=http://127.0.0.1:8188 cargo test --test comfy_real -- --ignored --nocapture`
//! El flujo se elige con `GALATEA_FLUJO_REAL` (por omisión `catvton`), p. ej.
//! `GALATEA_FLUJO_REAL=leffa`. El límite es de 300 s: la primera corrida de un flujo carga
//! sus modelos (Leffa en frío tarda ~160 s).
//!
//! Con `encuadre = "3:4"` (leffa) comprueba que el resultado vuelve con el tamaño exacto de la
//! foto de entrada; sin él (klein devuelve ~1 MP) solo que mida al menos 512 de lado. Que la foto no salga estirada ni con franjas, y que con Leffa
//! (`repaint`) la cara, el pelo y el resto de la ropa sigan siendo los de la foto, solo se
//! sabe mirando `experimento/ultima-cc.png`: esta prueba no lo ve.
use galatea::{
    flujos,
    motor::{Avance, Motores, Trabajo, comfy::Comfy, fijo::Fijo},
    prendas::Tipo,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

#[tokio::test]
#[ignore = "necesita ComfyUI real; ver el comentario del archivo"]
async fn el_flujo_real_genera_una_imagen_con_las_fotos_cc() {
    let url = std::env::var("GALATEA_COMFY_URL").expect("define GALATEA_COMFY_URL");
    let nombre = std::env::var("GALATEA_FLUJO_REAL").unwrap_or_else(|_| "catvton".to_string());
    let flujos = flujos::cargar(Path::new("flujos")).unwrap();
    let flujo = flujos::buscar(&flujos, &nombre)
        .unwrap_or_else(|| panic!("flujos/flujos.toml no declara «{nombre}»"))
        .clone();
    assert!(
        flujos::pesos_faltantes(&flujo, Path::new("datos/modelos")).is_empty(),
        "faltan pesos: corre scripts/descargar-modelos.sh {nombre}"
    );
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("resultado.png");
    let motores = Arc::new(Motores::nuevo(
        Fijo::nuevo(Path::new("flujos/fijo")),
        Some(Comfy::nuevo(
            &url,
            Duration::from_secs(300),
            Some(PathBuf::from("datos/comfy")),
        )),
    ));
    let encuadra = flujo.encuadre.is_some();
    let inicio = Instant::now();
    let mut rx = motores.generar(Trabajo {
        flujo,
        persona: PathBuf::from("pruebas/imagenes/persona-cc.jpg"),
        prenda: PathBuf::from("pruebas/imagenes/prenda-polera-cc.jpg"),
        prenda_id: "polera-cc".to_string(),
        tipo: Tipo::Superior,
        destino: destino.clone(),
        etiqueta: "real".to_string(),
    });
    let mut ultimo = None;
    while let Some(a) = rx.recv().await {
        ultimo = Some(a);
    }
    assert_eq!(ultimo, Some(Avance::Listo));
    let im = image::open(&destino).unwrap();
    if encuadra {
        let entrada = image::open("pruebas/imagenes/persona-cc.jpg").unwrap();
        assert_eq!(
            (im.width(), im.height()),
            (entrada.width(), entrada.height()),
            "con encuadre, el resultado debe volver al tamaño de la foto de entrada"
        );
    } else {
        assert!(im.width() >= 512 && im.height() >= 512);
    }
    eprintln!(
        "{nombre}: {} s, {}x{}",
        inicio.elapsed().as_secs(),
        im.width(),
        im.height()
    );
    std::fs::copy(&destino, "experimento/ultima-cc.png").ok();
}
