mod comun;

use futures_util::StreamExt;
use serde_json::{Value, json};

async fn subir_persona(s: &comun::Servidor) -> String {
    let bytes = std::fs::read("pruebas/prendas/azul.png").unwrap();
    let parte = reqwest::multipart::Part::bytes(bytes)
        .file_name("foto.png")
        .mime_str("image/png")
        .unwrap();
    let r = s
        .cliente
        .post(s.url("/sesiones"))
        .multipart(reqwest::multipart::Form::new().part("persona", parte))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    r.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Lee el SSE hasta `listo` o `error` y devuelve la lista de (evento, dato).
async fn leer_sse(s: &comun::Servidor, ruta: &str) -> Vec<(String, String)> {
    let r = s.cliente.get(s.url(ruta)).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(
        r.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );
    let mut flujo = r.bytes_stream();
    let mut texto = String::new();
    let mut eventos = Vec::new();
    while let Some(trozo) = flujo.next().await {
        texto.push_str(std::str::from_utf8(&trozo.unwrap()).unwrap());
        while let Some(fin) = texto.find("\n\n") {
            let bloque: String = texto.drain(..fin + 2).collect();
            let evento = bloque
                .lines()
                .find_map(|l| l.strip_prefix("event: "))
                .unwrap_or("message")
                .to_string();
            let dato = bloque
                .lines()
                .find_map(|l| l.strip_prefix("data: "))
                .unwrap_or("")
                .to_string();
            let terminal = evento == "listo" || evento == "error";
            eventos.push((evento, dato));
            if terminal {
                return eventos;
            }
        }
    }
    eventos
}

#[tokio::test]
async fn lista_prendas_y_sirve_miniaturas() {
    let s = comun::Servidor::nuevo().await;
    let prendas: Vec<Value> = s
        .cliente
        .get(s.url("/prendas"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(prendas.len(), 2);
    assert_eq!(prendas[0]["id"], "roja");
    assert_eq!(prendas[0]["tipo"], "superior");
    let r = s
        .cliente
        .get(s.url(prendas[0]["miniatura"].as_str().unwrap()))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["content-type"], "image/png");
    assert_eq!(
        s.cliente
            .get(s.url("/prendas/nada/miniatura.png"))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
}

#[tokio::test]
async fn recorrido_completo_con_el_flujo_fijo() {
    let s = comun::Servidor::nuevo().await;
    let id = subir_persona(&s).await;

    let r = s
        .cliente
        .post(s.url(&format!("/sesiones/{id}/generar")))
        .json(&json!({"prenda": "roja"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 202);
    let cuerpo: Value = r.json().await.unwrap();
    let trabajo = cuerpo["trabajo"].as_str().unwrap().to_string();
    assert_eq!(cuerpo["resultado"], 1);

    let eventos = leer_sse(&s, &format!("/sesiones/{id}/avance/{trabajo}")).await;
    assert_eq!(eventos.first().unwrap().0, "encola");
    assert!(eventos.iter().any(|(e, _)| e == "progreso"));
    assert_eq!(
        eventos.last().unwrap(),
        &("listo".to_string(), "1".to_string())
    );

    let r = s
        .cliente
        .get(s.url(&format!("/sesiones/{id}/resultado/1.png")))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let png = r.bytes().await.unwrap();
    let im = image::load_from_memory(&png).unwrap();
    assert_eq!(
        im.width(),
        300,
        "la fija de «roja» es la de pruebas/flujos/fijo/roja.png"
    );
    // la marca de agua cambió la esquina respecto del archivo fijo original
    let original = image::open("pruebas/flujos/fijo/roja.png").unwrap();
    // el sello mide 135 px de ancho y termina a 6 px del borde: (250, 388) está dentro
    assert_ne!(
        image::GenericImageView::get_pixel(&im, 250, 388),
        image::GenericImageView::get_pixel(&original, 250, 388)
    );

    assert_eq!(
        s.cliente
            .get(s.url(&format!("/sesiones/{id}/persona.jpg")))
            .send()
            .await
            .unwrap()
            .status(),
        200
    );

    // segundo trabajo con la misma foto reutiliza la sesión y numera 2
    let r = s
        .cliente
        .post(s.url(&format!("/sesiones/{id}/generar")))
        .json(&json!({"prenda": "azul", "flujo": "fijo"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.json::<Value>().await.unwrap()["resultado"], 2);

    assert_eq!(
        s.cliente
            .delete(s.url(&format!("/sesiones/{id}")))
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    assert_eq!(
        s.cliente
            .get(s.url(&format!("/sesiones/{id}/resultado/1.png")))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        s.cliente
            .get(s.url(&format!("/sesiones/{id}/persona.jpg")))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        s.cliente
            .delete(s.url(&format!("/sesiones/{id}")))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
}

#[tokio::test]
async fn rechaza_lo_que_no_es_imagen_y_lo_que_no_existe() {
    let s = comun::Servidor::nuevo().await;
    let parte = reqwest::multipart::Part::bytes(b"hola".to_vec())
        .file_name("x.jpg")
        .mime_str("image/jpeg")
        .unwrap();
    let r = s
        .cliente
        .post(s.url("/sesiones"))
        .multipart(reqwest::multipart::Form::new().part("persona", parte))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
    assert!(
        r.json::<Value>().await.unwrap()["error"]
            .as_str()
            .unwrap()
            .contains("imagen")
    );

    let id = subir_persona(&s).await;
    let r = s
        .cliente
        .post(s.url(&format!("/sesiones/{id}/generar")))
        .json(&json!({"prenda": "verde"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
    let r = s
        .cliente
        .post(s.url(&format!("/sesiones/{id}/generar")))
        .json(&json!({"prenda": "roja", "flujo": "inventado"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
    let r = s
        .cliente
        .post(s.url("/sesiones/00000000000000000000000000000000/generar"))
        .json(&json!({"prenda": "roja"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
    assert_eq!(
        s.cliente
            .get(s.url(&format!("/sesiones/{id}/avance/nada")))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
}

#[tokio::test]
async fn un_flujo_que_no_admite_el_tipo_de_la_prenda_da_422() {
    // «prueba» admite los tres tipos; lo restringimos con un TOML propio.
    let temporal = tempfile::tempdir().unwrap();
    let flujos = temporal.path().join("flujos");
    std::fs::create_dir_all(flujos.join("fijo")).unwrap();
    std::fs::copy("pruebas/flujos/prueba.json", flujos.join("prueba.json")).unwrap();
    std::fs::copy(
        "pruebas/flujos/fijo/generico.png",
        flujos.join("fijo/generico.png"),
    )
    .unwrap();
    std::fs::write(
        flujos.join("flujos.toml"),
        r#"
[[flujo]]
nombre = "prueba"
titulo = "Solo superior"
archivo = "prueba.json"
nodo_persona = "1"
nodo_prenda = "2"
nodo_salida = "4"
tipos = ["superior"]
"#,
    )
    .unwrap();
    let mut conf = galatea::Conf::para_pruebas(temporal.path());
    conf.flujos = flujos;
    let s = comun::Servidor::con_conf(conf, temporal).await;
    let id = subir_persona(&s).await;
    let r = s
        .cliente
        .post(s.url(&format!("/sesiones/{id}/generar")))
        .json(&json!({"prenda": "azul", "flujo": "prueba"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
    assert!(
        r.json::<Value>().await.unwrap()["error"]
            .as_str()
            .unwrap()
            .contains("vestido")
    );
}

#[tokio::test]
async fn flujos_y_salud_informan_pesos_faltantes() {
    let s = comun::Servidor::nuevo().await;
    let flujos: Vec<Value> = s
        .cliente
        .get(s.url("/flujos"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let prueba = flujos.iter().find(|f| f["nombre"] == "prueba").unwrap();
    assert_eq!(prueba["disponible"], false);
    assert_eq!(prueba["faltantes"][0], "prueba/peso.safetensors");
    let fijo = flujos.iter().find(|f| f["nombre"] == "fijo").unwrap();
    assert_eq!(fijo["disponible"], true);

    let salud: Value = s
        .cliente
        .get(s.url("/salud"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        salud["comfy"]
            .as_str()
            .unwrap()
            .contains("GALATEA_COMFY_URL")
    );
    assert_eq!(salud["sesiones"], 0);
}

/// JPEG 40×20 con un segmento EXIF mínimo (APP1) que dice «girar 90° a la derecha» (orientación 6),
/// como los que sacan los celulares apaisados.
fn jpeg_girado_por_exif() -> Vec<u8> {
    let imagen = image::RgbImage::from_pixel(40, 20, image::Rgb([200, 30, 30]));
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 90)
        .encode_image(&imagen)
        .unwrap();
    let tiff: [u8; 26] = [
        b'M', b'M', 0, 42, 0, 0, 0, 8, // cabecera TIFF big-endian, IFD en 8
        0, 1, // una entrada
        0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 6, 0, 0, // Orientation, SHORT, 1 valor: 6
        0, 0, 0, 0, // sin IFD siguiente
    ];
    let mut app1 = vec![0xFF, 0xE1];
    let largo = (2 + 6 + tiff.len()) as u16;
    app1.extend_from_slice(&largo.to_be_bytes());
    app1.extend_from_slice(b"Exif\0\0");
    app1.extend_from_slice(&tiff);
    let mut salida = jpeg[..2].to_vec(); // SOI
    salida.extend_from_slice(&app1);
    salida.extend_from_slice(&jpeg[2..]);
    salida
}

#[tokio::test]
async fn una_foto_girada_por_exif_se_guarda_derecha_y_otros_campos_se_ignoran() {
    let s = comun::Servidor::nuevo().await;
    let foto = jpeg_girado_por_exif();
    let leida = image::load_from_memory(&foto).unwrap();
    assert_eq!(
        (leida.width(), leida.height()),
        (40, 20),
        "sin aplicar EXIF la foto es apaisada"
    );
    let formulario = reqwest::multipart::Form::new()
        .text("otro", "no es la foto")
        .part(
            "persona",
            reqwest::multipart::Part::bytes(foto)
                .file_name("foto.jpg")
                .mime_str("image/jpeg")
                .unwrap(),
        );
    let r = s
        .cliente
        .post(s.url("/sesiones"))
        .multipart(formulario)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);
    let id = r.json::<Value>().await.unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let r = s
        .cliente
        .get(s.url(&format!("/sesiones/{id}/persona.jpg")))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["content-type"], "image/jpeg");
    assert_eq!(r.headers()["cache-control"], "no-store");
    let guardada = r.bytes().await.unwrap();
    assert_eq!(
        image::guess_format(&guardada).unwrap(),
        image::ImageFormat::Jpeg
    );
    let guardada = image::load_from_memory(&guardada).unwrap();
    assert_eq!(
        (guardada.width(), guardada.height()),
        (20, 40),
        "la orientación EXIF se aplicó"
    );
}

#[tokio::test]
async fn una_foto_de_mas_de_15_mb_se_rechaza_diciendo_el_limite() {
    let s = comun::Servidor::nuevo().await;
    // Una foto válida de más de 2 MB (el límite por omisión de axum) entra: el límite propio
    // de POST /sesiones está puesto.
    let mut semilla: u32 = 12345;
    let ruido = image::RgbImage::from_fn(1100, 1100, |_, _| {
        semilla = semilla.wrapping_mul(1_103_515_245).wrapping_add(12345);
        let b = semilla.to_be_bytes();
        image::Rgb([b[0], b[1], b[2]])
    });
    let mut png = Vec::new();
    image::DynamicImage::ImageRgb8(ruido)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    assert!(
        png.len() > 3 * 1024 * 1024,
        "la foto de prueba pesa {} bytes",
        png.len()
    );
    let parte = reqwest::multipart::Part::bytes(png)
        .file_name("grande.png")
        .mime_str("image/png")
        .unwrap();
    let r = s
        .cliente
        .post(s.url("/sesiones"))
        .multipart(reqwest::multipart::Form::new().part("persona", parte))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 201);

    let parte = reqwest::multipart::Part::bytes(vec![0u8; 17 * 1024 * 1024])
        .file_name("x.jpg")
        .mime_str("image/jpeg")
        .unwrap();
    let r = s
        .cliente
        .post(s.url("/sesiones"))
        .multipart(reqwest::multipart::Form::new().part("persona", parte))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
    assert!(
        r.json::<Value>().await.unwrap()["error"]
            .as_str()
            .unwrap()
            .contains("15 MB")
    );
}

#[tokio::test]
async fn un_cuerpo_mal_formado_responde_con_error_json_en_espanol() {
    let s = comun::Servidor::nuevo().await;
    let r = s
        .cliente
        .post(s.url("/sesiones"))
        .body("no es multipart")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
    assert!(
        r.json::<Value>().await.unwrap()["error"]
            .as_str()
            .unwrap()
            .contains("persona")
    );

    let id = subir_persona(&s).await;
    let r = s
        .cliente
        .post(s.url(&format!("/sesiones/{id}/generar")))
        .header("content-type", "application/json")
        .body("{roto")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
    assert!(
        r.json::<Value>().await.unwrap()["error"]
            .as_str()
            .unwrap()
            .contains("JSON")
    );

    // La generación no hereda el límite de 15 MB: el límite por omisión de axum (2 MB) sigue.
    let r = s
        .cliente
        .post(s.url(&format!("/sesiones/{id}/generar")))
        .header("content-type", "application/json")
        // JSON válido (el campo extra se ignora): solo el límite de tamaño puede rechazarlo.
        .body(format!(
            r#"{{"prenda": "roja", "relleno": "{}"}}"#,
            "a".repeat(3 * 1024 * 1024)
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 422);
}

/// Servidor con un `flujos/` propio: solo el Fijo, con `espera` segundos y las imágenes
/// fijas dadas como (nombre, bytes).
async fn servidor_con_fijo(espera: u64, imagenes: &[(&str, Vec<u8>)]) -> comun::Servidor {
    let temporal = tempfile::tempdir().unwrap();
    let flujos = temporal.path().join("flujos");
    std::fs::create_dir_all(flujos.join("fijo")).unwrap();
    for (nombre, bytes) in imagenes {
        std::fs::write(flujos.join("fijo").join(nombre), bytes).unwrap();
    }
    std::fs::write(
        flujos.join("flujos.toml"),
        format!(
            "[[flujo]]\nnombre = \"fijo\"\ntitulo = \"Fijo lento\"\nespera_segundos = {espera}\n"
        ),
    )
    .unwrap();
    let mut conf = galatea::Conf::para_pruebas(temporal.path());
    conf.flujos = flujos;
    comun::Servidor::con_conf(conf, temporal).await
}

/// Nombres de archivo de la carpeta de la sesión.
fn archivos_de_sesion(s: &comun::Servidor, id: &str) -> Vec<String> {
    let mut nombres: Vec<String> = std::fs::read_dir(s.conf.datos.join("sesiones").join(id))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    nombres.sort();
    nombres
}

async fn generar(s: &comun::Servidor, id: &str, prenda: &str) -> String {
    let r = s
        .cliente
        .post(s.url(&format!("/sesiones/{id}/generar")))
        .json(&json!({"prenda": prenda}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 202);
    r.json::<Value>().await.unwrap()["trabajo"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn tras_listo_solo_queda_el_resultado_sellado_en_disco() {
    let s = comun::Servidor::nuevo().await;
    let id = subir_persona(&s).await;
    let trabajo = generar(&s, &id, "roja").await;
    let eventos = leer_sse(&s, &format!("/sesiones/{id}/avance/{trabajo}")).await;
    assert_eq!(eventos.last().unwrap().0, "listo");
    assert_eq!(
        archivos_de_sesion(&s, &id),
        vec!["persona.jpg", "resultado-1.png"]
    );
}

#[tokio::test]
async fn mientras_se_genera_el_resultado_no_se_sirve_y_al_terminar_sale_sellado() {
    let roja = std::fs::read("pruebas/flujos/fijo/roja.png").unwrap();
    let s = servidor_con_fijo(1, &[("roja.png", roja)]).await;
    let id = subir_persona(&s).await;
    let trabajo = generar(&s, &id, "roja").await;
    let url = s.url(&format!("/sesiones/{id}/resultado/1.png"));
    assert_eq!(s.cliente.get(&url).send().await.unwrap().status(), 404);

    let eventos = leer_sse(&s, &format!("/sesiones/{id}/avance/{trabajo}")).await;
    assert_eq!(eventos.last().unwrap().0, "listo");
    let r = s.cliente.get(&url).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let im = image::load_from_memory(&r.bytes().await.unwrap()).unwrap();
    let original = image::open("pruebas/flujos/fijo/roja.png").unwrap();
    assert_ne!(
        image::GenericImageView::get_pixel(&im, 250, 388),
        image::GenericImageView::get_pixel(&original, 250, 388)
    );
}

#[tokio::test]
async fn si_no_se_puede_sellar_no_queda_nada_que_servir() {
    // La «imagen» fija no es un PNG: el motor la copia, pero la marca falla.
    let s = servidor_con_fijo(0, &[("generico.png", b"no es una imagen".to_vec())]).await;
    let id = subir_persona(&s).await;
    let trabajo = generar(&s, &id, "roja").await;
    let eventos = leer_sse(&s, &format!("/sesiones/{id}/avance/{trabajo}")).await;
    let (evento, dato) = eventos.last().unwrap();
    assert_eq!(evento, "error");
    assert!(dato.contains("marcar"), "{dato}");
    assert_eq!(
        s.cliente
            .get(s.url(&format!("/sesiones/{id}/resultado/1.png")))
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(archivos_de_sesion(&s, &id), vec!["persona.jpg"]);
}

#[tokio::test]
async fn un_trabajo_cuyo_avance_nadie_lee_no_traba_a_los_siguientes() {
    // El primero va a un ComfyUI falso que manda 200 progresos (más de lo que caben en los
    // dos canales de 32) y nadie abre su SSE. El segundo, con el Fijo, necesita el mismo turno.
    let temporal = tempfile::tempdir().unwrap();
    let datos_comfy = temporal.path().join("comfy");
    let (url, _falso) =
        comun::comfy_falso::levantar(comun::comfy_falso::Guion::Largo, &datos_comfy).await;
    let mut conf = galatea::Conf::para_pruebas(temporal.path());
    conf.comfy_url = Some(url);
    conf.comfy_datos = Some(datos_comfy);
    let s = comun::Servidor::con_conf(conf, temporal).await;

    let primera = subir_persona(&s).await;
    let r = s
        .cliente
        .post(s.url(&format!("/sesiones/{primera}/generar")))
        .json(&json!({"prenda": "roja", "flujo": "prueba"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 202);
    let segunda = subir_persona(&s).await;
    let trabajo = generar(&s, &segunda, "azul").await;
    let eventos = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        leer_sse(&s, &format!("/sesiones/{segunda}/avance/{trabajo}")),
    )
    .await
    .expect("el segundo trabajo quedó esperando el turno que retiene el primero");
    assert_eq!(eventos.last().unwrap().0, "listo");
    // y el primero terminó y quedó sellado aunque nadie leyó su avance
    let r = s
        .cliente
        .get(s.url(&format!("/sesiones/{primera}/resultado/1.png")))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
}

#[tokio::test]
async fn purgar_trabajos_quita_los_viejos_y_deja_los_recientes() {
    use galatea::web::sesiones::{TrabajoActivo, Trabajos, purgar_trabajos};
    use std::time::{Duration, Instant};
    let trabajos: Trabajos = Default::default();
    let ahora = Instant::now();
    let vencimiento = Duration::from_secs(15 * 60);
    for (id, edad) in [
        ("viejo", Duration::from_secs(16 * 60)),
        ("reciente", Duration::from_secs(60)),
    ] {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        trabajos.lock().await.insert(
            id.to_string(),
            TrabajoActivo {
                rx,
                resultado: 1,
                creado: ahora - edad,
            },
        );
    }
    assert_eq!(purgar_trabajos(&trabajos, ahora, vencimiento).await, 1);
    let mapa = trabajos.lock().await;
    assert!(mapa.contains_key("reciente"));
    assert!(!mapa.contains_key("viejo"));
}

#[tokio::test]
async fn borrar_la_sesion_a_mitad_de_generar_no_deja_su_carpeta() {
    // El motor tarda 1 s; el DELETE llega antes. Antes, el motor recreaba la carpeta y el
    // resultado sellado quedaba huérfano en disco hasta el reinicio.
    let roja = std::fs::read("pruebas/flujos/fijo/roja.png").unwrap();
    let s = servidor_con_fijo(1, &[("roja.png", roja)]).await;
    let id = subir_persona(&s).await;
    let _trabajo = generar(&s, &id, "roja").await;
    let r = s
        .cliente
        .delete(s.url(&format!("/sesiones/{id}")))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 204);
    // El avance ya no se puede pedir por HTTP (la sesión no existe): se espera a que el
    // motor termine y a que el reenvío haga lo suyo.
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    let carpeta = s.conf.datos.join("sesiones").join(&id);
    assert!(
        !carpeta.exists(),
        "quedó la carpeta de la sesión borrada con: {:?}",
        std::fs::read_dir(&carpeta)
            .map(|d| d.map(|e| e.unwrap().file_name()).collect::<Vec<_>>())
            .ok()
    );
}

#[tokio::test]
async fn si_se_cae_el_sse_antes_de_listo_el_resultado_igual_se_sella_y_se_sirve() {
    let roja = std::fs::read("pruebas/flujos/fijo/roja.png").unwrap();
    let s = servidor_con_fijo(1, &[("roja.png", roja)]).await;
    let id = subir_persona(&s).await;
    let trabajo = generar(&s, &id, "roja").await;
    // Abre el SSE, lee el primer evento y suelta la conexión (celular bloqueado).
    {
        let r = s
            .cliente
            .get(s.url(&format!("/sesiones/{id}/avance/{trabajo}")))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200);
        let mut flujo = r.bytes_stream();
        let primero = flujo.next().await.unwrap().unwrap();
        assert!(std::str::from_utf8(&primero).unwrap().contains("event:"));
    }
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    let r = s
        .cliente
        .get(s.url(&format!("/sesiones/{id}/resultado/1.png")))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(
        archivos_de_sesion(&s, &id),
        vec!["persona.jpg", "resultado-1.png"]
    );
}

#[tokio::test]
async fn al_arrancar_se_barren_los_restos_de_comfy() {
    let temporal = tempfile::tempdir().unwrap();
    let datos_comfy = temporal.path().join("comfy");
    std::fs::create_dir_all(datos_comfy.join("input")).unwrap();
    std::fs::create_dir_all(datos_comfy.join("output")).unwrap();
    let salida = datos_comfy
        .join("output")
        .join("galatea-abc123-1a2b3c4d_00001_.png");
    let persona = datos_comfy.join("input").join("galatea-abc123-persona.jpg");
    let prenda = datos_comfy.join("input").join("galatea-prenda-roja.png");
    for r in [&salida, &persona, &prenda] {
        std::fs::write(r, b"x").unwrap();
    }
    let mut conf = galatea::Conf::para_pruebas(temporal.path());
    conf.comfy_url = Some("http://127.0.0.1:1".to_string());
    conf.comfy_datos = Some(datos_comfy);
    let _s = comun::Servidor::con_conf(conf, temporal).await;
    assert!(!salida.exists());
    assert!(!persona.exists());
    assert!(prenda.exists());
}
