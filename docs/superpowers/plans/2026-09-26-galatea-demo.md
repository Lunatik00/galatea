# Galatea · Plan de implementación de la demo

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Una demo presencial del probador virtual: desde un celular en la red de la casa, una persona le saca una foto a otra, elige una prenda de una lista precargada y ve la imagen generada por ComfyUI en la GPU local, con la foto borrada al terminar.

**Architecture:** Servidor Rust (axum) en el host que sirve una página de una sola vista, maneja sesiones en memoria con archivos temporales y habla con ComfyUI por HTTP y WebSocket; ComfyUI corre en un contenedor con la GPU por CDI y cada modelo es un flujo JSON declarado en `flujos/flujos.toml`; Caddy en contenedor (red del host) da HTTPS con CA interna para que el navegador del celular abra la cámara. Un motor `Fijo` devuelve imágenes pregeneradas para probar y ensayar sin GPU.

**Tech Stack:** Rust 2024 (axum 0.8, tokio 1.53, askama 0.16, reqwest 0.13, tokio-tungstenite 0.30, image 0.25, toml 1.1, thiserror 2), Nix flake con crane y rust-overlay, Docker Compose (ComfyUI v0.35.0 sobre `pytorch/pytorch:2.14.0-cuda12.6-cudnn9`, Caddy), JavaScript sin dependencias en el navegador, Playwright desde nixpkgs para la prueba de punta a punta.

Especificación: `docs/superpowers/specs/2026-09-26-galatea-demo-design.md`.

## Global Constraints

- **Dependencias en su última versión estable**, comprobadas contra crates.io al tocarlas (regla de Crescent y Nexo Huella). Versiones verificadas el 2026-09-26: tokio 1.53.1, axum 0.8.9, tracing 0.1.44, tracing-subscriber 0.3.23, anyhow 1.0.104, thiserror 2.0.21, askama 0.16.1, toml 1.1.6, serde 1.0.229, serde_json 1.0.151, reqwest 0.13.5, tokio-tungstenite 0.30.0, futures-util 0.3.34, image 0.25.10, getrandom 0.4, clap 4.6.7, tempfile 3.27.0. Si al ejecutar hay una más nueva, usa la más nueva.
- **Toolchain de Rust:** la última estable del `rust-overlay` fijado en el flake (Crescent usa `1.98.1` fijada el 2026-09-21; comprueba si hay una más nueva al escribir el flake). Edición 2024.
- **Dominio en español:** `prenda`, `flujo`, `sesion`, `motor`, `marca`, `trabajo`, `avance`. Solo las palabras propias de Rust y de las bibliotecas quedan en inglés. Los mensajes de error que ve una persona dicen qué pasó y cómo arreglarlo, en español.
- **Sin base de datos.** Sesiones en memoria, archivos bajo `datos/sesiones/`.
- **Puertos** (los de Aylin y Crescent ya están ocupados en esta máquina: 8080, 8090, 8443, 8642, 8765, 8888): Galatea `127.0.0.1:8766`; Caddy `192.168.0.10:8444` (HTTPS) y `192.168.0.10:8091` (certificado); ComfyUI `127.0.0.1:8188`.
- **GPU:** RTX 3090, 24 GB, driver 615.71, CDI `nvidia.com/gpu=all`. Comparte la GPU con `crescent-ollama` y los contenedores de Aylin (`visjar-*`): `scripts/demo.sh` los apaga antes de la demo.
- **Usuario del host:** uid 1000, gid 100. El contenedor de ComfyUI corre con ese `user:` para que Galatea pueda borrar lo que ComfyUI escribe.
- **Commits:** Conventional Commits cortos, en español o inglés como en los repos hermanos, **sin** `Co-Authored-By` ni ningún trailer (regla global del autor). Un commit por paso que compila.
- **`nix flake check` es la autoridad.** Ninguna tarea termina sin que pase.
- **Informar solo lo ejecutado:** cada paso de verificación pega la salida real del comando.
- **Nada personal en el log:** ni identificadores completos de sesión ni nombres de archivo de fotos de personas. Usa `sesion::abreviar(id)` (6 caracteres).
- **Licencias:** para la demo se aceptan modelos CC BY-NC-SA. Cada flujo declara su licencia en `descripcion`.

---

## Estructura de archivos

```
galatea/
├── flake.nix, flake.lock, .envrc, .gitignore, askama.toml
├── Cargo.toml, Cargo.lock
├── CLAUDE.md                     convenciones (Tarea 1)
├── README.md, DEMO.md            (Tarea 11)
├── compose.yaml                  comfyui + caddy (Tarea 2 y 11)
├── comfyui/Dockerfile            ComfyUI v0.35.0 + nodos fijados (Tarea 2)
├── comfyui/requisitos-nodos.txt  pines de pip de los nodos (Tarea 2)
├── caddy/Caddyfile               (Tarea 11)
├── scripts/
│   ├── descargar-modelos.sh      lee flujos.toml, baja pesos, verifica sha256 (Tarea 2)
│   ├── descargar-pruebas.sh      imágenes CC (Tarea 2)
│   └── demo.sh                   apaga otros contenedores, levanta todo, imprime la URL (Tarea 11)
├── flujos/
│   ├── flujos.toml               (Tarea 2 y 12)
│   ├── catvton.json              (Tarea 2)
│   └── fijo/generico.png         (Tarea 7)
├── prendas/prendas.toml + imágenes de la tienda (fuera de git salvo el TOML de ejemplo)
├── pruebas/
│   ├── prendas/{prendas.toml, roja.png, azul.png}       fixtures (Tarea 3)
│   ├── flujos/{flujos.toml, prueba.json, fijo/generico.png}  (Tarea 4 y 7)
│   ├── imagenes/{persona-prueba.jpg, LICENCIAS.md}      (Tarea 2 y 10)
│   └── comfy/                    lo que el ComfyUI falso devuelve (Tarea 8)
├── estaticos/{estilo.css, app.js, marca.png}
├── plantillas/index.html
├── src/
│   ├── main.rs                   CLI: servir | experimento
│   ├── lib.rs                    Conf, Estado, preparar, construir_router, servir
│   ├── prendas.rs                (Tarea 3)
│   ├── flujos.rs                 (Tarea 4)
│   ├── sesion.rs                 (Tarea 5)
│   ├── marca.rs                  (Tarea 6)
│   ├── motor/{mod.rs, fijo.rs, comfy.rs}   (Tarea 7 y 8)
│   ├── web/{mod.rs, error.rs, estaticos.rs, pagina.rs, prendas.rs, sesiones.rs, flujos.rs, salud.rs}  (Tarea 9 y 10)
│   └── experimento.rs            (Tarea 12)
├── tests/
│   ├── comun/{mod.rs, comfy_falso.rs}
│   ├── salud.rs, prendas.rs, flujos.rs, sesion.rs, marca.rs, motor_fijo.rs, motor_comfy.rs, web.rs, comfy_real.rs
│   └── e2e/demo.spec.js          Playwright (Tarea 10)
├── playwright.config.js, package.json
├── experimento/                  resultados con fecha (fuera de git salvo los .md)
└── datos/                        fuera de git: modelos/, comfy/, sesiones/, miniaturas/, caddy/
```

Responsabilidades: `prendas` y `flujos` solo leen configuración y validan; `sesion` es el único que crea y borra carpetas de sesión; `motor::comfy` es el único que conoce la API de ComfyUI; `marca` es una función pura sobre bytes; `web` traduce HTTP a llamadas de los anteriores y nunca toca disco directamente salvo para leer un resultado ya escrito.

---

### Tarea 1: Esqueleto del proyecto, configuración y `/salud`

**Files:**
- Create: `flake.nix`, `.envrc`, `.gitignore`, `askama.toml`, `Cargo.toml`, `CLAUDE.md`, `src/main.rs`, `src/lib.rs`, `src/web/mod.rs`, `src/web/salud.rs`, `src/web/error.rs`, `tests/comun/mod.rs`, `tests/salud.rs`

**Interfaces:**
- Produces: `galatea::Conf` (campos abajo), `Conf::desde_entorno() -> anyhow::Result<Conf>`, `Conf::para_pruebas(raiz: &Path) -> Conf` (con feature `pruebas`), `galatea::Estado` (se completa en tareas siguientes), `galatea::construir_router(estado: Estado) -> axum::Router`, `galatea::preparar(conf: &Conf) -> anyhow::Result<Estado>`, `galatea::servir(conf: Conf) -> anyhow::Result<()>`, `web::error::Fallo` (error HTTP en JSON), `tests/comun::Servidor`.

- [ ] **Paso 1: Comprobar versiones antes de escribir nada**

```bash
cd galatea
for c in tokio axum tracing tracing-subscriber anyhow thiserror askama toml serde serde_json reqwest tokio-tungstenite futures-util image getrandom clap tempfile; do
  printf '%s = ' "$c"; curl -s "https://crates.io/api/v1/crates/$c" -H 'User-Agent: galatea-dev' | python3 -c "import sys,json; print(json.load(sys.stdin)['crate']['max_stable_version'])"
done
```
Anota las versiones; úsalas en `Cargo.toml` (mayor.menor). Comprueba también la última estable de Rust: `curl -s https://static.rust-lang.org/dist/channel-rust-stable.toml | grep -m1 '^version'`.

- [ ] **Paso 2: `Cargo.toml`**

```toml
[package]
name = "galatea"
version = "0.1.0"
edition = "2024"
description = "Probador virtual con IA generativa: demo presencial"

[[bin]]
name = "galatea"
path = "src/main.rs"

[dependencies]
tokio = { version = "1.53", features = ["rt-multi-thread", "macros", "signal", "fs", "sync", "time"] }
axum = { version = "0.8", features = ["json", "multipart"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1.0"
thiserror = "2.0"
askama = "0.16"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "1.1"
reqwest = { version = "0.13", default-features = false, features = ["json", "multipart", "charset", "http2"] }
tokio-tungstenite = "0.30"
futures-util = "0.3"
image = "0.25"
getrandom = "0.4"
clap = { version = "4.6", features = ["derive"] }
tokio-stream = "0.1"

[features]
# Ayudantes que solo tienen sentido probando (Conf::para_pruebas).
pruebas = []

[dev-dependencies]
galatea = { path = ".", features = ["pruebas"] }
axum = { version = "0.8", features = ["json", "multipart", "ws"] }
tempfile = "3.27"
```

(`tokio-stream` se usa en la Tarea 9 para el SSE; comprueba su versión también.)

- [ ] **Paso 3: `flake.nix`** (copiado de Crescent, sin sqlite, con Playwright como Aylin)

```nix
{
  description = "Galatea - probador virtual con IA generativa";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        # Versión fijada a propósito. Elegida el 2026-09-26: la más reciente
        # estable del rust-overlay fijado. Actualízala junto con flake.lock.
        cadenaRust = pkgs.rust-bin.stable."1.98.1".default.override {
          extensions = [ "rust-src" "clippy" "rustfmt" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain cadenaRust;

        # Las plantillas, los estáticos y las fixtures de pruebas van dentro del
        # binario o los leen las pruebas: tienen que sobrevivir el filtro.
        fuentes = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = ruta: tipo:
            (builtins.match ".*\\.(html|css|js|toml|json|png|jpg|md)$" ruta != null)
            || (builtins.match ".*/(plantillas|estaticos|pruebas)(/.*)?$" ruta != null)
            || (builtins.baseNameOf ruta == "askama.toml")
            || (craneLib.filterCargoSources ruta tipo);
          name = "fuentes";
        };

        argosComunes = {
          src = fuentes;
          strictDeps = true;
          pname = "galatea";
          version = "0.1.0";
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = [ ];
        };

        artefactos = craneLib.buildDepsOnly argosComunes;

        galatea = craneLib.buildPackage (argosComunes // {
          cargoArtifacts = artefactos;
          doCheck = false;
        });
      in
      {
        packages.default = galatea;
        packages.galatea = galatea;
        apps.default = flake-utils.lib.mkApp { drv = galatea; };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ galatea ];
          packages = with pkgs; [
            cadenaRust
            rust-analyzer
            (python3.withPackages (ps: [ ps.pillow ]))
            dejavu_fonts
            nodejs
            playwright-test
            curl
            jq
          ];
          RUST_LOG = "info,galatea=debug";
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
          PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
          # El resolvedor ESM de Node ignora NODE_PATH: `import "@playwright/test"`
          # necesita un node_modules real que apunte al paquete de nixpkgs.
          shellHook = ''
            mkdir -p node_modules
            ln -sfn ${pkgs.playwright-test}/lib/node_modules/@playwright node_modules/@playwright
            ln -sfn ${pkgs.playwright-test}/lib/node_modules/playwright-core node_modules/playwright-core
          '';
        };

        checks = {
          inherit galatea;
          clippy = craneLib.cargoClippy (argosComunes // {
            cargoArtifacts = artefactos;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });
          formato = craneLib.cargoFmt { inherit (argosComunes) pname version; src = fuentes; };
          pruebas = craneLib.cargoTest (argosComunes // { cargoArtifacts = artefactos; });
        };
      }
    );
}
```

`.envrc`: `use flake`. `askama.toml`:
```toml
[general]
dirs = ["plantillas"]
```
`.gitignore`:
```
target/
result*
.direnv/
datos/
prendas/*
!prendas/prendas.toml
experimento/*
!experimento/*.md
node_modules/
test-results/
playwright-report/
.superpowers/
.env
```

- [ ] **Paso 4: `CLAUDE.md`** (convenciones; copia las secciones «Dependencias», «Idioma», «Informar: solo lo que ejecutaste», «Trampas ya vistas» y «Git» de `../crescent/CLAUDE.md` tal cual, y agrega estas propias)

```markdown
# Convenciones de Galatea

Probador virtual con IA generativa. Demo presencial: una persona le saca una foto a otra
desde el celular y ve la prenda puesta. Rust + axum en el host, ComfyUI en contenedor con
la GPU, Caddy para HTTPS en la red local. **Sin base de datos:** sesiones en memoria y
archivos temporales que se borran.

## Reglas propias de Galatea

- **Las fotos de personas son efímeras.** Viven en `datos/sesiones/<id>/` y en
  `datos/comfy/{input,output}/` solo mientras dura la sesión (15 minutos). `sesion` es el
  único módulo que crea y borra esas carpetas; `motor::comfy` borra lo suyo en ComfyUI al
  terminar cada trabajo. Al arrancar, `datos/sesiones/` se vacía entero.
- **Nunca en el log:** ni el identificador completo de una sesión (usa `sesion::abreviar`),
  ni nombres de archivo de fotos, ni imágenes en base64.
- **Un motor es un flujo.** Agregar un modelo es exportar un flujo en formato API desde
  ComfyUI a `flujos/<nombre>.json` y declararlo en `flujos/flujos.toml`. Rust no sabe qué
  modelo hay detrás; solo conoce los identificadores de nodo del TOML.
- **`motor::comfy` es el único que habla con ComfyUI.** Nadie más hace HTTP a `:8188`.
- **Toda imagen generada sale con marca de agua** («imagen generada con IA»), aplicada
  por el servidor antes de servirla.
- **Licencias:** los modelos no comerciales sirven para la demo. Antes de cobrar por esto
  se cambia a Leffa (MIT) o FLUX.2 klein (Apache 2.0). Cada flujo dice su licencia en
  `descripcion`.

## Pruebas

Las de integración levantan el **router real** con `tests/comun/mod.rs`. El motor `Fijo`
es el doble de ComfyUI para todo lo que no sea el cliente de ComfyUI; ese cliente se prueba
contra `tests/comun/comfy_falso.rs`, un servidor axum que imita la API. La única prueba
contra el ComfyUI real está marcada `#[ignore]` y se corre a mano con el contenedor arriba.

La prueba de navegador usa Playwright desde nixpkgs (`playwright-test`), sin `npm install`;
la cámara se sustituye por el selector de archivo que la página ofrece siempre.

## GPU compartida

La RTX 3090 la usan también `crescent-ollama` y los contenedores de Aylin. `scripts/demo.sh`
los detiene antes de levantar ComfyUI y lo dice por pantalla. No los mates a mano con
`pkill`: usa `docker stop <nombre>`.
```

- [ ] **Paso 5: `src/lib.rs` con `Conf`, `Estado` mínimo, router y `servir`**

```rust
//! Galatea: probador virtual con IA generativa, demo presencial.
//! El servidor vive aquí y no en `main.rs` para que las pruebas levanten el mismo router.

pub mod web;

use anyhow::{Result, anyhow};
use axum::{Router, extract::Request, middleware::{self, Next}, response::IntoResponse, routing::get};
use std::{net::SocketAddr, path::PathBuf, time::{Duration, Instant}};
use tokio::signal;
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
        let comfy_url = std::env::var("GALATEA_COMFY_URL").ok().filter(|u| !u.is_empty());
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
        }
    }
}

/// Todo lo que los manejadores necesitan. Se completa en las tareas siguientes.
#[derive(Clone)]
pub struct Estado {
    pub conf: Conf,
}

/// Carga configuración y recursos; falla con un mensaje claro si algo falta.
pub async fn preparar(conf: &Conf) -> Result<Estado> {
    tokio::fs::create_dir_all(&conf.datos)
        .await
        .map_err(|e| anyhow!("no se pudo crear la carpeta de datos {}: {e}", conf.datos.display()))?;
    Ok(Estado { conf: conf.clone() })
}

/// Una línea por petición: método, ruta, estado, duración. Las rutas llevan
/// identificadores de sesión: se recortan a 6 caracteres antes de escribir.
async fn registro(peticion: Request, siguiente: Next) -> impl IntoResponse {
    let metodo = peticion.method().clone();
    let ruta = recortar_ids(peticion.uri().path());
    let inicio = Instant::now();
    let respuesta = siguiente.run(peticion).await;
    info!("{} {} {} {}ms", metodo, ruta, respuesta.status().as_u16(), inicio.elapsed().as_millis());
    respuesta
}

/// `/sesiones/0123456789abcdef.../resultado/1.png` → `/sesiones/012345…/resultado/1.png`.
fn recortar_ids(ruta: &str) -> String {
    ruta.split('/')
        .map(|trozo| if trozo.len() >= 32 && trozo.chars().all(|c| c.is_ascii_hexdigit()) {
            format!("{}…", &trozo[..6])
        } else {
            trozo.to_string()
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub fn construir_router(estado: Estado) -> Router {
    Router::new()
        .route("/salud", get(web::salud::ver))
        .fallback(|| async { web::error::Fallo::no_encontrado("esa ruta no existe") })
        .layer(middleware::from_fn(registro))
        .with_state(estado)
}

async fn señal_de_apagado() {
    let interrupcion = async { signal::ctrl_c().await.expect("no se pudo escuchar la interrupción") };
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
            anyhow!("el puerto {} ya está ocupado; elige otro con GALATEA_PUERTO=<puerto>", conf.puerto)
        } else {
            anyhow!("no se pudo escuchar en {socket}: {e}")
        }
    })?;
    info!("galatea escuchando en http://{}", escucha.local_addr()?);
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
```

- [ ] **Paso 6: `src/web/mod.rs`, `src/web/error.rs`, `src/web/salud.rs`**

```rust
// src/web/mod.rs
//! Capa web: manejadores que traducen HTTP a llamadas de los módulos del dominio.
pub mod error;
pub mod salud;
```

```rust
// src/web/error.rs
//! Errores HTTP en JSON, siempre con un mensaje en español que dice qué pasó y qué hacer.
use axum::{Json, http::StatusCode, response::{IntoResponse, Response}};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Fallo {
    #[serde(skip)]
    pub estado: StatusCode,
    pub error: String,
}

impl Fallo {
    pub fn no_encontrado(mensaje: &str) -> Response {
        Self { estado: StatusCode::NOT_FOUND, error: mensaje.to_string() }.into_response()
    }
    pub fn invalido(mensaje: impl Into<String>) -> Response {
        Self { estado: StatusCode::UNPROCESSABLE_ENTITY, error: mensaje.into() }.into_response()
    }
    pub fn interno(mensaje: impl Into<String>) -> Response {
        Self { estado: StatusCode::INTERNAL_SERVER_ERROR, error: mensaje.into() }.into_response()
    }
    pub fn no_disponible(mensaje: impl Into<String>) -> Response {
        Self { estado: StatusCode::SERVICE_UNAVAILABLE, error: mensaje.into() }.into_response()
    }
}

impl IntoResponse for Fallo {
    fn into_response(self) -> Response {
        let estado = self.estado;
        (estado, Json(self)).into_response()
    }
}
```

```rust
// src/web/salud.rs
//! GET /salud: versión y, más adelante, estado de ComfyUI y de los flujos.
use crate::Estado;
use axum::{Json, extract::State};
use serde_json::{Value, json};

pub async fn ver(State(_estado): State<Estado>) -> Json<Value> {
    Json(json!({ "programa": "galatea", "version": env!("CARGO_PKG_VERSION") }))
}
```

- [ ] **Paso 7: `src/main.rs`**

```rust
use anyhow::Result;
use clap::{Parser, Subcommand};
use galatea::{Conf, servir};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "galatea", about = "Probador virtual con IA generativa: demo presencial")]
struct Cli {
    #[command(subcommand)]
    comando: Option<Comando>,
}

#[derive(Subcommand)]
enum Comando {
    /// Levanta el servidor (también es lo que pasa sin subcomando).
    Servir,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();
    let cli = Cli::parse();
    match cli.comando {
        None | Some(Comando::Servir) => servir(Conf::desde_entorno()?).await,
    }
}
```

- [ ] **Paso 8: Ayudante de pruebas `tests/comun/mod.rs` y la primera prueba**

```rust
// tests/comun/mod.rs
//! Levanta el router real en un puerto efímero con una carpeta temporal.
#![allow(dead_code)]
use galatea::{Conf, construir_router, preparar};
use tempfile::TempDir;

pub struct Servidor {
    pub direccion: String,
    pub cliente: reqwest::Client,
    pub conf: Conf,
    _temporal: TempDir,
}

impl Servidor {
    pub async fn nuevo() -> Self {
        let temporal = TempDir::new().expect("no se pudo crear el directorio temporal");
        let conf = Conf::para_pruebas(temporal.path());
        Self::con_conf(conf, temporal).await
    }

    pub async fn con_conf(conf: Conf, temporal: TempDir) -> Self {
        let escucha = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("sin puerto efímero");
        let puerto = escucha.local_addr().unwrap().port();
        let estado = preparar(&conf).await.expect("preparar no debería fallar en pruebas");
        let router = construir_router(estado);
        tokio::spawn(async move { axum::serve(escucha, router).await.ok(); });
        Self {
            direccion: format!("http://127.0.0.1:{puerto}"),
            cliente: reqwest::Client::new(),
            conf,
            _temporal: temporal,
        }
    }

