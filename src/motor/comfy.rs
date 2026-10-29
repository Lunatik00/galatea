//! El único módulo que habla con ComfyUI: sube las imágenes, encola el flujo, sigue el
//! progreso por WebSocket, descarga la salida y borra lo que dejó en la carpeta compartida.
use super::{Avance, Trabajo};
use crate::{
    encuadre::{self, Marco},
    flujos::{self, Encuadre, Entradas},
};
use futures_util::StreamExt;
use serde_json::Value;
use std::{
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone, Debug)]
pub struct Comfy {
    base: String,
    cliente: reqwest::Client,
    limite: Duration,
    /// `datos/comfy` en el host, si está montada: para borrar input/ y output/.
    datos: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct Salida {
    filename: String,
    subfolder: String,
    tipo: String,
}

const COMO_LEVANTARLO: &str = "El motor de imágenes no responde. Levántalo con «docker compose up -d comfyui» o cambia al flujo Fijo.";
const MIRA_EL_LOG: &str = "mira qué pasó con «docker compose logs comfyui» y prueba de nuevo";

impl Comfy {
    pub fn nuevo(base: &str, limite: Duration, datos: Option<PathBuf>) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            cliente: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("cliente HTTP"),
            limite,
            datos,
        }
    }

    pub async fn disponible(&self) -> Result<(), String> {
        let r = self
            .cliente
            .get(format!("{}/system_stats", self.base))
            // Si el host no contesta, que el arranque no quede colgado un minuto.
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|_| COMO_LEVANTARLO.to_string())?;
        if r.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "ComfyUI respondió {} a /system_stats; {MIRA_EL_LOG}",
                r.status()
            ))
        }
    }

    pub async fn generar(
        &self,
        trabajo: &Trabajo,
        tx: &mpsc::Sender<Avance>,
    ) -> Result<(), String> {
        let nombre_persona = format!("galatea-{}-persona.jpg", trabajo.etiqueta);
        let nombre_prenda = format!(
            "galatea-prenda-{}.{}",
            trabajo.prenda_id,
            extension(&trabajo.prenda)
        );
        let resultado = async {
            // Si el flujo pide `encuadre`, la foto sube encuadrada a 3:4 (ver `encuadre`) directo
            // desde memoria: el lienzo rellenado solo existe en la carpeta input/ de ComfyUI,
            // que se limpia abajo. Si no, sube tal cual.
            const QUE: &str = "la foto de la persona";
            let (subida_persona, marco) = match trabajo.flujo.encuadre {
                Some(Encuadre::TresPorCuatro) => {
                    let (persona, marco) = encuadrar_persona(&trabajo.persona).await?;
                    let subida = self.subir_bytes(persona, &nombre_persona, QUE).await?;
                    (subida, Some(marco))
                }
                None => (
                    self.subir(&trabajo.persona, &nombre_persona, QUE).await?,
                    None,
                ),
            };
            // Si ComfyUI renombró la subida, esa copia también se borra.
            let _borrar_renombrada = Borrar {
                comfy: self,
                carpeta: "input",
                nombre: (subida_persona != nombre_persona).then_some(subida_persona.as_str()),
            };
            let subida_prenda = self
                .subir(&trabajo.prenda, &nombre_prenda, "la imagen de la prenda")
                .await?;
            self.correr(trabajo, &subida_persona, &subida_prenda, marco, tx)
                .await
        }
        .await;
        // Pase lo que pase (también si falló la subida de la prenda), la foto de la
        // persona no se queda en ComfyUI. Las prendas son del catálogo: se sobrescriben.
        self.borrar_en_datos("input", "", &nombre_persona);
        resultado
    }

    async fn correr(
        &self,
        trabajo: &Trabajo,
        persona: &str,
        prenda: &str,
        marco: Option<Marco>,
        tx: &mpsc::Sender<Avance>,
    ) -> Result<(), String> {
        let mut grafo = flujos::rellenar(
            &trabajo.flujo,
            &Entradas {
                persona,
                prenda,
                tipo: trabajo.tipo,
            },
        )
        .map_err(|e| e.to_string())?;
        let nodo_salida = trabajo.flujo.nodo_salida.clone();
        // ComfyUI cachea los nodos cuyas entradas no cambiaron: repetir la misma foto con
        // la misma prenda devolvería el archivo de la vez anterior, que ya borramos. Un
        // prefijo único obliga al nodo de salida a guardar un archivo nuevo (lo de arriba
        // sigue cacheado, así que no cuesta GPU).
        if let Some(n) = nodo_salida.as_deref()
            && let Some(entradas) = grafo[n]["inputs"].as_object_mut()
            && entradas.contains_key("filename_prefix")
        {
            let unico = &crate::sesion::Sesiones::generar_id()[..8];
            entradas.insert(
                "filename_prefix".to_string(),
                Value::from(format!("galatea-{}-{unico}", trabajo.etiqueta)),
            );
        }
        let client_id = crate::sesion::Sesiones::generar_id();
        let ws_url = format!(
            "{}/ws?clientId={client_id}",
            self.base.replacen("http", "ws", 1)
        );
        // El socket se abre ANTES de encolar: si no, los primeros mensajes se pierden.
        // Con límite propio: un host que acepta el TCP y no contesta no debe colgar el turno.
        let (ws, _) = tokio::time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async(&ws_url),
        )
        .await
        .map_err(|_| COMO_LEVANTARLO.to_string())?
        .map_err(|_| COMO_LEVANTARLO.to_string())?;
        let prompt_id = self.encolar(grafo, &client_id).await?;
        let seguimiento = self.seguir(ws, &prompt_id, nodo_salida.as_deref(), tx);
        // Al vencer el límite, el future se suelta y con él el socket.
        let salida = match tokio::time::timeout(self.limite, seguimiento).await {
            Ok(r) => r?,
            Err(_) => {
                self.interrumpir(&prompt_id).await;
                return Err(format!(
                    "la generación pasó de {} segundos y se interrumpió; prueba de nuevo o elige un flujo más liviano",
                    self.limite.as_secs_f64()
                ));
            }
        };
        let descarga = self.descargar(&salida).await;
        // La salida también es la foto de alguien: se borra se haya podido bajar o no.
        self.borrar_en_datos(&salida.tipo, &salida.subfolder, &salida.filename);
        let bytes = desencuadrar_salida(descarga?, marco).await?;
        // Sin la ruta en los mensajes: lleva el id de la sesión. La carpeta de `destino` no se
        // crea aquí: es de la sesión (o del experimento); si ya se borró, esto falla.
        tokio::fs::write(&trabajo.destino, &bytes).await.map_err(|e| {
            format!("no se pudo guardar la imagen generada ({e}); revisa el espacio y los permisos de datos/")
        })
    }

    async fn subir(&self, ruta: &Path, nombre: &str, que: &str) -> Result<String, String> {
        // Sin la ruta en el mensaje: lleva el id de la sesión y el nombre de la foto.
        let bytes = tokio::fs::read(ruta).await.map_err(|e| {
            format!("no se pudo leer {que} para mandarla a ComfyUI ({e}); vuelve a sacar la foto o elige otra prenda")
        })?;
        self.subir_bytes(bytes, nombre, que).await
    }

    /// Sube `bytes` a input/ con el nombre `nombre` y devuelve el nombre con que ComfyUI
    /// lo guardó (puede renombrarlo).
    async fn subir_bytes(&self, bytes: Vec<u8>, nombre: &str, que: &str) -> Result<String, String> {
        let parte = reqwest::multipart::Part::bytes(bytes)
            .file_name(nombre.to_string())
            .mime_str(mime_de(nombre))
            .map_err(|e| format!("no se pudo preparar la subida de {que}: {e}"))?;
        let formulario = reqwest::multipart::Form::new()
            .part("image", parte)
            .text("overwrite", "true")
            .text("type", "input");
        let r = self
            .cliente
            .post(format!("{}/upload/image", self.base))
            .multipart(formulario)
            .send()
            .await
            .map_err(|_| COMO_LEVANTARLO.to_string())?;
        if !r.status().is_success() {
            return Err(format!(
                "ComfyUI rechazó la subida de {que} ({}); {MIRA_EL_LOG}",
                r.status()
            ));
        }
        let cuerpo: Value = r.json().await.map_err(|e| {
            format!("ComfyUI respondió algo inesperado al subir {que} ({e}); {MIRA_EL_LOG}")
        })?;
        Ok(cuerpo["name"].as_str().unwrap_or(nombre).to_string())
    }

    async fn encolar(&self, grafo: Value, client_id: &str) -> Result<String, String> {
        let r = self
            .cliente
            .post(format!("{}/prompt", self.base))
            .json(&serde_json::json!({ "prompt": grafo, "client_id": client_id }))
            .send()
            .await
            .map_err(|_| COMO_LEVANTARLO.to_string())?;
        let estado = r.status();
        let cuerpo: Value = r.json().await.map_err(|e| {
            format!("ComfyUI respondió algo inesperado al encolar el flujo ({e}); {MIRA_EL_LOG}")
        })?;
        if !estado.is_success() {
            return Err(explicar_rechazo(&cuerpo));
        }
        cuerpo["prompt_id"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| {
                format!("ComfyUI aceptó el flujo pero no devolvió prompt_id; {MIRA_EL_LOG}")
            })
    }

    async fn seguir(
        &self,
        mut ws: tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        prompt_id: &str,
        nodo_salida: Option<&str>,
        tx: &mpsc::Sender<Avance>,
    ) -> Result<Salida, String> {
        let mut salida: Option<Salida> = None;
        let mut empezado = false;
        while let Some(mensaje) = ws.next().await {
            let texto = match mensaje {
                Ok(Message::Text(t)) => t,
                // Los binarios son vistas previas: no se usan.
                Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => continue,
                Ok(Message::Close(_)) => break,
                Ok(Message::Frame(_)) => continue,
                Err(e) => {
                    return Err(format!(
                        "se cortó la conexión con ComfyUI a mitad del trabajo ({e}); {MIRA_EL_LOG}"
                    ));
                }
            };
            let Ok(v) = serde_json::from_str::<Value>(texto.as_str()) else {
                continue;
            };
            let tipo = v["type"].as_str().unwrap_or("");
            let datos = &v["data"];
            let es_nuestro = datos["prompt_id"].as_str() == Some(prompt_id);
            match tipo {
                "status" if !empezado => {
                    let restantes = datos["status"]["exec_info"]["queue_remaining"]
                        .as_u64()
                        .unwrap_or(0) as usize;
                    let _ = tx.send(Avance::EnCola(restantes.saturating_sub(1))).await;
                }
                "execution_start" if es_nuestro => {
                    empezado = true;
                    let _ = tx.send(Avance::Progreso(0.0)).await;
                }
                "progress" if es_nuestro => {
                    let (valor, maximo) = (
                        datos["value"].as_f64().unwrap_or(0.0),
                        datos["max"].as_f64().unwrap_or(1.0).max(1.0),
                    );
                    let _ = tx.send(Avance::Progreso((valor / maximo) as f32)).await;
                }
                "executed" if es_nuestro => {
                    let coincide = nodo_salida.is_none_or(|n| datos["node"].as_str() == Some(n));
                    if coincide && let Some(s) = primera_imagen(&datos["output"]) {
                        salida = Some(s);
                    }
                }
                "executing" if es_nuestro && datos["node"].is_null() => {
                    return match salida.take() {
                        Some(s) => Ok(s),
                        None => self.salida_del_historial(prompt_id, nodo_salida).await,
                    };
                }
                "execution_error" if es_nuestro => {
                    return Err(format!(
                        "ComfyUI falló en el nodo {} ({}): {}; prueba de nuevo y, si se repite, reinicia el motor con «docker compose restart comfyui»",
                        datos["node_id"].as_str().unwrap_or("?"),
                        datos["node_type"].as_str().unwrap_or("?"),
                        datos["exception_message"]
                            .as_str()
                            .unwrap_or("sin detalle")
                            .trim()
                    ));
                }
                "execution_interrupted" if es_nuestro => {
                    return Err(
                        "alguien interrumpió la generación en ComfyUI; prueba de nuevo".to_string(),
                    );
                }
                _ => {}
            }
        }
        Err(format!(
            "ComfyUI cerró la conexión antes de terminar; {MIRA_EL_LOG}"
        ))
    }

    async fn salida_del_historial(
        &self,
        prompt_id: &str,
        nodo_salida: Option<&str>,
    ) -> Result<Salida, String> {
        let r = self
            .cliente
            .get(format!("{}/history/{prompt_id}", self.base))
            .send()
            .await
            .map_err(|_| COMO_LEVANTARLO.to_string())?;
        let h: Value = r.json().await.map_err(|e| {
            format!("ComfyUI respondió algo inesperado al pedir el historial ({e}); {MIRA_EL_LOG}")
        })?;
        let salidas = &h[prompt_id]["outputs"];
        let candidatos: Vec<&Value> = match nodo_salida {
            Some(n) => vec![&salidas[n]],
            None => salidas
                .as_object()
                .map(|m| m.values().collect())
                .unwrap_or_default(),
        };
        candidatos
            .into_iter()
            .find_map(primera_imagen)
            .ok_or_else(|| {
                "el flujo terminó sin producir ninguna imagen; revisa nodo_salida en flujos.toml"
                    .to_string()
            })
    }

    async fn descargar(&self, salida: &Salida) -> Result<Vec<u8>, String> {
        let r = self
            .cliente
            .get(format!("{}/view", self.base))
            .query(&[
                ("filename", salida.filename.as_str()),
                ("subfolder", salida.subfolder.as_str()),
                ("type", salida.tipo.as_str()),
            ])
            .send()
            .await
            .map_err(|_| COMO_LEVANTARLO.to_string())?;
        if !r.status().is_success() {
            return Err(format!(
                "ComfyUI respondió {} al pedir la imagen generada; {MIRA_EL_LOG}",
                r.status()
            ));
        }
        r.bytes().await.map(|b| b.to_vec()).map_err(|e| {
            format!("se cortó la descarga de la imagen generada ({e}); prueba de nuevo")
        })
    }

    /// Quita el prompt de la cola (si todavía no empezó, `/interrupt` no lo alcanza) y
    /// después interrumpe el que esté corriendo.
    async fn interrumpir(&self, prompt_id: &str) {
        if let Err(e) = self
            .cliente
            .post(format!("{}/queue", self.base))
            .json(&serde_json::json!({ "delete": [prompt_id] }))
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            tracing::warn!("no se pudo pedir a ComfyUI que quite el trabajo de la cola: {e}");
        }
        if let Err(e) = self
            .cliente
            .post(format!("{}/interrupt", self.base))
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            tracing::warn!("no se pudo pedir a ComfyUI que interrumpa: {e}");
        }
    }

    /// Borra un archivo concreto de `datos/comfy/<carpeta>/<subcarpeta>/`: nunca vacía
    /// carpetas, porque en `input/` viven también archivos que no son nuestros.
    fn borrar_en_datos(&self, carpeta: &str, subcarpeta: &str, nombre: &str) {
        let Some(datos) = &self.datos else { return };
        // Los nombres vienen de ComfyUI: nada de `..` ni rutas absolutas.
        let seguro = |p: &str| {
            Path::new(p)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
        };
        if !matches!(carpeta, "input" | "output")
            || !seguro(subcarpeta)
            || nombre.is_empty()
            || !seguro(nombre)
        {
            tracing::warn!(
                "ComfyUI devolvió una ruta de archivo rara en {carpeta}; no se borra nada"
            );
            return;
        }
        let ruta = datos.join(carpeta).join(subcarpeta).join(nombre);
        if ruta.is_file()
            && let Err(e) = std::fs::remove_file(&ruta)
        {
            tracing::warn!("no se pudo borrar un archivo en la carpeta {carpeta} de ComfyUI: {e}");
        }
    }
}

