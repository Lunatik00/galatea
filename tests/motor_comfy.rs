mod comun;

use comun::comfy_falso::{self, Guion};
use galatea::{
    flujos,
    motor::{Avance, Motores, Trabajo, comfy::Comfy, fijo::Fijo},
    prendas::Tipo,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

fn trabajo(flujos: &[flujos::Flujo], destino: PathBuf) -> Trabajo {
    Trabajo {
        flujo: flujos::buscar(flujos, "prueba").unwrap().clone(),
        persona: PathBuf::from("pruebas/prendas/azul.png"), // cualquier imagen sirve de «foto»
        prenda: PathBuf::from("pruebas/prendas/roja.png"),
        prenda_id: "roja".to_string(),
        tipo: Tipo::Inferior,
        destino,
        etiqueta: "abcdef".to_string(),
    }
}

async fn recoger(mut rx: tokio::sync::mpsc::Receiver<Avance>) -> Vec<Avance> {
    let mut v = Vec::new();
    while let Some(a) = rx.recv().await {
        v.push(a);
    }
    v
}

fn motores(url: &str, limite: Duration, datos: &Path) -> Arc<Motores> {
    Arc::new(Motores::nuevo(
        Fijo::nuevo(Path::new("pruebas/flujos/fijo")),
        Some(Comfy::nuevo(url, limite, Some(datos.to_path_buf()))),
    ))
}

#[tokio::test]
async fn sube_rellena_sigue_el_progreso_descarga_y_limpia() {
    let dir = tempfile::tempdir().unwrap();
    let datos = dir.path().join("comfy");
    let (url, falso) = comfy_falso::levantar(Guion::Exito, &datos).await;
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let destino = dir.path().join("resultado-1.png");
    let avances = recoger(
        motores(&url, Duration::from_secs(5), &datos).generar(trabajo(&flujos, destino.clone())),
    )
    .await;

    assert_eq!(avances.last(), Some(&Avance::Listo), "{avances:?}");
    assert!(avances.contains(&Avance::Progreso(1.0)));
    assert_eq!(
        std::fs::read(&destino).unwrap(),
        include_bytes!("../pruebas/comfy/salida.png")
    );

    let subidos = falso.subidos.lock().unwrap().clone();
    assert_eq!(subidos.len(), 2);
    assert!(
        subidos
            .iter()
            .any(|n| n.starts_with("galatea-abcdef-persona")),
        "{subidos:?}"
    );
    assert!(
        subidos.iter().any(|n| n == "galatea-prenda-roja.png"),
        "{subidos:?}"
    );

    let grafo = falso.grafo.lock().unwrap().clone().unwrap();
    assert_eq!(grafo["1"]["inputs"]["image"], subidos[0]);
    assert_eq!(grafo["2"]["inputs"]["image"], "galatea-prenda-roja.png");
    assert_eq!(grafo["3"]["inputs"]["cloth_type"], "lower");
    // prefijo único en el nodo de salida: ComfyUI no puede devolver una salida cacheada ya borrada
    let prefijo = grafo["4"]["inputs"]["filename_prefix"].as_str().unwrap();
    assert!(
        prefijo.starts_with("galatea-abcdef-") && prefijo.len() > "galatea-abcdef-".len(),
        "{prefijo}"
    );

    // limpieza: ni la entrada ni la salida quedan en la carpeta compartida
    assert!(
        !datos
            .join("output")
            .join(comfy_falso::ARCHIVO_SALIDA)
            .exists()
    );
    assert!(!datos.join("input").join(&subidos[0]).exists());
}

#[tokio::test]
async fn un_prompt_rechazado_explica_el_nodo() {
    let dir = tempfile::tempdir().unwrap();
    let datos = dir.path().join("comfy");
    let (url, _falso) = comfy_falso::levantar(Guion::Rechazo, &datos).await;
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let avances = recoger(
        motores(&url, Duration::from_secs(5), &datos)
            .generar(trabajo(&flujos, dir.path().join("r.png"))),
    )
    .await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("cloth_type") && m.contains("3"), "{m}"),
        otro => panic!("{otro:?}"),
    }
}

