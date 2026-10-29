//! Galatea: probador virtual con IA generativa, demo presencial.
//! El servidor vive aquí y no en `main.rs` para que las pruebas levanten el mismo router.

pub mod encuadre;
pub mod experimento;
pub mod flujos;
pub mod marca;
pub mod motor;
pub mod prendas;
pub mod sesion;
pub mod web;

use anyhow::{Result, anyhow};
use axum::{
    Router,
    extract::{DefaultBodyLimit, Request},
    http::{HeaderValue, Method, header},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{delete, get, post},
};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::signal;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
use tracing::info;

#[derive(Clone, Debug)]
pub struct Conf {
    pub direccion: String,
    pub puerto: u16,
    /// Carpeta de trabajo: sesiones/, miniaturas/, modelos/ si no se indica otra.
    pub datos: PathBuf,
    /// Carpeta con `prendas.toml` y las imágenes de las prendas.
    pub prendas: PathBuf,
    /// Carpeta con `flujos.toml`, los JSON y `fijo/`.
    pub flujos: PathBuf,
    /// Donde `descargar-modelos.sh` deja los pesos (montada en ComfyUI como /comfy/models).
    pub modelos: PathBuf,
    /// `None` = solo funciona el flujo Fijo.
    pub comfy_url: Option<String>,
    /// Carpeta `datos/comfy` compartida con el contenedor (input/, output/), para borrar.
    pub comfy_datos: Option<PathBuf>,
    pub flujo_por_defecto: String,
    pub vencimiento: Duration,
    pub limite: Duration,
    /// Carpeta con el cliente compilado (index.html y assets/). Se sirve como fallback.
    pub web: PathBuf,
    /// Orígenes que pueden llamar a la API desde otro origen (la app Capacitor). Vacío = sin CORS.
    pub origenes: Vec<String>,
}

fn variable(nombre: &str, por_omision: &str) -> String {
    std::env::var(nombre).unwrap_or_else(|_| por_omision.to_string())
}

fn numero(nombre: &str, por_omision: &str) -> Result<u64> {
    variable(nombre, por_omision)
        .parse::<u64>()
        .map_err(|e| anyhow!("{nombre} no es un número entero: {e}"))
}

impl Conf {
    pub fn desde_entorno() -> Result<Self> {
        let puerto = variable("GALATEA_PUERTO", "8766")
            .parse::<u16>()
            .map_err(|e| anyhow!("GALATEA_PUERTO no es un puerto válido: {e}"))?;
        let datos = PathBuf::from(variable("GALATEA_DATOS", "./datos"));
        let comfy_url = std::env::var("GALATEA_COMFY_URL")
            .ok()
            .filter(|u| !u.is_empty());
        Ok(Self {
            direccion: variable("GALATEA_DIRECCION", "127.0.0.1"),
            puerto,
            prendas: PathBuf::from(variable("GALATEA_PRENDAS", "./prendas")),
            flujos: PathBuf::from(variable("GALATEA_FLUJOS", "./flujos")),
            modelos: PathBuf::from(variable("GALATEA_MODELOS", "./datos/modelos")),
            comfy_datos: comfy_url
                .as_ref()
                .map(|_| PathBuf::from(variable("GALATEA_COMFY_DATOS", "./datos/comfy"))),
            comfy_url,
            flujo_por_defecto: variable("GALATEA_FLUJO_POR_DEFECTO", "fijo"),
            vencimiento: Duration::from_secs(60 * numero("GALATEA_VENCIMIENTO_MIN", "15")?),
            limite: Duration::from_secs(numero("GALATEA_LIMITE_SEG", "120")?),
            datos,
            web: PathBuf::from(variable("GALATEA_WEB", "./web/dist")),
            origenes: variable("GALATEA_ORIGENES", "https://localhost")
                .split(',')
                .map(str::trim)
                .filter(|o| !o.is_empty())
                .map(str::to_string)
                .collect(),
        })
    }

    /// Para pruebas: puerto efímero, datos en una carpeta temporal, fixtures de `pruebas/`,
    /// sin ComfyUI (solo el flujo Fijo, con espera cero).
    #[cfg(any(test, feature = "pruebas"))]
    pub fn para_pruebas(raiz: &std::path::Path) -> Self {
        Self {
            direccion: "127.0.0.1".to_string(),
            puerto: 0,
            datos: raiz.join("datos"),
            prendas: PathBuf::from("./pruebas/prendas"),
            flujos: PathBuf::from("./pruebas/flujos"),
            modelos: raiz.join("modelos"),
            comfy_url: None,
            comfy_datos: None,
            flujo_por_defecto: "fijo".to_string(),
            vencimiento: Duration::from_secs(15 * 60),
            limite: Duration::from_secs(5),
            web: PathBuf::from("./pruebas/web"),
            origenes: vec!["https://localhost".to_string()],
        }
    }
}