/// Barre de `datos/comfy` lo que un trabajo pudo dejar atrás (una salida escrita después de
/// un límite de tiempo, la foto subida por un proceso que murió a mitad): borra los archivos
/// `output/galatea-*` y las fotos `input/galatea-*-persona*` cuya fecha de modificación tiene
/// `vencimiento` o más respecto de `ahora`. Nunca toca las prendas (`galatea-prenda-*`), ni
/// carpetas, ni archivos ajenos. Con `vencimiento` cero borra todo lo nuestro (al arrancar).
/// Devuelve cuántos archivos borró.
pub async fn barrer_comfy(datos: &Path, ahora: SystemTime, vencimiento: Duration) -> usize {
    let mut borrados = 0;
    for carpeta in ["output", "input"] {
        let Ok(mut entradas) = tokio::fs::read_dir(datos.join(carpeta)).await else {
            continue;
        };
        while let Ok(Some(entrada)) = entradas.next_entry().await {
            let nombre = entrada.file_name();
            let Some(nombre) = nombre.to_str() else {
                continue;
            };
            if !es_resto_nuestro(carpeta, nombre) {
                continue;
            }
            let Ok(meta) = entrada.metadata().await else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            // Una fecha en el futuro cuenta como edad cero.
            let edad = meta
                .modified()
                .ok()
                .and_then(|m| ahora.duration_since(m).ok())
                .unwrap_or(Duration::ZERO);
            if edad < vencimiento {
                continue;
            }
            match tokio::fs::remove_file(entrada.path()).await {
                Ok(()) => borrados += 1,
                Err(e) => tracing::warn!(
                    "no se pudo borrar un resto en la carpeta {carpeta} de ComfyUI: {e}"
                ),
            }
        }
    }
    borrados
}

