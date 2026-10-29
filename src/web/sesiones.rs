//! Sesiones: subir la foto, generar, seguir el avance, ver resultados y terminar.
use crate::{
    Estado, flujos, marca,
    motor::{Avance, Trabajo},
    prendas, sesion,
    web::error::Fallo,
};
use axum::{
    Json,
    extract::{
        Multipart, Path, State,
        multipart::{MultipartError, MultipartRejection},
        rejection::JsonRejection,
    },
    http::{StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    io::Cursor,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{Mutex, mpsc};

pub const MAXIMO_FOTO: usize = 15 * 1024 * 1024;

pub struct TrabajoActivo {
    pub rx: mpsc::Receiver<Avance>,
    pub resultado: u32,
    pub creado: Instant,
}

pub type Trabajos = Arc<Mutex<HashMap<String, TrabajoActivo>>>;

/// Quita los trabajos cuyo avance nadie leyó en `vencimiento`. Soltar el receptor termina
/// también la tarea de reenvío que esperaba para entregar `Listo` o `Error`.
pub async fn purgar_trabajos(trabajos: &Trabajos, ahora: Instant, vencimiento: Duration) -> usize {
    let mut mapa = trabajos.lock().await;
    let antes = mapa.len();
    mapa.retain(|_, t| ahora.saturating_duration_since(t.creado) < vencimiento);
    antes - mapa.len()
}

/// POST /sesiones: recibe la foto, la endereza según EXIF, la guarda como JPEG.
pub async fn crear(
    State(estado): State<Estado>,
    partes: Result<Multipart, MultipartRejection>,
) -> Response {
    let Ok(mut partes) = partes else {
        return Fallo::invalido(
            "la foto se envía como multipart/form-data en el campo «persona»; súbela desde la página de la demo",
        );
    };
    let mut foto: Option<Vec<u8>> = None;
    loop {
        match partes.next_field().await {
            Ok(Some(campo)) => {
                // Cualquier otro campo se descarta sin leerlo entero en memoria.
                if campo.name() != Some("persona") {
                    continue;
                }
                match campo.bytes().await {
                    Ok(b) if b.len() <= MAXIMO_FOTO => foto = Some(b.to_vec()),
                    Ok(_) => return Fallo::invalido(FOTO_GRANDE),
                    Err(e) => return fallo_de_lectura(&e),
                }
            }
            Ok(None) => break,
            Err(e) => return fallo_de_lectura(&e),
        }
    }
    let Some(foto) = foto else {
        return Fallo::invalido("falta el campo «persona» con la foto");
    };
    let jpeg = match normalizar_jpeg(&foto) {
        Ok(j) => j,
        Err(_) => {
            return Fallo::invalido(
                "el archivo no es una imagen que se pueda leer (usa JPEG o PNG)",
            );
        }
    };
    match estado.sesiones.crear(&jpeg).await {
        Ok(id) => (StatusCode::CREATED, Json(json!({ "id": id }))).into_response(),
        Err(e) => {
            tracing::error!("no se pudo crear la sesión: {e}");
            Fallo::interno("no se pudo guardar la foto en el servidor; revisa el espacio en disco")
        }
    }
}

const FOTO_GRANDE: &str = "la foto pesa más de 15 MB; baja la resolución de la cámara";

/// Un envío que excede el límite de cuerpo llega como error de multipart con estado 413.
fn fallo_de_lectura(e: &MultipartError) -> Response {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        Fallo::invalido(FOTO_GRANDE)
    } else {
        Fallo::invalido(
            "el envío de la foto llegó incompleto o mal formado; vuelve a sacarla y súbela otra vez",
        )
    }
}

/// Decodifica, aplica la orientación EXIF (los celulares la usan) y reencodifica a JPEG 90.
fn normalizar_jpeg(bytes: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    use image::{ImageDecoder, ImageReader, codecs::jpeg::JpegEncoder};
    let lector = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut decodificador = lector.into_decoder()?;
    let orientacion = decodificador.orientation()?;
    let mut imagen = image::DynamicImage::from_decoder(decodificador)?;
    imagen.apply_orientation(orientacion);
    let mut salida = Vec::new();
    JpegEncoder::new_with_quality(&mut salida, 90).encode_image(&imagen.to_rgb8())?;
    Ok(salida)
}

#[derive(Deserialize)]
pub struct PeticionGenerar {
    pub prenda: String,
    pub flujo: Option<String>,
}

/// POST /sesiones/{id}/generar
pub async fn generar(
    State(estado): State<Estado>,
    Path(id): Path<String>,
    peticion: Result<Json<PeticionGenerar>, JsonRejection>,
) -> Response {
    let Ok(Json(p)) = peticion else {
        return Fallo::invalido(
            "el cuerpo debe ser JSON con la forma {\"prenda\": \"<id>\", \"flujo\": \"<nombre>\"} (flujo es opcional)",
        );
    };
    if estado.sesiones.obtener(&id).await.is_none() {
        return Fallo::no_encontrado(
            "esa sesión no existe o ya venció (15 minutos); toma una foto nueva",
        );
    }
    let Some(prenda) = prendas::buscar(&estado.prendas, &p.prenda) else {
        return Fallo::no_encontrado("no hay una prenda con ese id");
    };
    let nombre_flujo = p
        .flujo
        .unwrap_or_else(|| estado.conf.flujo_por_defecto.clone());
    let Some(flujo) = flujos::buscar(&estado.flujos, &nombre_flujo) else {
        return Fallo::no_encontrado(&format!("no hay un flujo llamado «{nombre_flujo}»"));
    };
    if !flujo.admite(prenda.tipo) {
        return Fallo::invalido(format!(
            "el flujo «{}» no admite prendas de tipo {}; elige otro flujo u otra prenda",
            flujo.titulo,
            prenda.tipo.como_texto()
        ));
    }
    let Some((numero, final_)) = estado.sesiones.reservar_resultado(&id).await else {
        return Fallo::no_encontrado("esa sesión venció mientras se preparaba el trabajo");
    };
    // El motor escribe a un nombre que `resultado` nunca sirve; solo el PNG sellado toma el
    // nombre final, y lo toma de golpe (rename).
    let temporal = final_.with_extension("sin-marca.png");
    let trabajo = Trabajo {
        flujo: flujo.clone(),
        persona: estado.sesiones.persona(&id),
        prenda: prendas::ruta(&estado.conf.prendas, prenda),
        prenda_id: prenda.id.clone(),
        tipo: prenda.tipo,
        destino: temporal.clone(),
        etiqueta: sesion::abreviar(&id).to_string(),
    };
    let mut rx_motor = estado.motores.generar(trabajo);
    // Entre el motor y el SSE va la marca de agua: cuando llega Listo, se sella el temporal.
    // Este reenvío vacía siempre el canal del motor: EnCola y Progreso se descartan si nadie
    // lee el SSE (canal lleno o receptor soltado: celular bloqueado, cambio de red), así el
    // motor nunca se traba con el turno de la GPU y `Listo` siempre llega a sellarse: la
    // página puede ir a buscar el resultado aunque se le haya caído el SSE.
    let (tx, rx) = mpsc::channel(32);
    let sesiones = estado.sesiones.clone();
    tokio::spawn(async move {
        while let Some(avance) = rx_motor.recv().await {
            let mut terminal = match avance {
                Avance::EnCola(_) | Avance::Progreso(_) => {
                    let _ = tx.try_send(avance);
                    continue;
                }
                Avance::Listo => match sellar(&temporal, &final_).await {
                    Ok(()) => Avance::Listo,
                    Err(e) => {
                        Avance::Error(format!("la imagen se generó pero no se pudo marcar: {e}"))
                    }
                },
                Avance::Error(m) => {
                    // Un motor que falla a mitad puede dejar el temporal escrito.
                    let _ = tokio::fs::remove_file(&temporal).await;
                    Avance::Error(m)
                }
            };
            // Si la sesión terminó mientras se generaba (DELETE, vencimiento), lo escrito
            // después del borrado no puede quedar en disco: la sesión borra lo que quede.
            if sesiones.obtener(&id).await.is_none() {
                sesiones.borrar_restos(&id).await;
                terminal = Avance::Error(
                    "la sesión terminó mientras se generaba y la imagen se borró; toma una foto nueva"
                        .to_string(),
                );
            }
            let _ = tx.send(terminal).await;
            break;
        }
    });
    let trabajo_id = sesion::Sesiones::generar_id();
    estado.trabajos.lock().await.insert(
        trabajo_id.clone(),
        TrabajoActivo {
            rx,
            resultado: numero,
            creado: Instant::now(),
        },
    );
    (
        StatusCode::ACCEPTED,
        Json(json!({ "trabajo": trabajo_id, "resultado": numero })),
    )
        .into_response()
}

/// Lee el temporal sin marca, escribe el PNG sellado en `final_.tmp` y lo renombra a
/// `final_` (atómico en el mismo sistema de archivos). Pase lo que pase, el temporal se
/// borra: una imagen sin marca nunca queda en disco con un nombre que se sirva.
async fn sellar(temporal: &std::path::Path, final_: &std::path::Path) -> Result<(), String> {
    let intermedio = final_.with_extension("tmp");
    let resultado = async {
        let bytes = tokio::fs::read(temporal).await.map_err(|e| e.to_string())?;
        let sellado = marca::aplicar(&bytes).map_err(|e| e.to_string())?;
        tokio::fs::write(&intermedio, sellado)
            .await
            .map_err(|e| e.to_string())?;
        tokio::fs::rename(&intermedio, final_)
            .await
            .map_err(|e| e.to_string())
    }
    .await;
    let _ = tokio::fs::remove_file(temporal).await;
    if resultado.is_err() {
        let _ = tokio::fs::remove_file(&intermedio).await;
    }
    resultado
}

/// GET /sesiones/{id}/avance/{trabajo}: SSE hasta `listo` o `error`.
pub async fn avance(
    State(estado): State<Estado>,
    Path((id, trabajo)): Path<(String, String)>,
) -> Response {
    if estado.sesiones.obtener(&id).await.is_none() {
        return Fallo::no_encontrado("esa sesión no existe o ya venció");
    }
    let Some(activo) = estado.trabajos.lock().await.remove(&trabajo) else {
        return Fallo::no_encontrado("no hay un trabajo con ese id (¿ya se leyó su avance?)");
    };
    let resultado = activo.resultado;
    let eventos = tokio_stream::wrappers::ReceiverStream::new(activo.rx).map(move |a| {
        Ok::<Event, std::convert::Infallible>(match a {
            Avance::EnCola(n) => Event::default().event("encola").data(n.to_string()),
            Avance::Progreso(f) => Event::default().event("progreso").data(format!("{f:.3}")),
            Avance::Listo => Event::default().event("listo").data(resultado.to_string()),
            Avance::Error(m) => Event::default().event("error").data(m),
        })
    });
    Sse::new(eventos)
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// GET /sesiones/{id}/resultado/{n}.png
pub async fn resultado(
    State(estado): State<Estado>,
    Path((id, archivo)): Path<(String, String)>,
) -> Response {
    let Some(n) = archivo
        .strip_suffix(".png")
        .and_then(|s| s.parse::<u32>().ok())
    else {
        return Fallo::no_encontrado("el resultado se pide como <n>.png");
    };
    let Some(s) = estado.sesiones.obtener(&id).await else {
        return Fallo::no_encontrado("esa sesión no existe o ya venció");
    };
    servir_archivo(s.carpeta.join(format!("resultado-{n}.png")), "image/png").await
}

/// GET /sesiones/{id}/persona.jpg
pub async fn persona(State(estado): State<Estado>, Path(id): Path<String>) -> Response {
    if estado.sesiones.obtener(&id).await.is_none() {
        return Fallo::no_encontrado("esa sesión no existe o ya venció");
    }
    servir_archivo(estado.sesiones.persona(&id), "image/jpeg").await
}

async fn servir_archivo(ruta: std::path::PathBuf, tipo: &'static str) -> Response {
    match tokio::fs::read(&ruta).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, tipo),
                (header::CACHE_CONTROL, "no-store"),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => Fallo::no_encontrado("esa imagen no existe o todavía no se generó"),
    }
}

/// DELETE /sesiones/{id}
pub async fn terminar(State(estado): State<Estado>, Path(id): Path<String>) -> Response {
    if estado.sesiones.terminar(&id).await {
        StatusCode::NO_CONTENT.into_response()
    } else {
        Fallo::no_encontrado("esa sesión no existe o ya se había borrado")
    }
}