    pub fn url(&self, ruta: &str) -> String {
        format!("{}{}", self.direccion, ruta)
    }
}
```

```rust
// tests/salud.rs
mod comun;

#[tokio::test]
async fn salud_responde_con_la_version() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/salud")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let cuerpo: serde_json::Value = r.json().await.unwrap();
    assert_eq!(cuerpo["programa"], "galatea");
}

#[tokio::test]
async fn ruta_inexistente_devuelve_404_en_json() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/no-existe")).send().await.unwrap();
    assert_eq!(r.status(), 404);
    let cuerpo: serde_json::Value = r.json().await.unwrap();
    assert!(cuerpo["error"].as_str().unwrap().contains("no existe"));
}
```

`tests/salud.rs` necesita `reqwest` como dev-dependency: agrega `reqwest = { version = "0.13", default-features = false, features = ["json", "multipart", "charset", "http2", "stream"] }` a `[dev-dependencies]`.

- [ ] **Paso 9: Entrar al devShell, compilar y correr**

```bash
cd galatea && direnv allow && nix develop -c cargo test
```
Esperado: 3 pruebas pasan (`recorta_identificadores_hexadecimales_largos`, `salud_responde_con_la_version`, `ruta_inexistente_devuelve_404_en_json`). Si `rust-bin.stable."1.98.1"` no existe en el overlay fijado, mira `nix eval .#... ` o el lock y elige la última disponible.

- [ ] **Paso 10: `nix flake check` y commit**

```bash
nix flake check 2>&1 | tail -5
git add -A && git commit -m "feat: esqueleto del servidor, configuración y /salud"
```

---

### Tarea 2: ComfyUI en contenedor con CatVTON, imágenes de prueba y primer flujo (la tarea de riesgo)

Esta tarea no toca Rust. Su resultado es un contenedor que arranca, un `flujos/catvton.json` en formato API que produce una imagen con las dos imágenes Creative Commons, y las cifras de VRAM y tiempo anotadas. **Está limitada a un día de trabajo:** si CatVTON no corre en ese plazo, se anota por qué en `experimento/2026-09-XX-nodos.md`, se deja el contenedor con ComfyUI base funcionando (verificado con `/system_stats`) y se sigue con las tareas de Rust, que no dependen de ningún flujo real.

**Files:**
- Create: `comfyui/Dockerfile`, `comfyui/requisitos-nodos.txt`, `compose.yaml`, `scripts/descargar-modelos.sh`, `scripts/descargar-pruebas.sh`, `pruebas/imagenes/LICENCIAS.md`, `flujos/flujos.toml`, `flujos/catvton.json`, `experimento/2026-09-XX-nodos.md`

**Interfaces:**
- Produces: contenedor `galatea-comfyui` en `http://127.0.0.1:8188`; carpetas del host `datos/modelos/` (→ `/comfy/models`), `datos/comfy/input/` y `datos/comfy/output/` (→ `/comfy/input`, `/comfy/output`); `flujos/flujos.toml` con las entradas `fijo` y `catvton` en el formato que `flujos::cargar` (Tarea 4) lee.

- [ ] **Paso 1: Imágenes Creative Commons de prueba**

`scripts/descargar-pruebas.sh`:
```bash
#!/usr/bin/env bash
# Baja las imágenes Creative Commons de la comprobación de flujo. Atribución en
# pruebas/imagenes/LICENCIAS.md. No son fotos de nadie del entorno del proyecto.
set -euo pipefail
cd "$(dirname "$0")/.."
destino=pruebas/imagenes
mkdir -p "$destino"
# Wikimedia pide un User-Agent con contacto: GALATEA_CONTACTO="tu@correo".
ua="galatea-dev/0.1 (contacto: ${GALATEA_CONTACTO:-sin contacto})"
bajar() { # nombre url
  [ -s "$destino/$1" ] && { echo "ya está: $1"; return; }
  curl -sSL -A "$ua" -o "$destino/$1" "$2" && echo "bajado: $1"
}
bajar prenda-polera-cc.jpg "https://commons.wikimedia.org/w/index.php?title=Special:FilePath/T-shirt_(AM_2002.94.1.1-1).jpg&width=1024"
bajar persona-cc.jpg "https://commons.wikimedia.org/w/index.php?title=Special:FilePath/Gordon_Cooper_posing_in_suit.jpg&width=1024"
file "$destino"/*.jpg
```

`pruebas/imagenes/LICENCIAS.md`:
```markdown
# Imágenes de prueba y su licencia

| Archivo | Origen | Licencia | Autor |
|---|---|---|---|
| prenda-polera-cc.jpg | https://commons.wikimedia.org/wiki/File:T-shirt_(AM_2002.94.1.1-1).jpg | CC BY 4.0 | Auckland War Memorial Museum |
| persona-cc.jpg | https://commons.wikimedia.org/wiki/File:Gordon_Cooper_posing_in_suit.jpg | Dominio público | NASA |
| persona-prueba.jpg | generada con Pillow en la Tarea 10 (silueta sintética) | CC0 | este proyecto |

Los `.jpg` están fuera de git (`.gitignore`); `scripts/descargar-pruebas.sh` los baja.
```

Agrega `pruebas/imagenes/*.jpg` y `!pruebas/imagenes/persona-prueba.jpg` al `.gitignore`. Corre `chmod +x scripts/descargar-pruebas.sh && scripts/descargar-pruebas.sh` y pega la salida de `file`.

- [ ] **Paso 2: `comfyui/Dockerfile`**

La base es la imagen oficial de PyTorch con CUDA 12.6 (el driver 615 la soporta), fijada por digest (obtenido el 2026-09-26 de Docker Hub, arquitectura amd64). Empieza con la variante `runtime`; si algún nodo exige compilar (detectron2), cambia a `devel` (`sha256:0e968d2570373aeb781603c21925a030053b432945967da14f6f8423e1f0cc79`).

```dockerfile
# ComfyUI v0.35.0 (la misma versión que empaqueta nixpkgs el 2026-09-26) sobre la
# imagen oficial de PyTorch 2.14 + CUDA 12.6. Todo fijado: la imagen por digest, ComfyUI
# por tag, los nodos por commit, pip por requisitos-nodos.txt.
FROM pytorch/pytorch:2.14.0-cuda12.6-cudnn9-runtime@sha256:a77983eb7a3042ccf41a94da547a7f54a47fb70da15c36d67a8e8a329ebaa098

ARG COMFYUI_TAG=v0.35.0
ARG UID=1000
ARG GID=100

RUN apt-get update && apt-get install -y --no-install-recommends git curl libgl1 libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g ${GID} -o comfy 2>/dev/null || true \
    && useradd -m -u ${UID} -g ${GID} -o comfy

WORKDIR /comfy
RUN git clone --depth 1 --branch ${COMFYUI_TAG} https://github.com/comfyanonymous/ComfyUI.git app \
    && pip install --no-cache-dir -r app/requirements.txt

# Nodos personalizados, cada uno a un commit fijo. Se agregan en la Tarea 2 (CatVTON) y 12.
COPY requisitos-nodos.txt /comfy/requisitos-nodos.txt
RUN set -eux; cd app/custom_nodes; \
    git clone https://github.com/chflame163/ComfyUI_CatVTON_Wrapper.git && (cd ComfyUI_CatVTON_Wrapper && git checkout e2ae9b62c53a); \
    git clone https://github.com/StartHua/Comfyui_segformer_b2_clothes.git && (cd Comfyui_segformer_b2_clothes && git checkout $(git rev-parse HEAD)); \
    pip install --no-cache-dir -r /comfy/requisitos-nodos.txt

# Los modelos, la entrada y la salida viven fuera de la imagen.
RUN mkdir -p /comfy/models /comfy/input /comfy/output /comfy/temp /comfy/user \
    && chown -R ${UID}:${GID} /comfy
USER ${UID}:${GID}
EXPOSE 8188
CMD ["python", "app/main.py", "--listen", "0.0.0.0", "--port", "8188", \
     "--base-directory", "/comfy", "--disable-auto-launch"]
```

Notas para quien ejecuta: (a) `--base-directory /comfy` hace que ComfyUI use `/comfy/models`, `/comfy/input`, `/comfy/output`, `/comfy/user` (comprueba en `app/main.py --help` de v0.35.0 que la opción existe; si no, usa `--input-directory`, `--output-directory` y `extra_model_paths.yaml`). (b) Sustituye `$(git rev-parse HEAD)` del nodo segformer por el commit real que veas y anótalo. (c) `requisitos-nodos.txt` se llena con lo que pidan los `requirements.txt` de ambos nodos, fijando versión exacta de cada paquete (`pip freeze` dentro del contenedor tras la primera construcción y copia las líneas relevantes). El wrapper de CatVTON recibe una **máscara** como entrada; la máscara la produce el nodo segformer (segmentación de ropa `mattmdjaga/segformer_b2_clothes`, sin compilar nada). Si el wrapper resultara exigir detectron2, cambia a la imagen `devel` y agrega `pip install git+https://github.com/facebookresearch/detectron2@<commit>`.

- [ ] **Paso 3: `compose.yaml` (solo ComfyUI por ahora; Caddy se suma en la Tarea 11)**

```yaml
# ComfyUI con la GPU por CDI (NixOS con hardware.nvidia-container-toolkit.enable = true).
# Corre como el usuario del host (1000:100) para que Galatea pueda borrar lo que escribe.
services:
  comfyui:
    build:
      context: ./comfyui
      args:
        UID: "1000"
        GID: "100"
    image: galatea-comfyui:0.35.0
    container_name: galatea-comfyui
    user: "1000:100"
    ports:
      - "127.0.0.1:8188:8188"
    volumes:
      - ./datos/modelos:/comfy/models
      - ./datos/comfy/input:/comfy/input
      - ./datos/comfy/output:/comfy/output
      - ./datos/comfy/user:/comfy/user
      - ./datos/comfy/temp:/comfy/temp
      - ./datos/hf-cache:/home/comfy/.cache/huggingface
    environment:
      - HF_HUB_OFFLINE=0
    deploy:
      resources:
        reservations:
          devices:
            - driver: cdi
              device_ids: ["nvidia.com/gpu=all"]
              capabilities: [gpu]
    restart: unless-stopped
```

```bash
mkdir -p datos/modelos datos/comfy/{input,output,user,temp} datos/hf-cache
docker compose build comfyui 2>&1 | tail -20
docker compose up -d comfyui && sleep 20 && curl -s http://127.0.0.1:8188/system_stats | jq '.devices[0] | {name, vram_total, vram_free}'
```
Esperado: nombre `NVIDIA GeForce RTX 3090` y `vram_total` ≈ 25.7e9. Pega la salida. Si `crescent-ollama` tiene poco libre, no importa todavía.

- [ ] **Paso 4: Pesos de CatVTON con `scripts/descargar-modelos.sh`**

El script lee `flujos/flujos.toml` con Python (`tomllib`) y baja cada `[[flujo.peso]]`:
```bash
#!/usr/bin/env bash
# Baja los pesos declarados en flujos/flujos.toml a datos/modelos/<destino> y verifica sha256.
# Uso: scripts/descargar-modelos.sh [nombre-de-flujo]   (sin argumento: todos)
set -euo pipefail
cd "$(dirname "$0")/.."
python3 - "$@" <<'PY'
import hashlib, pathlib, subprocess, sys, tomllib
flujos = tomllib.loads(pathlib.Path("flujos/flujos.toml").read_text())["flujo"]
pedidos = set(sys.argv[1:])
base = pathlib.Path("datos/modelos")
for f in flujos:
    if pedidos and f["nombre"] not in pedidos:
        continue
    for p in f.get("peso", []):
        destino = base / p["destino"]
        destino.parent.mkdir(parents=True, exist_ok=True)
        if destino.exists():
            print(f"ya está: {destino}")
        else:
            print(f"bajando {p['url']} → {destino}")
            subprocess.run(["curl", "-L", "--fail", "--progress-bar", "-o", str(destino), p["url"]], check=True)
        h = hashlib.sha256(destino.read_bytes()).hexdigest()
        if p["sha256"] and h != p["sha256"]:
            sys.exit(f"sha256 distinto para {destino}: esperado {p['sha256']}, obtenido {h}")
        print(f"ok: {destino.name} {h[:12]}")
PY
```

Los pesos que CatVTON (wrapper) necesita, según el código del nodo (léelo en `custom_nodes/ComfyUI_CatVTON_Wrapper/py/*.py` para confirmar carpeta y nombres): el repositorio `zhengchong/CatVTON` de Hugging Face (`mix-48k-1024/attention/*.safetensors`, `DensePose/`, `SCHP/` si el wrapper los usa), el VAE `stabilityai/sd-vae-ft-mse` y la base `booksforcharlie/stable-diffusion-inpainting`. El nodo segformer baja `mattmdjaga/segformer_b2_clothes` solo a `datos/hf-cache` en el primer uso. Declara cada archivo en `flujos.toml` con su URL `resolve/main` y su sha256 (obtén el hash con `curl -sI` no sirve; bájalo una vez y calcula `sha256sum`). Si son demasiados archivos, usa `huggingface-cli download <repo> --local-dir datos/modelos/CatVTON/<repo>` dentro del script y verifica solo los `.safetensors` grandes.

- [ ] **Paso 5: Armar el flujo en la interfaz y exportarlo en formato API**

1. Abre `http://127.0.0.1:8188` en el navegador del escritorio. Activa en ajustes «Dev mode» para que aparezca «Save (API format)».
2. Sube `pruebas/imagenes/persona-cc.jpg` y `prenda-polera-cc.jpg` con dos nodos **LoadImage**.
3. Persona → nodo segformer (máscara de «upper clothes»; incluye brazos si el nodo lo permite) → **CatVTON Wrapper** (`image`=persona, `mask`=máscara, `refer_image`=prenda, `steps` 40, `cfg` 2.5, `mixed_precision` bf16, `seed` fijo) → **SaveImage** con `filename_prefix` `galatea`.
4. Ejecuta. Mide con `nvidia-smi --query-gpu=memory.used --format=csv -l 1` durante la ejecución y anota el pico y los segundos.
5. «Save (API format)» → `flujos/catvton.json`. Abre el JSON y anota los identificadores de los nodos LoadImage de persona y prenda, y del SaveImage.

- [ ] **Paso 6: `flujos/flujos.toml`**

```toml
# Cada flujo es un motor. Los identificadores de nodo son los del JSON en formato API.
# Ver docs/superpowers/specs/2026-09-26-galatea-demo-design.md §6.

[[flujo]]
nombre = "fijo"
titulo = "Fijo (sin GPU)"
descripcion = "Devuelve una imagen pregenerada de flujos/fijo/. Para ensayar la demo y para las pruebas."
espera_segundos = 8

[[flujo]]
nombre = "catvton"
titulo = "CatVTON"
descripcion = "SD1.5 inpainting, < 8 GB de VRAM. Licencia CC BY-NC-SA 4.0: solo demo, no comercial."
archivo = "catvton.json"
nodo_persona = "<id LoadImage persona>"
nodo_prenda = "<id LoadImage prenda>"
nodo_salida = "<id SaveImage>"
tipos = ["superior"]
# nodo_tipo se omite: la máscara la decide el nodo segformer; cuando el flujo admita
# inferior/vestido, apunta nodo_tipo al nodo cuyo campo elige la clase y define valores_tipo.

[[flujo.peso]]
url = "https://huggingface.co/zhengchong/CatVTON/resolve/main/mix-48k-1024/attention/model.safetensors"
destino = "CatVTON/mix-48k-1024/attention/model.safetensors"
sha256 = "<calcular>"
# ... una entrada por archivo que el nodo lea desde /comfy/models
```

Sustituye cada `<...>` por el valor real; ningún marcador puede quedar en el archivo commiteado.

- [ ] **Paso 7: Comprobación por la API, sin interfaz**

```bash
python3 - <<'PY'
import json, urllib.request, uuid, time
grafo = json.load(open("flujos/catvton.json"))
# rellena a mano los nombres tal como quedaron subidos por la interfaz
cid = str(uuid.uuid4())
r = urllib.request.urlopen(urllib.request.Request("http://127.0.0.1:8188/prompt",
    data=json.dumps({"prompt": grafo, "client_id": cid}).encode(), headers={"Content-Type": "application/json"}))
pid = json.load(r)["prompt_id"]; print("prompt_id", pid)
while True:
    h = json.load(urllib.request.urlopen(f"http://127.0.0.1:8188/history/{pid}"))
    if pid in h: break
    time.sleep(1)
print(json.dumps(h[pid]["outputs"], indent=1)[:800])
PY
ls -la datos/comfy/output/
```
Esperado: `outputs` con `images: [{filename: "galatea_00001_.png", subfolder: "", type: "output"}]` y el archivo en `datos/comfy/output/` **propiedad de 1000:100** (`ls -la`). Si es de root, el `user:` del compose no está funcionando: arréglalo antes de seguir.

- [ ] **Paso 8: Anotar y commitear**

`experimento/2026-09-XX-nodos.md` con: nodo usado y commit, imagen base y digest, segundos por imagen, VRAM pico, resolución de salida, y una frase sobre la calidad con las imágenes CC. Luego:
```bash
git add comfyui compose.yaml scripts flujos pruebas/imagenes/LICENCIAS.md experimento/*.md .gitignore
git commit -m "feat: ComfyUI en contenedor con CatVTON y primer flujo"
```

---

### Tarea 3: Módulo `prendas`

**Files:**
- Create: `src/prendas.rs`, `pruebas/prendas/prendas.toml`, `pruebas/prendas/roja.png`, `pruebas/prendas/azul.png`, `tests/prendas.rs`
- Modify: `src/lib.rs` (agregar `pub mod prendas;`, campo `prendas: Arc<Vec<Prenda>>` en `Estado`, carga y miniaturas en `preparar`)

**Interfaces:**
- Produces:
  - `prendas::Tipo { Superior, Inferior, Vestido }` con `como_texto(self) -> &'static str` y `desde_texto(&str) -> Option<Tipo>`; serde `rename_all = "lowercase"`.
  - `prendas::Prenda { id: String, nombre: String, archivo: String, tipo: Tipo }`.
  - `prendas::cargar(carpeta: &Path) -> Result<Vec<Prenda>, ErrorPrendas>`.
  - `prendas::ruta(carpeta: &Path, prenda: &Prenda) -> PathBuf`.
  - `prendas::generar_miniaturas(carpeta: &Path, prendas: &[Prenda], destino: &Path) -> Result<(), ErrorPrendas>` (escribe `destino/<id>.png`, lado mayor 256).
  - `prendas::buscar<'a>(prendas: &'a [Prenda], id: &str) -> Option<&'a Prenda>`.
  - `prendas::LADO_MINIATURA: u32 = 256`.

- [ ] **Paso 1: Fixtures**

```bash
mkdir -p pruebas/prendas && nix develop -c python3 - <<'PY'
from PIL import Image, ImageDraw
for nombre, color in [("roja", (200, 40, 40)), ("azul", (40, 80, 200))]:
    im = Image.new("RGB", (300, 400), (245, 245, 245))
    d = ImageDraw.Draw(im); d.rounded_rectangle((60, 40, 240, 360), radius=30, fill=color)
    im.save(f"pruebas/prendas/{nombre}.png", optimize=True)
PY
cat > pruebas/prendas/prendas.toml <<'TOML'
[[prenda]]
id = "roja"
nombre = "Polera roja"
archivo = "roja.png"
tipo = "superior"

[[prenda]]
id = "azul"
nombre = "Vestido azul"
archivo = "azul.png"
tipo = "vestido"
TOML
```

- [ ] **Paso 2: Prueba que falla**

```rust
// tests/prendas.rs
use galatea::prendas::{self, ErrorPrendas, Tipo};
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
    std::fs::write(dir.path().join("prendas.toml"), r#"
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
"#).unwrap();
    assert!(matches!(prendas::cargar(dir.path()), Err(ErrorPrendas::IdRepetido(id)) if id == "roja"));

    std::fs::write(dir.path().join("prendas.toml"), r#"
[[prenda]]
id = "fantasma"
nombre = "No está"
archivo = "fantasma.png"
tipo = "inferior"
"#).unwrap();
    let e = prendas::cargar(dir.path()).unwrap_err();
    assert!(matches!(e, ErrorPrendas::ArchivoFaltante { .. }));
    assert!(e.to_string().contains("fantasma"), "el mensaje nombra el archivo: {e}");
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
```
Agrega `image = "0.25"` a `[dev-dependencies]` (ya está en `[dependencies]`; en dev basta con usarla desde el crate, pero las pruebas de integración necesitan la dependencia directa).

- [ ] **Paso 3: Correr para verla fallar**

`nix develop -c cargo test --test prendas` → falla con `unresolved import galatea::prendas`.

- [ ] **Paso 4: Implementación `src/prendas.rs`**