/// `output/galatea-*` (salidas de un trabajo) o `input/galatea-*-persona*` (fotos subidas).
fn es_resto_nuestro(carpeta: &str, nombre: &str) -> bool {
    if !nombre.starts_with("galatea-") {
        return false;
    }
    match carpeta {
        "output" => true,
        "input" => {
            !nombre.starts_with("galatea-prenda-")
                && Path::new(nombre)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.contains("-persona"))
        }
        _ => false,
    }
}

/// Lee la foto de la persona (con su orientación EXIF, por si viene de `galatea experimento`
/// sin pasar por la sesión) y la encuadra a 3:4. Si ya era 3:4, sube los bytes tal cual.
async fn encuadrar_persona(ruta: &Path) -> Result<(Vec<u8>, Marco), String> {
    // Sin la ruta en los mensajes: lleva el id de la sesión y el nombre de la foto.
    let bytes = tokio::fs::read(ruta).await.map_err(|e| {
        format!("no se pudo leer la foto de la persona para mandarla a ComfyUI ({e}); vuelve a sacar la foto")
    })?;
    tokio::task::spawn_blocking(move || {
        use image::{ImageDecoder, ImageReader};
        let no_es_imagen = |e: image::ImageError| {
            format!(
                "la foto de la persona no se pudo leer como imagen ({e}); vuelve a sacar la foto"
            )
        };
        let mut decodificador = ImageReader::new(std::io::Cursor::new(&bytes))
            .with_guessed_format()
            .map_err(|e| no_es_imagen(e.into()))?
            .into_decoder()
            .map_err(no_es_imagen)?;
        let orientacion = decodificador.orientation().map_err(no_es_imagen)?;
        let mut foto = image::DynamicImage::from_decoder(decodificador).map_err(no_es_imagen)?;
        foto.apply_orientation(orientacion);
        let (lienzo, marco) = encuadre::encuadrar(&foto);
        if marco.es_completo() && orientacion == image::metadata::Orientation::NoTransforms {
            return Ok((bytes, marco));
        }
        let jpeg = encuadre::a_jpeg(&lienzo).map_err(|e| {
            format!("no se pudo preparar la foto de la persona ({e}); prueba de nuevo")
        })?;
        Ok((jpeg, marco))
    })
    .await
    .map_err(|e| {
        format!("se cayó la preparación de la foto de la persona ({e}); prueba de nuevo")
    })?
}

