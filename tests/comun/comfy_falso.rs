//! Un servidor axum que imita lo justo de la API de ComfyUI para probar `motor::comfy`.
#![allow(dead_code)]
use axum::{
    Json, Router,
    extract::{
        Multipart, Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Qué guion sigue el falso: cómo termina el trabajo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Guion {
    /// status → execution_start → 3 progress → executed → executing(null).
    Exito,
    /// Responde 400 al POST /prompt con node_errors.
    Rechazo,
    /// execution_error por el WebSocket.
    ErrorEnEjecucion,
    /// Nunca termina: para probar el límite de tiempo. Cuenta los POST /interrupt.
    Colgado,
    /// Como `Exito`, pero con 200 mensajes de progreso: más de lo que caben en los canales
    /// de avance, para probar que un trabajo sin lector no traba el turno de la GPU.
    Largo,
    /// Como `Exito`, pero la salida es la foto de la persona que se subió, escalada a
    /// 768×1024 como hace Leffa: para probar el encuadre 3:4 y el recorte de vuelta.
    EcoPersona,
    /// Como `Exito`, pero el archivo de salida no es una imagen.
    Basura,
}

#[derive(Clone)]
pub struct Falso {
    pub guion: Guion,
    /// Nombres de archivo subidos por /upload/image.
    pub subidos: Arc<Mutex<Vec<String>>>,
    /// (nombre, ancho, alto) de cada imagen subida, decodificada por el falso.
    pub dimensiones: Arc<Mutex<Vec<(String, u32, u32)>>>,
    /// El último grafo recibido en /prompt.
    pub grafo: Arc<Mutex<Option<Value>>>,
    pub interrupciones: Arc<Mutex<usize>>,
    /// Los cuerpos recibidos en POST /queue (p. ej. `{"delete": ["pid-1"]}`).
    pub colas: Arc<Mutex<Vec<Value>>>,
    /// Carpeta que hace de `datos/comfy` (input/ y output/): el falso escribe ahí la salida.
    pub datos: std::path::PathBuf,
}

pub const NODO_SALIDA: &str = "4";
pub const ARCHIVO_SALIDA: &str = "galatea_00001_.png";

pub async fn levantar(guion: Guion, datos: &std::path::Path) -> (String, Falso) {
    std::fs::create_dir_all(datos.join("input")).unwrap();
    std::fs::create_dir_all(datos.join("output")).unwrap();
    let falso = Falso {
        guion,
        subidos: Arc::default(),
        dimensiones: Arc::default(),
        grafo: Arc::default(),
        interrupciones: Arc::default(),
        colas: Arc::default(),
        datos: datos.to_path_buf(),
    };
    let router = Router::new()
        .route(
            "/system_stats",
            get(|| async {
                Json(json!({"devices": [{"name": "falsa", "vram_total": 1, "vram_free": 1}]}))
            }),
        )
        .route("/upload/image", post(subir))
        .route("/prompt", post(prompt))
        .route("/ws", get(ws))
        .route("/history/{id}", get(historial))
        .route("/view", get(ver))
        .route("/interrupt", post(interrumpir))
        .route("/queue", post(cola))
        .with_state(falso.clone());
    let escucha = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let puerto = escucha.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(escucha, router).await.ok();
    });
    (format!("http://127.0.0.1:{puerto}"), falso)
}

async fn subir(State(f): State<Falso>, mut partes: Multipart) -> Json<Value> {
    let mut nombre = String::new();
    while let Some(campo) = partes.next_field().await.unwrap() {
        if campo.name() == Some("image") {
            nombre = campo.file_name().unwrap_or("sin-nombre").to_string();
            let bytes = campo.bytes().await.unwrap();
            std::fs::write(f.datos.join("input").join(&nombre), &bytes).unwrap();
            if let Ok(im) = image::load_from_memory(&bytes) {
                f.dimensiones
                    .lock()
                    .unwrap()
                    .push((nombre.clone(), im.width(), im.height()));
            }
        }
    }
    f.subidos.lock().unwrap().push(nombre.clone());
    Json(json!({"name": nombre, "subfolder": "", "type": "input"}))
}