/// Todo lo que los manejadores necesitan.
#[derive(Clone)]
pub struct Estado {
    pub conf: Conf,
    pub prendas: Arc<Vec<prendas::Prenda>>,
    pub flujos: Arc<Vec<flujos::Flujo>>,
    pub sesiones: sesion::Sesiones,
    pub motores: Arc<motor::Motores>,
    /// Trabajos lanzados cuyo avance todavía no se leyó por SSE.
    pub trabajos: web::sesiones::Trabajos,
}

/// Carga configuración y recursos; falla con un mensaje claro si algo falta.
pub async fn preparar(conf: &Conf) -> Result<Estado> {
    if !conf.web.join("index.html").is_file() {
        return Err(anyhow!(
            "no existe el cliente web en {} (falta index.html); compílalo con scripts/web.sh \
             o indica la carpeta con GALATEA_WEB",
            conf.web.display()
        ));
    }
    tokio::fs::create_dir_all(&conf.datos).await.map_err(|e| {
        anyhow!(
            "no se pudo crear la carpeta de datos {}: {e}",
            conf.datos.display()
        )
    })?;
    let lista = prendas::cargar(&conf.prendas)?;
    let miniaturas = conf.datos.join("miniaturas");
    prendas::generar_miniaturas(&conf.prendas, &lista, &miniaturas)?;
    info!(
        "{} prendas cargadas de {}",
        lista.len(),
        conf.prendas.display()
    );
    let lista_flujos = flujos::cargar(&conf.flujos)?;
    if flujos::buscar(&lista_flujos, &conf.flujo_por_defecto).is_none() {
        return Err(anyhow!(
            "GALATEA_FLUJO_POR_DEFECTO={} no está en {}/flujos.toml",
            conf.flujo_por_defecto,
            conf.flujos.display()
        ));
    }
    for f in &lista_flujos {
        let faltan = flujos::pesos_faltantes(f, &conf.modelos);
        if faltan.is_empty() {
            info!("flujo «{}» listo", f.nombre);
        } else {
            tracing::warn!(
                "flujo «{}» sin pesos: {}; córrelo con scripts/descargar-modelos.sh {}",
                f.nombre,
                faltan.join(", "),
                f.nombre
            );
        }
    }
    let sesiones = sesion::Sesiones::nueva(&conf.datos.join("sesiones"), conf.vencimiento)?;
    // Lo que una ejecución anterior dejó en la carpeta de ComfyUI ya venció por definición.
    if let Some(datos) = &conf.comfy_datos {
        let n = motor::comfy::barrer_comfy(datos, SystemTime::now(), Duration::ZERO).await;
        if n > 0 {
            info!("{n} restos de trabajos anteriores borrados de la carpeta de ComfyUI");
        }
    }
    let comfy = conf
        .comfy_url
        .as_ref()
        .map(|url| motor::comfy::Comfy::nuevo(url, conf.limite, conf.comfy_datos.clone()));
    let motores = Arc::new(motor::Motores::nuevo(
        motor::fijo::Fijo::nuevo(&conf.flujos.join("fijo")),
        comfy,
    ));
    match motores.disponible().await {
        Ok(()) => info!("ComfyUI responde"),
        Err(e) => tracing::warn!("{e}"),
    }
    Ok(Estado {
        conf: conf.clone(),
        prendas: Arc::new(lista),
        flujos: Arc::new(lista_flujos),
        sesiones,
        motores,
        trabajos: Arc::default(),
    })
}

/// Una línea por petición: método, ruta, estado, duración. Las rutas llevan
/// identificadores de sesión: se recortan a 6 caracteres antes de escribir.
async fn registro(peticion: Request, siguiente: Next) -> impl IntoResponse {
    let metodo = peticion.method().clone();
    let ruta = recortar_ids(peticion.uri().path());
    let inicio = Instant::now();
    let respuesta = siguiente.run(peticion).await;
    info!(
        "{} {} {} {}ms",
        metodo,
        ruta,
        respuesta.status().as_u16(),
        inicio.elapsed().as_millis()
    );
    respuesta
}

/// `/sesiones/0123456789abcdef.../resultado/1.png` → `/sesiones/012345…/resultado/1.png`.
fn recortar_ids(ruta: &str) -> String {
    sesion::redactar(ruta)
}