/// Comprueba que la salida de ComfyUI sea una imagen y, si el flujo pidió `encuadre` (`marco`),
/// le quita el relleno y la devuelve al tamaño de la foto original, en PNG. Una salida que no
/// es una imagen es un error del trabajo, con o sin encuadre. Si es una imagen pero no se
/// puede recortar (p. ej. el flujo no respetó la proporción 3:4), avisa en el log y la deja
/// como vino: mejor una imagen con franjas que ninguna.
async fn desencuadrar_salida(bytes: Vec<u8>, marco: Option<Marco>) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || {
        let imagen = image::load_from_memory(&bytes).map_err(|e| {
            format!("ComfyUI devolvió un archivo que no es una imagen ({e}); prueba de nuevo y, si se repite, {MIRA_EL_LOG}")
        })?;
        let Some(marco) = marco else {
            return Ok(bytes);
        };
        let (ancho, alto) = (imagen.width(), imagen.height());
        if marco.es_completo() && (ancho, alto) == (marco.ancho, marco.alto) {
            return Ok(bytes);
        }
        let Some(recorte) = encuadre::desencuadrar(&imagen, &marco) else {
            tracing::warn!(
                "no se pudo recortar la salida de ComfyUI ({ancho}×{alto}) al marco del lienzo enviado ({}×{}); se sirve como vino. Revisa que el flujo devuelva la proporción de la entrada",
                marco.ancho_lienzo,
                marco.alto_lienzo
            );
            return Ok(bytes);
        };
        Ok(match encuadre::a_png(&recorte) {
            Ok(png) => png,
            Err(e) => {
                tracing::warn!("no se pudo codificar la salida recortada ({e}); se sirve como vino");
                bytes
            }
        })
    })
    .await
    .map_err(|e| format!("se cayó el recorte de la imagen generada ({e}); prueba de nuevo"))?
}