```rust
//! Las prendas precargadas: `prendas.toml` más sus imágenes. Solo lectura y validación.
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::{Path, PathBuf}};

pub const LADO_MINIATURA: u32 = 256;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Tipo {
    Superior,
    Inferior,
    Vestido,
}

impl Tipo {
    pub fn como_texto(self) -> &'static str {
        match self {
            Tipo::Superior => "superior",
            Tipo::Inferior => "inferior",
            Tipo::Vestido => "vestido",
        }
    }

    pub fn desde_texto(texto: &str) -> Option<Tipo> {
        match texto {
            "superior" => Some(Tipo::Superior),
            "inferior" => Some(Tipo::Inferior),
            "vestido" => Some(Tipo::Vestido),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Prenda {
    pub id: String,
    pub nombre: String,
    pub archivo: String,
    pub tipo: Tipo,
}

#[derive(Deserialize)]
struct Archivo {
    #[serde(default)]
    prenda: Vec<Prenda>,
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorPrendas {
    #[error("no se pudo leer {ruta}: {fuente}. Crea el archivo con al menos una [[prenda]].")]
    Lectura { ruta: PathBuf, #[source] fuente: std::io::Error },
    #[error("{ruta} tiene un error de formato: {fuente}")]
    Formato { ruta: PathBuf, #[source] fuente: toml::de::Error },
    #[error("{ruta} no tiene ninguna prenda; agrega al menos una entrada [[prenda]]")]
    Vacio { ruta: PathBuf },
    #[error("la prenda «{0}» aparece dos veces en prendas.toml; los id deben ser únicos")]
    IdRepetido(String),
    #[error("el id «{0}» solo puede llevar letras minúsculas, números y guiones")]
    IdInvalido(String),
    #[error("la prenda «{id}» apunta a {ruta}, que no existe; revisa el campo archivo")]
    ArchivoFaltante { id: String, ruta: PathBuf },
    #[error("no se pudo abrir la imagen de la prenda «{id}»: {fuente}")]
    Imagen { id: String, #[source] fuente: image::ImageError },
    #[error("no se pudo escribir la miniatura de «{id}» en {ruta}: {fuente}")]
    Miniatura { id: String, ruta: PathBuf, #[source] fuente: std::io::Error },
}

pub fn cargar(carpeta: &Path) -> Result<Vec<Prenda>, ErrorPrendas> {
    let ruta = carpeta.join("prendas.toml");
    let texto = fs::read_to_string(&ruta).map_err(|fuente| ErrorPrendas::Lectura { ruta: ruta.clone(), fuente })?;
    let archivo: Archivo = toml::from_str(&texto).map_err(|fuente| ErrorPrendas::Formato { ruta: ruta.clone(), fuente })?;
    if archivo.prenda.is_empty() {
        return Err(ErrorPrendas::Vacio { ruta });
    }
    let mut vistos = HashSet::new();
    for p in &archivo.prenda {
        if !id_valido(&p.id) {
            return Err(ErrorPrendas::IdInvalido(p.id.clone()));
        }
        if !vistos.insert(p.id.clone()) {
            return Err(ErrorPrendas::IdRepetido(p.id.clone()));
        }
        let imagen = self::ruta(carpeta, p);
        if !imagen.is_file() {
            return Err(ErrorPrendas::ArchivoFaltante { id: p.id.clone(), ruta: imagen });
        }
    }
    Ok(archivo.prenda)
}

fn id_valido(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

pub fn ruta(carpeta: &Path, prenda: &Prenda) -> PathBuf {
    carpeta.join(&prenda.archivo)
}

pub fn buscar<'a>(prendas: &'a [Prenda], id: &str) -> Option<&'a Prenda> {
    prendas.iter().find(|p| p.id == id)
}

/// Escribe `destino/<id>.png` con el lado mayor en `LADO_MINIATURA`.
pub fn generar_miniaturas(carpeta: &Path, prendas: &[Prenda], destino: &Path) -> Result<(), ErrorPrendas> {
    fs::create_dir_all(destino).map_err(|fuente| ErrorPrendas::Miniatura {
        id: String::new(), ruta: destino.to_path_buf(), fuente,
    })?;
    for p in prendas {
        let imagen = image::open(ruta(carpeta, p)).map_err(|fuente| ErrorPrendas::Imagen { id: p.id.clone(), fuente })?;
        let miniatura = imagen.thumbnail(LADO_MINIATURA, LADO_MINIATURA);
        let salida = destino.join(format!("{}.png", p.id));
        miniatura.save(&salida).map_err(|e| ErrorPrendas::Miniatura {
            id: p.id.clone(),
            ruta: salida.clone(),
            fuente: std::io::Error::other(e),
        })?;
    }
    Ok(())
}
```

- [ ] **Paso 5: Integrar en `lib.rs`**

En `Estado` agrega `pub prendas: Arc<Vec<prendas::Prenda>>`. En `preparar`:
```rust
let lista = prendas::cargar(&conf.prendas)?;
let miniaturas = conf.datos.join("miniaturas");
prendas::generar_miniaturas(&conf.prendas, &lista, &miniaturas)?;
info!("{} prendas cargadas de {}", lista.len(), conf.prendas.display());
```
(`?` funciona porque `ErrorPrendas` implementa `std::error::Error`; anyhow lo convierte.)

- [ ] **Paso 6: Verificar y commitear**

`nix develop -c cargo test` → todo verde. `nix flake check` verde (clippy con `--deny warnings`). 
```bash
git add -A && git commit -m "feat: módulo prendas con validación y miniaturas"
```

---

### Tarea 4: Módulo `flujos`

**Files:**
- Create: `src/flujos.rs`, `pruebas/flujos/flujos.toml`, `pruebas/flujos/prueba.json`, `tests/flujos.rs`
- Modify: `src/lib.rs` (`pub mod flujos;`, campo `flujos: Arc<Vec<Flujo>>`, carga en `preparar`)

**Interfaces:**
- Consumes: `prendas::Tipo`.
- Produces:
  - `flujos::Peso { url, destino, sha256 }`.
  - `flujos::Flujo { nombre, titulo, descripcion, archivo: Option<String>, nodo_persona: Option<String>, nodo_prenda: Option<String>, nodo_tipo: Option<String>, nodo_salida: Option<String>, campo_persona: String, campo_prenda: String, campo_tipo: String, valores_tipo: HashMap<Tipo, String>, tipos: Vec<Tipo>, espera_segundos: u64, pesos: Vec<Peso>, grafo: Option<serde_json::Value> }` con `es_fijo(&self) -> bool` y `admite(&self, tipo: Tipo) -> bool`.
  - `flujos::Entradas<'a> { persona: &'a str, prenda: &'a str, tipo: Tipo }`.
  - `flujos::cargar(carpeta: &Path) -> Result<Vec<Flujo>, ErrorFlujos>` (agrega el flujo `fijo` si el TOML no lo declara).
  - `flujos::rellenar(flujo: &Flujo, entradas: &Entradas) -> Result<serde_json::Value, ErrorFlujos>` (pura).
  - `flujos::pesos_faltantes(flujo: &Flujo, modelos: &Path) -> Vec<String>`.
  - `flujos::buscar<'a>(flujos: &'a [Flujo], nombre: &str) -> Option<&'a Flujo>`.

- [ ] **Paso 1: Fixtures**

`pruebas/flujos/prueba.json` (formato API mínimo; los tipos de nodo no importan para la prueba de relleno):
```json
{
  "1": { "class_type": "LoadImage", "inputs": { "image": "PERSONA.jpg", "upload": "image" } },
  "2": { "class_type": "LoadImage", "inputs": { "image": "PRENDA.jpg", "upload": "image" } },
  "3": { "class_type": "TryOnFalso", "inputs": { "image": ["1", 0], "refer_image": ["2", 0], "cloth_type": "upper", "steps": 2 } },
  "4": { "class_type": "SaveImage", "inputs": { "images": ["3", 0], "filename_prefix": "galatea" } }
}
```
`pruebas/flujos/flujos.toml`:
```toml
[[flujo]]
nombre = "fijo"
titulo = "Fijo (sin GPU)"
descripcion = "Imágenes pregeneradas."
espera_segundos = 0

[[flujo]]
nombre = "prueba"
titulo = "Flujo de prueba"
descripcion = "Grafo mínimo para probar el relleno y el cliente de ComfyUI."
archivo = "prueba.json"
nodo_persona = "1"
nodo_prenda = "2"
nodo_tipo = "3"
nodo_salida = "4"
tipos = ["superior", "inferior", "vestido"]

[[flujo.peso]]
url = "https://example.invalid/peso.safetensors"
destino = "prueba/peso.safetensors"
sha256 = ""
```

- [ ] **Paso 2: Pruebas que fallan**

```rust
// tests/flujos.rs
use galatea::{flujos::{self, Entradas, ErrorFlujos}, prendas::Tipo};
use std::path::Path;

#[test]
fn carga_los_flujos_de_prueba_con_su_grafo() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let nombres: Vec<_> = flujos.iter().map(|f| f.nombre.as_str()).collect();
    assert_eq!(nombres, ["fijo", "prueba"]);
    let prueba = flujos::buscar(&flujos, "prueba").unwrap();
    assert!(prueba.grafo.is_some());
    assert!(prueba.admite(Tipo::Vestido));
    assert!(flujos::buscar(&flujos, "fijo").unwrap().es_fijo());
    assert!(flujos::buscar(&flujos, "fijo").unwrap().admite(Tipo::Inferior));
}

#[test]
fn agrega_fijo_si_el_toml_no_lo_declara() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy("pruebas/flujos/prueba.json", dir.path().join("prueba.json")).unwrap();
    std::fs::write(dir.path().join("flujos.toml"), r#"
[[flujo]]
nombre = "prueba"
titulo = "P"
archivo = "prueba.json"
nodo_persona = "1"
nodo_prenda = "2"
nodo_salida = "4"
tipos = ["superior"]
"#).unwrap();
    let flujos = flujos::cargar(dir.path()).unwrap();
    assert!(flujos.iter().any(|f| f.es_fijo()));
}

#[test]
fn rellenar_cambia_solo_los_nodos_declarados() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let prueba = flujos::buscar(&flujos, "prueba").unwrap();
    let lleno = flujos::rellenar(prueba, &Entradas { persona: "p.jpg", prenda: "r.png", tipo: Tipo::Inferior }).unwrap();
    assert_eq!(lleno["1"]["inputs"]["image"], "p.jpg");
    assert_eq!(lleno["2"]["inputs"]["image"], "r.png");
    assert_eq!(lleno["3"]["inputs"]["cloth_type"], "lower");
    assert_eq!(lleno["3"]["inputs"]["steps"], 2, "lo demás no cambia");
    assert_eq!(lleno["4"]["inputs"]["filename_prefix"], "galatea");
    // el original queda intacto
    assert_eq!(prueba.grafo.as_ref().unwrap()["1"]["inputs"]["image"], "PERSONA.jpg");
}

#[test]
fn rechaza_nodos_que_no_existen_en_el_grafo() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy("pruebas/flujos/prueba.json", dir.path().join("prueba.json")).unwrap();
    std::fs::write(dir.path().join("flujos.toml"), r#"
[[flujo]]
nombre = "roto"
titulo = "R"
archivo = "prueba.json"
nodo_persona = "99"
nodo_prenda = "2"
nodo_salida = "4"
tipos = ["superior"]
"#).unwrap();
    let e = flujos::cargar(dir.path()).unwrap_err();
    assert!(matches!(e, ErrorFlujos::NodoInexistente { ref nodo, .. } if nodo == "99"), "{e}");
}

#[test]
fn pesos_faltantes_lista_los_que_no_estan() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let prueba = flujos::buscar(&flujos, "prueba").unwrap();
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(flujos::pesos_faltantes(prueba, dir.path()), vec!["prueba/peso.safetensors".to_string()]);
    std::fs::create_dir_all(dir.path().join("prueba")).unwrap();
    std::fs::write(dir.path().join("prueba/peso.safetensors"), b"x").unwrap();
    assert!(flujos::pesos_faltantes(prueba, dir.path()).is_empty());
    assert!(flujos::pesos_faltantes(flujos::buscar(&flujos, "fijo").unwrap(), dir.path()).is_empty());
}
```

- [ ] **Paso 3: Correr para verla fallar** (`cargo test --test flujos` → import sin resolver).

- [ ] **Paso 4: Implementación `src/flujos.rs`**

```rust
//! Los motores como archivos: `flujos.toml` describe cada flujo de ComfyUI (formato API)
//! y qué nodos se rellenan. Rust no sabe qué modelo hay detrás.
use crate::prendas::Tipo;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::{HashMap, HashSet}, fs, path::{Path, PathBuf}};

pub const NOMBRE_FIJO: &str = "fijo";

#[derive(Clone, Debug, Deserialize)]
pub struct Peso {
    pub url: String,
    pub destino: String,
    #[serde(default)]
    pub sha256: String,
}

fn campo_imagen() -> String { "image".to_string() }
fn campo_tipo() -> String { "cloth_type".to_string() }
fn espera_por_omision() -> u64 { 8 }
fn valores_tipo_por_omision() -> HashMap<Tipo, String> {
    HashMap::from([
        (Tipo::Superior, "upper".to_string()),
        (Tipo::Inferior, "lower".to_string()),
        (Tipo::Vestido, "overall".to_string()),
    ])
}

#[derive(Clone, Debug, Deserialize)]
pub struct Flujo {
    pub nombre: String,
    pub titulo: String,
    #[serde(default)]
    pub descripcion: String,
    pub archivo: Option<String>,
    pub nodo_persona: Option<String>,
    pub nodo_prenda: Option<String>,
    pub nodo_tipo: Option<String>,
    pub nodo_salida: Option<String>,
    #[serde(default = "campo_imagen")]
    pub campo_persona: String,
    #[serde(default = "campo_imagen")]
    pub campo_prenda: String,
    #[serde(default = "campo_tipo")]
    pub campo_tipo: String,
    #[serde(default = "valores_tipo_por_omision")]
    pub valores_tipo: HashMap<Tipo, String>,
    #[serde(default)]
    pub tipos: Vec<Tipo>,
    #[serde(default = "espera_por_omision")]
    pub espera_segundos: u64,
    #[serde(default, rename = "peso")]
    pub pesos: Vec<Peso>,
    /// El grafo en formato API, cargado desde `archivo`. `None` en el flujo fijo.
    #[serde(skip)]
    pub grafo: Option<Value>,
}

impl Flujo {
    pub fn es_fijo(&self) -> bool {
        self.nombre == NOMBRE_FIJO
    }

    /// El fijo admite cualquier tipo: devuelve una imagen pregenerada.
    pub fn admite(&self, tipo: Tipo) -> bool {
        self.es_fijo() || self.tipos.contains(&tipo)
    }

    fn fijo_por_omision() -> Flujo {
        Flujo {
            nombre: NOMBRE_FIJO.to_string(),
            titulo: "Fijo (sin GPU)".to_string(),
            descripcion: "Devuelve una imagen pregenerada de fijo/.".to_string(),
            archivo: None,
            nodo_persona: None,
            nodo_prenda: None,
            nodo_tipo: None,
            nodo_salida: None,
            campo_persona: campo_imagen(),
            campo_prenda: campo_imagen(),
            campo_tipo: campo_tipo(),
            valores_tipo: valores_tipo_por_omision(),
            tipos: Vec::new(),
            espera_segundos: espera_por_omision(),
            pesos: Vec::new(),
            grafo: None,
        }
    }
}

#[derive(Deserialize)]
struct Archivo {
    #[serde(default)]
    flujo: Vec<Flujo>,
}

pub struct Entradas<'a> {
    pub persona: &'a str,
    pub prenda: &'a str,
    pub tipo: Tipo,
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorFlujos {
    #[error("no se pudo leer {ruta}: {fuente}")]
    Lectura { ruta: PathBuf, #[source] fuente: std::io::Error },
    #[error("{ruta} tiene un error de formato: {fuente}")]
    Formato { ruta: PathBuf, #[source] fuente: toml::de::Error },
    #[error("{ruta} no es un flujo válido en formato API de ComfyUI: {fuente}")]
    Json { ruta: PathBuf, #[source] fuente: serde_json::Error },
    #[error("el flujo «{0}» aparece dos veces en flujos.toml")]
    NombreRepetido(String),
    #[error("el flujo «{0}» no tiene archivo; todo flujo salvo «fijo» necesita archivo = \"<nombre>.json\"")]
    SinArchivo(String),
    #[error("el flujo «{flujo}» no declara {campo}; sin él no se sabe qué nodo rellenar")]
    SinNodo { flujo: String, campo: &'static str },
    #[error("el flujo «{flujo}» apunta al nodo «{nodo}», que no existe en su JSON; exporta el flujo otra vez y copia el id correcto")]
    NodoInexistente { flujo: String, nodo: String },
    #[error("el flujo «{flujo}» no tiene un valor para el tipo «{tipo}» en valores_tipo")]
    SinValorTipo { flujo: String, tipo: &'static str },
}

pub fn cargar(carpeta: &Path) -> Result<Vec<Flujo>, ErrorFlujos> {
    let ruta = carpeta.join("flujos.toml");
    let texto = fs::read_to_string(&ruta).map_err(|fuente| ErrorFlujos::Lectura { ruta: ruta.clone(), fuente })?;
    let archivo: Archivo = toml::from_str(&texto).map_err(|fuente| ErrorFlujos::Formato { ruta: ruta.clone(), fuente })?;
    let mut flujos = archivo.flujo;
    if !flujos.iter().any(Flujo::es_fijo) {
        flujos.insert(0, Flujo::fijo_por_omision());
    }
    let mut vistos = HashSet::new();
    for f in flujos.iter_mut() {
        if !vistos.insert(f.nombre.clone()) {
            return Err(ErrorFlujos::NombreRepetido(f.nombre.clone()));
        }
        if f.es_fijo() {
            continue;
        }
        let nombre_json = f.archivo.clone().ok_or_else(|| ErrorFlujos::SinArchivo(f.nombre.clone()))?;
        let ruta_json = carpeta.join(nombre_json);
        let texto = fs::read_to_string(&ruta_json).map_err(|fuente| ErrorFlujos::Lectura { ruta: ruta_json.clone(), fuente })?;
        let grafo: Value = serde_json::from_str(&texto).map_err(|fuente| ErrorFlujos::Json { ruta: ruta_json.clone(), fuente })?;
        for (campo, nodo) in [("nodo_persona", &f.nodo_persona), ("nodo_prenda", &f.nodo_prenda), ("nodo_salida", &f.nodo_salida)] {
            let nodo = nodo.as_ref().ok_or(ErrorFlujos::SinNodo { flujo: f.nombre.clone(), campo })?;
            comprobar_nodo(&grafo, &f.nombre, nodo)?;
        }
        if let Some(nodo) = &f.nodo_tipo {
            comprobar_nodo(&grafo, &f.nombre, nodo)?;
        }
        f.grafo = Some(grafo);
    }
    Ok(flujos)
}

fn comprobar_nodo(grafo: &Value, flujo: &str, nodo: &str) -> Result<(), ErrorFlujos> {
    if grafo.get(nodo).and_then(|n| n.get("inputs")).is_some() {
        Ok(())
    } else {
        Err(ErrorFlujos::NodoInexistente { flujo: flujo.to_string(), nodo: nodo.to_string() })
    }
}

pub fn buscar<'a>(flujos: &'a [Flujo], nombre: &str) -> Option<&'a Flujo> {
    flujos.iter().find(|f| f.nombre == nombre)
}

/// Copia el grafo y escribe los nombres de archivo y el tipo en los nodos declarados.
pub fn rellenar(flujo: &Flujo, entradas: &Entradas) -> Result<Value, ErrorFlujos> {
    let mut grafo = flujo.grafo.clone().ok_or_else(|| ErrorFlujos::SinArchivo(flujo.nombre.clone()))?;
    poner(&mut grafo, flujo, flujo.nodo_persona.as_deref(), "nodo_persona", &flujo.campo_persona, Value::from(entradas.persona))?;
    poner(&mut grafo, flujo, flujo.nodo_prenda.as_deref(), "nodo_prenda", &flujo.campo_prenda, Value::from(entradas.prenda))?;
    if let Some(nodo) = flujo.nodo_tipo.as_deref() {
        let valor = flujo.valores_tipo.get(&entradas.tipo).ok_or(ErrorFlujos::SinValorTipo {
            flujo: flujo.nombre.clone(),
            tipo: entradas.tipo.como_texto(),
        })?;
        poner(&mut grafo, flujo, Some(nodo), "nodo_tipo", &flujo.campo_tipo, Value::from(valor.as_str()))?;
    }
    Ok(grafo)
}

fn poner(grafo: &mut Value, flujo: &Flujo, nodo: Option<&str>, campo: &'static str, clave: &str, valor: Value) -> Result<(), ErrorFlujos> {
    let nodo = nodo.ok_or(ErrorFlujos::SinNodo { flujo: flujo.nombre.clone(), campo })?;
    let entradas = grafo
        .get_mut(nodo)
        .and_then(|n| n.get_mut("inputs"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| ErrorFlujos::NodoInexistente { flujo: flujo.nombre.clone(), nodo: nodo.to_string() })?;
    entradas.insert(clave.to_string(), valor);
    Ok(())
}

/// Los `destino` de `[[flujo.peso]]` que no están bajo `modelos/`.
pub fn pesos_faltantes(flujo: &Flujo, modelos: &Path) -> Vec<String> {
    flujo.pesos.iter().filter(|p| !modelos.join(&p.destino).is_file()).map(|p| p.destino.clone()).collect()
}
```

- [ ] **Paso 5: Integrar en `lib.rs`**: `pub mod flujos;`, campo `pub flujos: Arc<Vec<flujos::Flujo>>` en `Estado`, y en `preparar`:
```rust
let lista_flujos = flujos::cargar(&conf.flujos)?;
if flujos::buscar(&lista_flujos, &conf.flujo_por_defecto).is_none() {
    return Err(anyhow!("GALATEA_FLUJO_POR_DEFECTO={} no está en {}/flujos.toml", conf.flujo_por_defecto, conf.flujos.display()));
}
for f in &lista_flujos {
    let faltan = flujos::pesos_faltantes(f, &conf.modelos);
    if faltan.is_empty() { info!("flujo «{}» listo", f.nombre); } else { tracing::warn!("flujo «{}» sin pesos: {}; córrelo con scripts/descargar-modelos.sh {}", f.nombre, faltan.join(", "), f.nombre); }
}
```

- [ ] **Paso 6: Verificar y commitear** (`cargo test`, `nix flake check`; `git commit -m "feat: módulo flujos: carga, validación y relleno del grafo"`).

---

### Tarea 5: Módulo `sesion`

**Files:**
- Create: `src/sesion.rs`, `tests/sesion.rs`
- Modify: `src/lib.rs` (`pub mod sesion;`, campo `sesiones: sesion::Sesiones`, creación en `preparar`, tarea de fondo de vencimiento en `servir`)

**Interfaces:**
- Produces:
  - `sesion::Sesion { id: String, creada: Instant, carpeta: PathBuf, resultados: u32 }` (Clone).
  - `sesion::Sesiones` (Clone, compartible): `nueva(raiz: &Path, vencimiento: Duration) -> io::Result<Sesiones>` (vacía `raiz`), `generar_id() -> String` (32 hex), `crear(&self, foto: &[u8]) -> io::Result<String>`, `obtener(&self, id) -> Option<Sesion>`, `persona(&self, id) -> PathBuf`, `reservar_resultado(&self, id) -> Option<(u32, PathBuf)>`, `terminar(&self, id) -> bool`, `vencer_antiguas(&self, ahora: Instant) -> usize`, `cantidad(&self) -> usize`. Todos los métodos con `&self` son `async` salvo `generar_id`, `persona` y `nueva`.
  - `sesion::abreviar(id: &str) -> &str`.
  - `sesion::NOMBRE_PERSONA: &str = "persona.jpg"`.

- [ ] **Paso 1: Pruebas que fallan**