async fn prompt(State(f): State<Falso>, Json(cuerpo): Json<Value>) -> Response {
    *f.grafo.lock().unwrap() = cuerpo.get("prompt").cloned();
    if f.guion == Guion::Rechazo {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "error": {"type": "prompt_outputs_failed_validation", "message": "Prompt outputs failed validation"},
            "node_errors": {"3": {"errors": [{"message": "Value not in list", "details": "cloth_type: 'lower' not in ['upper']"}]}}
        }))).into_response();
    }
    Json(json!({"prompt_id": "pid-1", "number": 1, "node_errors": {}})).into_response()
}

async fn ws(State(f): State<Falso>, mejora: WebSocketUpgrade) -> Response {
    mejora.on_upgrade(move |socket| guion_ws(socket, f))
}

async fn enviar(socket: &mut WebSocket, v: Value) -> Result<(), axum::Error> {
    socket.send(Message::Text(v.to_string().into())).await
}

async fn guion_ws(mut socket: WebSocket, f: Falso) {
    let _ = enviar(
        &mut socket,
        json!({"type": "status", "data": {"status": {"exec_info": {"queue_remaining": 1}}}}),
    )
    .await;
    match f.guion {
        Guion::Colgado => loop {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if enviar(&mut socket, json!({"type": "progress", "data": {"value": 1, "max": 100, "prompt_id": "pid-1", "node": "3"}})).await.is_err() { break; }
        },
        Guion::ErrorEnEjecucion => {
            let _ = enviar(
                &mut socket,
                json!({"type": "execution_start", "data": {"prompt_id": "pid-1"}}),
            )
            .await;
            let _ = enviar(&mut socket, json!({"type": "execution_error", "data": {"prompt_id": "pid-1", "node_id": "3", "node_type": "TryOnFalso", "exception_message": "CUDA out of memory"}})).await;
        }
        _ => {
            let _ = enviar(
                &mut socket,
                json!({"type": "execution_start", "data": {"prompt_id": "pid-1"}}),
            )
            .await;
            let pasos = if f.guion == Guion::Largo { 200 } else { 3 };
            for v in 1..=pasos {
                let _ = enviar(&mut socket, json!({"type": "progress", "data": {"value": v, "max": pasos, "prompt_id": "pid-1", "node": "3"}})).await;
            }
            let salida = f.datos.join("output").join(ARCHIVO_SALIDA);
            if f.guion == Guion::EcoPersona {
                let persona = f
                    .subidos
                    .lock()
                    .unwrap()
                    .iter()
                    .find(|n| n.contains("-persona"))
                    .cloned()
                    .expect("se subió la persona");
                image::open(f.datos.join("input").join(persona))
                    .unwrap()
                    .resize_exact(768, 1024, image::imageops::FilterType::Lanczos3)
                    .save_with_format(&salida, image::ImageFormat::Png)
                    .unwrap();
            } else if f.guion == Guion::Basura {
                std::fs::write(&salida, b"esto no es un PNG").unwrap();
            } else {
                std::fs::write(&salida, include_bytes!("../../pruebas/comfy/salida.png")).unwrap();
            }
            let _ = enviar(&mut socket, json!({"type": "executed", "data": {"prompt_id": "pid-1", "node": NODO_SALIDA, "output": {"images": [{"filename": ARCHIVO_SALIDA, "subfolder": "", "type": "output"}]}}})).await;
            let _ = enviar(
                &mut socket,
                json!({"type": "executing", "data": {"prompt_id": "pid-1", "node": null}}),
            )
            .await;
        }
    }
    // Mantén el socket abierto hasta que el cliente cierre.
    while let Some(Ok(_)) = socket.recv().await {}
}

async fn historial(Path(id): Path<String>) -> Json<Value> {
    Json(
        json!({ id: { "outputs": { NODO_SALIDA: { "images": [{"filename": ARCHIVO_SALIDA, "subfolder": "", "type": "output"}] } }, "status": {"completed": true} } }),
    )
}

async fn ver(State(f): State<Falso>, Query(q): Query<HashMap<String, String>>) -> Response {
    let ruta = f
        .datos
        .join(q.get("type").map(String::as_str).unwrap_or("output"))
        .join(q.get("filename").cloned().unwrap_or_default());
    match std::fs::read(&ruta) {
        Ok(bytes) => ([(header::CONTENT_TYPE, "image/png")], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn interrumpir(State(f): State<Falso>) -> StatusCode {
    *f.interrupciones.lock().unwrap() += 1;
    StatusCode::OK
}

async fn cola(State(f): State<Falso>, Json(cuerpo): Json<Value>) -> StatusCode {
    f.colas.lock().unwrap().push(cuerpo);
    StatusCode::OK
}