#[tokio::test]
async fn un_error_de_ejecucion_llega_con_su_mensaje() {
    let dir = tempfile::tempdir().unwrap();
    let datos = dir.path().join("comfy");
    let (url, _falso) = comfy_falso::levantar(Guion::ErrorEnEjecucion, &datos).await;
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let avances = recoger(
        motores(&url, Duration::from_secs(5), &datos)
            .generar(trabajo(&flujos, dir.path().join("r.png"))),
    )
    .await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("CUDA out of memory"), "{m}"),
        otro => panic!("{otro:?}"),
    }
}

#[tokio::test]
async fn si_se_pasa_del_limite_interrumpe_y_avisa() {
    let dir = tempfile::tempdir().unwrap();
    let datos = dir.path().join("comfy");
    let (url, falso) = comfy_falso::levantar(Guion::Colgado, &datos).await;
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let avances = recoger(
        motores(&url, Duration::from_millis(700), &datos)
            .generar(trabajo(&flujos, dir.path().join("r.png"))),
    )
    .await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("segundos") || m.contains("tiempo"), "{m}"),
        otro => panic!("{otro:?}"),
    }
    assert_eq!(*falso.interrupciones.lock().unwrap(), 1);
    // Antes de interrumpir, el prompt se quita de la cola por si todavía no empezó.
    assert_eq!(
        *falso.colas.lock().unwrap(),
        vec![serde_json::json!({"delete": ["pid-1"]})]
    );
}

#[tokio::test]
async fn comfy_apagado_da_un_error_que_dice_como_levantarlo() {
    let dir = tempfile::tempdir().unwrap();
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let m = motores("http://127.0.0.1:1", Duration::from_secs(2), dir.path());
    assert!(
        m.disponible()
            .await
            .unwrap_err()
            .contains("docker compose up")
    );
    let avances = recoger(m.generar(trabajo(&flujos, dir.path().join("r.png")))).await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("docker compose up"), "{m}"),
        otro => panic!("{otro:?}"),
    }
}

#[tokio::test]
async fn la_foto_se_borra_aunque_falle_la_subida_de_la_prenda_y_lo_ajeno_se_queda() {
    let dir = tempfile::tempdir().unwrap();
    let datos = dir.path().join("comfy");
    let (url, falso) = comfy_falso::levantar(Guion::Exito, &datos).await;
    // En producción input/ tiene imágenes de ejemplo y la carpeta 3d/: no se tocan.
    std::fs::write(datos.join("input").join("ejemplo-cc.png"), b"ajeno").unwrap();
    std::fs::create_dir_all(datos.join("input").join("3d")).unwrap();
    std::fs::write(datos.join("output").join("otro_00001_.png"), b"ajeno").unwrap();
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let mut t = trabajo(&flujos, dir.path().join("r.png"));
    t.prenda = PathBuf::from("pruebas/prendas/no-existe.png");
    let avances = recoger(motores(&url, Duration::from_secs(5), &datos).generar(t)).await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("prenda") && !m.contains("no-existe"), "{m}"),
        otro => panic!("{otro:?}"),
    }
    let subidos = falso.subidos.lock().unwrap().clone();
    assert_eq!(subidos, vec!["galatea-abcdef-persona.jpg".to_string()]);
    assert!(!datos.join("input").join(&subidos[0]).exists());
    assert!(datos.join("input").join("ejemplo-cc.png").exists());
    assert!(datos.join("input").join("3d").is_dir());
    assert!(datos.join("output").join("otro_00001_.png").exists());
}