```rust
// tests/sesion.rs
use galatea::sesion::{self, Sesiones};
use std::time::{Duration, Instant};

#[tokio::test]
async fn crear_guarda_la_foto_y_terminar_la_borra() {
    let dir = tempfile::tempdir().unwrap();
    let s = Sesiones::nueva(dir.path(), Duration::from_secs(900)).unwrap();
    let id = s.crear(b"no es un jpg pero da igual aqui").await.unwrap();
    assert_eq!(id.len(), 32);
    assert!(s.persona(&id).is_file());
    let (n, ruta) = s.reservar_resultado(&id).await.unwrap();
    assert_eq!(n, 1);
    assert!(ruta.ends_with("resultado-1.png"));
    assert_eq!(s.reservar_resultado(&id).await.unwrap().0, 2);
    assert!(s.terminar(&id).await);
    assert!(!s.persona(&id).exists());
    assert!(s.obtener(&id).await.is_none());
    assert!(!s.terminar(&id).await, "terminar dos veces devuelve false");
}

#[tokio::test]
async fn vencer_borra_solo_las_viejas() {
    let dir = tempfile::tempdir().unwrap();
    let s = Sesiones::nueva(dir.path(), Duration::from_secs(60)).unwrap();
    let vieja = s.crear(b"a").await.unwrap();
    let nueva = s.crear(b"b").await.unwrap();
    // «ahora» es el reloj inyectado: 61 s después de crear ambas no vence nada distinto,
    // así que forzamos la antigüedad de una tocando su `creada`.
    s.envejecer_para_pruebas(&vieja, Duration::from_secs(120)).await;
    let vencidas = s.vencer_antiguas(Instant::now()).await;
    assert_eq!(vencidas, 1);
    assert!(s.obtener(&vieja).await.is_none());
    assert!(s.obtener(&nueva).await.is_some());
    assert_eq!(s.cantidad().await, 1);
}

#[test]
fn nueva_vacia_la_carpeta_al_arrancar() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("basura")).unwrap();
    std::fs::write(dir.path().join("basura/persona.jpg"), b"x").unwrap();
    let _s = Sesiones::nueva(dir.path(), Duration::from_secs(1)).unwrap();
    assert!(!dir.path().join("basura").exists());
    assert!(dir.path().is_dir());
}

#[test]
fn abreviar_deja_seis_caracteres() {
    assert_eq!(sesion::abreviar("0123456789abcdef"), "012345");
    assert_eq!(sesion::abreviar("abc"), "abc");
}
```

- [ ] **Paso 2: Correr para verla fallar.**

- [ ] **Paso 3: Implementación `src/sesion.rs`**

```rust
//! Sesiones de la demo: una foto, sus resultados, y su borrado. Único módulo que crea y
//! borra carpetas bajo `datos/sesiones/`.
use std::{collections::HashMap, io, path::{Path, PathBuf}, sync::Arc, time::{Duration, Instant}};
use tokio::sync::RwLock;

pub const NOMBRE_PERSONA: &str = "persona.jpg";

#[derive(Clone, Debug)]
pub struct Sesion {
    pub id: String,
    pub creada: Instant,
    pub carpeta: PathBuf,
    pub resultados: u32,
}

#[derive(Clone)]
pub struct Sesiones {
    interior: Arc<RwLock<HashMap<String, Sesion>>>,
    raiz: PathBuf,
    vencimiento: Duration,
}

/// Los primeros seis caracteres: lo único de un id que puede ir al log.
pub fn abreviar(id: &str) -> &str {
    &id[..id.len().min(6)]
}

impl Sesiones {
    /// Vacía `raiz` por completo: una sesión de una ejecución anterior ya venció por definición.
    pub fn nueva(raiz: &Path, vencimiento: Duration) -> io::Result<Self> {
        if raiz.exists() {
            std::fs::remove_dir_all(raiz)?;
        }
        std::fs::create_dir_all(raiz)?;
        Ok(Self { interior: Arc::new(RwLock::new(HashMap::new())), raiz: raiz.to_path_buf(), vencimiento })
    }

    pub fn generar_id() -> String {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).expect("el sistema debería poder dar bytes aleatorios");
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub async fn crear(&self, foto: &[u8]) -> io::Result<String> {
        let id = Self::generar_id();
        let carpeta = self.raiz.join(&id);
        tokio::fs::create_dir_all(&carpeta).await?;
        tokio::fs::write(carpeta.join(NOMBRE_PERSONA), foto).await?;
        self.interior.write().await.insert(id.clone(), Sesion { id: id.clone(), creada: Instant::now(), carpeta, resultados: 0 });
        tracing::info!("sesión {} creada", abreviar(&id));
        Ok(id)
    }

    pub async fn obtener(&self, id: &str) -> Option<Sesion> {
        self.interior.read().await.get(id).cloned()
    }

    pub fn persona(&self, id: &str) -> PathBuf {
        self.raiz.join(id).join(NOMBRE_PERSONA)
    }

    /// Reserva el siguiente número de resultado y devuelve su ruta (aún no escrita).
    pub async fn reservar_resultado(&self, id: &str) -> Option<(u32, PathBuf)> {
        let mut mapa = self.interior.write().await;
        let s = mapa.get_mut(id)?;
        s.resultados += 1;
        Some((s.resultados, s.carpeta.join(format!("resultado-{}.png", s.resultados))))
    }

    pub async fn terminar(&self, id: &str) -> bool {
        let quitada = self.interior.write().await.remove(id);
        match quitada {
            Some(s) => {
                if let Err(e) = tokio::fs::remove_dir_all(&s.carpeta).await {
                    tracing::error!("no se pudo borrar la carpeta de la sesión {}: {e}", abreviar(id));
                }
                tracing::info!("sesión {} terminada y borrada", abreviar(id));
                true
            }
            None => false,
        }
    }

    /// Borra las sesiones más viejas que `vencimiento` respecto de `ahora`. Devuelve cuántas.
    pub async fn vencer_antiguas(&self, ahora: Instant) -> usize {
        let vencidas: Vec<String> = self
            .interior
            .read()
            .await
            .values()
            .filter(|s| ahora.duration_since(s.creada) >= self.vencimiento)
            .map(|s| s.id.clone())
            .collect();
        let mut n = 0;
        for id in vencidas {
            if self.terminar(&id).await {
                n += 1;
            }
        }
        n
    }

    pub async fn cantidad(&self) -> usize {
        self.interior.read().await.len()
    }

    /// Solo pruebas: retrasa `creada` para simular el paso del tiempo sin dormir.
    #[cfg(any(test, feature = "pruebas"))]
    pub async fn envejecer_para_pruebas(&self, id: &str, cuanto: Duration) {
        if let Some(s) = self.interior.write().await.get_mut(id) {
            s.creada = Instant::now().checked_sub(cuanto).expect("Instant no debería desbordar");
        }
    }
}
```

- [ ] **Paso 4: Integrar en `lib.rs`**: `pub mod sesion;`; `Estado.sesiones: sesion::Sesiones`; en `preparar`: `let sesiones = sesion::Sesiones::nueva(&conf.datos.join("sesiones"), conf.vencimiento)?;`; en `servir`, antes de `axum::serve`:
```rust
let vencedor = estado.sesiones.clone();
tokio::spawn(async move {
    let mut cada = tokio::time::interval(Duration::from_secs(60));
    loop {
        cada.tick().await;
        let n = vencedor.vencer_antiguas(Instant::now()).await;
        if n > 0 { info!("{n} sesiones vencidas y borradas"); }
    }
});
```

- [ ] **Paso 5: Verificar y commitear** (`git commit -m "feat: sesiones en memoria con borrado y vencimiento"`).

---

### Tarea 6: Módulo `marca` (marca de agua)

**Files:**
- Create: `estaticos/marca.png`, `src/marca.rs`, `tests/marca.rs`
- Modify: `src/lib.rs` (`pub mod marca;`)

**Interfaces:**
- Produces: `marca::aplicar(png: &[u8]) -> Result<Vec<u8>, image::ImageError>`; `marca::TEXTO: &str = "imagen generada con IA"`.

- [ ] **Paso 1: Generar el sello una vez** (PNG RGBA con texto blanco sobre banda negra semitransparente)

```bash
nix develop -c python3 - <<'PY'
from PIL import Image, ImageDraw, ImageFont
import glob
fuente = glob.glob("/nix/store/*dejavu-fonts*/share/fonts/truetype/DejaVuSans-Bold.ttf")[0]
f = ImageFont.truetype(fuente, 36)
texto = "imagen generada con IA"
ancho = int(f.getlength(texto)) + 40
im = Image.new("RGBA", (ancho, 60), (0, 0, 0, 140))
ImageDraw.Draw(im).text((20, 10), texto, font=f, fill=(255, 255, 255, 230))
im.save("estaticos/marca.png", optimize=True)
print(im.size)
PY
```

- [ ] **Paso 2: Prueba que falla**

```rust
// tests/marca.rs
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
    assert_ne!(dentro, Rgba([128, 128, 128, 255]), "la banda inferior derecha lleva el sello");
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
```

- [ ] **Paso 3: Correr para verla fallar.**

- [ ] **Paso 4: Implementación `src/marca.rs`**

```rust
//! Marca de agua «imagen generada con IA» en la esquina inferior derecha.
use image::{DynamicImage, GenericImageView, ImageFormat, imageops};
use std::io::Cursor;

pub const TEXTO: &str = "imagen generada con IA";
const SELLO: &[u8] = include_bytes!("../estaticos/marca.png");

/// El sello ocupa el 45 % del ancho de la imagen, con un margen del 2 %.
pub fn aplicar(png: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    let mut base = image::load_from_memory(png)?.to_rgba8();
    let sello = image::load_from_memory(SELLO)?;
    let (ancho, alto) = base.dimensions();
    let ancho_sello = (ancho as f32 * 0.45).max(40.0) as u32;
    let alto_sello = (ancho_sello as f32 * sello.height() as f32 / sello.width() as f32).max(8.0) as u32;
    let sello = sello.resize_exact(ancho_sello, alto_sello, imageops::FilterType::Triangle).to_rgba8();
    let margen = (ancho as f32 * 0.02) as i64;
    let x = ancho as i64 - ancho_sello as i64 - margen;
    let y = alto as i64 - alto_sello as i64 - margen;
    imageops::overlay(&mut base, &sello, x.max(0), y.max(0));
    let mut salida = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(base).write_to(&mut salida, ImageFormat::Png)?;
    Ok(salida.into_inner())
}
```

- [ ] **Paso 5: Verificar y commitear** (`git add estaticos/marca.png src/marca.rs tests/marca.rs src/lib.rs && git commit -m "feat: marca de agua en las imágenes generadas"`).

---

### Tarea 7: Motor `Fijo` y despachador `Motores`

**Files:**
- Create: `src/motor/mod.rs`, `src/motor/fijo.rs`, `pruebas/flujos/fijo/generico.png`, `pruebas/flujos/fijo/roja.png`, `flujos/fijo/generico.png`, `tests/motor_fijo.rs`
- Modify: `src/lib.rs` (`pub mod motor;`, campo `motores: Arc<motor::Motores>` en `Estado`, construcción en `preparar`)

**Interfaces:**
- Consumes: `flujos::Flujo`, `prendas::Tipo`.
- Produces:
  - `motor::Avance { EnCola(usize), Progreso(f32), Listo, Error(String) }` (Clone, Debug, PartialEq).
  - `motor::Trabajo { flujo: Flujo, persona: PathBuf, prenda: PathBuf, prenda_id: String, tipo: Tipo, destino: PathBuf, etiqueta: String }` (`etiqueta` = id de sesión abreviado, para nombres de archivo en ComfyUI y para el log).
  - `motor::Motores { fijo: fijo::Fijo, comfy: Option<comfy::Comfy>, turno: Arc<Semaphore> }` con `nuevo(fijo, comfy) -> Motores`, `generar(&self, trabajo: Trabajo) -> mpsc::Receiver<Avance>` (encola, adquiere el turno, corre, y el último mensaje es siempre `Listo` o `Error`), `disponible(&self) -> Result<(), String>`.
  - `motor::fijo::Fijo { carpeta: PathBuf }` con `nuevo(carpeta) -> Fijo` y `generar(&self, trabajo: &Trabajo, tx: &mpsc::Sender<Avance>) -> Result<(), String>`.
  - `motor::comfy::Comfy` se define en la Tarea 8; aquí `comfy` queda `None` y `mod comfy` se crea vacío con un `struct Comfy;` provisional **solo si hace falta para compilar** (mejor: crea `src/motor/comfy.rs` con el `struct Comfy` y `generar`/`disponible` que devuelven `Err("todavía no implementado")`, y la Tarea 8 lo reemplaza).

- [ ] **Paso 1: Fixtures**: copia `pruebas/prendas/roja.png` a `pruebas/flujos/fijo/roja.png` y genera `pruebas/flujos/fijo/generico.png` y `flujos/fijo/generico.png` con Pillow (600×800, fondo gris claro, un rectángulo y el texto «Fijo»), igual que en la Tarea 3.

- [ ] **Paso 2: Prueba que falla**

```rust
// tests/motor_fijo.rs
use galatea::{flujos, motor::{Avance, Motores, Trabajo, fijo::Fijo}, prendas::Tipo};
use std::{path::{Path, PathBuf}, sync::Arc};

fn trabajo(flujos: &[flujos::Flujo], prenda_id: &str, destino: PathBuf) -> Trabajo {
    Trabajo {
        flujo: flujos::buscar(flujos, "fijo").unwrap().clone(),
        persona: PathBuf::from("pruebas/imagenes/persona-prueba.jpg"),
        prenda: PathBuf::from("pruebas/prendas/roja.png"),
        prenda_id: prenda_id.to_string(),
        tipo: Tipo::Superior,
        destino,
        etiqueta: "abcdef".to_string(),
    }
}

async fn recoger(mut rx: tokio::sync::mpsc::Receiver<Avance>) -> Vec<Avance> {
    let mut v = Vec::new();
    while let Some(a) = rx.recv().await { v.push(a); }
    v
}

#[tokio::test]
async fn fijo_copia_la_imagen_de_la_prenda_si_existe() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("resultado-1.png");
    let motores = Arc::new(Motores::nuevo(Fijo::nuevo(Path::new("pruebas/flujos/fijo")), None));
    let avances = recoger(motores.generar(trabajo(&flujos, "roja", destino.clone()))).await;
    assert_eq!(avances.last(), Some(&Avance::Listo));
    assert!(avances.iter().any(|a| matches!(a, Avance::Progreso(_))));
    assert_eq!(std::fs::read(&destino).unwrap(), std::fs::read("pruebas/flujos/fijo/roja.png").unwrap());
}

#[tokio::test]
async fn fijo_usa_generico_si_no_hay_imagen_de_la_prenda() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("r.png");
    let motores = Arc::new(Motores::nuevo(Fijo::nuevo(Path::new("pruebas/flujos/fijo")), None));
    let avances = recoger(motores.generar(trabajo(&flujos, "azul", destino.clone()))).await;
    assert_eq!(avances.last(), Some(&Avance::Listo));
    assert_eq!(std::fs::read(&destino).unwrap(), std::fs::read("pruebas/flujos/fijo/generico.png").unwrap());
}

#[tokio::test]
async fn sin_carpeta_fijo_el_error_dice_que_falta() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let motores = Arc::new(Motores::nuevo(Fijo::nuevo(&dir.path().join("no-existe")), None));
    let avances = recoger(motores.generar(trabajo(&flujos, "roja", dir.path().join("r.png")))).await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("generico.png"), "{m}"),
        otro => panic!("esperaba Error, hubo {otro:?}"),
    }
}

#[tokio::test]
async fn un_flujo_de_comfy_sin_comfy_configurado_da_error_claro() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let motores = Arc::new(Motores::nuevo(Fijo::nuevo(Path::new("pruebas/flujos/fijo")), None));
    let mut t = trabajo(&flujos, "roja", dir.path().join("r.png"));
    t.flujo = flujos::buscar(&flujos, "prueba").unwrap().clone();
    let avances = recoger(motores.generar(t)).await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("GALATEA_COMFY_URL"), "{m}"),
        otro => panic!("esperaba Error, hubo {otro:?}"),
    }
}

#[tokio::test]
async fn dos_trabajos_a_la_vez_el_segundo_espera_su_turno() {
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let motores = Arc::new(Motores::nuevo(Fijo::nuevo(Path::new("pruebas/flujos/fijo")), None));
    // El primero tarda un segundo para que el segundo lo encuentre ocupando el turno.
    // Con el runtime de un hilo de #[tokio::test], las tareas arrancan en orden de spawn.
    let mut primero = trabajo(&flujos, "roja", dir.path().join("1.png"));
    primero.flujo.espera_segundos = 1;
    let rx1 = motores.generar(primero);
    let rx2 = motores.generar(trabajo(&flujos, "roja", dir.path().join("2.png")));
    let (a1, a2) = tokio::join!(recoger(rx1), recoger(rx2));
    assert_eq!(a1.last(), Some(&Avance::Listo));
    assert_eq!(a2.last(), Some(&Avance::Listo));
    assert_eq!(a1.first(), Some(&Avance::EnCola(0)));
    assert_eq!(a2.first(), Some(&Avance::EnCola(1)), "el segundo vio uno delante: {a2:?}");
}
```
`persona-prueba.jpg` se genera en la Tarea 10; para esta prueba basta con que la ruta exista o no: el Fijo no la lee. Si prefieres, genera ya `pruebas/imagenes/persona-prueba.jpg` con Pillow (480×720, fondo claro, silueta oscura) y commitéalo.

- [ ] **Paso 3: Correr para verla fallar.**

- [ ] **Paso 4: Implementación `src/motor/mod.rs`**

```rust
//! Los motores de generación. `Motores` decide según el flujo: `fijo` va al motor Fijo,
//! cualquier otro a ComfyUI. Un solo trabajo corre a la vez (la GPU no se reparte).
pub mod comfy;
pub mod fijo;

use crate::{flujos::Flujo, prendas::Tipo};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{Semaphore, mpsc};

#[derive(Clone, Debug, PartialEq)]
pub enum Avance {
    /// Cuántos trabajos hay delante (0 = el siguiente en entrar).
    EnCola(usize),
    /// 0.0 a 1.0.
    Progreso(f32),
    /// `trabajo.destino` ya está escrito.
    Listo,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct Trabajo {
    pub flujo: Flujo,
    pub persona: PathBuf,
    pub prenda: PathBuf,
    pub prenda_id: String,
    pub tipo: Tipo,
    pub destino: PathBuf,
    /// Id de sesión abreviado: nombra archivos en ComfyUI y aparece en el log.
    pub etiqueta: String,
}

pub struct Motores {
    pub fijo: fijo::Fijo,
    pub comfy: Option<comfy::Comfy>,
    turno: Arc<Semaphore>,
    esperando: Arc<std::sync::atomic::AtomicUsize>,
}

impl Motores {
    pub fn nuevo(fijo: fijo::Fijo, comfy: Option<comfy::Comfy>) -> Self {
        Self { fijo, comfy, turno: Arc::new(Semaphore::new(1)), esperando: Arc::default() }
    }

    /// Encola el trabajo y devuelve el canal por el que llegan los avances.
    /// El último mensaje es siempre `Listo` o `Error`, y después el canal se cierra.
    pub fn generar(self: &Arc<Self>, trabajo: Trabajo) -> mpsc::Receiver<Avance> {
        let (tx, rx) = mpsc::channel(32);
        let motores = Arc::clone(self);
        tokio::spawn(async move {
            use std::sync::atomic::Ordering;
            let delante = motores.esperando.fetch_add(1, Ordering::SeqCst);
            let _ = tx.send(Avance::EnCola(delante)).await;
            let _permiso = motores.turno.acquire().await.expect("el semáforo no se cierra");
            motores.esperando.fetch_sub(1, Ordering::SeqCst);
            let resultado = if trabajo.flujo.es_fijo() {
                motores.fijo.generar(&trabajo, &tx).await
            } else {
                match &motores.comfy {
                    Some(c) => c.generar(&trabajo, &tx).await,
                    None => Err("El motor de imágenes no está configurado: define GALATEA_COMFY_URL (por ejemplo http://127.0.0.1:8188) o usa el flujo Fijo.".to_string()),
                }
            };
            let ultimo = match resultado {
                Ok(()) => Avance::Listo,
                Err(mensaje) => {
                    tracing::warn!("trabajo {} falló: {mensaje}", trabajo.etiqueta);
                    Avance::Error(mensaje)
                }
            };
            let _ = tx.send(ultimo).await;
        });
        rx
    }

    /// ¿Responde ComfyUI? El Fijo siempre está disponible.
    pub async fn disponible(&self) -> Result<(), String> {
        match &self.comfy {
            Some(c) => c.disponible().await,
            None => Err("sin GALATEA_COMFY_URL: solo el flujo Fijo".to_string()),
        }
    }
}
```

Nota: `generar` toma `self: &Arc<Self>`; `Estado.motores` es `Arc<Motores>`, así que en `web` se llama `estado.motores.generar(trabajo)`.

`src/motor/fijo.rs`:
```rust
//! Devuelve `carpeta/<prenda_id>.png` o `carpeta/generico.png` tras una espera simulada.
use super::{Avance, Trabajo};
use std::{path::{Path, PathBuf}, time::Duration};
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct Fijo {
    pub carpeta: PathBuf,
}

impl Fijo {
    pub fn nuevo(carpeta: &Path) -> Self {
        Self { carpeta: carpeta.to_path_buf() }
    }

    pub async fn generar(&self, trabajo: &Trabajo, tx: &mpsc::Sender<Avance>) -> Result<(), String> {
        let propia = self.carpeta.join(format!("{}.png", trabajo.prenda_id));
        let generico = self.carpeta.join("generico.png");
        let origen = if propia.is_file() { propia } else if generico.is_file() { generico } else {
            return Err(format!(
                "el flujo Fijo no encontró {} ni {}; agrega al menos generico.png en esa carpeta",
                self.carpeta.join(format!("{}.png", trabajo.prenda_id)).display(),
                self.carpeta.join("generico.png").display()
            ));
        };
        let pasos = 10u32;
        let paso = Duration::from_millis(trabajo.flujo.espera_segundos * 1000 / u64::from(pasos));
        for i in 1..=pasos {
            tokio::time::sleep(paso).await;
            let _ = tx.send(Avance::Progreso(i as f32 / pasos as f32)).await;
        }
        if let Some(padre) = trabajo.destino.parent() {
            tokio::fs::create_dir_all(padre).await.map_err(|e| format!("no se pudo crear {}: {e}", padre.display()))?;
        }
        tokio::fs::copy(&origen, &trabajo.destino)
            .await
            .map_err(|e| format!("no se pudo copiar la imagen fija a {}: {e}", trabajo.destino.display()))?;
        Ok(())
    }
}
```