pub fn construir_router(estado: Estado) -> Router {
    let router = Router::new()
        .route("/salud", get(web::salud::ver))
        .route("/prendas", get(web::prendas::listar))
        .route("/prendas/{id}/miniatura.png", get(web::prendas::miniatura))
        .route("/flujos", get(web::flujos::listar))
        // Solo la subida de la foto admite cuerpos grandes; el resto queda con el límite por omisión.
        .route(
            "/sesiones",
            post(web::sesiones::crear).layer(DefaultBodyLimit::max(
                web::sesiones::MAXIMO_FOTO + 64 * 1024,
            )),
        )
        .route("/sesiones/{id}/generar", post(web::sesiones::generar))
        .route(
            "/sesiones/{id}/avance/{trabajo}",
            get(web::sesiones::avance),
        )
        .route(
            "/sesiones/{id}/resultado/{archivo}",
            get(web::sesiones::resultado),
        )
        .route("/sesiones/{id}/persona.jpg", get(web::sesiones::persona))
        .route("/sesiones/{id}", delete(web::sesiones::terminar))
        // Todo lo que no es API es el cliente: archivos de `web`, e index.html para las
        // rutas de la SPA (/ajustes, etc.).
        .fallback_service(
            ServeDir::new(&estado.conf.web)
                .fallback(ServeFile::new(estado.conf.web.join("index.html"))),
        )
        .layer(middleware::from_fn(registro));
    let router = router.with_state(estado.clone());
    match cors_para(&estado.conf.origenes) {
        Some(cors) => router.layer(cors),
        None => router,
    }
}

/// CORS solo para los orígenes configurados: la app Capacitor vive en https://localhost.
/// Las sesiones van por id en la URL, sin cookies, así que no se permiten credenciales.
fn cors_para(origenes: &[String]) -> Option<CorsLayer> {
    if origenes.is_empty() {
        return None;
    }
    let lista: Vec<HeaderValue> = origenes
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();
    Some(
        CorsLayer::new()
            .allow_origin(lista)
            .allow_methods([Method::GET, Method::HEAD, Method::POST, Method::DELETE])
            .allow_headers([header::CONTENT_TYPE])
            .max_age(Duration::from_secs(3600)),
    )
}

async fn señal_de_apagado() {
    let interrupcion = async {
        signal::ctrl_c()
            .await
            .expect("no se pudo escuchar la interrupción")
    };
    let terminacion = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("no se pudo escuchar SIGTERM")
            .recv()
            .await;
    };
    tokio::select! { _ = interrupcion => {}, _ = terminacion => {} }
    info!("señal de apagado recibida, cerrando");
}

pub async fn servir(conf: Conf) -> Result<()> {
    let estado = preparar(&conf).await?;
    let socket: SocketAddr = format!("{}:{}", conf.direccion, conf.puerto).parse()?;
    let escucha = tokio::net::TcpListener::bind(socket).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::AddrInUse {
            anyhow!(
                "el puerto {} ya está ocupado; elige otro con GALATEA_PUERTO=<puerto>",
                conf.puerto
            )
        } else {
            anyhow!("no se pudo escuchar en {socket}: {e}")
        }
    })?;
    info!("galatea escuchando en http://{}", escucha.local_addr()?);
    let vencedor = estado.sesiones.clone();
    let trabajos = estado.trabajos.clone();
    let vencimiento = conf.vencimiento;
    let comfy_datos = conf.comfy_datos.clone();
    tokio::spawn(async move {
        let mut cada = tokio::time::interval(Duration::from_secs(60));
        loop {
            cada.tick().await;
            let n = vencedor.vencer_antiguas(Instant::now()).await;
            if n > 0 {
                info!("{n} sesiones vencidas y borradas");
            }
            let n = web::sesiones::purgar_trabajos(&trabajos, Instant::now(), vencimiento).await;
            if n > 0 {
                info!("{n} trabajos sin leer descartados");
            }
            if let Some(datos) = &comfy_datos {
                let n = motor::comfy::barrer_comfy(datos, SystemTime::now(), vencimiento).await;
                if n > 0 {
                    info!("{n} restos vencidos borrados de la carpeta de ComfyUI");
                }
            }
        }
    });
    axum::serve(escucha, construir_router(estado))
        .with_graceful_shutdown(señal_de_apagado())
        .await?;
    info!("servidor apagado");
    Ok(())
}

#[cfg(test)]
mod pruebas {
    use super::recortar_ids;

    #[test]
    fn recorta_identificadores_hexadecimales_largos() {
        let id = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            recortar_ids(&format!("/sesiones/{id}/resultado/1.png")),
            "/sesiones/012345…/resultado/1.png"
        );
        assert_eq!(recortar_ids("/prendas"), "/prendas");
    }
}