#[tokio::test]
async fn barrer_comfy_borra_solo_los_restos_nuestros_vencidos() {
    use galatea::motor::comfy::barrer_comfy;
    use std::time::SystemTime;
    let dir = tempfile::tempdir().unwrap();
    let datos = dir.path().join("comfy");
    let (input, output) = (datos.join("input"), datos.join("output"));
    std::fs::create_dir_all(input.join("3d")).unwrap();
    std::fs::create_dir_all(output.join("galatea-carpeta")).unwrap();
    let ahora = SystemTime::now();
    let hace = |minutos: u64| ahora - Duration::from_secs(minutos * 60);
    // (ruta, edad en minutos)
    let archivos = [
        (output.join("galatea-abc123-1a2b3c4d_00001_.png"), 20), // vieja: se borra
        (output.join("galatea-def456-5e6f7a8b_00001_.png"), 1),  // fresca: se queda
        (output.join("otro_00001_.png"), 60),                    // ajena: se queda
        (input.join("galatea-abc123-persona.jpg"), 20),          // vieja: se borra
        (input.join("galatea-def456-persona.jpg"), 1),           // fresca: se queda
        (input.join("galatea-prenda-roja.png"), 60),             // prenda: se queda
        (input.join("ejemplo-cc.png"), 60),                      // ajena: se queda
    ];
    for (ruta, minutos) in &archivos {
        std::fs::write(ruta, b"x").unwrap();
        std::fs::File::options()
            .write(true)
            .open(ruta)
            .unwrap()
            .set_modified(hace(*minutos))
            .unwrap();
    }
    let n = barrer_comfy(&datos, ahora, Duration::from_secs(15 * 60)).await;
    assert_eq!(n, 2);
    for (ruta, minutos) in &archivos {
        let nuestro_vencido = *minutos == 20;
        assert_eq!(ruta.exists(), !nuestro_vencido, "{}", ruta.display());
    }
    assert!(input.join("3d").is_dir());
    assert!(output.join("galatea-carpeta").is_dir());

    // Con vencimiento cero (al arrancar) se va todo lo nuestro, menos la prenda.
    let n = barrer_comfy(&datos, ahora, Duration::ZERO).await;
    assert_eq!(n, 2);
    assert!(!output.join("galatea-def456-5e6f7a8b_00001_.png").exists());
    assert!(!input.join("galatea-def456-persona.jpg").exists());
    assert!(input.join("galatea-prenda-roja.png").exists());
    assert!(output.join("otro_00001_.png").exists());
    assert!(input.join("ejemplo-cc.png").exists());
    assert!(output.join("galatea-carpeta").is_dir());
}

#[tokio::test]
async fn la_persona_sube_encuadrada_a_3x4_y_el_resultado_vuelve_a_su_tamano() {
    use image::{DynamicImage, Rgb, RgbImage};
    let dir = tempfile::tempdir().unwrap();
    let datos = dir.path().join("comfy");
    let (url, falso) = comfy_falso::levantar(Guion::EcoPersona, &datos).await;
    // Una foto de celular 9:16, con un color por mitad: izquierda roja, derecha verde.
    let (rojo, verde) = (Rgb([200u8, 30, 30]), Rgb([30u8, 160, 30]));
    let foto = dir.path().join("persona.jpg");
    DynamicImage::ImageRgb8(RgbImage::from_fn(450, 800, |x, _| {
        if x < 225 { rojo } else { verde }
    }))
    .save_with_format(&foto, image::ImageFormat::Jpeg)
    .unwrap();
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let destino = dir.path().join("resultado-1.sin-marca.png");
    let mut t = trabajo(&flujos, destino.clone());
    t.persona = foto;
    let avances = recoger(motores(&url, Duration::from_secs(5), &datos).generar(t)).await;
    assert_eq!(avances.last(), Some(&Avance::Listo), "{avances:?}");

    let dimensiones = falso.dimensiones.lock().unwrap().clone();
    let persona = dimensiones
        .iter()
        .find(|(n, _, _)| n == "galatea-abcdef-persona.jpg")
        .unwrap_or_else(|| panic!("{dimensiones:?}"));
    assert_eq!((persona.1, persona.2), (600, 800), "la persona sube a 3:4");

    let final_ = image::open(&destino).unwrap().to_rgb8();
    assert_eq!(
        final_.dimensions(),
        (450, 800),
        "el resultado vuelve al tamaño de la foto"
    );
    // Sin las franjas blancas del relleno: los bordes son la foto.
    let cerca = |a: &Rgb<u8>, b: Rgb<u8>| a.0.iter().zip(b.0).all(|(x, y)| x.abs_diff(y) <= 16);
    assert!(
        cerca(final_.get_pixel(2, 400), rojo),
        "{:?}",
        final_.get_pixel(2, 400)
    );
    assert!(
        cerca(final_.get_pixel(447, 400), verde),
        "{:?}",
        final_.get_pixel(447, 400)
    );
    // Y la foto encuadrada no se queda en la carpeta de ComfyUI.
    assert!(
        !datos
            .join("input")
            .join("galatea-abcdef-persona.jpg")
            .exists()
    );
}