`src/motor/comfy.rs` provisional (la Tarea 8 lo reemplaza entero):
```rust
//! Cliente de ComfyUI. Provisional hasta la Tarea 8.
use super::{Avance, Trabajo};
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct Comfy;

impl Comfy {
    pub async fn generar(&self, _trabajo: &Trabajo, _tx: &mpsc::Sender<Avance>) -> Result<(), String> {
        Err("el cliente de ComfyUI todavía no está implementado".to_string())
    }
    pub async fn disponible(&self) -> Result<(), String> {
        Err("el cliente de ComfyUI todavía no está implementado".to_string())
    }
}
```

- [ ] **Paso 5: Integrar en `lib.rs`**: `pub mod motor;`; `Estado.motores: Arc<motor::Motores>`; en `preparar`: `let motores = Arc::new(motor::Motores::nuevo(motor::fijo::Fijo::nuevo(&conf.flujos.join("fijo")), None));` (la Tarea 8 construye `Some(Comfy)` cuando hay `comfy_url`).

- [ ] **Paso 6: Verificar y commitear** (`git commit -m "feat: motor fijo y despachador con turno único"`).

---

### Tarea 8: Cliente de ComfyUI (`motor::comfy`) probado contra un ComfyUI falso

**Files:**
- Create (reemplazar): `src/motor/comfy.rs`; Create: `tests/comun/comfy_falso.rs`, `tests/motor_comfy.rs`, `pruebas/comfy/salida.png`
- Modify: `src/lib.rs` (`preparar` construye `Comfy` si hay `comfy_url`), `tests/comun/mod.rs` (`pub mod comfy_falso;`)

**Interfaces:**
- Consumes: `flujos::rellenar`, `flujos::Entradas`, `motor::{Avance, Trabajo}`.
- Produces: `motor::comfy::Comfy` con `nuevo(base_url: &str, limite: Duration, datos: Option<PathBuf>) -> Comfy`, `generar(&self, trabajo: &Trabajo, tx: &mpsc::Sender<Avance>) -> Result<(), String>`, `disponible(&self) -> Result<(), String>`.
- API de ComfyUI que se usa (v0.35.0): `POST /upload/image` (multipart `image`, `overwrite=true`, `type=input`; responde `{"name","subfolder","type"}`), `POST /prompt` (`{"prompt": <grafo>, "client_id"}` → `{"prompt_id","number","node_errors"}`; con error de validación responde 400 y `{"error":{"message"},"node_errors"}`), `GET /ws?clientId=<id>` (mensajes JSON `{"type","data"}` de tipos `status`, `execution_start`, `executing`, `progress`, `executed`, `execution_error`, `execution_interrupted`, y frames binarios de vista previa que se ignoran), `GET /history/<prompt_id>` (`{<prompt_id>: {"outputs": {<nodo>: {"images": [{"filename","subfolder","type"}]}}, "status": {...}}}`), `GET /view?filename=&subfolder=&type=output`, `POST /interrupt`, `GET /system_stats`.

- [ ] **Paso 1: El ComfyUI falso `tests/comun/comfy_falso.rs`**

```rust
//! Un servidor axum que imita lo justo de la API de ComfyUI para probar `motor::comfy`.
#![allow(dead_code)]
use axum::{
    Json, Router,
    extract::{Multipart, Path, Query, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::{Arc, Mutex}};

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
}

#[derive(Clone)]
pub struct Falso {
    pub guion: Guion,
    /// Nombres de archivo subidos por /upload/image.
    pub subidos: Arc<Mutex<Vec<String>>>,
    /// El último grafo recibido en /prompt.
    pub grafo: Arc<Mutex<Option<Value>>>,
    pub interrupciones: Arc<Mutex<usize>>,
    /// Carpeta que hace de `datos/comfy` (input/ y output/): el falso escribe ahí la salida.
    pub datos: std::path::PathBuf,
}

pub const NODO_SALIDA: &str = "4";
pub const ARCHIVO_SALIDA: &str = "galatea_00001_.png";

pub async fn levantar(guion: Guion, datos: &std::path::Path) -> (String, Falso) {
    std::fs::create_dir_all(datos.join("input")).unwrap();
    std::fs::create_dir_all(datos.join("output")).unwrap();
    let falso = Falso { guion, subidos: Arc::default(), grafo: Arc::default(), interrupciones: Arc::default(), datos: datos.to_path_buf() };
    let router = Router::new()
        .route("/system_stats", get(|| async { Json(json!({"devices": [{"name": "falsa", "vram_total": 1, "vram_free": 1}]})) }))
        .route("/upload/image", post(subir))
        .route("/prompt", post(prompt))
        .route("/ws", get(ws))
        .route("/history/{id}", get(historial))
        .route("/view", get(ver))
        .route("/interrupt", post(interrumpir))
        .with_state(falso.clone());
    let escucha = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let puerto = escucha.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(escucha, router).await.ok(); });
    (format!("http://127.0.0.1:{puerto}"), falso)
}

async fn subir(State(f): State<Falso>, mut partes: Multipart) -> Json<Value> {
    let mut nombre = String::new();
    while let Some(campo) = partes.next_field().await.unwrap() {
        if campo.name() == Some("image") {
            nombre = campo.file_name().unwrap_or("sin-nombre").to_string();
            let bytes = campo.bytes().await.unwrap();
            std::fs::write(f.datos.join("input").join(&nombre), &bytes).unwrap();
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

async fn guion_ws(mut socket: WebSocket, f: Falso) {
    let enviar = |socket: &mut WebSocket, v: Value| {
        let texto = v.to_string();
        async move { socket.send(Message::Text(texto.into())).await }
    };
    let _ = enviar(&mut socket, json!({"type": "status", "data": {"status": {"exec_info": {"queue_remaining": 1}}}})).await;
    match f.guion {
        Guion::Colgado => {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                if enviar(&mut socket, json!({"type": "progress", "data": {"value": 1, "max": 100, "prompt_id": "pid-1", "node": "3"}})).await.is_err() { break; }
            }
        }
        Guion::ErrorEnEjecucion => {
            let _ = enviar(&mut socket, json!({"type": "execution_start", "data": {"prompt_id": "pid-1"}})).await;
            let _ = enviar(&mut socket, json!({"type": "execution_error", "data": {"prompt_id": "pid-1", "node_id": "3", "node_type": "TryOnFalso", "exception_message": "CUDA out of memory"}})).await;
        }
        _ => {
            let _ = enviar(&mut socket, json!({"type": "execution_start", "data": {"prompt_id": "pid-1"}})).await;
            for v in 1..=3 {
                let _ = enviar(&mut socket, json!({"type": "progress", "data": {"value": v, "max": 3, "prompt_id": "pid-1", "node": "3"}})).await;
            }
            std::fs::write(f.datos.join("output").join(ARCHIVO_SALIDA), include_bytes!("../../pruebas/comfy/salida.png")).unwrap();
            let _ = enviar(&mut socket, json!({"type": "executed", "data": {"prompt_id": "pid-1", "node": NODO_SALIDA, "output": {"images": [{"filename": ARCHIVO_SALIDA, "subfolder": "", "type": "output"}]}}})).await;
            let _ = enviar(&mut socket, json!({"type": "executing", "data": {"prompt_id": "pid-1", "node": null}})).await;
        }
    }
    // Mantén el socket abierto hasta que el cliente cierre.
    while let Some(Ok(_)) = socket.recv().await {}
}

async fn historial(Path(id): Path<String>) -> Json<Value> {
    Json(json!({ id: { "outputs": { NODO_SALIDA: { "images": [{"filename": ARCHIVO_SALIDA, "subfolder": "", "type": "output"}] } }, "status": {"completed": true} } }))
}

async fn ver(State(f): State<Falso>, Query(q): Query<HashMap<String, String>>) -> Response {
    let ruta = f.datos.join(q.get("type").map(String::as_str).unwrap_or("output")).join(q.get("filename").cloned().unwrap_or_default());
    match std::fs::read(&ruta) {
        Ok(bytes) => ([(header::CONTENT_TYPE, "image/png")], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn interrumpir(State(f): State<Falso>) -> StatusCode {
    *f.interrupciones.lock().unwrap() += 1;
    StatusCode::OK
}
```

Genera `pruebas/comfy/salida.png` con Pillow (256×384, un degradado cualquiera) y commitéalo. En `Cargo.toml`, `[dev-dependencies]` necesita `axum` con `ws` y `multipart` (ya está) y `tokio` con `time` (ya).

- [ ] **Paso 2: Pruebas del cliente `tests/motor_comfy.rs`**

```rust
mod comun;

use comun::comfy_falso::{self, Guion};
use galatea::{flujos, motor::{Avance, Motores, Trabajo, comfy::Comfy, fijo::Fijo}, prendas::Tipo};
use std::{path::{Path, PathBuf}, sync::Arc, time::Duration};

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
    while let Some(a) = rx.recv().await { v.push(a); }
    v
}

fn motores(url: &str, limite: Duration, datos: &Path) -> Arc<Motores> {
    Arc::new(Motores::nuevo(Fijo::nuevo(Path::new("pruebas/flujos/fijo")), Some(Comfy::nuevo(url, limite, Some(datos.to_path_buf())))))
}

#[tokio::test]
async fn sube_rellena_sigue_el_progreso_descarga_y_limpia() {
    let dir = tempfile::tempdir().unwrap();
    let datos = dir.path().join("comfy");
    let (url, falso) = comfy_falso::levantar(Guion::Exito, &datos).await;
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let destino = dir.path().join("resultado-1.png");
    let avances = recoger(motores(&url, Duration::from_secs(5), &datos).generar(trabajo(&flujos, destino.clone()))).await;

    assert_eq!(avances.last(), Some(&Avance::Listo), "{avances:?}");
    assert!(avances.contains(&Avance::Progreso(1.0)));
    assert_eq!(std::fs::read(&destino).unwrap(), include_bytes!("../pruebas/comfy/salida.png"));

    let subidos = falso.subidos.lock().unwrap().clone();
    assert_eq!(subidos.len(), 2);
    assert!(subidos.iter().any(|n| n.starts_with("galatea-abcdef-persona")), "{subidos:?}");
    assert!(subidos.iter().any(|n| n == "galatea-prenda-roja.png"), "{subidos:?}");

    let grafo = falso.grafo.lock().unwrap().clone().unwrap();
    assert_eq!(grafo["1"]["inputs"]["image"], subidos[0]);
    assert_eq!(grafo["2"]["inputs"]["image"], "galatea-prenda-roja.png");
    assert_eq!(grafo["3"]["inputs"]["cloth_type"], "lower");

    // limpieza: ni la entrada ni la salida quedan en la carpeta compartida
    assert!(!datos.join("output").join(comfy_falso::ARCHIVO_SALIDA).exists());
    assert!(!datos.join("input").join(&subidos[0]).exists());
}

#[tokio::test]
async fn un_prompt_rechazado_explica_el_nodo() {
    let dir = tempfile::tempdir().unwrap();
    let datos = dir.path().join("comfy");
    let (url, _falso) = comfy_falso::levantar(Guion::Rechazo, &datos).await;
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let avances = recoger(motores(&url, Duration::from_secs(5), &datos).generar(trabajo(&flujos, dir.path().join("r.png")))).await;
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
    let avances = recoger(motores(&url, Duration::from_secs(5), &datos).generar(trabajo(&flujos, dir.path().join("r.png")))).await;
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
    let avances = recoger(motores(&url, Duration::from_millis(700), &datos).generar(trabajo(&flujos, dir.path().join("r.png")))).await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("segundos") || m.contains("tiempo"), "{m}"),
        otro => panic!("{otro:?}"),
    }
    assert_eq!(*falso.interrupciones.lock().unwrap(), 1);
}

#[tokio::test]
async fn comfy_apagado_da_un_error_que_dice_como_levantarlo() {
    let dir = tempfile::tempdir().unwrap();
    let flujos = flujos::cargar(Path::new("pruebas/flujos")).unwrap();
    let m = motores("http://127.0.0.1:1", Duration::from_secs(2), dir.path());
    assert!(m.disponible().await.unwrap_err().contains("docker compose up"));
    let avances = recoger(m.generar(trabajo(&flujos, dir.path().join("r.png")))).await;
    match avances.last() {
        Some(Avance::Error(m)) => assert!(m.contains("docker compose up"), "{m}"),
        otro => panic!("{otro:?}"),
    }
}
```

- [ ] **Paso 3: Correr para verlas fallar** (compilan contra el `Comfy` provisional y fallan por el mensaje).

- [ ] **Paso 4: Implementación `src/motor/comfy.rs`**

```rust
//! El único módulo que habla con ComfyUI: sube las imágenes, encola el flujo, sigue el
//! progreso por WebSocket, descarga la salida y borra lo que dejó en la carpeta compartida.
use super::{Avance, Trabajo};
use crate::flujos::{self, Entradas};
use futures_util::StreamExt;
use serde_json::Value;
use std::{path::{Path, PathBuf}, time::Duration};
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

impl Comfy {
    pub fn nuevo(base: &str, limite: Duration, datos: Option<PathBuf>) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            cliente: reqwest::Client::builder().timeout(Duration::from_secs(60)).build().expect("cliente HTTP"),
            limite,
            datos,
        }
    }

    pub async fn disponible(&self) -> Result<(), String> {
        let r = self.cliente.get(format!("{}/system_stats", self.base)).send().await.map_err(|_| COMO_LEVANTARLO.to_string())?;
        if r.status().is_success() { Ok(()) } else { Err(format!("ComfyUI respondió {} a /system_stats", r.status())) }
    }

    pub async fn generar(&self, trabajo: &Trabajo, tx: &mpsc::Sender<Avance>) -> Result<(), String> {
        let nombre_persona = format!("galatea-{}-persona.jpg", trabajo.etiqueta);
        let nombre_prenda = format!("galatea-prenda-{}.{}", trabajo.prenda_id, extension(&trabajo.prenda));
        let subida_persona = self.subir(&trabajo.persona, &nombre_persona).await?;
        let subida_prenda = self.subir(&trabajo.prenda, &nombre_prenda).await?;
        let resultado = self.correr(trabajo, &subida_persona, &subida_prenda, tx).await;
        // Pase lo que pase, la foto de la persona no se queda en ComfyUI.
        self.borrar_en_datos("input", &subida_persona);
        if let Ok(salida) = &resultado {
            self.borrar_en_datos(&salida.tipo, &salida.filename);
        }
        resultado.map(|_| ())
    }

    async fn correr(&self, trabajo: &Trabajo, persona: &str, prenda: &str, tx: &mpsc::Sender<Avance>) -> Result<Salida, String> {
        let grafo = flujos::rellenar(&trabajo.flujo, &Entradas { persona, prenda, tipo: trabajo.tipo }).map_err(|e| e.to_string())?;
        let client_id = crate::sesion::Sesiones::generar_id();
        let ws_url = format!("{}/ws?clientId={client_id}", self.base.replacen("http", "ws", 1));
        // El socket se abre ANTES de encolar: si no, los primeros mensajes se pierden.
        let (ws, _) = tokio_tungstenite::connect_async(&ws_url).await.map_err(|_| COMO_LEVANTARLO.to_string())?;
        let prompt_id = self.encolar(grafo, &client_id).await?;
        let nodo_salida = trabajo.flujo.nodo_salida.clone();
        let seguimiento = self.seguir(ws, &prompt_id, nodo_salida.as_deref(), tx);
        let salida = match tokio::time::timeout(self.limite, seguimiento).await {
            Ok(r) => r?,
            Err(_) => {
                self.interrumpir().await;
                return Err(format!(
                    "la generación pasó de {} segundos y se interrumpió; prueba de nuevo o elige un flujo más liviano",
                    self.limite.as_secs()
                ));
            }
        };
        self.descargar(&salida, &trabajo.destino).await?;
        Ok(salida)
    }

    async fn subir(&self, ruta: &Path, nombre: &str) -> Result<String, String> {
        let bytes = tokio::fs::read(ruta).await.map_err(|e| format!("no se pudo leer {}: {e}", ruta.display()))?;
        let parte = reqwest::multipart::Part::bytes(bytes).file_name(nombre.to_string()).mime_str(mime_de(nombre)).map_err(|e| e.to_string())?;
        let formulario = reqwest::multipart::Form::new().part("image", parte).text("overwrite", "true").text("type", "input");
        let r = self.cliente.post(format!("{}/upload/image", self.base)).multipart(formulario).send().await.map_err(|_| COMO_LEVANTARLO.to_string())?;
        if !r.status().is_success() {
            return Err(format!("ComfyUI rechazó la subida de la imagen ({})", r.status()));
        }
        let cuerpo: Value = r.json().await.map_err(|e| format!("respuesta rara al subir la imagen: {e}"))?;
        Ok(cuerpo["name"].as_str().unwrap_or(nombre).to_string())
    }

    async fn encolar(&self, grafo: Value, client_id: &str) -> Result<String, String> {
        let r = self.cliente
            .post(format!("{}/prompt", self.base))
            .json(&serde_json::json!({ "prompt": grafo, "client_id": client_id }))
            .send().await.map_err(|_| COMO_LEVANTARLO.to_string())?;
        let estado = r.status();
        let cuerpo: Value = r.json().await.map_err(|e| format!("respuesta rara de ComfyUI al encolar: {e}"))?;
        if !estado.is_success() {
            return Err(explicar_rechazo(&cuerpo));
        }
        cuerpo["prompt_id"].as_str().map(str::to_string).ok_or_else(|| "ComfyUI no devolvió prompt_id".to_string())
    }

    async fn seguir(
        &self,
        mut ws: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        prompt_id: &str,
        nodo_salida: Option<&str>,
        tx: &mpsc::Sender<Avance>,
    ) -> Result<Salida, String> {
        let mut salida: Option<Salida> = None;
        let mut empezado = false;
        while let Some(mensaje) = ws.next().await {
            let texto = match mensaje {
                Ok(Message::Text(t)) => t,
                Ok(Message::Binary(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => continue,
                Ok(Message::Close(_)) => break,
                Ok(Message::Frame(_)) => continue,
                Err(e) => return Err(format!("se cortó la conexión con ComfyUI: {e}")),
            };
            let Ok(v) = serde_json::from_str::<Value>(texto.as_str()) else { continue };
            let tipo = v["type"].as_str().unwrap_or("");
            let datos = &v["data"];
            let es_nuestro = datos["prompt_id"].as_str() == Some(prompt_id);
            match tipo {
                "status" if !empezado => {
                    let restantes = datos["status"]["exec_info"]["queue_remaining"].as_u64().unwrap_or(0) as usize;
                    let _ = tx.send(Avance::EnCola(restantes.saturating_sub(1))).await;
                }
                "execution_start" if es_nuestro => {
                    empezado = true;
                    let _ = tx.send(Avance::Progreso(0.0)).await;
                }
                "progress" if es_nuestro => {
                    let (valor, maximo) = (datos["value"].as_f64().unwrap_or(0.0), datos["max"].as_f64().unwrap_or(1.0).max(1.0));
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
                        "ComfyUI falló en el nodo {} ({}): {}",
                        datos["node_id"].as_str().unwrap_or("?"),
                        datos["node_type"].as_str().unwrap_or("?"),
                        datos["exception_message"].as_str().unwrap_or("sin detalle")
                    ));
                }
                "execution_interrupted" if es_nuestro => return Err("la generación se interrumpió en ComfyUI".to_string()),
                _ => {}
            }
        }
        Err("ComfyUI cerró la conexión antes de terminar".to_string())
    }

    async fn salida_del_historial(&self, prompt_id: &str, nodo_salida: Option<&str>) -> Result<Salida, String> {
        let h: Value = self.cliente.get(format!("{}/history/{prompt_id}", self.base)).send().await.map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
        let salidas = &h[prompt_id]["outputs"];
        let candidatos: Vec<&Value> = match nodo_salida {
            Some(n) => vec![&salidas[n]],
            None => salidas.as_object().map(|m| m.values().collect()).unwrap_or_default(),
        };
        candidatos.into_iter().find_map(primera_imagen).ok_or_else(|| "el flujo terminó sin producir ninguna imagen; revisa nodo_salida en flujos.toml".to_string())
    }

    async fn descargar(&self, salida: &Salida, destino: &Path) -> Result<(), String> {
        let r = self.cliente
            .get(format!("{}/view", self.base))
            .query(&[("filename", salida.filename.as_str()), ("subfolder", salida.subfolder.as_str()), ("type", salida.tipo.as_str())])
            .send().await.map_err(|e| format!("no se pudo bajar la imagen de ComfyUI: {e}"))?;
        if !r.status().is_success() {
            return Err(format!("ComfyUI respondió {} al pedir la imagen generada", r.status()));
        }
        let bytes = r.bytes().await.map_err(|e| e.to_string())?;
        if let Some(padre) = destino.parent() {
            tokio::fs::create_dir_all(padre).await.map_err(|e| e.to_string())?;
        }
        tokio::fs::write(destino, &bytes).await.map_err(|e| format!("no se pudo guardar {}: {e}", destino.display()))
    }

    async fn interrumpir(&self) {
        let _ = self.cliente.post(format!("{}/interrupt", self.base)).send().await;
    }

    fn borrar_en_datos(&self, carpeta: &str, nombre: &str) {
        if let Some(datos) = &self.datos {
            let ruta = datos.join(carpeta).join(nombre);
            if ruta.is_file() && let Err(e) = std::fs::remove_file(&ruta) {
                tracing::warn!("no se pudo borrar {} en la carpeta de ComfyUI: {e}", carpeta);
            }
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
                partes.push(format!("nodo {nodo}: {} ({})", err["message"].as_str().unwrap_or(""), err["details"].as_str().unwrap_or("")));
            }
        }
    }
    partes.join("; ")
}

fn extension(ruta: &Path) -> String {
    ruta.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase()
}

fn mime_de(nombre: &str) -> &'static str {
    if nombre.ends_with(".jpg") || nombre.ends_with(".jpeg") { "image/jpeg" } else { "image/png" }
}
```

Notas: `Option::is_none_or` y `if let ... && let ...` requieren Rust ≥ 1.88 (edición 2024): con 1.98 están. Si clippy se queja de `replacen("http", "ws", 1)` con `https`, está bien: produce `wss`.

- [ ] **Paso 5: Integrar en `lib.rs` `preparar`**:
```rust
let comfy = conf.comfy_url.as_ref().map(|url| motor::comfy::Comfy::nuevo(url, conf.limite, conf.comfy_datos.clone()));
let motores = Arc::new(motor::Motores::nuevo(motor::fijo::Fijo::nuevo(&conf.flujos.join("fijo")), comfy));
match motores.disponible().await { Ok(()) => info!("ComfyUI responde"), Err(e) => tracing::warn!("{e}") }
```