/// Borra un archivo subido al soltarse, pase lo que pase con el trabajo.
struct Borrar<'a> {
    comfy: &'a Comfy,
    carpeta: &'static str,
    nombre: Option<&'a str>,
}

impl Drop for Borrar<'_> {
    fn drop(&mut self) {
        if let Some(n) = self.nombre {
            self.comfy.borrar_en_datos(self.carpeta, "", n);
        }
    }
}

fn primera_imagen(salida: &Value) -> Option<Salida> {
    let im = salida["images"].as_array()?.first()?;
    Some(Salida {
        filename: im["filename"].as_str()?.to_string(),
        subfolder: im["subfolder"].as_str().unwrap_or("").to_string(),
        tipo: im["type"].as_str().unwrap_or("output").to_string(),
    })
}

fn explicar_rechazo(cuerpo: &Value) -> String {
    let mut partes = vec![format!(
        "ComfyUI rechazó el flujo: {}",
        cuerpo["error"]["message"].as_str().unwrap_or("sin detalle")
    )];
    if let Some(nodos) = cuerpo["node_errors"].as_object() {
        for (nodo, e) in nodos {
            for err in e["errors"].as_array().into_iter().flatten() {
                partes.push(format!(
                    "nodo {nodo}: {} ({})",
                    err["message"].as_str().unwrap_or(""),
                    err["details"].as_str().unwrap_or("")
                ));
            }
        }
    }
    partes.push("revisa el flujo y los identificadores de nodo en flujos.toml".to_string());
    partes.join("; ")
}

fn extension(ruta: &Path) -> String {
    ruta.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase()
}

fn mime_de(nombre: &str) -> &'static str {
    if nombre.ends_with(".jpg") || nombre.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "image/png"
    }
}