#[tokio::test]
async fn una_salida_que_no_es_imagen_termina_en_error_y_no_se_guarda() {
    // Con encuadre (leffa) y sin él (klein, catvton): nunca se da por buena.
    for encuadre in [Some(flujos::Encuadre::TresPorCuatro), None] {
        let dir = tempfile::tempdir().unwrap();
        let datos = dir.path().join("comfy");
        let (url, _falso) = comfy_falso::levantar(Guion::Basura, &datos).await;
        let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
        let destino = dir.path().join("resultado-1.sin-marca.png");
        let mut t = trabajo(&flujos, destino.clone());
        t.flujo.encuadre = encuadre;
        let avances = recoger(motores(&url, Duration::from_secs(5), &datos).generar(t)).await;
        match avances.last() {
            Some(Avance::Error(m)) => assert!(m.contains("no es una imagen"), "{encuadre:?}: {m}"),
            otro => panic!("{encuadre:?}: {otro:?}"),
        }
        assert!(
            !destino.exists(),
            "{encuadre:?}: no se guarda un resultado ilegible"
        );
        // La salida de ComfyUI se borra igual.
        assert!(
            !datos
                .join("output")
                .join(comfy_falso::ARCHIVO_SALIDA)
                .exists()
        );
    }
}

#[tokio::test]
async fn sin_encuadre_la_persona_sube_tal_cual_y_el_resultado_no_se_recorta() {
    use image::{DynamicImage, Rgb, RgbImage};
    let dir = tempfile::tempdir().unwrap();
    let datos = dir.path().join("comfy");
    let (url, falso) = comfy_falso::levantar(Guion::EcoPersona, &datos).await;
    let foto = dir.path().join("persona.jpg");
    DynamicImage::ImageRgb8(RgbImage::from_pixel(450, 800, Rgb([200, 30, 30])))
        .save_with_format(&foto, image::ImageFormat::Jpeg)
        .unwrap();
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let destino = dir.path().join("resultado-1.sin-marca.png");
    let mut t = trabajo(&flujos, destino.clone());
    t.flujo.encuadre = None; // como klein o catvton
    t.persona = foto.clone();
    let avances = recoger(motores(&url, Duration::from_secs(5), &datos).generar(t)).await;
    assert_eq!(avances.last(), Some(&Avance::Listo), "{avances:?}");

    let dimensiones = falso.dimensiones.lock().unwrap().clone();
    let persona = dimensiones
        .iter()
        .find(|(n, _, _)| n == "galatea-abcdef-persona.jpg")
        .unwrap_or_else(|| panic!("{dimensiones:?}"));
    assert_eq!(
        (persona.1, persona.2),
        (450, 800),
        "la persona sube sin rellenar"
    );
    // El falso devuelve 768×1024: sin encuadre, así se sirve.
    let final_ = image::open(&destino).unwrap();
    assert_eq!(
        (final_.width(), final_.height()),
        (768, 1024),
        "sin recortar"
    );
}