- [ ] **Paso 6: Verificar y commitear** (`cargo test --test motor_comfy`, luego todo; `nix flake check`; `git commit -m "feat: cliente de ComfyUI con progreso por WebSocket y limpieza"`).

---

### Tarea 9: Rutas web (JSON y SSE) probadas con el motor Fijo

**Files:**
- Create: `src/web/prendas.rs`, `src/web/sesiones.rs`, `src/web/flujos.rs`, `tests/web.rs`
- Modify: `src/web/mod.rs`, `src/web/salud.rs`, `src/lib.rs` (rutas, campo `trabajos` en `Estado`)

**Interfaces:**
- Consumes: todo lo anterior.
- Produces las rutas de la especificación §8:

| Método y ruta | Entrada | Salida |
|---|---|---|
| `GET /prendas` | — | `200 [{ "id", "nombre", "tipo", "miniatura": "/prendas/<id>/miniatura.png" }]` |
| `GET /prendas/{id}/miniatura.png` | — | PNG o 404 |
| `POST /sesiones` | multipart, campo `persona` (JPEG/PNG ≤ 15 MB) | `201 { "id" }`; 422 si no es imagen |
| `POST /sesiones/{id}/generar` | JSON `{ "prenda": "<id>", "flujo": "<nombre>"? }` | `202 { "trabajo", "resultado": n }`; 404 sesión/prenda/flujo; 422 tipo no admitido |
| `GET /sesiones/{id}/avance/{trabajo}` | — | SSE: eventos `encola` (n), `progreso` (0.0–1.0), `listo` (n del resultado), `error` (mensaje); 404 si no hay tal trabajo |
| `GET /sesiones/{id}/resultado/{n}.png` | — | PNG con marca de agua o 404 |
| `GET /sesiones/{id}/persona.jpg` | — | JPEG o 404 |
| `DELETE /sesiones/{id}` | — | 204 o 404 |
| `GET /flujos` | — | `200 [{ "nombre", "titulo", "descripcion", "tipos", "disponible", "faltantes" }]` |
| `GET /salud` | — | `{ "programa", "version", "comfy": "ok" \| "<error>", "flujos": [{ "nombre", "faltantes" }], "sesiones": n }` |

- Tipo interno: `web::sesiones::Trabajos = Arc<tokio::sync::Mutex<HashMap<String, TrabajoActivo>>>` con `TrabajoActivo { rx: mpsc::Receiver<Avance>, resultado: u32 }`; campo `trabajos: Trabajos` en `Estado`.

- [ ] **Paso 1: Pruebas de integración que fallan (`tests/web.rs`)**

```rust
mod comun;

use futures_util::StreamExt;
use serde_json::{Value, json};

async fn subir_persona(s: &comun::Servidor) -> String {
    let bytes = std::fs::read("pruebas/prendas/azul.png").unwrap();
    let parte = reqwest::multipart::Part::bytes(bytes).file_name("foto.png").mime_str("image/png").unwrap();
    let r = s.cliente.post(s.url("/sesiones")).multipart(reqwest::multipart::Form::new().part("persona", parte)).send().await.unwrap();
    assert_eq!(r.status(), 201);
    r.json::<Value>().await.unwrap()["id"].as_str().unwrap().to_string()
}

/// Lee el SSE hasta `listo` o `error` y devuelve la lista de (evento, dato).
async fn leer_sse(s: &comun::Servidor, ruta: &str) -> Vec<(String, String)> {
    let r = s.cliente.get(s.url(ruta)).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.headers()["content-type"].to_str().unwrap().starts_with("text/event-stream"));
    let mut flujo = r.bytes_stream();
    let mut texto = String::new();
    let mut eventos = Vec::new();
    while let Some(trozo) = flujo.next().await {
        texto.push_str(std::str::from_utf8(&trozo.unwrap()).unwrap());
        while let Some(fin) = texto.find("\n\n") {
            let bloque: String = texto.drain(..fin + 2).collect();
            let evento = bloque.lines().find_map(|l| l.strip_prefix("event: ")).unwrap_or("message").to_string();
            let dato = bloque.lines().find_map(|l| l.strip_prefix("data: ")).unwrap_or("").to_string();
            let terminal = evento == "listo" || evento == "error";
            eventos.push((evento, dato));
            if terminal { return eventos; }
        }
    }
    eventos
}

#[tokio::test]
async fn lista_prendas_y_sirve_miniaturas() {
    let s = comun::Servidor::nuevo().await;
    let prendas: Vec<Value> = s.cliente.get(s.url("/prendas")).send().await.unwrap().json().await.unwrap();
    assert_eq!(prendas.len(), 2);
    assert_eq!(prendas[0]["id"], "roja");
    assert_eq!(prendas[0]["tipo"], "superior");
    let r = s.cliente.get(s.url(prendas[0]["miniatura"].as_str().unwrap())).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()["content-type"], "image/png");
    assert_eq!(s.cliente.get(s.url("/prendas/nada/miniatura.png")).send().await.unwrap().status(), 404);
}

#[tokio::test]
async fn recorrido_completo_con_el_flujo_fijo() {
    let s = comun::Servidor::nuevo().await;
    let id = subir_persona(&s).await;

    let r = s.cliente.post(s.url(&format!("/sesiones/{id}/generar"))).json(&json!({"prenda": "roja"})).send().await.unwrap();
    assert_eq!(r.status(), 202);
    let cuerpo: Value = r.json().await.unwrap();
    let trabajo = cuerpo["trabajo"].as_str().unwrap().to_string();
    assert_eq!(cuerpo["resultado"], 1);

    let eventos = leer_sse(&s, &format!("/sesiones/{id}/avance/{trabajo}")).await;
    assert_eq!(eventos.first().unwrap().0, "encola");
    assert!(eventos.iter().any(|(e, _)| e == "progreso"));
    assert_eq!(eventos.last().unwrap(), &("listo".to_string(), "1".to_string()));

    let r = s.cliente.get(s.url(&format!("/sesiones/{id}/resultado/1.png"))).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let png = r.bytes().await.unwrap();
    let im = image::load_from_memory(&png).unwrap();
    assert_eq!(im.width(), 300, "la fija de «roja» es la de pruebas/flujos/fijo/roja.png");
    // la marca de agua cambió la esquina respecto del archivo fijo original
    let original = image::open("pruebas/flujos/fijo/roja.png").unwrap();
    // el sello mide 135 px de ancho y termina a 6 px del borde: (250, 388) está dentro
    assert_ne!(image::GenericImageView::get_pixel(&im, 250, 388), image::GenericImageView::get_pixel(&original, 250, 388));

    assert_eq!(s.cliente.get(s.url(&format!("/sesiones/{id}/persona.jpg"))).send().await.unwrap().status(), 200);

    // segundo trabajo con la misma foto reutiliza la sesión y numera 2
    let r = s.cliente.post(s.url(&format!("/sesiones/{id}/generar"))).json(&json!({"prenda": "azul", "flujo": "fijo"})).send().await.unwrap();
    assert_eq!(r.json::<Value>().await.unwrap()["resultado"], 2);

    assert_eq!(s.cliente.delete(s.url(&format!("/sesiones/{id}"))).send().await.unwrap().status(), 204);
    assert_eq!(s.cliente.get(s.url(&format!("/sesiones/{id}/resultado/1.png"))).send().await.unwrap().status(), 404);
    assert_eq!(s.cliente.get(s.url(&format!("/sesiones/{id}/persona.jpg"))).send().await.unwrap().status(), 404);
    assert_eq!(s.cliente.delete(s.url(&format!("/sesiones/{id}"))).send().await.unwrap().status(), 404);
}

#[tokio::test]
async fn rechaza_lo_que_no_es_imagen_y_lo_que_no_existe() {
    let s = comun::Servidor::nuevo().await;
    let parte = reqwest::multipart::Part::bytes(b"hola".to_vec()).file_name("x.jpg").mime_str("image/jpeg").unwrap();
    let r = s.cliente.post(s.url("/sesiones")).multipart(reqwest::multipart::Form::new().part("persona", parte)).send().await.unwrap();
    assert_eq!(r.status(), 422);
    assert!(r.json::<Value>().await.unwrap()["error"].as_str().unwrap().contains("imagen"));

    let id = subir_persona(&s).await;
    let r = s.cliente.post(s.url(&format!("/sesiones/{id}/generar"))).json(&json!({"prenda": "verde"})).send().await.unwrap();
    assert_eq!(r.status(), 404);
    let r = s.cliente.post(s.url(&format!("/sesiones/{id}/generar"))).json(&json!({"prenda": "roja", "flujo": "inventado"})).send().await.unwrap();
    assert_eq!(r.status(), 404);
    let r = s.cliente.post(s.url("/sesiones/00000000000000000000000000000000/generar")).json(&json!({"prenda": "roja"})).send().await.unwrap();
    assert_eq!(r.status(), 404);
    assert_eq!(s.cliente.get(s.url(&format!("/sesiones/{id}/avance/nada"))).send().await.unwrap().status(), 404);
}

#[tokio::test]
async fn un_flujo_que_no_admite_el_tipo_de_la_prenda_da_422() {
    // «prueba» admite los tres tipos; lo restringimos con un TOML propio.
    let temporal = tempfile::tempdir().unwrap();
    let flujos = temporal.path().join("flujos");
    std::fs::create_dir_all(flujos.join("fijo")).unwrap();
    std::fs::copy("pruebas/flujos/prueba.json", flujos.join("prueba.json")).unwrap();
    std::fs::copy("pruebas/flujos/fijo/generico.png", flujos.join("fijo/generico.png")).unwrap();
    std::fs::write(flujos.join("flujos.toml"), r#"
[[flujo]]
nombre = "prueba"
titulo = "Solo superior"
archivo = "prueba.json"
nodo_persona = "1"
nodo_prenda = "2"
nodo_salida = "4"
tipos = ["superior"]
"#).unwrap();
    let mut conf = galatea::Conf::para_pruebas(temporal.path());
    conf.flujos = flujos;
    let s = comun::Servidor::con_conf(conf, temporal).await;
    let id = subir_persona(&s).await;
    let r = s.cliente.post(s.url(&format!("/sesiones/{id}/generar"))).json(&json!({"prenda": "azul", "flujo": "prueba"})).send().await.unwrap();
    assert_eq!(r.status(), 422);
    assert!(r.json::<Value>().await.unwrap()["error"].as_str().unwrap().contains("vestido"));
}

#[tokio::test]
async fn flujos_y_salud_informan_pesos_faltantes() {
    let s = comun::Servidor::nuevo().await;
    let flujos: Vec<Value> = s.cliente.get(s.url("/flujos")).send().await.unwrap().json().await.unwrap();
    let prueba = flujos.iter().find(|f| f["nombre"] == "prueba").unwrap();
    assert_eq!(prueba["disponible"], false);
    assert_eq!(prueba["faltantes"][0], "prueba/peso.safetensors");
    let fijo = flujos.iter().find(|f| f["nombre"] == "fijo").unwrap();
    assert_eq!(fijo["disponible"], true);

    let salud: Value = s.cliente.get(s.url("/salud")).send().await.unwrap().json().await.unwrap();
    assert!(salud["comfy"].as_str().unwrap().contains("GALATEA_COMFY_URL"));
    assert_eq!(salud["sesiones"], 0);
}
```
Agrega `futures-util = "0.3"` a `[dev-dependencies]` (ya está en `[dependencies]`; hace falta declararla para las pruebas de integración).

- [ ] **Paso 2: Correr para verlas fallar.**

- [ ] **Paso 3: `src/web/prendas.rs`**

```rust
//! GET /prendas y sus miniaturas.
use crate::{Estado, prendas, web::error::Fallo};
use axum::{Json, extract::{Path, State}, http::header, response::{IntoResponse, Response}};
use serde::Serialize;

#[derive(Serialize)]
pub struct PrendaVista {
    pub id: String,
    pub nombre: String,
    pub tipo: &'static str,
    pub miniatura: String,
}

pub async fn listar(State(estado): State<Estado>) -> Json<Vec<PrendaVista>> {
    Json(estado.prendas.iter().map(|p| PrendaVista {
        id: p.id.clone(),
        nombre: p.nombre.clone(),
        tipo: p.tipo.como_texto(),
        miniatura: format!("/prendas/{}/miniatura.png", p.id),
    }).collect())
}

pub async fn miniatura(State(estado): State<Estado>, Path(id): Path<String>) -> Response {
    if prendas::buscar(&estado.prendas, &id).is_none() {
        return Fallo::no_encontrado("no hay una prenda con ese id");
    }
    match tokio::fs::read(estado.conf.datos.join("miniaturas").join(format!("{id}.png"))).await {
        Ok(bytes) => ([(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "public, max-age=3600")], bytes).into_response(),
        Err(e) => {
            tracing::error!("miniatura de «{id}» ilegible: {e}");
            Fallo::interno("la miniatura no está en disco; reinicia el servidor para regenerarlas")
        }
    }
}
```

- [ ] **Paso 4: `src/web/sesiones.rs`**

```rust
//! Sesiones: subir la foto, generar, seguir el avance, ver resultados y terminar.
use crate::{Estado, flujos, marca, motor::{Avance, Trabajo}, prendas, sesion, web::error::Fallo};
use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response, sse::{Event, KeepAlive, Sse}},
};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, io::Cursor, sync::Arc};
use tokio::sync::{Mutex, mpsc};

pub const MAXIMO_FOTO: usize = 15 * 1024 * 1024;

pub struct TrabajoActivo {
    pub rx: mpsc::Receiver<Avance>,
    pub resultado: u32,
}

pub type Trabajos = Arc<Mutex<HashMap<String, TrabajoActivo>>>;

/// POST /sesiones: recibe la foto, la endereza según EXIF, la guarda como JPEG.
pub async fn crear(State(estado): State<Estado>, mut partes: Multipart) -> Response {
    let mut foto: Option<Vec<u8>> = None;
    while let Ok(Some(campo)) = partes.next_field().await {
        if campo.name() == Some("persona") {
            match campo.bytes().await {
                Ok(b) if b.len() <= MAXIMO_FOTO => foto = Some(b.to_vec()),
                Ok(_) => return Fallo::invalido("la foto pesa más de 15 MB; baja la resolución de la cámara"),
                Err(e) => return Fallo::invalido(format!("no se pudo leer la foto: {e}")),
            }
        }
    }
    let Some(foto) = foto else {
        return Fallo::invalido("falta el campo «persona» con la foto");
    };
    let jpeg = match normalizar_jpeg(&foto) {
        Ok(j) => j,
        Err(_) => return Fallo::invalido("el archivo no es una imagen que se pueda leer (usa JPEG o PNG)"),
    };
    match estado.sesiones.crear(&jpeg).await {
        Ok(id) => (StatusCode::CREATED, Json(json!({ "id": id }))).into_response(),
        Err(e) => {
            tracing::error!("no se pudo crear la sesión: {e}");
            Fallo::interno("no se pudo guardar la foto en el servidor; revisa el espacio en disco")
        }
    }
}

/// Decodifica, aplica la orientación EXIF (los celulares la usan) y reencodifica a JPEG 90.
fn normalizar_jpeg(bytes: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    use image::{ImageDecoder, ImageReader, codecs::jpeg::JpegEncoder};
    let mut lector = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    lector.no_limits();
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
pub async fn generar(State(estado): State<Estado>, Path(id): Path<String>, Json(p): Json<PeticionGenerar>) -> Response {
    if estado.sesiones.obtener(&id).await.is_none() {
        return Fallo::no_encontrado("esa sesión no existe o ya venció (15 minutos); toma una foto nueva");
    }
    let Some(prenda) = prendas::buscar(&estado.prendas, &p.prenda) else {
        return Fallo::no_encontrado("no hay una prenda con ese id");
    };
    let nombre_flujo = p.flujo.unwrap_or_else(|| estado.conf.flujo_por_defecto.clone());
    let Some(flujo) = flujos::buscar(&estado.flujos, &nombre_flujo) else {
        return Fallo::no_encontrado(&format!("no hay un flujo llamado «{nombre_flujo}»"));
    };
    if !flujo.admite(prenda.tipo) {
        return Fallo::invalido(format!(
            "el flujo «{}» no admite prendas de tipo {}; elige otro flujo u otra prenda",
            flujo.titulo, prenda.tipo.como_texto()
        ));
    }
    let Some((numero, destino)) = estado.sesiones.reservar_resultado(&id).await else {
        return Fallo::no_encontrado("esa sesión venció mientras se preparaba el trabajo");
    };
    let trabajo = Trabajo {
        flujo: flujo.clone(),
        persona: estado.sesiones.persona(&id),
        prenda: prendas::ruta(&estado.conf.prendas, prenda),
        prenda_id: prenda.id.clone(),
        tipo: prenda.tipo,
        destino: destino.clone(),
        etiqueta: sesion::abreviar(&id).to_string(),
    };
    let mut rx_motor = estado.motores.generar(trabajo);
    // Entre el motor y el SSE va la marca de agua: cuando llega Listo, el PNG se sella en su sitio.
    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async move {
        while let Some(avance) = rx_motor.recv().await {
            let avance = match avance {
                Avance::Listo => match sellar(&destino).await {
                    Ok(()) => Avance::Listo,
                    Err(e) => Avance::Error(format!("la imagen se generó pero no se pudo marcar: {e}")),
                },
                otro => otro,
            };
            if tx.send(avance).await.is_err() {
                break;
            }
        }
    });
    let trabajo_id = sesion::Sesiones::generar_id();
    estado.trabajos.lock().await.insert(trabajo_id.clone(), TrabajoActivo { rx, resultado: numero });
    (StatusCode::ACCEPTED, Json(json!({ "trabajo": trabajo_id, "resultado": numero }))).into_response()
}

async fn sellar(ruta: &std::path::Path) -> Result<(), String> {
    let bytes = tokio::fs::read(ruta).await.map_err(|e| e.to_string())?;
    let sellado = marca::aplicar(&bytes).map_err(|e| e.to_string())?;
    tokio::fs::write(ruta, sellado).await.map_err(|e| e.to_string())
}

/// GET /sesiones/{id}/avance/{trabajo}: SSE hasta `listo` o `error`.
pub async fn avance(State(estado): State<Estado>, Path((id, trabajo)): Path<(String, String)>) -> Response {
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
    Sse::new(eventos).keep_alive(KeepAlive::default()).into_response()
}

/// GET /sesiones/{id}/resultado/{n}.png
pub async fn resultado(State(estado): State<Estado>, Path((id, archivo)): Path<(String, String)>) -> Response {
    let Some(n) = archivo.strip_suffix(".png").and_then(|s| s.parse::<u32>().ok()) else {
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
        Ok(bytes) => ([(header::CONTENT_TYPE, tipo), (header::CACHE_CONTROL, "no-store")], bytes).into_response(),
        Err(_) => Fallo::no_encontrado("esa imagen no existe o todavía no se generó"),
    }
}

/// DELETE /sesiones/{id}
pub async fn terminar(State(estado): State<Estado>, Path(id): Path<String>) -> Response {
    if estado.sesiones.terminar(&id).await { StatusCode::NO_CONTENT.into_response() } else { Fallo::no_encontrado("esa sesión no existe o ya se había borrado") }
}
```

`use futures_util::StreamExt;` hace falta para `.map` sobre `ReceiverStream` (`tokio-stream` ya está en `[dependencies]` desde la Tarea 1). Si `ImageDecoder::orientation` no existe en la versión de `image` que instales, usa `image::metadata::Orientation` según su documentación; el objetivo es que una foto con EXIF girado salga derecha.

- [ ] **Paso 5: `src/web/flujos.rs` y `src/web/salud.rs`**

```rust
// src/web/flujos.rs
//! GET /flujos: qué motores hay y cuáles tienen sus pesos.
use crate::{Estado, flujos};
use axum::{Json, extract::State};
use serde::Serialize;

#[derive(Serialize)]
pub struct FlujoVista {
    pub nombre: String,
    pub titulo: String,
    pub descripcion: String,
    pub tipos: Vec<&'static str>,
    pub disponible: bool,
    pub faltantes: Vec<String>,
}

pub async fn listar(State(estado): State<Estado>) -> Json<Vec<FlujoVista>> {
    Json(estado.flujos.iter().map(|f| {
        let faltantes = flujos::pesos_faltantes(f, &estado.conf.modelos);
        FlujoVista {
            nombre: f.nombre.clone(),
            titulo: f.titulo.clone(),
            descripcion: f.descripcion.clone(),
            tipos: if f.es_fijo() { vec!["superior", "inferior", "vestido"] } else { f.tipos.iter().map(|t| t.como_texto()).collect() },
            disponible: faltantes.is_empty(),
            faltantes,
        }
    }).collect())
}
```

```rust
// src/web/salud.rs
//! GET /salud: versión, ComfyUI, pesos por flujo, sesiones vivas.
use crate::{Estado, flujos};
use axum::{Json, extract::State};
use serde_json::{Value, json};

pub async fn ver(State(estado): State<Estado>) -> Json<Value> {
    let comfy = match estado.motores.disponible().await { Ok(()) => "ok".to_string(), Err(e) => e };
    let lista: Vec<Value> = estado.flujos.iter().map(|f| json!({ "nombre": f.nombre, "faltantes": flujos::pesos_faltantes(f, &estado.conf.modelos) })).collect();
    Json(json!({
        "programa": "galatea",
        "version": env!("CARGO_PKG_VERSION"),
        "comfy": comfy,
        "flujos": lista,
        "sesiones": estado.sesiones.cantidad().await,
    }))
}
```

- [ ] **Paso 6: Rutas en `lib.rs`**

```rust
use axum::{extract::DefaultBodyLimit, routing::{delete, post}};
// en construir_router:
Router::new()
    .route("/salud", get(web::salud::ver))
    .route("/prendas", get(web::prendas::listar))
    .route("/prendas/{id}/miniatura.png", get(web::prendas::miniatura))
    .route("/flujos", get(web::flujos::listar))
    .route("/sesiones", post(web::sesiones::crear).layer(DefaultBodyLimit::max(web::sesiones::MAXIMO_FOTO + 64 * 1024)))
    .route("/sesiones/{id}/generar", post(web::sesiones::generar))
    .route("/sesiones/{id}/avance/{trabajo}", get(web::sesiones::avance))
    .route("/sesiones/{id}/resultado/{archivo}", get(web::sesiones::resultado))
    .route("/sesiones/{id}/persona.jpg", get(web::sesiones::persona))
    .route("/sesiones/{id}", delete(web::sesiones::terminar))
    .fallback(...)
```
y `Estado.trabajos: web::sesiones::Trabajos` inicializado con `Arc::default()` en `preparar`. `src/web/mod.rs` declara `pub mod flujos; pub mod prendas; pub mod sesiones;`.

- [ ] **Paso 7: Verificar y commitear** (`cargo test --test web`, todo, `nix flake check`; `git commit -m "feat: rutas de sesiones, generación con SSE, prendas y flujos"`).

---

### Tarea 10: La página (cámara, estados, SSE) y la prueba de navegador

**Files:**
- Create: `plantillas/index.html`, `estaticos/estilo.css`, `estaticos/app.js`, `src/web/pagina.rs`, `src/web/estaticos.rs`, `pruebas/imagenes/persona-prueba.jpg`, `playwright.config.js`, `package.json`, `tests/e2e/demo.spec.js`
- Modify: `src/web/mod.rs`, `src/lib.rs` (rutas `/`, `/estaticos/app.js`, `/estaticos/estilo.css`)

**Interfaces:**
- Consumes: las rutas JSON/SSE de la Tarea 9.
- Produces: `GET /` (HTML), `GET /estaticos/estilo.css`, `GET /estaticos/app.js`. En la página, atributos `data-estado` y `data-accion` estables para Playwright: `[data-estado="aviso"|"prendas"|"camara"|"generando"|"resultado"]`, botones `[data-accion="entendido"|"capturar"|"usar"|"repetir"|"otra-prenda"|"repetir-foto"|"descargar"|"terminar"|"reintentar"]`, `input[type=file][data-accion="archivo"]`, `[data-prenda="<id>"]`, `img[data-imagen="resultado"]`, `img[data-imagen="original"]`, `[data-mensaje]` para errores y confirmaciones, `select[data-flujos]` visible solo con `?flujos=1`.

- [ ] **Paso 1: `src/web/pagina.rs` y `src/web/estaticos.rs`**

```rust
// src/web/pagina.rs
//! GET /: la única página. Todo el estado vive en el navegador (estaticos/app.js).
use askama::Template;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "index.html")]
struct Pagina {
    version: &'static str,
}

pub async fn inicio() -> Response {
    Html(Pagina { version: env!("CARGO_PKG_VERSION") }.render().expect("index.html no debería fallar al renderizar")).into_response()
}
```

```rust
// src/web/estaticos.rs
//! CSS y JS dentro del binario: la demo no depende de un directorio de estáticos al desplegar.
use axum::{http::{HeaderValue, header}, response::{IntoResponse, Response}};

const ESTILO: &str = include_str!("../../estaticos/estilo.css");
const APP: &str = include_str!("../../estaticos/app.js");

pub async fn estilo() -> Response {
    ([(header::CONTENT_TYPE, HeaderValue::from_static("text/css; charset=utf-8")), (header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))], ESTILO).into_response()
}

pub async fn app() -> Response {
    ([(header::CONTENT_TYPE, HeaderValue::from_static("text/javascript; charset=utf-8")), (header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))], APP).into_response()
}
```
Rutas: `.route("/", get(web::pagina::inicio)).route("/estaticos/estilo.css", get(web::estaticos::estilo)).route("/estaticos/app.js", get(web::estaticos::app))`.

- [ ] **Paso 2: `plantillas/index.html`**

```html
<!doctype html>
<html lang="es-CL">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="theme-color" content="#1b1b2f">
<title>Galatea · probador</title>
<link rel="stylesheet" href="/estaticos/estilo.css">
</head>
<body>
<header><h1>Galatea</h1><span class="version">v{{ version }}</span></header>

<main>
  <section data-estado="aviso">
    <h2>Antes de la foto</h2>
    <p>La foto sirve solo para mostrar cómo queda la prenda. Se borra al terminar o a los quince minutos, y no se usa para entrenar nada. No se fotografía a menores.</p>
    <button class="principal" data-accion="entendido">Entendido</button>
  </section>

  <section data-estado="prendas" hidden>
    <h2>Elige una prenda</h2>
    <div class="cuadricula" data-lista-prendas></div>
  </section>

  <section data-estado="camara" hidden>
    <div class="prenda-elegida"><img data-imagen="prenda-elegida" alt=""><span data-nombre-prenda></span></div>
    <div class="visor">
      <video autoplay playsinline muted></video>
      <canvas hidden></canvas>
      <img data-imagen="previa" alt="" hidden>
      <div class="silueta" aria-hidden="true"></div>
    </div>
    <ul class="guia"><li>De pie</li><li>Cuerpo entero</li><li>Brazos separados</li></ul>
    <div class="acciones">
      <button class="principal" data-accion="capturar">Tomar foto</button>
      <label class="secundario">Subir foto<input type="file" accept="image/*" capture="environment" data-accion="archivo" hidden></label>
    </div>
    <div class="acciones" data-confirmar hidden>
      <button class="principal" data-accion="usar">Usar esta</button>
      <button class="secundario" data-accion="repetir">Repetir</button>
    </div>
  </section>

  <section data-estado="generando" hidden>
    <h2>Generando…</h2>
    <progress max="1" value="0" data-progreso></progress>
    <p data-mensaje-progreso>Preparando</p>
    <button class="secundario" data-accion="reintentar" hidden>Reintentar</button>
  </section>

  <section data-estado="resultado" hidden>
    <div class="comparar">
      <img data-imagen="resultado" alt="Persona con la prenda puesta (imagen generada con IA)">
      <img data-imagen="original" alt="Foto original" hidden>
    </div>
    <button class="secundario" data-accion="comparar">Ver original</button>
    <label data-selector-flujos hidden>Motor <select data-flujos></select></label>
    <div class="acciones">
      <button class="principal" data-accion="otra-prenda">Otra prenda</button>
      <button class="secundario" data-accion="repetir-foto">Repetir foto</button>
      <a class="secundario" data-accion="descargar" download="galatea.png">Descargar</a>
      <button class="peligro" data-accion="terminar">Terminar y borrar</button>
    </div>
  </section>

  <p class="mensaje" data-mensaje hidden></p>
</main>
<script src="/estaticos/app.js"></script>
</body>
</html>
```

- [ ] **Paso 3: `estaticos/estilo.css`** (móvil primero, oscuro, botones grandes)

```css
:root { --fondo:#1b1b2f; --tarjeta:#26263f; --texto:#f2f2f7; --acento:#c9a7ff; --peligro:#ff6b6b; }
* { box-sizing:border-box; }
body { margin:0; background:var(--fondo); color:var(--texto); font:16px/1.4 system-ui, sans-serif; }
header { display:flex; justify-content:space-between; align-items:baseline; padding:12px 16px; }
h1 { margin:0; font-size:20px; letter-spacing:.08em; } .version { opacity:.5; font-size:12px; }
main { padding:0 16px 32px; max-width:560px; margin:0 auto; }
section { margin-bottom:16px; }
button, .secundario, .principal { font:inherit; border:0; border-radius:14px; padding:16px; width:100%; margin-top:10px; cursor:pointer; display:block; text-align:center; text-decoration:none; }
.principal { background:var(--acento); color:#1b1b2f; font-weight:700; }
.secundario { background:var(--tarjeta); color:var(--texto); }
.peligro { background:transparent; color:var(--peligro); border:1px solid var(--peligro); }
.cuadricula { display:grid; grid-template-columns:repeat(auto-fill, minmax(140px, 1fr)); gap:12px; }
.cuadricula button { padding:0; margin:0; background:var(--tarjeta); overflow:hidden; }
.cuadricula img { width:100%; aspect-ratio:3/4; object-fit:cover; display:block; }
.cuadricula span { display:block; padding:8px; font-size:14px; }
.prenda-elegida { display:flex; gap:10px; align-items:center; margin-bottom:8px; } .prenda-elegida img { width:44px; height:58px; object-fit:cover; border-radius:8px; }
.visor { position:relative; background:#000; border-radius:16px; overflow:hidden; aspect-ratio:3/4; }
.visor video, .visor img { width:100%; height:100%; object-fit:cover; display:block; }
.silueta { position:absolute; inset:6% 28%; border:2px dashed rgba(255,255,255,.55); border-radius:45% 45% 20% 20% / 25% 25% 10% 10%; pointer-events:none; }
.guia { display:flex; gap:12px; justify-content:center; padding:0; list-style:none; font-size:14px; opacity:.85; } .guia li::before { content:"✓ "; color:var(--acento); }
progress { width:100%; height:14px; accent-color:var(--acento); }
.comparar img { width:100%; border-radius:16px; display:block; }
.mensaje { background:var(--tarjeta); border-left:4px solid var(--acento); padding:12px; border-radius:8px; }
.mensaje.error { border-color:var(--peligro); }
[hidden] { display:none !important; }
```

- [ ] **Paso 4: `estaticos/app.js`** (máquina de estados; sin dependencias)

```javascript
// Galatea: una página, cinco estados. Todo el estado de la demo vive aquí y en el servidor
// solo durante la sesión (15 min). La foto nunca se guarda en el celular.
(() => {
  const $ = (sel, raiz = document) => raiz.querySelector(sel);
  const $$ = (sel, raiz = document) => [...raiz.querySelectorAll(sel)];
  const estados = Object.fromEntries($$("[data-estado]").map(s => [s.dataset.estado, s]));
  const mensaje = $("[data-mensaje]");
  const conFlujos = new URLSearchParams(location.search).get("flujos") === "1";

  const sesion = { id: null, prenda: null, flujo: null, resultado: null, trabajo: null, fuente: null, foto: null };
  let camara = null;

  function mostrar(nombre) {
    Object.entries(estados).forEach(([n, s]) => { s.hidden = n !== nombre; });
    ocultarMensaje();
  }
  function avisar(texto, esError = false) {
    mensaje.textContent = texto; mensaje.hidden = false; mensaje.classList.toggle("error", esError);
  }
  function ocultarMensaje() { mensaje.hidden = true; }

  async function pedir(url, opciones = {}) {
    const r = await fetch(url, opciones);
    if (r.status === 204) return null;
    const cuerpo = await r.json().catch(() => ({}));
    if (!r.ok) throw new Error(cuerpo.error || `error ${r.status}`);
    return cuerpo;
  }

  // 0 · aviso
  $("[data-accion=entendido]").onclick = () => { cargarPrendas(); mostrar("prendas"); };

  // 1 · prendas
  async function cargarPrendas() {
    const lista = $("[data-lista-prendas]");
    if (lista.childElementCount) return;
    const prendas = await pedir("/prendas");
    prendas.forEach(p => {
      const b = document.createElement("button");
      b.dataset.prenda = p.id;
      b.innerHTML = `<img alt="" src="${p.miniatura}"><span>${p.nombre}</span>`;
      b.onclick = () => elegirPrenda(p);
      lista.appendChild(b);
    });
  }
  function elegirPrenda(p) {
    sesion.prenda = p;
    $("[data-imagen=prenda-elegida]").src = p.miniatura;
    $("[data-nombre-prenda]").textContent = p.nombre;
    if (sesion.id && sesion.foto) { generar(); } else { abrirCamara(); mostrar("camara"); }
  }

  // 2 · cámara
  const video = $("video"), canvas = $("canvas"), previa = $("[data-imagen=previa]");
  async function abrirCamara() {
    previa.hidden = true; video.hidden = false; $("[data-confirmar]").hidden = true; $("[data-accion=capturar]").hidden = false;
    if (camara) return;
    try {
      camara = await navigator.mediaDevices.getUserMedia({ video: { facingMode: { ideal: "environment" }, width: { ideal: 1080 }, height: { ideal: 1440 } }, audio: false });
      video.srcObject = camara;
    } catch (e) {
      avisar("No se pudo abrir la cámara (" + e.message + "). Usa «Subir foto».");
    }
  }
  function cerrarCamara() { if (camara) { camara.getTracks().forEach(t => t.stop()); camara = null; video.srcObject = null; } }
  $("[data-accion=capturar]").onclick = () => {
    if (!camara) { avisar("La cámara no está abierta. Usa «Subir foto»."); return; }
    canvas.width = video.videoWidth; canvas.height = video.videoHeight;
    canvas.getContext("2d").drawImage(video, 0, 0);
    canvas.toBlob(b => mostrarPrevia(b), "image/jpeg", 0.92);
  };
  $("[data-accion=archivo]").onchange = e => { const f = e.target.files[0]; if (f) mostrarPrevia(f); e.target.value = ""; };
  function mostrarPrevia(blob) {
    sesion.foto = blob;
    previa.src = URL.createObjectURL(blob); previa.hidden = false; video.hidden = true;
    $("[data-confirmar]").hidden = false; $("[data-accion=capturar]").hidden = true;
  }
  $("[data-accion=repetir]").onclick = () => { sesion.foto = null; abrirCamara(); };
  $("[data-accion=usar]").onclick = async () => {
    try {
      const datos = new FormData(); datos.append("persona", sesion.foto, "persona.jpg");
      const { id } = await pedir("/sesiones", { method: "POST", body: datos });
      sesion.id = id; cerrarCamara(); generar();
    } catch (e) { avisar(e.message, true); }
  };

  // 3 · generando
  const barra = $("[data-progreso]"), textoProgreso = $("[data-mensaje-progreso]"), reintentar = $("[data-accion=reintentar]");
  async function generar() {
    mostrar("generando"); barra.value = 0; textoProgreso.textContent = "Preparando"; reintentar.hidden = true;
    try {
      const cuerpo = { prenda: sesion.prenda.id }; if (sesion.flujo) cuerpo.flujo = sesion.flujo;
      const { trabajo, resultado } = await pedir(`/sesiones/${sesion.id}/generar`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(cuerpo) });
      sesion.trabajo = trabajo; sesion.resultado = resultado;
      seguir();
    } catch (e) { fallo(e.message); }
  }
  function seguir() {
    if (sesion.fuente) sesion.fuente.close();
    const f = new EventSource(`/sesiones/${sesion.id}/avance/${sesion.trabajo}`);
    sesion.fuente = f;
    f.addEventListener("encola", e => { textoProgreso.textContent = e.data === "0" ? "Empezando" : `Hay ${e.data} delante en la cola`; });
    f.addEventListener("progreso", e => { barra.value = Number(e.data); textoProgreso.textContent = `${Math.round(Number(e.data) * 100)} %`; });
    f.addEventListener("listo", e => { f.close(); sesion.fuente = null; mostrarResultado(Number(e.data)); });
    f.addEventListener("error", e => { f.close(); sesion.fuente = null; fallo(e.data || "Se perdió la conexión con el servidor"); });
  }
  function fallo(texto) { avisar(texto, true); reintentar.hidden = false; }
  reintentar.onclick = () => generar();

  // 4 · resultado
  const imgResultado = $("[data-imagen=resultado]"), imgOriginal = $("[data-imagen=original]");
  async function mostrarResultado(n) {
    imgResultado.src = `/sesiones/${sesion.id}/resultado/${n}.png?t=${Date.now()}`;
    imgOriginal.src = `/sesiones/${sesion.id}/persona.jpg`;
    imgResultado.hidden = false; imgOriginal.hidden = true;
    $("[data-accion=descargar]").href = imgResultado.src;
    if (conFlujos) await llenarFlujos();
    mostrar("resultado");
  }
  $("[data-accion=comparar]").onclick = e => {
    const verOriginal = imgOriginal.hidden;
    imgOriginal.hidden = !verOriginal; imgResultado.hidden = verOriginal;
    e.target.textContent = verOriginal ? "Ver resultado" : "Ver original";
  };
  async function llenarFlujos() {
    const sel = $("[data-flujos]"); $("[data-selector-flujos]").hidden = false;
    if (!sel.childElementCount) {
      (await pedir("/flujos")).forEach(f => {
        const o = document.createElement("option"); o.value = f.nombre; o.textContent = f.disponible ? f.titulo : `${f.titulo} (faltan pesos)`; o.disabled = !f.disponible; sel.appendChild(o);
      });
      sel.onchange = () => { sesion.flujo = sel.value; generar(); };
    }
  }
  $("[data-accion=otra-prenda]").onclick = () => mostrar("prendas");
  $("[data-accion=repetir-foto]").onclick = async () => { await terminarSesion(false); abrirCamara(); mostrar("camara"); };
  $("[data-accion=terminar]").onclick = () => terminarSesion(true);
  async function terminarSesion(volverAlInicio) {
    if (sesion.id) { try { await pedir(`/sesiones/${sesion.id}`, { method: "DELETE" }); } catch (_) {} }
    Object.assign(sesion, { id: null, resultado: null, trabajo: null, foto: null });
    if (volverAlInicio) { mostrar("prendas"); avisar("Listo: la foto y las imágenes se borraron del servidor."); }
  }
  window.addEventListener("pagehide", () => { if (sesion.id) navigator.sendBeacon && fetch(`/sesiones/${sesion.id}`, { method: "DELETE", keepalive: true }); });

  mostrar("aviso");
})();
```

- [ ] **Paso 5: Imagen sintética `pruebas/imagenes/persona-prueba.jpg`** (Pillow, 480×720: fondo claro, óvalo para la cabeza y rectángulos para torso y piernas). Commitéala; está permitida por `.gitignore` gracias a la excepción.

- [ ] **Paso 6: Playwright**

`package.json`: `{ "name": "galatea", "private": true, "type": "module" }`.

`playwright.config.js`:
```javascript
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests/e2e",
  timeout: 60_000,
  use: { baseURL: "http://127.0.0.1:8767", viewport: { width: 390, height: 844 } },
  webServer: {
    command: "cargo run --quiet",
    url: "http://127.0.0.1:8767/salud",
    reuseExistingServer: false,
    env: {
      GALATEA_PUERTO: "8767",
      GALATEA_PRENDAS: "pruebas/prendas",
      GALATEA_FLUJOS: "pruebas/flujos",
      GALATEA_DATOS: "target/e2e-datos",
      GALATEA_MODELOS: "target/e2e-datos/modelos",
      RUST_LOG: "warn",
    },
  },
});
```

`tests/e2e/demo.spec.js`:
```javascript
import { test, expect } from "@playwright/test";

test("recorrido completo con el flujo fijo y borrado al terminar", async ({ page, request }) => {
  await page.goto("/");
  await expect(page.locator('[data-estado="aviso"]')).toBeVisible();
  await page.click('[data-accion="entendido"]');

  await expect(page.locator('[data-prenda="roja"]')).toBeVisible();
  await page.click('[data-prenda="roja"]');

  await expect(page.locator('[data-estado="camara"]')).toBeVisible();
  await page.setInputFiles('input[data-accion="archivo"]', "pruebas/imagenes/persona-prueba.jpg");
  await expect(page.locator('[data-accion="usar"]')).toBeVisible();
  await page.click('[data-accion="usar"]');

  await expect(page.locator('[data-estado="resultado"]')).toBeVisible({ timeout: 30_000 });
  const resultado = page.locator('img[data-imagen="resultado"]');
  await expect(resultado).toBeVisible();
  const url = await resultado.getAttribute("src");
  expect(url).toMatch(/\/sesiones\/[0-9a-f]{32}\/resultado\/1\.png/);
  expect((await request.get(url)).status()).toBe(200);

  await page.click('[data-accion="comparar"]');
  await expect(page.locator('img[data-imagen="original"]')).toBeVisible();

  await page.click('[data-accion="terminar"]');
  await expect(page.locator('[data-estado="prendas"]')).toBeVisible();
  await expect(page.locator("[data-mensaje]")).toContainText("se borraron");
  expect((await request.get(url)).status()).toBe(404);
});

test("el selector de flujos aparece solo con ?flujos=1", async ({ page }) => {
  await page.goto("/?flujos=1");
  await page.click('[data-accion="entendido"]');
  await page.click('[data-prenda="azul"]');
  await page.setInputFiles('input[data-accion="archivo"]', "pruebas/imagenes/persona-prueba.jpg");
  await page.click('[data-accion="usar"]');
  await expect(page.locator("[data-selector-flujos]")).toBeVisible({ timeout: 30_000 });
  const opciones = page.locator("select[data-flujos] option");
  await expect(opciones).toHaveCount(2);
  await expect(opciones.nth(1)).toBeDisabled(); // «prueba» sin pesos
});
```

- [ ] **Paso 7: Correr todo**

```bash
nix develop -c cargo test
nix develop -c npx playwright test 2>&1 | tail -15
```
Esperado: 2 pruebas de Playwright en verde. Si `npx` no encuentra el binario, usa `nix develop -c playwright test`. Prueba también a mano en el escritorio (`cargo run` con las mismas variables y abrir `http://127.0.0.1:8766`) para ver la cámara real del portátil si tiene.

- [ ] **Paso 8: Commit** (`git add -A && git commit -m "feat: página de la demo con cámara, avance y prueba de navegador"`).

---

### Tarea 11: HTTPS en la red local (Caddy), guion de la demo y prueba contra el ComfyUI real

**Files:**
- Create: `caddy/Caddyfile`, `scripts/demo.sh`, `DEMO.md`, `README.md`, `tests/comfy_real.rs`, `prendas/prendas.toml` (ejemplo con las dos imágenes CC)
- Modify: `compose.yaml` (servicio `caddy`), `.gitignore` (`datos/caddy` ya cae bajo `datos/`)

**Interfaces:**
- Produces: `https://192.168.0.10:8444/` sirviendo Galatea al celular; `http://192.168.0.10:8091/caddy-root.crt` para instalar la CA en el celular; `scripts/demo.sh` como único comando para levantar la demo.

- [ ] **Paso 1: `caddy/Caddyfile`** (Caddy corre con `network_mode: host`, así que llega a `127.0.0.1:8766` directamente)

```caddyfile
{
	# Los navegadores omiten SNI con una IP literal; sin esto el celular recibe un error TLS.
	default_sni 192.168.0.10
}

https://192.168.0.10:8444 {
	tls internal
	# Solo desde la propia página: nada de otras webs embebiendo el probador.
	@cross-site header Sec-Fetch-Site cross-site
	handle @cross-site {
		respond 403
	}
	reverse_proxy 127.0.0.1:8766 {
		# El SSE no debe pasar por búfer.
		flush_interval -1
	}
}

:8091 {
	bind 192.168.0.10
	handle /caddy-root.crt {
		root * /data/caddy/pki/authorities/local
		rewrite * /root.crt
		file_server
	}
	handle {
		redir https://192.168.0.10:8444{uri}
	}
}
```

Servicio en `compose.yaml`:
```yaml
  caddy:
    image: caddy@sha256:df7f1c2fb114453b951de51a98efc010db1655a92c2e86be6706714e2417a78d
    container_name: galatea-caddy
    network_mode: host
    volumes:
      - ./caddy/Caddyfile:/etc/caddy/Caddyfile:ro
      - ./datos/caddy:/data
    restart: unless-stopped
```
(El digest es el que usa Aylin el 2026-09-26; si `docker pull caddy:2` da uno más nuevo, usa ese y anótalo.)

- [ ] **Paso 2: `scripts/demo.sh`**

```bash
#!/usr/bin/env bash
# Levanta la demo completa: apaga lo que compite por la GPU, sube ComfyUI y Caddy, arranca
# Galatea en el host y muestra la URL para el celular. Ctrl-C apaga Galatea (los contenedores
# quedan arriba; `docker compose down` los baja).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== GPU antes:"; nvidia-smi --query-gpu=memory.used,memory.total --format=csv,noheader
for c in crescent-ollama visjar-hermes-1 visjar-voice-gateway-1 visjar-caddy-1 scout-llamacpp; do
  if docker ps --format '{{.Names}}' | grep -qx "$c"; then
    echo "deteniendo $c (comparte la GPU o el puerto)"; docker stop "$c" >/dev/null
  fi
done

mkdir -p datos/modelos datos/comfy/{input,output,user,temp} datos/hf-cache datos/caddy
docker compose up -d comfyui caddy
echo -n "esperando a ComfyUI"; for _ in $(seq 1 60); do curl -sf http://127.0.0.1:8188/system_stats >/dev/null && break; echo -n .; sleep 2; done; echo
curl -s http://127.0.0.1:8188/system_stats | jq -r '.devices[0] | "GPU: \(.name) · libre \(.vram_free/1e9|floor) GB"'

export GALATEA_COMFY_URL=${GALATEA_COMFY_URL:-http://127.0.0.1:8188}
export GALATEA_FLUJO_POR_DEFECTO=${GALATEA_FLUJO_POR_DEFECTO:-catvton}
export RUST_LOG=${RUST_LOG:-info}
echo
echo "== En el celular (misma red Wi-Fi):"
echo "   1) instala el certificado una vez: http://192.168.0.10:8091/caddy-root.crt"
echo "   2) abre https://192.168.0.10:8444/   (con ?flujos=1 para elegir motor)"
echo
exec nix run . -- servir
```
`chmod +x scripts/demo.sh`. Si Galatea no está construida con `nix run` (tarda la primera vez), `cargo run --release` sirve igual dentro del devShell.

- [ ] **Paso 3: `prendas/prendas.toml` de ejemplo** apuntando a las imágenes CC (fuera de git) para que la demo arranque sin fotos de la tienda:
```toml
# Prendas de la demo. Las imágenes viven en esta carpeta y no entran a git.
# Para la comprobación de flujo sirven las Creative Commons de scripts/descargar-pruebas.sh:
#   cp pruebas/imagenes/prenda-polera-cc.jpg prendas/
[[prenda]]
id = "polera-cc"
nombre = "Polera amarilla (prueba)"
archivo = "prenda-polera-cc.jpg"
tipo = "superior"
```

- [ ] **Paso 4: Prueba contra el ComfyUI real, ignorada por defecto (`tests/comfy_real.rs`)**

```rust
//! Solo con el contenedor arriba: `GALATEA_COMFY_URL=http://127.0.0.1:8188 cargo test --test comfy_real -- --ignored`
use galatea::{flujos, motor::{Avance, Motores, Trabajo, comfy::Comfy, fijo::Fijo}, prendas::Tipo};
use std::{path::{Path, PathBuf}, sync::Arc, time::{Duration, Instant}};

#[tokio::test]
#[ignore = "necesita ComfyUI real; ver el comentario del archivo"]
async fn catvton_genera_una_imagen_con_las_fotos_cc() {
    let url = std::env::var("GALATEA_COMFY_URL").expect("define GALATEA_COMFY_URL");
    let flujos = flujos::cargar(Path::new("flujos")).unwrap();
    let flujo = flujos::buscar(&flujos, "catvton").expect("flujos/flujos.toml declara catvton").clone();
    assert!(flujos::pesos_faltantes(&flujo, Path::new("datos/modelos")).is_empty(), "faltan pesos: corre scripts/descargar-modelos.sh catvton");
    let dir = tempfile::tempdir().unwrap();
    let destino = dir.path().join("resultado.png");
    let motores = Arc::new(Motores::nuevo(Fijo::nuevo(Path::new("flujos/fijo")), Some(Comfy::nuevo(&url, Duration::from_secs(180), Some(PathBuf::from("datos/comfy"))))));
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
    while let Some(a) = rx.recv().await { ultimo = Some(a); }
    assert_eq!(ultimo, Some(Avance::Listo));
    let im = image::open(&destino).unwrap();
    assert!(im.width() >= 512 && im.height() >= 512);
    eprintln!("catvton: {} s, {}x{}", inicio.elapsed().as_secs(), im.width(), im.height());
    std::fs::copy(&destino, "experimento/ultima-cc.png").ok();
}
```
Córrela con el contenedor arriba y pega la línea `catvton: N s`. Comprueba después que `datos/comfy/input/` y `datos/comfy/output/` **no** contienen `galatea-*` (la limpieza real).

- [ ] **Paso 5: `DEMO.md`** (guion de ocho minutos, como en Nexo Huella)

```markdown
# La demo de Galatea

## Antes de salir de casa
1. `scripts/demo.sh` en una terminal; espera «GPU: … libre N GB» y la URL.
2. En el celular, misma Wi-Fi: instala el certificado desde `http://192.168.0.10:8091/caddy-root.crt`
   (Android: Ajustes → Seguridad → Instalar certificado → CA; iPhone: Perfil descargado → Instalar,
   y luego Ajustes → General → Información → Ajustes de confianza de certificados).
3. Abre `https://192.168.0.10:8444/` y haz un recorrido completo con el flujo Fijo (`?flujos=1`)
   para comprobar cámara y borrado. Después vuelve al flujo por defecto.
4. Prendas: copia las fotos de la tienda a `prendas/` y complétalas en `prendas/prendas.toml`;
   reinicia Galatea para regenerar miniaturas.

## Los ocho minutos
1. (1 min) El problema: «¿cómo me quedaría a mí?» y por qué las fotos en maniquí no responden.
2. (1 min) El aviso de privacidad, leído en voz alta: se borra al terminar.
3. (3 min) Elegir prenda, tomar la foto a la voluntaria, esperar mirando la barra, ver el resultado
   y comparar con la original. Probar una segunda prenda con la misma foto.
4. (1 min) «Terminar y borrar» delante de la voluntaria; mostrar el mensaje de confirmación.
5. (2 min) Qué viene: tallas con medidas reales, modelos genéricas sin foto, correr fuera de casa.

## Si algo falla
- ComfyUI no responde → en el celular, `?flujos=1` y elegir «Fijo (sin GPU)»: la demo sigue con
  imágenes pregeneradas. En la terminal, `docker compose logs --tail 50 comfyui`.
- La cámara no abre → «Subir foto» usa la cámara nativa del celular.
- Certificado rechazado → revisar que la hora del celular esté bien y que el certificado
  instalado sea el de `datos/caddy/pki/authorities/local/root.crt` actual (si se borró `datos/caddy`,
  hay que reinstalarlo).
```

`README.md`: qué es, los tres procesos, `scripts/demo.sh`, cómo agregar un flujo (exportar en formato API + entrada en `flujos.toml` + `scripts/descargar-modelos.sh <nombre>`), cómo correr pruebas (`cargo test`, `playwright test`, la ignorada), y enlace a la especificación y a este plan.

- [ ] **Paso 6: Prueba de verdad desde el celular** y anota en `experimento/2026-09-XX-demo-celular.md`: modelo de celular, navegador, si la cámara abrió a la primera, segundos hasta el resultado, y cualquier tropiezo. Commit: `git commit -m "feat: HTTPS en la LAN, guion de demo y prueba contra ComfyUI real"`.

---

### Tarea 12: Subcomando `experimento` y flujos alternativos (IDM-VTON, Leffa, klein)

**Files:**
- Create: `src/experimento.rs`, `tests/experimento.rs`, `experimento/2026-09-XX-nodos.md` (ampliar el de la Tarea 2)
- Modify: `src/main.rs` (subcomando), `src/lib.rs` (`pub mod experimento;`), `comfyui/Dockerfile`, `comfyui/requisitos-nodos.txt`, `flujos/flujos.toml`, `flujos/*.json`

**Interfaces:**
- Produces: `galatea experimento --personas <dir> --prendas <dir> --flujos <a,b,c> --salida <dir>` que genera la matriz completa y escribe `<salida>/tabla.csv` (`flujo,persona,prenda,segundos,estado,archivo`) y `<salida>/hoja.html` (cuadrícula de todas las imágenes con su fila para calificar a ojo).
- `experimento::correr(estado: &Estado, opciones: Opciones) -> anyhow::Result<Resumen>` con `Opciones { personas: PathBuf, prendas: PathBuf, flujos: Vec<String>, salida: PathBuf }` y `Resumen { filas: Vec<Fila> }`, `Fila { flujo: String, persona: String, prenda: String, segundos: f32, estado: String, archivo: Option<PathBuf> }`.

- [ ] **Paso 1: Prueba con el flujo Fijo (`tests/experimento.rs`)**

```rust
use galatea::{Conf, experimento::{self, Opciones}, preparar};

#[tokio::test]
async fn la_matriz_con_fijo_produce_tabla_y_hoja() {
    let temporal = tempfile::tempdir().unwrap();
    let conf = Conf::para_pruebas(temporal.path());
    let estado = preparar(&conf).await.unwrap();
    let personas = temporal.path().join("personas");
    std::fs::create_dir_all(&personas).unwrap();
    std::fs::copy("pruebas/imagenes/persona-prueba.jpg", personas.join("a.jpg")).unwrap();
    std::fs::copy("pruebas/imagenes/persona-prueba.jpg", personas.join("b.jpg")).unwrap();
    let salida = temporal.path().join("salida");
    let resumen = experimento::correr(&estado, Opciones {
        personas,
        prendas: "pruebas/prendas".into(),
        flujos: vec!["fijo".to_string()],
        salida: salida.clone(),
    }).await.unwrap();
    assert_eq!(resumen.filas.len(), 4, "2 personas x 2 prendas x 1 flujo");
    assert!(resumen.filas.iter().all(|f| f.estado == "ok"));
    let csv = std::fs::read_to_string(salida.join("tabla.csv")).unwrap();
    assert!(csv.starts_with("flujo,persona,prenda,segundos,estado,archivo\n"));
    assert_eq!(csv.lines().count(), 5);
    let html = std::fs::read_to_string(salida.join("hoja.html")).unwrap();
    assert!(html.contains("fijo/a/roja.png"));
    assert!(salida.join("fijo/a/roja.png").is_file());
}
```

- [ ] **Paso 2: Implementación `src/experimento.rs`**

```rust
//! Corre la matriz personas × prendas × flujos y deja una tabla CSV y una hoja HTML para
//! calificar a ojo. Usa los mismos motores que la página.
use crate::{Estado, flujos, motor::{Avance, Trabajo}, prendas};
use anyhow::{Result, anyhow};
use std::{path::PathBuf, time::Instant};

pub struct Opciones {
    pub personas: PathBuf,
    pub prendas: PathBuf,
    pub flujos: Vec<String>,
    pub salida: PathBuf,
}

pub struct Fila {
    pub flujo: String,
    pub persona: String,
    pub prenda: String,
    pub segundos: f32,
    pub estado: String,
    pub archivo: Option<PathBuf>,
}

pub struct Resumen {
    pub filas: Vec<Fila>,
}

pub async fn correr(estado: &Estado, opciones: Opciones) -> Result<Resumen> {
    let lista_prendas = prendas::cargar(&opciones.prendas)?;
    let mut personas: Vec<PathBuf> = std::fs::read_dir(&opciones.personas)
        .map_err(|e| anyhow!("no se pudo leer {}: {e}", opciones.personas.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| matches!(p.extension().and_then(|x| x.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("jpg" | "jpeg" | "png")))
        .collect();
    personas.sort();
    if personas.is_empty() {
        return Err(anyhow!("no hay imágenes .jpg/.png en {}", opciones.personas.display()));
    }
    let mut filas = Vec::new();
    for nombre_flujo in &opciones.flujos {
        let flujo = flujos::buscar(&estado.flujos, nombre_flujo).ok_or_else(|| anyhow!("no existe el flujo «{nombre_flujo}»"))?;
        for persona in &personas {
            let base_persona = persona.file_stem().unwrap().to_string_lossy().to_string();
            for prenda in &lista_prendas {
                let destino = opciones.salida.join(nombre_flujo).join(&base_persona).join(format!("{}.png", prenda.id));
                let inicio = Instant::now();
                let (estado_fila, archivo) = if !flujo.admite(prenda.tipo) {
                    ("tipo no admitido".to_string(), None)
                } else {
                    let mut rx = estado.motores.generar(Trabajo {
                        flujo: flujo.clone(),
                        persona: persona.clone(),
                        prenda: prendas::ruta(&opciones.prendas, prenda),
                        prenda_id: prenda.id.clone(),
                        tipo: prenda.tipo,
                        destino: destino.clone(),
                        etiqueta: format!("exp-{base_persona}"),
                    });
                    let mut ultimo = None;
                    while let Some(a) = rx.recv().await { ultimo = Some(a); }
                    match ultimo {
                        Some(Avance::Listo) => ("ok".to_string(), Some(destino.clone())),
                        Some(Avance::Error(m)) => (format!("error: {m}"), None),
                        _ => ("sin respuesta".to_string(), None),
                    }
                };
                eprintln!("{nombre_flujo} · {base_persona} · {} · {:.1} s · {estado_fila}", prenda.id, inicio.elapsed().as_secs_f32());
                filas.push(Fila { flujo: nombre_flujo.clone(), persona: base_persona.clone(), prenda: prenda.id.clone(), segundos: inicio.elapsed().as_secs_f32(), estado: estado_fila, archivo });
            }
        }
    }
    std::fs::create_dir_all(&opciones.salida)?;
    std::fs::write(opciones.salida.join("tabla.csv"), tabla_csv(&filas))?;
    std::fs::write(opciones.salida.join("hoja.html"), hoja_html(&filas, &opciones.salida))?;
    Ok(Resumen { filas })
}

fn tabla_csv(filas: &[Fila]) -> String {
    let mut s = String::from("flujo,persona,prenda,segundos,estado,archivo\n");
    for f in filas {
        s.push_str(&format!("{},{},{},{:.1},\"{}\",{}\n", f.flujo, f.persona, f.prenda, f.segundos, f.estado.replace('"', "'"), f.archivo.as_ref().map(|a| a.display().to_string()).unwrap_or_default()));
    }
    s
}

fn hoja_html(filas: &[Fila], raiz: &std::path::Path) -> String {
    let mut s = String::from("<!doctype html><meta charset=utf-8><title>Experimento Galatea</title><style>body{font-family:system-ui;background:#111;color:#eee}figure{display:inline-block;margin:6px;text-align:center}img{height:360px;display:block}figcaption{font-size:12px}</style><h1>Experimento</h1><p>Marca en papel: ¿la mostrarías a una clienta? sí / no</p>");
    for f in filas {
        let ruta = f.archivo.as_ref().and_then(|a| a.strip_prefix(raiz).ok()).map(|a| a.display().to_string());
        s.push_str(&match ruta {
            Some(r) => format!("<figure><img src=\"{r}\"><figcaption>{} · {} · {} · {:.0} s</figcaption></figure>", f.flujo, f.persona, f.prenda, f.segundos),
            None => format!("<figure><div style=\"height:360px;width:240px;background:#333\"></div><figcaption>{} · {} · {} · {}</figcaption></figure>", f.flujo, f.persona, f.prenda, f.estado),
        });
    }
    s
}
```

`src/main.rs`: subcomando
```rust
/// Corre personas × prendas × flujos y deja tabla.csv y hoja.html en --salida.
Experimento {
    #[arg(long)] personas: PathBuf,
    #[arg(long, default_value = "./prendas")] prendas: PathBuf,
    #[arg(long, value_delimiter = ',')] flujos: Vec<String>,
    #[arg(long)] salida: PathBuf,
},
```
que hace `let conf = Conf::desde_entorno()?; let estado = galatea::preparar(&conf).await?; let r = galatea::experimento::correr(&estado, Opciones { .. }).await?; println!("{} filas; tabla en {}", r.filas.len(), salida.join("tabla.csv").display());`.

- [ ] **Paso 3: Verificar y commitear** (`cargo test --test experimento`; `git commit -m "feat: subcomando experimento con tabla y hoja de contactos"`).

- [ ] **Paso 4: Flujos alternativos, uno por uno, cada uno con veredicto escrito**

Para cada candidato: agrega el nodo al `Dockerfile` a commit fijo, sus pines a `requisitos-nodos.txt`, reconstruye (`docker compose build comfyui`), arma el flujo en la interfaz con las imágenes CC, exporta a `flujos/<nombre>.json`, declara en `flujos.toml` con pesos y sha256, corre `scripts/descargar-modelos.sh <nombre>` y `galatea experimento --personas pruebas/imagenes --flujos <nombre> --salida experimento/$(date +%F)-<nombre>`. Anota en `experimento/2026-09-XX-nodos.md`: commit del nodo, VRAM pico (`nvidia-smi -l 1` en paralelo), segundos por imagen, resolución, licencia, y **veredicto**: «queda» / «descartado porque …». Un candidato que no corre en medio día se descarta y se sigue con el siguiente.

1. **IDM-VTON** — nodo `TemryL/ComfyUI-IDM-VTON` (commit `5a8334c58c39`, 2024-08-20; GPL-3.0; exige ≥ 16 GB y `huggingface_hub==0.25.2`; depende de `ComfyUI Segment Anything` y `ControlNet Auxiliary Preprocessors` para máscara y DensePose). Licencia del modelo CC BY-NC-SA. Riesgo: dependencias viejas contra PyTorch 2.14; si `pip` no resuelve, descartar.
2. **Leffa** — nodo `Bionic-AI-Solutions/ComfyUI-Leffa` (commit `90536512f7a9`, 2026-06-29, el más mantenido) o `StartHua/Comfyui_leffa` (`713b8754514e`). Pesos `franciszzj/Leffa` (~34 GB en total; bajar solo el subconjunto de try-on). Licencia MIT: **candidato a producto**. Necesita máscara agnóstica y DensePose: reutiliza la máscara del segformer y comprueba si el nodo trae DensePose.
3. **FLUX.2 klein 4B + LoRA** — sin nodo extra: ComfyUI v0.35 carga klein de forma nativa. Pesos: `black-forest-labs/FLUX.2-klein-4B` (comprueba en la ficha si exige aceptar términos; es Apache 2.0) y el LoRA `xocialize/tryon-FLUX.2-klein-4B-lora`. El LoRA espera **tres referencias**: la persona con la ropa enmascarada en gris, la prenda superior y la inferior, con el prompt disparador exacto `TRYON a full-body photo of a person. Replace the outfit with the top and bottom as shown in the reference images. The final image is a full body shot.` Para prendas superiores solas, prueba pasando la misma prenda o una imagen neutra como «bottom» y anota qué pasa. La máscara gris se produce con el segformer y un nodo de composición. Este flujo necesita `nodo_tipo = none` y probablemente un cuarto nodo (la máscara); si el TOML no alcanza, agrega un campo `nodo_extra` documentado en la spec **antes** de tocar Rust.

- [ ] **Paso 5: Elegir el flujo por defecto y cerrar**

Con la tabla de las imágenes CC de cada flujo, elige el que mejor se vea y quepa en VRAM, ponlo en `scripts/demo.sh` (`GALATEA_FLUJO_POR_DEFECTO`) y en `README.md`. Deja el resto en `flujos.toml` (el selector oculto los muestra; los que no tengan pesos aparecen deshabilitados). Commit: `git commit -m "feat: flujos alternativos evaluados y flujo por defecto"`.

El experimento con las prendas reales de la tienda y las tres voluntarias (spec §12, pasos 3 y 4) no es una tarea de este plan: es trabajo con personas. El comando queda listo: `galatea experimento --personas experimento/voluntarias --prendas prendas --flujos catvton,leffa --salida experimento/$(date +%F)-tienda`, y la hoja HTML es lo que se le muestra a la tienda para que marque sí o no.

---

## Autorrevisión del plan (hecha el 2026-09-26)

**Cobertura de la especificación.** §3 alcance → Tareas 9 y 10 (dentro) y §14 del plan (fuera). §4 flujo y pantalla → Tarea 10 (cinco estados, selector oculto, borrado visible). §5 arquitectura → Tareas 2 y 11 (compose, Dockerfile, Caddy; Galatea corre en el host y ComfyUI publica en loopback, con Caddy en red del host: ajuste respecto de «solo red de compose», anotado en Restricciones). §6 flujos → Tarea 4 (con `campo_*` y `valores_tipo` que la spec no detallaba). §7 prendas → Tarea 3. §8 servidor → Tareas 5, 7, 8, 9. §9 errores → mensajes en cada tarea. §10 privacidad → Tareas 5, 8 (limpieza en ComfyUI), 10 (aviso), y `tmpfs` queda como opción manual documentada en DEMO.md, no como requisito. §11 pruebas → cada tarea, más `comfy_falso` (8), Playwright (10) e ignorada real (11). §12 experimento → Tareas 2 y 12. §13 imágenes CC → Tarea 2. §14 futuro → sin tarea, por diseño.

**Marcadores.** Los únicos `<...>` que quedan están en la Tarea 2 (ids de nodo y sha256 de pesos) y se rellenan al exportar el flujo; el paso lo dice explícitamente. No hay «TBD».

**Consistencia de tipos.** `Avance::Listo` sin carga en todas las tareas; `Trabajo` con `etiqueta` desde la Tarea 7 y usado en 8, 9, 11 y 12; `Motores::generar(self: &Arc<Self>, Trabajo) -> Receiver<Avance>` igual en 7, 8, 9, 11 y 12; `Sesiones::reservar_resultado -> Option<(u32, PathBuf)>` en 5 y 9; `flujos::Entradas { persona, prenda, tipo }` en 4 y 8; `Conf::para_pruebas` con `flujos` mutable usado en la prueba de 422 (Tarea 9). `Comfy::nuevo(&str, Duration, Option<PathBuf>)` en 8, 11 y `lib.rs`.

**Desviaciones respecto de la spec, a propósito:** (1) la página usa JSON + JavaScript propio en vez de htmx, porque cámara, SSE y máquina de estados son JavaScript de todos modos; askama sigue sirviendo la página. (2) Galatea corre en el host, no en contenedor, como Crescent. (3) El sello de marca de agua es un PNG pregenerado con Pillow, no texto renderizado en Rust, para no arrastrar una fuente al binario.
