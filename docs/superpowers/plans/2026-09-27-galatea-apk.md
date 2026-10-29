# Galatea · Plan de implementación: cliente Svelte y APK con Capacitor

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convertir el cliente de la demo en una app Svelte que el servidor Rust sirve para el navegador y que Capacitor empaqueta en un APK de Android, con la CA de la LAN dentro del APK y la dirección del servidor configurable.

**Architecture:** Un solo cliente en `web/` (Svelte 5 + Vite) produce `web/dist/`. El servidor axum lo sirve como archivos estáticos con fallback a `index.html` y acepta CORS desde el origen interno de Capacitor (`https://localhost`). El proyecto Android generado por Capacitor vive en `web/android/`, commiteado, con permiso de cámara y la CA raíz de Caddy como recurso. Node, JDK, Gradle y el SDK de Android entran al `devShell` del flake; el build del cliente entra a `nix flake check`; el APK se compila con un script porque Gradle necesita red.

**Tech Stack:** Rust 2024 + axum 0.8 + tower-http 0.7.1 (`fs`, `cors`); Svelte 5.57.1, Vite 8.3.1, @sveltejs/vite-plugin-svelte 7.3.1, Vitest 5.0.2; Capacitor 8.5.2 (`core`, `cli`, `android`) y @capacitor/app 8.1.1; nixpkgs fijado en `flake.lock` (rev `e94cb152`): `nodejs_22` 22.23.3, `jdk21`, `gradle_8` 8.14.4, `androidenv` con plataforma 36, build-tools 36.0.0, imagen de sistema `google_apis` x86_64 36; Playwright desde nixpkgs (`playwright-test`).

**Spec:** `docs/superpowers/specs/2026-09-27-galatea-apk-design.md`. Ante cualquier duda, manda la especificación.

## Global Constraints

- **Versiones más recientes** (CLAUDE.md): al añadir una dependencia se comprueba la última estable ese día. Las de arriba se comprobaron el 2026-09-27 (npm: `npm view <paquete> version`; crates: `curl -s https://crates.io/api/v1/crates/<crate> -H 'User-Agent: galatea-dev'`). Si al instalar hay una más nueva, se usa esa y se dice en el informe.
- **Dominio en español**: nombres de archivos, funciones, variables, mensajes. Solo quedan en inglés las palabras propias de las bibliotecas.
- **Mensajes de error completos en español**, con qué pasó y cómo arreglarlo.
- **Commits planos**: sin `Co-Authored-By`, `Claude-Session` ni otros trailers.
- **`nix flake check` es la autoridad.** Ninguna tarea termina sin que pase (clippy con `--deny warnings`, fmt, pruebas Rust y, desde la Tarea 1, el build del cliente).
- **Informar solo lo ejecutado**: la salida real del comando en el informe; nada de «debería» ni «por inspección». Si una prueba protege algo, rómpelo a propósito y comprueba que se pone roja.
- **Los selectores `data-estado` y `data-accion` de la demo se conservan** con los mismos valores; Ajustes agrega `data-estado="ajustes"` y `data-accion="probar" | "guardar" | "cerrar-ajustes"`.
- **Nada personal en git**: `datos/`, `web/android/app/src/main/res/raw/caddy_root.crt`, `web/dist/`, `web/node_modules/`, `.superpowers/` quedan fuera.
- **Identificador de la app:** `cl.galatea.probador`, provisional. **Nombre visible:** «Probador Galatea». Nunca el dominio ni la marca de la empresa del usuario.
- **Origen interno de Capacitor en Android:** `https://localhost` (esquema `https`, hostname `localhost`, valores por defecto de Capacitor 8).
- **Dirección del servidor en la LAN:** `https://192.168.0.10:8444` (Caddy). CA raíz en `datos/caddy/caddy/pki/authorities/local/root.crt`.
- **La GPU no se toca en este plan** (ni ComfyUI ni `demo.sh` salvo la llamada a `scripts/web.sh`). No se matan procesos con `pkill`; los contenedores se manejan con `docker stop <nombre>` si hiciera falta.
- Los comandos se ejecutan dentro de `nix develop -c …` (o en un shell `nix develop`), desde la raíz del repo, salvo que se diga otra cosa.

---

## Estructura de archivos

**Nuevos**

| Archivo | Responsabilidad |
|---|---|
| `web/package.json`, `web/package-lock.json` | dependencias del cliente; `scripts`: `dev`, `build`, `test` |
| `web/vite.config.js` | plugin Svelte, salida `dist/`, proxy de la API en desarrollo, configuración de Vitest |
| `web/index.html`, `web/src/main.js` | arranque; lee `?flujos=1` |
| `web/src/App.svelte` | cabecera y elección de pantalla según el store |
| `web/src/estado.js` | store central y acciones (la máquina de estados de la demo) |
| `web/src/api.js` | URL base, llamadas HTTP, SSE, sondeo, `FalloApi`; sin Svelte |
| `web/src/ajustes.js` | dirección del servidor: valor por defecto, lectura, validación, guardado |
| `web/src/nativo.js` | lo que solo pasa dentro de Capacitor: botón Atrás |
| `web/src/estilo.css` | el `estaticos/estilo.css` actual, movido |
| `web/src/pantallas/{Aviso,Prendas,Camara,Generando,Resultado,Ajustes}.svelte` | una pantalla por archivo |
| `web/src/*.test.js` | Vitest de `api.js`, `ajustes.js`, `estado.js` |
| `web/capacitor.config.json` | `appId`, `appName`, `webDir`, depuración del WebView |
| `web/android/` | proyecto Android de Capacitor (commiteado) |
| `web/public/icono.png` | ícono de la app, generado |
| `pruebas/web/index.html`, `pruebas/web/assets/prueba.js` | cliente de mentira para las pruebas Rust de estáticos |
| `recursos/marca.png` | la marca de agua (antes `estaticos/marca.png`) |
| `tests/estaticos.rs` | pruebas Rust de estáticos y CORS |
| `scripts/web.sh` | `npm ci` si hace falta y `vite build` |
| `scripts/apk.sh` | CA → recurso, build web con `VITE_SERVIDOR`, `cap sync`, `gradle assembleDebug`, copia a `datos/apk/` |
| `scripts/icono.py` | genera el ícono (luna creciente) en todas las densidades |
| `scripts/emulador.sh` | crea/arranca el AVD, instala el APK, lanza la app, saca captura |
| `experimento/2026-09-27-apk.md` | checklist manual del APK y evidencia |

**Modificados**

| Archivo | Cambio |
|---|---|
| `flake.nix` | `nodejs_22`, `checks.web`/`packages.web` (buildNpmPackage), SDK Android + JDK + Gradle en el devShell, filtro de fuentes sin `web/`, `plantillas/`, `estaticos/` |
| `Cargo.toml` | sale `askama`; entra `tower-http = { version = "0.7.1", features = ["fs", "cors"] }` |
| `src/lib.rs` | `Conf.web`, `Conf.origenes`, comprobación en `preparar`, `ServeDir` + `CorsLayer` en `construir_router` |
| `src/web/mod.rs` | salen `pagina` y `estaticos` |
| `src/marca.rs` | `include_bytes!("../recursos/marca.png")` |
| `playwright.config.js` | compila el cliente antes de arrancar, `GALATEA_WEB=web/dist` |
| `tests/e2e/demo.spec.js` | mismos seis casos adaptados; nuevo caso de Ajustes |
| `scripts/demo.sh` | llama a `scripts/web.sh` y exporta `GALATEA_WEB` |
| `.gitignore` | `web/dist/`, `web/node_modules/`, `web/android/app/src/main/res/raw/`, `datos/` ya está |
| `README.md`, `DEMO.md`, `CLAUDE.md` | cliente Svelte, APK, variables nuevas, nota de `experimento` |

**Eliminados (Tarea 5):** `askama.toml`, `plantillas/`, `estaticos/`, `src/web/pagina.rs`, `src/web/estaticos.rs`.

---

### Tarea 1: Andamiaje de `web/` y build del cliente en Nix

**Files:**
- Create: `web/package.json`, `web/vite.config.js`, `web/index.html`, `web/src/main.js`, `web/src/App.svelte`, `web/src/estilo.css`, `web/src/humo.test.js`, `web/.gitignore`, `scripts/web.sh`
- Modify: `flake.nix`, `.gitignore`

**Interfaces:**
- Produces: `web/dist/index.html` y `web/dist/assets/*` tras `scripts/web.sh`; `nix build .#web` deja el mismo `dist` en `result/`; `npm test` en `web/` corre Vitest.

- [ ] **Paso 1: Crear el proyecto con las versiones más recientes**

Comprueba las versiones ese día y usa las que salgan (si son más nuevas que las de aquí, úsalas y dilo):

```bash
for p in svelte vite @sveltejs/vite-plugin-svelte vitest; do printf '%s ' "$p"; npm view "$p" version; done
```

`web/package.json`:

```json
{
  "name": "galatea-web",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "test": "vitest run"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^7.3.1",
    "svelte": "^5.57.1",
    "vite": "^8.3.1",
    "vitest": "^5.0.2"
  }
}
```

`web/vite.config.js`:

```js
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// En desarrollo (`npm run dev`) la API se reenvía al servidor Rust local.
const API = ["/prendas", "/flujos", "/sesiones", "/salud"];

export default defineConfig({
  plugins: [svelte()],
  build: { outDir: "dist", emptyOutDir: true },
  server: { proxy: Object.fromEntries(API.map(r => [r, "http://127.0.0.1:8766"])) },
  test: { environment: "node", include: ["src/**/*.test.js"] },
});
```

`web/index.html`:

```html
<!doctype html>
<html lang="es-CL">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="theme-color" content="#1b1b2f">
<title>Galatea · probador</title>
</head>
<body>
<div id="app"></div>
<script type="module" src="/src/main.js"></script>
</body>
</html>
```

`web/src/main.js`:

```js
import { mount } from "svelte";
import "./estilo.css";
import App from "./App.svelte";

mount(App, { target: document.getElementById("app") });
```

`web/src/App.svelte` (provisional; la Tarea 4 lo reemplaza):

```svelte
<header><h1>Galatea</h1></header>
<main><p data-andamiaje>Cliente en construcción.</p></main>
```

`web/src/estilo.css`: copia exacta de `estaticos/estilo.css` (no lo borres de ahí todavía).

`web/src/humo.test.js` (para que Vitest tenga algo que correr y el check falle si Vitest se rompe):

```js
import { describe, expect, it } from "vitest";

describe("andamiaje", () => {
  it("vitest corre", () => {
    expect(1 + 1).toBe(2);
  });
});
```

`web/.gitignore`:

```
node_modules/
dist/
```

- [ ] **Paso 2: Instalar y fijar dependencias**

```bash
cd web && npm install && npm run build && npm test && cd ..
```

Esperado: `dist/index.html` y `dist/assets/index-*.js` existen; Vitest `1 passed`. Si Vitest 5 y Vite 8 se pelean por versiones de pares, instala la combinación más nueva que funcione y dilo en el informe.

- [ ] **Paso 3: `scripts/web.sh`**

```bash
#!/usr/bin/env bash
# Compila el cliente Svelte en web/dist. Lo sirve el servidor Rust (GALATEA_WEB) y lo
# empaqueta Capacitor. Hace `npm ci` solo si node_modules falta o el lock cambió.
set -euo pipefail
cd "$(dirname "$0")/../web"
if [ ! -f node_modules/.package-lock.json ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
  npm ci
fi
npm run build
```

`chmod +x scripts/web.sh`.

- [ ] **Paso 4: Flake: Node 22, build del cliente como check y filtro de fuentes**

En `flake.nix`:

1. En el `devShell`, reemplaza `nodejs` por `nodejs_22` (Capacitor 8 exige Node 22 o superior).
2. Añade antes de `in { … }`:

```nix
        # El cliente Svelte, compilado sin red: las dependencias vienen fijadas por
        # package-lock.json y npmDepsHash. Cambiar dependencias = actualizar el hash con
        #   nix run nixpkgs#prefetch-npm-deps -- web/package-lock.json
        web = pkgs.buildNpmPackage {
          pname = "galatea-web";
          version = "0.1.0";
          src = pkgs.lib.cleanSourceWith {
            src = ./web;
            filter = ruta: tipo:
              !(tipo == "directory" && builtins.elem (baseNameOf ruta) [ "node_modules" "android" "dist" ]);
            name = "web";
          };
          npmDepsHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
          nodejs = pkgs.nodejs_22;
          npmBuildScript = "build";
          doCheck = true;
          checkPhase = "npm test";
          installPhase = "cp -r dist $out";
        };
```

3. Expón `packages.web = web;` y `checks.web = web;`.
4. En `fuentes`, excluye la carpeta `web/` de la raíz (si no, `node_modules` entraría al build de Rust). Envuelve el filtro actual:

```nix
          filter = ruta: tipo:
            !(pkgs.lib.hasPrefix (toString ./web) ruta)
            && (
              (builtins.match ".*\\.(html|css|js|toml|json|png|jpg|md)$" ruta != null)
              || (builtins.match ".*/(plantillas|estaticos|pruebas)(/.*)?$" ruta != null)
              || (builtins.baseNameOf ruta == "askama.toml")
              || (craneLib.filterCargoSources ruta tipo)
            );
```

Obtén el hash real y pégalo en `npmDepsHash`:

```bash
nix run nixpkgs#prefetch-npm-deps -- web/package-lock.json
```

- [ ] **Paso 5: Verificar**

```bash
nix build .#web && ls result/ && nix flake check
```

Esperado: `result/index.html` y `result/assets/`; `flake check` en verde con el check `web` incluido (mira que aparezca en la salida con `nix flake check -L 2>&1 | grep -i web`).

Comprueba que el check protege algo: cambia `toBe(2)` por `toBe(3)` en `humo.test.js`, corre `nix build .#web` y mira que falle; deshaz el cambio.

- [ ] **Paso 6: Añadir a `.gitignore` de la raíz**

```
web/dist/
web/node_modules/
```

- [ ] **Paso 7: Commit**

```bash
git add web scripts/web.sh flake.nix .gitignore
git commit -m "web: andamiaje del cliente Svelte y build en Nix"
```

---

### Tarea 2: Servidor: sirve `GALATEA_WEB` y responde CORS

**Files:**
- Modify: `Cargo.toml`, `src/lib.rs`, `playwright.config.js`, `scripts/demo.sh`
- Create: `pruebas/web/index.html`, `pruebas/web/assets/prueba.js`, `tests/estaticos.rs`

**Interfaces:**
- Consumes: `web/dist` de la Tarea 1 (en Playwright y en `demo.sh`).
- Produces: `Conf.web: PathBuf` (`GALATEA_WEB`, por defecto `./web/dist`), `Conf.origenes: Vec<String>` (`GALATEA_ORIGENES`, por defecto `https://localhost`; vacío = sin CORS). `construir_router` sirve archivos de `conf.web` como *fallback* con `index.html` para rutas sin archivo. La ruta `/` sigue mostrando la página antigua hasta la Tarea 5.

- [ ] **Paso 1: Fixture del cliente de mentira**

`pruebas/web/index.html`:

```html
<!doctype html><html lang="es-CL"><head><meta charset="utf-8"><title>cliente de prueba</title></head>
<body><p data-prueba>cliente de prueba</p><script type="module" src="/assets/prueba.js"></script></body></html>
```

`pruebas/web/assets/prueba.js`:

```js
console.log("cliente de prueba");
```

- [ ] **Paso 2: Pruebas que fallan**

`tests/estaticos.rs`:

```rust
//! El servidor sirve el cliente compilado (GALATEA_WEB) y responde CORS al origen de la app.
mod comun;

use galatea::Conf;
use reqwest::{Method, header};
use tempfile::TempDir;

#[tokio::test]
async fn sirve_index_y_assets_de_la_carpeta_web() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/index.html")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/html"));
    assert!(r.text().await.unwrap().contains("cliente de prueba"));

    let r = s.cliente.get(s.url("/assets/prueba.js")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let tipo = r.headers()[header::CONTENT_TYPE].to_str().unwrap().to_string();
    assert!(tipo.contains("javascript"), "content-type fue {tipo}");
}

#[tokio::test]
async fn una_ruta_sin_archivo_devuelve_index_para_la_spa() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/ajustes")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.text().await.unwrap().contains("cliente de prueba"));
}

#[tokio::test]
async fn la_api_sigue_ganando_al_fallback() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/prendas")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.headers()[header::CONTENT_TYPE].to_str().unwrap().starts_with("application/json"));
}

#[tokio::test]
async fn preflight_desde_el_origen_de_la_app_se_acepta() {
    let s = comun::Servidor::nuevo().await;
    let r = s
        .cliente
        .request(Method::OPTIONS, s.url("/sesiones"))
        .header(header::ORIGIN, "https://localhost")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], "https://localhost");
    let metodos = r.headers()[header::ACCESS_CONTROL_ALLOW_METHODS].to_str().unwrap().to_string();
    for m in ["GET", "HEAD", "POST", "DELETE"] {
        assert!(metodos.contains(m), "faltó {m} en {metodos}");
    }
}

#[tokio::test]
async fn un_origen_ajeno_no_recibe_cabeceras_cors() {
    let s = comun::Servidor::nuevo().await;
    let r = s
        .cliente
        .get(s.url("/prendas"))
        .header(header::ORIGIN, "https://otro.example")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
}

#[tokio::test]
async fn sin_origenes_configurados_no_hay_cors() {
    let temporal = TempDir::new().unwrap();
    let mut conf = Conf::para_pruebas(temporal.path());
    conf.origenes = vec![];
    let s = comun::Servidor::con_conf(conf, temporal).await;
    let r = s
        .cliente
        .get(s.url("/prendas"))
        .header(header::ORIGIN, "https://localhost")
        .send()
        .await
        .unwrap();
    assert!(r.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).is_none());
}

#[tokio::test]
async fn sin_cliente_compilado_preparar_falla_con_instruccion() {
    let temporal = TempDir::new().unwrap();
    let mut conf = Conf::para_pruebas(temporal.path());
    conf.web = temporal.path().join("no-existe");
    let error = galatea::preparar(&conf).await.err().expect("debía fallar").to_string();
    assert!(error.contains("scripts/web.sh"), "mensaje: {error}");
}
```

- [ ] **Paso 3: Correr y ver que falla**

```bash
nix develop -c cargo test --test estaticos
```

Esperado: error de compilación (`conf.origenes`, `conf.web` no existen).

- [ ] **Paso 4: Dependencia**

`Cargo.toml`, en `[dependencies]` (versión comprobada el 2026-09-27; vuelve a comprobar):

```toml
tower-http = { version = "0.7.1", features = ["fs", "cors"] }
```

- [ ] **Paso 5: `Conf`**

En `src/lib.rs`, añade a `Conf`:

```rust
    /// Carpeta con el cliente compilado (index.html y assets/). Se sirve como fallback.
    pub web: PathBuf,
    /// Orígenes que pueden llamar a la API desde otro origen (la app Capacitor). Vacío = sin CORS.
    pub origenes: Vec<String>,
```

En `desde_entorno`:

```rust
            web: PathBuf::from(variable("GALATEA_WEB", "./web/dist")),
            origenes: variable("GALATEA_ORIGENES", "https://localhost")
                .split(',')
                .map(str::trim)
                .filter(|o| !o.is_empty())
                .map(str::to_string)
                .collect(),
```

En `para_pruebas`:

```rust
            web: PathBuf::from("./pruebas/web"),
            origenes: vec!["https://localhost".to_string()],
```

- [ ] **Paso 6: `preparar` comprueba el cliente**

Al inicio de `preparar`, tras crear `datos`:

```rust
    if !conf.web.join("index.html").is_file() {
        return Err(anyhow!(
            "no existe el cliente web en {} (falta index.html); compílalo con scripts/web.sh \
             o indica la carpeta con GALATEA_WEB",
            conf.web.display()
        ));
    }
```

- [ ] **Paso 7: `construir_router` con `ServeDir` y CORS**

Imports nuevos en `src/lib.rs`:

```rust
use axum::http::{HeaderValue, Method, header};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
```

Reemplaza la línea `.fallback(|| async { web::error::Fallo::no_encontrado("esa ruta no existe") })` y el cierre por:

```rust
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
```

(convierte el encadenamiento en `let router = Router::new()…;` para poder ramificar). Añade:

```rust
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
```

`Fallo::no_encontrado` sigue usándose en los manejadores; si clippy avisa de algo sin uso, quítalo.

- [ ] **Paso 8: Correr las pruebas**

```bash
nix develop -c cargo test --test estaticos && nix develop -c cargo test
```

Esperado: 7 nuevas en verde y el resto igual que antes. Rompe a propósito: comenta `.allow_methods(...)` y mira que `preflight_desde_el_origen_de_la_app_se_acepta` falle; deshaz.

- [ ] **Paso 9: Playwright y `demo.sh` con el cliente compilado**

`playwright.config.js`, en `webServer`:

```js
    command: "sh -c 'scripts/web.sh && cargo run --quiet'",
    timeout: 300_000,
```

y en `env`: `GALATEA_WEB: "web/dist",`. Las seis pruebas siguen apuntando a la página antigua en `/`, así que deben seguir en verde:

```bash
nix develop -c npx playwright test
```

`scripts/demo.sh`: antes de `exec nix run . -- servir` añade:

```bash
scripts/web.sh
export GALATEA_WEB=${GALATEA_WEB:-web/dist}
```

- [ ] **Paso 10: `nix flake check` y commit**

```bash
nix flake check
git add Cargo.toml Cargo.lock src/lib.rs tests/estaticos.rs pruebas/web playwright.config.js scripts/demo.sh
git commit -m "servidor: sirve el cliente compilado y responde CORS a la app"
```

---

### Tarea 3: Lógica del cliente: `ajustes.js`, `api.js`, `estado.js` con Vitest

**Files:**
- Create: `web/src/ajustes.js`, `web/src/api.js`, `web/src/estado.js`, `web/src/ajustes.test.js`, `web/src/api.test.js`, `web/src/estado.test.js`
- Delete: `web/src/humo.test.js`

**Interfaces:**
- Consumes: la API HTTP de la demo (`/prendas`, `/flujos`, `POST /sesiones`, `POST /sesiones/{id}/generar`, SSE `/sesiones/{id}/avance/{trabajo}`, `HEAD|GET /sesiones/{id}/resultado/{n}.png`, `/sesiones/{id}/persona.jpg`, `DELETE /sesiones/{id}`, `/salud`).
- Produces (los usa la Tarea 4):
  - `ajustes.js`: `servidor(): string`, `guardarServidor(valor: string): void` (lanza `Error` si `validarServidor` no devuelve `""`), `validarServidor(valor: string): string` (`""` = válido), `normalizarServidor(valor): string`, `porDefecto(): string`.
  - `api.js`: `class FalloApi extends Error { clase: "red"|"http"|"vencida", estado: number|null }`, `url(ruta)`, `prendas()`, `flujos()`, `crearSesion(foto: Blob): Promise<{id}>`, `generar(id, prenda, flujo): Promise<{trabajo, resultado}>`, `avance(id, trabajo, oyentes: {encola, progreso, listo, error}): {cerrar()}`, `buscarResultado(id, n, opciones): Promise<"listo"|"agotado">`, `resultadoUrl(id, n)`, `personaUrl(id)`, `terminar(id, {keepalive}): Promise<boolean>`, `salud()`, `SONDEO_MS = 2000`, `SONDEO_LIMITE_MS = 300_000`, `VENCIDA` (texto).
  - `estado.js`: `estado` (store `writable`), `inicial()`, acciones `entendido`, `elegirPrenda(p)`, `fotoLista(blob)`, `descartarFoto`, `usarFoto`, `generar`, `reintentar`, `volverAPrendas`, `otraPrenda`, `repetirFoto`, `terminar`, `elegirFlujo(nombre)`, `abrirAjustes`, `cerrarAjustes`, `avisar(texto, error)`, `ocultarMensaje`, `activarFlujos()`.

- [ ] **Paso 1: `ajustes.js` con sus pruebas**

`web/src/ajustes.test.js`:

```js
import { beforeEach, describe, expect, it, vi } from "vitest";
import { guardarServidor, normalizarServidor, porDefecto, servidor, validarServidor } from "./ajustes.js";

function almacenFalso() {
  const m = new Map();
  return { getItem: k => (m.has(k) ? m.get(k) : null), setItem: (k, v) => m.set(k, String(v)), removeItem: k => m.delete(k) };
}

describe("ajustes", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", almacenFalso());
    vi.stubEnv("VITE_SERVIDOR", "");
  });

  it("sin nada guardado usa el valor por defecto del build", () => {
    expect(servidor()).toBe("");
    vi.stubEnv("VITE_SERVIDOR", "https://192.168.0.10:8444");
    expect(porDefecto()).toBe("https://192.168.0.10:8444");
    expect(servidor()).toBe("https://192.168.0.10:8444");
  });

  it("guarda y normaliza sin barra final", () => {
    guardarServidor("https://192.168.0.20:8444/");
    expect(servidor()).toBe("https://192.168.0.20:8444");
  });

  it("vacío vuelve al valor por defecto", () => {
    guardarServidor("https://192.168.0.20:8444");
    guardarServidor("");
    expect(servidor()).toBe("");
  });

  it("rechaza lo que no sea https", () => {
    expect(validarServidor("http://192.168.0.20:8444")).toMatch(/https/);
    expect(validarServidor("192.168.0.20")).toMatch(/https/);
    expect(validarServidor("")).toBe("");
    expect(validarServidor("https://x.local")).toBe("");
    expect(() => guardarServidor("ftp://x")).toThrow();
  });

  it("si localStorage falla, sigue funcionando con el valor por defecto", () => {
    vi.stubGlobal("localStorage", { getItem() { throw new Error("bloqueado"); }, setItem() { throw new Error("bloqueado"); } });
    expect(servidor()).toBe("");
    expect(() => guardarServidor("https://x.local")).not.toThrow();
  });
});
```

`web/src/ajustes.js`:

```js
// Dirección base del servidor. Vacía = mismo origen (la web servida por Rust).
// En el APK el build fija VITE_SERVIDOR y la persona puede cambiarla en Ajustes.
const CLAVE = "galatea.servidor";

export function porDefecto() {
  return normalizarServidor(import.meta.env.VITE_SERVIDOR ?? "");
}

export function normalizarServidor(valor) {
  return String(valor ?? "").trim().replace(/\/+$/, "");
}

/** "" si es válida; si no, el mensaje para la persona. Vacía es válida (mismo origen). */
export function validarServidor(valor) {
  const v = normalizarServidor(valor);
  if (v === "") return "";
  let u;
  try { u = new URL(v); } catch { return "Escribe la dirección completa, por ejemplo https://192.168.0.10:8444"; }
  if (u.protocol !== "https:") return "La dirección debe empezar con https:// (la cámara y la app lo exigen)";
  if (u.pathname !== "/" || u.search || u.hash) return "Deja solo https://dirección:puerto, sin ruta";
  return "";
}

export function servidor() {
  try {
    const guardado = localStorage.getItem(CLAVE);
    if (guardado !== null) return normalizarServidor(guardado);
  } catch { /* sin almacenamiento: se usa el valor por defecto */ }
  return porDefecto();
}

export function guardarServidor(valor) {
  const error = validarServidor(valor);
  if (error) throw new Error(error);
  const v = normalizarServidor(valor);
  try {
    if (v === porDefecto()) localStorage.removeItem(CLAVE);
    else localStorage.setItem(CLAVE, v);
  } catch { /* sin almacenamiento: vale solo hasta cerrar la app */ }
}
```

```bash
cd web && npx vitest run src/ajustes.test.js
```

Esperado: 5 en verde (antes de crear `ajustes.js`, comprueba que fallan por módulo inexistente).

- [ ] **Paso 2: `api.js` con sus pruebas**

`web/src/api.test.js`:

```js
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as api from "./api.js";

function respuesta(estado, cuerpo) {
  return {
    ok: estado >= 200 && estado < 300,
    status: estado,
    json: async () => { if (cuerpo === undefined) throw new Error("sin json"); return cuerpo; },
  };
}

describe("api", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", { getItem: () => null, setItem() {}, removeItem() {} });
    vi.stubEnv("VITE_SERVIDOR", "");
  });
  afterEach(() => { vi.unstubAllGlobals(); vi.unstubAllEnvs(); });

  it("con base vacía las URL son relativas; con base remota, absolutas", () => {
    expect(api.url("/prendas")).toBe("/prendas");
    vi.stubEnv("VITE_SERVIDOR", "https://192.168.0.10:8444");
    expect(api.url("/prendas")).toBe("https://192.168.0.10:8444/prendas");
    expect(api.personaUrl("abc")).toBe("https://192.168.0.10:8444/sesiones/abc/persona.jpg");
    expect(api.resultadoUrl("abc", 2)).toMatch(/^https:\/\/192\.168\.0\.10:8444\/sesiones\/abc\/resultado\/2\.png\?t=\d+$/);
  });

  it("pedir devuelve el JSON y manda el cuerpo correcto en generar", async () => {
    const fetch = vi.fn(async () => respuesta(200, { trabajo: "t1", resultado: 1 }));
    vi.stubGlobal("fetch", fetch);
    const r = await api.generar("abc", "roja", "");
    expect(r).toEqual({ trabajo: "t1", resultado: 1 });
    const [url, opciones] = fetch.mock.calls[0];
    expect(url).toBe("/sesiones/abc/generar");
    expect(JSON.parse(opciones.body)).toEqual({ prenda: "roja" });
    await api.generar("abc", "roja", "leffa");
    expect(JSON.parse(fetch.mock.calls[1][1].body)).toEqual({ prenda: "roja", flujo: "leffa" });
  });

  it("clasifica los fallos: red, vencida (404) y http", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => { throw new TypeError("Failed to fetch"); }));
    await expect(api.prendas()).rejects.toMatchObject({ clase: "red", estado: null });
    vi.stubGlobal("fetch", vi.fn(async () => respuesta(404, { error: "esa sesión no existe" })));
    await expect(api.generar("abc", "roja", "")).rejects.toMatchObject({ clase: "vencida", estado: 404, message: api.VENCIDA });
    vi.stubGlobal("fetch", vi.fn(async () => respuesta(503, { error: "motor apagado" })));
    await expect(api.generar("abc", "roja", "")).rejects.toMatchObject({ clase: "http", estado: 503, message: "motor apagado" });
    vi.stubGlobal("fetch", vi.fn(async () => respuesta(500)));
    await expect(api.prendas()).rejects.toMatchObject({ clase: "http", message: "error 500" });
  });

  it("terminar confirma con 204 o 404 y no con otros", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => respuesta(204)));
    expect(await api.terminar("abc")).toBe(true);
    vi.stubGlobal("fetch", vi.fn(async () => respuesta(404)));
    expect(await api.terminar("abc")).toBe(true);
    vi.stubGlobal("fetch", vi.fn(async () => respuesta(500)));
    expect(await api.terminar("abc")).toBe(false);
    vi.stubGlobal("fetch", vi.fn(async () => { throw new TypeError("sin red"); }));
    expect(await api.terminar("abc")).toBe(false);
  });

  it("buscarResultado insiste hasta que el HEAD responde 200", async () => {
    const estados = [500, 404, 200];
    vi.stubGlobal("fetch", vi.fn(async () => respuesta(estados.shift())));
    const esperas = [];
    const r = await api.buscarResultado("abc", 1, { esperar: ms => { esperas.push(ms); return Promise.resolve(); } });
    expect(r).toBe("listo");
    expect(esperas).toEqual([2000, 2000, 2000]);
  });

  it("buscarResultado se rinde al agotar el límite", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => respuesta(404)));
    let t = 0;
    const r = await api.buscarResultado("abc", 1, { esperar: async () => { t += 100_000; }, ahora: () => t, limite: 250_000 });
    expect(r).toBe("agotado");
  });

  it("buscarResultado para si la sesión dejó de ser la vigente", async () => {
    const fetch = vi.fn(async () => respuesta(404));
    vi.stubGlobal("fetch", fetch);
    let vueltas = 0;
    const r = await api.buscarResultado("abc", 1, { esperar: async () => {}, vigente: () => ++vueltas < 3 });
    expect(r).toBe("abandonado");
    expect(fetch).toHaveBeenCalledTimes(1); // la segunda vuelta se detiene antes de preguntar
  });

  it("el límite de sondeo cubre los 240 s del servidor", () => {
    expect(api.SONDEO_LIMITE_MS).toBeGreaterThanOrEqual(240_000);
  });
});
```

`web/src/api.js`:

```js
// Todo lo que habla con el servidor. JavaScript puro, sin Svelte, para probarlo con Vitest.
import { servidor } from "./ajustes.js";

export const SONDEO_MS = 2000;
// Por encima de GALATEA_LIMITE_SEG=240 que exporta scripts/demo.sh.
export const SONDEO_LIMITE_MS = 300_000;
export const VENCIDA = "Pasaron más de 15 minutos; la foto ya se borró. Toma una nueva.";

export class FalloApi extends Error {
  /** @param {"red"|"http"|"vencida"} clase */
  constructor(clase, estado, texto) {
    super(texto);
    this.clase = clase;
    this.estado = estado;
  }
}

export function url(ruta) {
  return servidor() + ruta;
}

async function pedir(ruta, opciones = {}) {
  let r;
  try {
    r = await fetch(url(ruta), opciones);
  } catch (e) {
    throw new FalloApi("red", null, `No encuentro el servidor en ${servidor() || "esta dirección"} (${e.message}).`);
  }
  if (r.status === 204) return null;
  const cuerpo = await r.json().catch(() => ({}));
  if (r.status === 404) throw new FalloApi("vencida", 404, VENCIDA);
  if (!r.ok) throw new FalloApi("http", r.status, cuerpo.error || `error ${r.status}`);
  return cuerpo;
}

export const prendas = () => pedir("/prendas");
export const flujos = () => pedir("/flujos");
export const salud = () => pedir("/salud");

export function crearSesion(foto) {
  const datos = new FormData();
  datos.append("persona", foto, "persona.jpg");
  return pedir("/sesiones", { method: "POST", body: datos });
}

export function generar(id, prenda, flujo) {
  const cuerpo = { prenda };
  if (flujo) cuerpo.flujo = flujo;
  return pedir(`/sesiones/${id}/generar`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(cuerpo),
  });
}

/** SSE del avance. `oyentes.error(texto|null)`: con texto es un error del servidor; null, se cortó la conexión. */
export function avance(id, trabajo, oyentes) {
  const f = new EventSource(url(`/sesiones/${id}/avance/${trabajo}`));
  f.addEventListener("encola", e => oyentes.encola(Number(e.data)));
  f.addEventListener("progreso", e => oyentes.progreso(Number(e.data)));
  f.addEventListener("listo", e => { f.close(); oyentes.listo(Number(e.data)); });
  f.addEventListener("error", e => { f.close(); oyentes.error(e.data || null); });
  return { cerrar: () => f.close() };
}

/**
 * Cuando se cayó el SSE: pregunta con HEAD cada `cada` ms si el resultado ya existe.
 * Devuelve "listo", "agotado" (pasó `limite`) o "abandonado" (`vigente()` dio false).
 */
export async function buscarResultado(id, n, opciones = {}) {
  const {
    cada = SONDEO_MS,
    limite = SONDEO_LIMITE_MS,
    vigente = () => true,
    ahora = () => Date.now(),
    esperar = ms => new Promise(r => setTimeout(r, ms)),
  } = opciones;
  const hasta = ahora() + limite;
  for (;;) {
    await esperar(cada);
    if (!vigente()) return "abandonado";
    try {
      const r = await fetch(url(`/sesiones/${id}/resultado/${n}.png`), { method: "HEAD" });
      if (!vigente()) return "abandonado";
      if (r.ok) return "listo";
    } catch { /* sin red todavía: se reintenta */ }
    if (ahora() >= hasta) return "agotado";
  }
}

export const resultadoUrl = (id, n) => url(`/sesiones/${id}/resultado/${n}.png?t=${Date.now()}`);
export const personaUrl = id => url(`/sesiones/${id}/persona.jpg`);

/** true solo si el servidor confirmó el borrado: 204, o 404 si ya no existía. */
export async function terminar(id, { keepalive = false } = {}) {
  try {
    const r = await fetch(url(`/sesiones/${id}`), { method: "DELETE", keepalive });
    return r.status === 204 || r.status === 404;
  } catch {
    return false;
  }
}
```

```bash
cd web && npx vitest run src/api.test.js
```

Esperado: 8 en verde.

- [ ] **Paso 3: `estado.js` con sus pruebas**

`web/src/estado.test.js` (mockea `api.js`):

```js
import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

vi.mock("./api.js", async importar => {
  const real = await importar();
  return {
    ...real,
    prendas: vi.fn(async () => [{ id: "roja", nombre: "Roja", miniatura: "/prendas/roja/miniatura.png" }]),
    crearSesion: vi.fn(async () => ({ id: "s1" })),
    generar: vi.fn(async () => ({ trabajo: "t1", resultado: 1 })),
    avance: vi.fn(() => ({ cerrar: vi.fn() })),
    buscarResultado: vi.fn(async () => "listo"),
    terminar: vi.fn(async () => true),
    flujos: vi.fn(async () => [{ nombre: "fijo", titulo: "Fijo", disponible: true }]),
  };
});

import * as api from "./api.js";
import * as acciones from "./estado.js";

const { estado } = acciones;
const foto = new Blob(["x"], { type: "image/jpeg" });
const prenda = { id: "roja", nombre: "Roja", miniatura: "/prendas/roja/miniatura.png" };

describe("estado", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    estado.set(acciones.inicial());
  });

  it("arranca en el aviso y pasa a prendas", () => {
    expect(get(estado).pantalla).toBe("aviso");
    acciones.entendido();
    expect(get(estado).pantalla).toBe("prendas");
  });

  it("elegir prenda sin foto abre la cámara; con sesión y foto, genera directo", async () => {
    acciones.elegirPrenda(prenda);
    expect(get(estado).pantalla).toBe("camara");
    expect(get(estado).prenda).toEqual(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    expect(api.crearSesion).toHaveBeenCalledWith(foto);
    expect(api.generar).toHaveBeenCalledWith("s1", "roja", "");
    expect(get(estado).pantalla).toBe("generando");
    expect(get(estado).sesion).toEqual({ id: "s1", trabajo: "t1", resultado: 1 });
    // Segunda prenda con la misma foto: no se crea otra sesión.
    acciones.otraPrenda();
    expect(get(estado).pantalla).toBe("prendas");
    await acciones.elegirPrenda({ ...prenda, id: "azul" });
    expect(api.crearSesion).toHaveBeenCalledTimes(1);
    expect(api.generar).toHaveBeenLastCalledWith("s1", "azul", "");
  });

  it("el avance actualiza el progreso y «listo» muestra el resultado", async () => {
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    const oyentes = api.avance.mock.calls[0][2];
    oyentes.encola(2);
    expect(get(estado).progreso.texto).toBe("Hay 2 delante en la cola");
    oyentes.progreso(0.5);
    expect(get(estado).progreso).toEqual({ valor: 0.5, texto: "50 %" });
    oyentes.listo(1);
    expect(get(estado).pantalla).toBe("resultado");
    expect(get(estado).sesion.resultado).toBe(1);
  });

  it("si el SSE se corta sin datos, va a buscar el resultado", async () => {
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    const oyentes = api.avance.mock.calls[0][2];
    await oyentes.error(null);
    expect(api.buscarResultado).toHaveBeenCalledWith("s1", 1, expect.any(Object));
    expect(get(estado).pantalla).toBe("resultado");
  });

  it("si el servidor falla, queda el mensaje y los botones de reintentar/volver", async () => {
    api.generar.mockRejectedValueOnce(new api.FalloApi("http", 503, "motor apagado"));
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    expect(get(estado).pantalla).toBe("generando");
    expect(get(estado).mensaje).toEqual({ texto: "motor apagado", error: true });
    expect(get(estado).fallo).toBe(true);
    acciones.volverAPrendas();
    expect(get(estado).pantalla).toBe("prendas");
    expect(get(estado).sesion.id).toBe("s1"); // la sesión y la foto se conservan
  });

  it("404 al generar = sesión vencida: vuelve a la cámara sin foto y con el aviso", async () => {
    api.generar.mockRejectedValueOnce(new api.FalloApi("vencida", 404, api.VENCIDA));
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    const e = get(estado);
    expect(e.pantalla).toBe("camara");
    expect(e.foto).toBeNull();
    expect(e.sesion.id).toBeNull();
    expect(e.mensaje).toEqual({ texto: api.VENCIDA, error: true });
  });

  it("terminar borra la sesión y lo dice; si no se confirma, lo dice como error", async () => {
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    await acciones.terminar();
    expect(api.terminar).toHaveBeenCalledWith("s1");
    expect(get(estado).pantalla).toBe("prendas");
    expect(get(estado).sesion.id).toBeNull();
    expect(get(estado).mensaje.texto).toContain("se borraron");
    api.terminar.mockResolvedValueOnce(false);
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    await acciones.terminar();
    expect(get(estado).mensaje).toMatchObject({ error: true });
    expect(get(estado).mensaje.texto).toContain("No se pudo confirmar");
  });

  it("repetir foto termina la sesión y vuelve a la cámara", async () => {
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    await acciones.repetirFoto();
    expect(api.terminar).toHaveBeenCalledWith("s1");
    expect(get(estado).pantalla).toBe("camara");
    expect(get(estado).foto).toBeNull();
  });

  it("elegir flujo vuelve a generar con ese flujo", async () => {
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    await acciones.elegirFlujo("leffa");
    expect(api.generar).toHaveBeenLastCalledWith("s1", "roja", "leffa");
  });

  it("ajustes recuerda a dónde volver", () => {
    acciones.entendido();
    acciones.abrirAjustes();
    expect(get(estado).pantalla).toBe("ajustes");
    acciones.cerrarAjustes();
    expect(get(estado).pantalla).toBe("prendas");
  });

  it("un fallo de red al elegir prenda deja el mensaje con la dirección y el botón de ajustes", async () => {
    api.crearSesion.mockRejectedValueOnce(new api.FalloApi("red", null, "No encuentro el servidor en https://x (fetch)."));
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    expect(get(estado).pantalla).toBe("camara");
    expect(get(estado).mensaje).toEqual({ texto: "No encuentro el servidor en https://x (fetch).", error: true, ajustes: true });
  });
});
```

`web/src/estado.js`:

```js
// La máquina de estados de la demo. Los componentes leen `estado` y llaman a estas acciones;
// nadie más modifica el store.
import { get, writable } from "svelte/store";
import * as api from "./api.js";

export function inicial() {
  return {
    pantalla: "aviso", // aviso | prendas | camara | generando | resultado | ajustes
    prenda: null,
    flujo: "",
    conFlujos: false, // ?flujos=1 muestra el selector de motor en el resultado
    foto: null, // Blob de la foto tomada o subida, mientras dura la sesión
    sesion: { id: null, trabajo: null, resultado: null },
    progreso: { valor: 0, texto: "Preparando" },
    fallo: false, // en «generando»: mostrar Reintentar / Volver
    mensaje: null, // { texto, error, ajustes? }
    volverA: "aviso", // a dónde regresa Ajustes
  };
}

export const estado = writable(inicial());

let fuente = null; // el SSE abierto, si hay
function cerrarFuente() {
  if (fuente) { fuente.cerrar(); fuente = null; }
}
const cambiar = parche => estado.update(e => ({ ...e, ...parche }));

export function avisar(texto, error = false, extra = {}) {
  cambiar({ mensaje: { texto, error, ...extra } });
}
export function ocultarMensaje() {
  cambiar({ mensaje: null });
}
export function activarFlujos() {
  cambiar({ conFlujos: true });
}

function mostrar(pantalla) {
  cambiar({ pantalla, mensaje: null });
}

function informarFallo(e) {
  // Un fallo de red ofrece el botón de Ajustes: casi siempre es la dirección del servidor.
  const esRed = e instanceof api.FalloApi && e.clase === "red";
  avisar(e.message, true, esRed ? { ajustes: true } : {});
}

export function entendido() {
  mostrar("prendas");
}

export async function elegirPrenda(prenda) {
  cambiar({ prenda });
  const { sesion, foto } = get(estado);
  if (sesion.id && foto) await generar();
  else mostrar("camara");
}

export function fotoLista(foto) {
  cambiar({ foto, mensaje: null });
}
export function descartarFoto() {
  cambiar({ foto: null });
}

export async function usarFoto() {
  const { foto } = get(estado);
  try {
    const { id } = await api.crearSesion(foto);
    cambiar({ sesion: { id, trabajo: null, resultado: null } });
  } catch (e) {
    informarFallo(e);
    return;
  }
  await generar();
}

export async function generar() {
  cerrarFuente();
  cambiar({ pantalla: "generando", progreso: { valor: 0, texto: "Preparando" }, fallo: false, mensaje: null });
  const { sesion, prenda, flujo } = get(estado);
  try {
    const { trabajo, resultado } = await api.generar(sesion.id, prenda.id, flujo);
    cambiar({ sesion: { ...sesion, trabajo, resultado } });
    seguir(sesion.id, trabajo, resultado);
  } catch (e) {
    if (e instanceof api.FalloApi && e.clase === "vencida") { sesionVencida(); return; }
    fallar(e);
  }
}
export const reintentar = generar;

function fallar(e) {
  informarFallo(e);
  cambiar({ fallo: true });
}

function sesionVencida() {
  cerrarFuente();
  cambiar({ pantalla: "camara", foto: null, sesion: { id: null, trabajo: null, resultado: null } });
  avisar(api.VENCIDA, true);
}

function seguir(id, trabajo, resultado) {
  fuente = api.avance(id, trabajo, {
    encola: n => cambiar({ progreso: { valor: 0, texto: n === 0 ? "Empezando" : `Hay ${n} delante en la cola` } }),
    progreso: v => cambiar({ progreso: { valor: v, texto: `${Math.round(v * 100)} %` } }),
    listo: n => { fuente = null; mostrarResultado(n); },
    error: async texto => {
      fuente = null;
      if (texto) { fallar(new api.FalloApi("http", null, texto)); return; }
      // Sin datos se cortó la conexión (celular bloqueado, cambio de red): el servidor igual
      // termina y sella, así que se va a buscar el resultado.
      cambiar({ progreso: { valor: 0, texto: "Se cortó la conexión; esperando el resultado" } });
      const vigente = () => { const s = get(estado).sesion; return s.id === id && s.resultado === resultado; };
      const fin = await api.buscarResultado(id, resultado, { vigente });
      if (fin === "listo") mostrarResultado(resultado);
      else if (fin === "agotado") fallar(new api.FalloApi("red", null, "Se perdió la conexión con el servidor"));
    },
  });
}

function mostrarResultado(n) {
  estado.update(e => ({ ...e, sesion: { ...e.sesion, resultado: n }, pantalla: "resultado", mensaje: null }));
}

export function volverAPrendas() {
  cerrarFuente();
  mostrar("prendas");
}
export function otraPrenda() {
  mostrar("prendas");
}

export async function elegirFlujo(flujo) {
  cambiar({ flujo });
  await generar();
}

async function cerrarSesion() {
  const { sesion } = get(estado);
  cerrarFuente();
  const borrada = sesion.id ? await api.terminar(sesion.id) : true;
  cambiar({ foto: null, sesion: { id: null, trabajo: null, resultado: null } });
  return borrada;
}

export async function repetirFoto() {
  await cerrarSesion();
  mostrar("camara");
}

export async function terminar() {
  const borrada = await cerrarSesion();
  mostrar("prendas");
  if (borrada) avisar("Listo: la foto y las imágenes se borraron del servidor.");
  else avisar("No se pudo confirmar el borrado en el servidor. La sesión se borra sola a los 15 minutos.", true);
}

export function abrirAjustes() {
  estado.update(e => ({ ...e, volverA: e.pantalla, pantalla: "ajustes", mensaje: null }));
}
export function cerrarAjustes() {
  estado.update(e => ({ ...e, pantalla: e.volverA, mensaje: null }));
}

/** Para `pagehide` en la web: borra la sesión aunque la página se esté cerrando. */
export function alCerrarPagina() {
  const { sesion } = get(estado);
  if (sesion.id) api.terminar(sesion.id, { keepalive: true });
}
```

```bash
cd web && npx vitest run
```

Esperado: todas en verde (5 + 8 + 11). Borra `web/src/humo.test.js`.

- [ ] **Paso 4: Check de Nix, hash y commit**

`svelte/store` viene con `svelte`, sin dependencias nuevas, así que `npmDepsHash` no cambia. Comprueba:

```bash
nix build .#web && nix flake check
git add web/src
git rm -q web/src/humo.test.js
git commit -m "web: lógica del cliente (ajustes, api, estado) con pruebas"
```

---

### Tarea 4: Pantallas Svelte y Playwright contra `/index.html`

**Files:**
- Create: `web/src/pantallas/Aviso.svelte`, `Prendas.svelte`, `Camara.svelte`, `Generando.svelte`, `Resultado.svelte`, `Ajustes.svelte`
- Modify: `web/src/App.svelte`, `web/src/main.js`, `web/src/estilo.css`, `tests/e2e/demo.spec.js`

**Interfaces:**
- Consumes: `estado.js`, `api.js`, `ajustes.js` de la Tarea 3.
- Produces: el cliente completo en `web/dist`. Hasta la Tarea 5 se prueba en `/index.html` (lo sirve el fallback de la Tarea 2); la ruta `/` sigue siendo la página antigua.

- [ ] **Paso 1: `App.svelte`**

```svelte
<script>
  import { onMount } from "svelte";
  import { abrirAjustes, activarFlujos, alCerrarPagina, estado } from "./estado.js";
  import { salud } from "./api.js";
  import Aviso from "./pantallas/Aviso.svelte";
  import Prendas from "./pantallas/Prendas.svelte";
  import Camara from "./pantallas/Camara.svelte";
  import Generando from "./pantallas/Generando.svelte";
  import Resultado from "./pantallas/Resultado.svelte";
  import Ajustes from "./pantallas/Ajustes.svelte";

  const pantallas = { aviso: Aviso, prendas: Prendas, camara: Camara, generando: Generando, resultado: Resultado, ajustes: Ajustes };
  let version = $state("");
  const Pantalla = $derived(pantallas[$estado.pantalla]);

  onMount(() => {
    if (new URLSearchParams(location.search).get("flujos") === "1") activarFlujos();
    salud().then(s => { version = s.version; }).catch(() => {});
    window.addEventListener("pagehide", alCerrarPagina);
    return () => window.removeEventListener("pagehide", alCerrarPagina);
  });
</script>

<header>
  <h1>Galatea</h1>
  {#if version}<span class="version">v{version}</span>{/if}
</header>
<main>
  <section data-estado={$estado.pantalla}>
    <Pantalla />
  </section>
  {#if $estado.mensaje}
    <p class="mensaje" class:error={$estado.mensaje.error} data-mensaje>
      {$estado.mensaje.texto}
      {#if $estado.mensaje.ajustes && $estado.pantalla !== "ajustes"}
        <button class="secundario" data-accion="ir-ajustes" onclick={abrirAjustes}>Ajustes</button>
      {/if}
    </p>
  {/if}
</main>
```

`main.js` no cambia: ya monta `App`.

- [ ] **Paso 2: Pantallas**

`Aviso.svelte` (la Tarea 7 le añade la variante para la app):

```svelte
<script>
  import { abrirAjustes, entendido } from "../estado.js";
</script>

<h2>Antes de la foto</h2>
<p>La foto sirve solo para mostrar cómo queda la prenda. Se borra al terminar o a los quince minutos, y no se usa para entrenar nada. No se fotografía a menores.</p>
<button class="principal" data-accion="entendido" onclick={entendido}>Entendido</button>
<button class="enlace" data-accion="abrir-ajustes" onclick={abrirAjustes}>Ajustes del servidor</button>
```

`Prendas.svelte`:

```svelte
<script>
  import { onMount } from "svelte";
  import { avisar, elegirPrenda } from "../estado.js";
  import { prendas } from "../api.js";
  let lista = $state([]);
  onMount(async () => {
    try { lista = await prendas(); } catch (e) { avisar(e.message, true, { ajustes: e.clase === "red" }); }
  });
</script>

<h2>Elige una prenda</h2>
<div class="cuadricula" data-lista-prendas>
  {#each lista as p (p.id)}
    <button data-prenda={p.id} onclick={() => elegirPrenda(p)}>
      <img alt="" src={p.miniatura}><span>{p.nombre}</span>
    </button>
  {/each}
</div>
```

`p.miniatura` es relativa (`/prendas/<id>/miniatura.png`); en el APK hace falta la base: usa `src={url(p.miniatura)}` importando `url` de `../api.js`. Hazlo así desde ya.

`Camara.svelte`:

```svelte
<script>
  import { onDestroy, onMount } from "svelte";
  import { avisar, descartarFoto, estado, fotoLista, usarFoto } from "../estado.js";
  import { url } from "../api.js";

  let video = $state();
  let camara = null;
  let previa = $state("");

  async function abrirCamara() {
    if (camara) return;
    try {
      camara = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: { ideal: "environment" }, width: { ideal: 1080 }, height: { ideal: 1440 } },
        audio: false,
      });
      video.srcObject = camara;
    } catch (e) {
      const previo = $estado.mensaje?.error ? $estado.mensaje.texto + " " : "";
      avisar(previo + "No se pudo abrir la cámara (" + e.message + "). Usa «Subir foto».", Boolean(previo));
    }
  }
  function cerrarCamara() {
    if (camara) { camara.getTracks().forEach(t => t.stop()); camara = null; }
    if (video) video.srcObject = null;
  }
  function capturar() {
    if (!camara) { avisar("La cámara no está abierta. Usa «Subir foto»."); return; }
    const canvas = document.createElement("canvas");
    canvas.width = video.videoWidth; canvas.height = video.videoHeight;
    canvas.getContext("2d").drawImage(video, 0, 0);
    canvas.toBlob(mostrarPrevia, "image/jpeg", 0.92);
  }
  function archivo(e) {
    const f = e.target.files[0];
    if (f) mostrarPrevia(f);
    e.target.value = "";
  }
  function mostrarPrevia(blob) {
    if (previa) URL.revokeObjectURL(previa);
    previa = URL.createObjectURL(blob);
    fotoLista(blob);
  }
  function repetir() {
    previa = "";
    descartarFoto();
    abrirCamara();
  }
  async function usar() {
    cerrarCamara();
    await usarFoto();
  }

  onMount(abrirCamara);
  onDestroy(() => { cerrarCamara(); if (previa) URL.revokeObjectURL(previa); });
  // Si el store perdió la foto (sesión vencida), la vista previa también desaparece.
  $effect(() => { if (!$estado.foto && previa) { previa = ""; abrirCamara(); } });
</script>

<div class="prenda-elegida">
  <img data-imagen="prenda-elegida" alt="" src={url($estado.prenda.miniatura)}><span data-nombre-prenda>{$estado.prenda.nombre}</span>
</div>
<div class="visor">
  <video bind:this={video} autoplay playsinline muted hidden={Boolean(previa)}></video>
  {#if previa}<img data-imagen="previa" alt="" src={previa}>{/if}
  <div class="silueta" aria-hidden="true"></div>
</div>
<ul class="guia"><li>De pie</li><li>Cuerpo entero</li><li>Brazos separados</li></ul>
{#if !previa}
  <div class="acciones">
    <button class="principal" data-accion="capturar" onclick={capturar}>Tomar foto</button>
    <label class="secundario">Subir foto<input type="file" accept="image/*" capture="environment" data-accion="archivo" hidden onchange={archivo}></label>
  </div>
{:else}
  <div class="acciones" data-confirmar>
    <button class="principal" data-accion="usar" onclick={usar}>Usar esta</button>
    <button class="secundario" data-accion="repetir" onclick={repetir}>Repetir</button>
  </div>
{/if}
```

Detalle que las pruebas Playwright exigen: cuando la sesión vence, `[data-accion="usar"]` debe quedar oculto y el `input[data-accion="archivo"]` visible otra vez. El `$effect` de arriba lo garantiza porque `sesionVencida` pone `foto: null`.

`Generando.svelte`:

```svelte
<script>
  import { estado, reintentar, volverAPrendas } from "../estado.js";
</script>

<h2>Generando…</h2>
<progress max="1" value={$estado.progreso.valor} data-progreso></progress>
<p data-mensaje-progreso>{$estado.progreso.texto}</p>
{#if $estado.fallo}
  <button class="secundario" data-accion="reintentar" onclick={reintentar}>Reintentar</button>
  <button class="secundario" data-accion="volver" onclick={volverAPrendas}>Volver a las prendas</button>
{/if}
```

`Resultado.svelte`:

```svelte
<script>
  import { onMount } from "svelte";
  import { elegirFlujo, estado, otraPrenda, repetirFoto, terminar } from "../estado.js";
  import { flujos, personaUrl, resultadoUrl } from "../api.js";

  let verOriginal = $state(false);
  let lista = $state([]);
  const resultado = $derived(resultadoUrl($estado.sesion.id, $estado.sesion.resultado));
  const original = $derived(personaUrl($estado.sesion.id));

  onMount(async () => {
    if ($estado.conFlujos) { try { lista = await flujos(); } catch { lista = []; } }
  });
</script>

<div class="comparar">
  <img data-imagen="resultado" alt="Persona con la prenda puesta (imagen generada con IA)" src={resultado} hidden={verOriginal}>
  <img data-imagen="original" alt="Foto original" src={original} hidden={!verOriginal}>
</div>
<button class="secundario" data-accion="comparar" onclick={() => (verOriginal = !verOriginal)}>
  {verOriginal ? "Ver resultado" : "Ver original"}
</button>
{#if $estado.conFlujos}
  <label data-selector-flujos>Motor
    <select data-flujos value={$estado.flujo} onchange={e => elegirFlujo(e.target.value)}>
      {#each lista as f (f.nombre)}
        <option value={f.nombre} disabled={!f.disponible}>{f.disponible ? f.titulo : `${f.titulo} (faltan pesos)`}</option>
      {/each}
    </select>
  </label>
{/if}
<div class="acciones">
  <button class="principal" data-accion="otra-prenda" onclick={otraPrenda}>Otra prenda</button>
  <button class="secundario" data-accion="repetir-foto" onclick={repetirFoto}>Repetir foto</button>
  <a class="secundario" data-accion="descargar" download="galatea.png" href={resultado}>Descargar</a>
  <button class="peligro" data-accion="terminar" onclick={terminar}>Terminar y borrar</button>
</div>
```

`Ajustes.svelte`:

```svelte
<script>
  import { cerrarAjustes } from "../estado.js";
  import { guardarServidor, porDefecto, servidor, validarServidor } from "../ajustes.js";
  import { salud } from "../api.js";

  let valor = $state(servidor());
  let aviso = $state({ texto: "", error: false });

  async function probar() {
    const invalido = validarServidor(valor);
    if (invalido) { aviso = { texto: invalido, error: true }; return; }
    // Prueba contra el valor escrito, no el guardado: se guarda temporalmente y se restaura.
    const anterior = servidor();
    try {
      guardarServidor(valor);
      const s = await salud();
      aviso = { texto: `Servidor Galatea v${s.version} responde.`, error: false };
    } catch (e) {
      aviso = { texto: e.message, error: true };
    } finally {
      guardarServidor(anterior);
    }
  }
  function guardar() {
    const invalido = validarServidor(valor);
    if (invalido) { aviso = { texto: invalido, error: true }; return; }
    guardarServidor(valor);
    valor = servidor();
    aviso = { texto: "Guardado.", error: false };
  }
</script>

<h2>Servidor</h2>
<p>Dirección del servidor Galatea. Vacía = el mismo sitio desde el que se abrió esta página{porDefecto() ? ` (por defecto ${porDefecto()})` : ""}.</p>
<input type="url" inputmode="url" placeholder="https://192.168.0.10:8444" bind:value={valor} data-campo="servidor">
{#if aviso.texto}<p class="mensaje" class:error={aviso.error} data-aviso-ajustes>{aviso.texto}</p>{/if}
<div class="acciones">
  <button class="secundario" data-accion="probar" onclick={probar}>Probar</button>
  <button class="principal" data-accion="guardar" onclick={guardar}>Guardar</button>
  <button class="secundario" data-accion="cerrar-ajustes" onclick={cerrarAjustes}>Volver</button>
</div>
```

Añade a `estilo.css`:

```css
.enlace { background:transparent; color:var(--acento); text-decoration:underline; padding:8px; font-size:14px; }
input[type=url] { width:100%; font:inherit; padding:12px; border-radius:10px; border:1px solid var(--tarjeta); background:var(--tarjeta); color:var(--texto); }
.mensaje button { width:auto; display:inline-block; margin:8px 0 0; padding:8px 14px; }
```

- [ ] **Paso 3: Compilar y mirar que carga**

```bash
scripts/web.sh && nix develop -c cargo run --quiet &   # con GALATEA_PRENDAS=pruebas/prendas GALATEA_FLUJOS=pruebas/flujos GALATEA_DATOS=target/e2e-datos GALATEA_WEB=web/dist GALATEA_PUERTO=8767
curl -s http://127.0.0.1:8767/index.html | grep -c 'assets/index'
```

Esperado: `1`. Detén el servidor con Ctrl-C (o `kill` del PID que anotaste con `$!`; nunca `pkill`).

- [ ] **Paso 4: Adaptar Playwright**

En `tests/e2e/demo.spec.js`, cambia **todas** las `page.goto("/")` por `page.goto("/index.html")` y `page.goto("/?flujos=1")` por `page.goto("/index.html?flujos=1")` (temporal; la Tarea 5 lo devuelve a `/`). Añade al final:

```js
test("ajustes: prueba la dirección, guarda y vuelve", async ({ page, baseURL }) => {
  await page.goto("/index.html");
  await page.click('[data-accion="abrir-ajustes"]');
  await expect(page.locator('[data-estado="ajustes"]')).toBeVisible();
  await page.fill('[data-campo="servidor"]', "http://inseguro.local");
  await page.click('[data-accion="probar"]');
  await expect(page.locator("[data-aviso-ajustes]")).toContainText("https://");
  // Vacía = mismo origen: Probar consulta /salud y muestra la versión.
  await page.fill('[data-campo="servidor"]', "");
  await page.click('[data-accion="probar"]');
  await expect(page.locator("[data-aviso-ajustes]")).toContainText("responde");
  await page.click('[data-accion="guardar"]');
  await expect(page.locator("[data-aviso-ajustes]")).toContainText("Guardado");
  await page.click('[data-accion="cerrar-ajustes"]');
  await expect(page.locator('[data-estado="aviso"]')).toBeVisible();
});
```

```bash
nix develop -c npx playwright test
```

Esperado: 7 en verde. Si alguno falla, el fallo está en las pantallas o en el store, no en la prueba: las seis primeras describen el comportamiento aprobado de la demo.

- [ ] **Paso 5: Check y commit**

```bash
nix flake check
git add web/src tests/e2e/demo.spec.js
git commit -m "web: pantallas Svelte con el recorrido de la demo y ajustes del servidor"
```

---

### Tarea 5: El cliente Svelte reemplaza la página antigua

**Files:**
- Delete: `askama.toml`, `plantillas/index.html`, `estaticos/estilo.css`, `estaticos/app.js`, `src/web/pagina.rs`, `src/web/estaticos.rs`
- Move: `estaticos/marca.png` → `recursos/marca.png`
- Modify: `Cargo.toml`, `src/lib.rs`, `src/web/mod.rs`, `src/marca.rs`, `flake.nix`, `tests/e2e/demo.spec.js`, `README.md`, `DEMO.md`, `CLAUDE.md`

**Interfaces:**
- Produces: `GET /` devuelve `index.html` del cliente Svelte. Nada más cambia para los clientes.

- [ ] **Paso 1: Prueba Rust que falla**

Añade a `tests/estaticos.rs`:

```rust
#[tokio::test]
async fn la_raiz_es_el_cliente_svelte() {
    let s = comun::Servidor::nuevo().await;
    let r = s.cliente.get(s.url("/")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.text().await.unwrap().contains("cliente de prueba"));
}
```

```bash
nix develop -c cargo test --test estaticos la_raiz
```

Esperado: falla (la raíz devuelve la página askama con «Galatea»).

- [ ] **Paso 2: Quitar la página antigua**

- `src/lib.rs`: elimina las tres rutas `"/"`, `"/estaticos/estilo.css"`, `"/estaticos/app.js"`.
- `src/web/mod.rs`: elimina `pub mod estaticos;` y `pub mod pagina;`.
- `git rm src/web/pagina.rs src/web/estaticos.rs askama.toml plantillas/index.html estaticos/estilo.css estaticos/app.js`.
- `git mv estaticos/marca.png recursos/marca.png`; en `src/marca.rs`: `include_bytes!("../recursos/marca.png")`.
- `Cargo.toml`: elimina `askama = "0.16"`.
- `flake.nix`, filtro `fuentes`: reemplaza `(plantillas|estaticos|pruebas)` por `(recursos|pruebas)` y elimina la línea de `askama.toml`.

- [ ] **Paso 3: Playwright vuelve a `/`**

En `tests/e2e/demo.spec.js`, deshaz el cambio temporal: `page.goto("/index.html")` → `page.goto("/")` y `"/index.html?flujos=1"` → `"/?flujos=1"`.

- [ ] **Paso 4: Correr todo**

```bash
nix develop -c cargo test && nix develop -c npx playwright test && nix flake check
```

Esperado: todo en verde; `cargo tree -i askama` dice que ya no está.

- [ ] **Paso 5: Documentación**

`README.md`:
- En «Tres procesos», la fila de Galatea: «página» → «sirve el cliente Svelte (`web/dist`)».
- Nueva sección «## Cliente» antes de «Pruebas»:

```markdown
## Cliente

El cliente vive en `web/` (Svelte 5 + Vite). `scripts/web.sh` lo compila en `web/dist`, que el
servidor sirve como fallback (`GALATEA_WEB`, por defecto `./web/dist`). Para iterar:
`cd web && npm run dev` abre Vite en `http://localhost:5173` con la API reenviada a `127.0.0.1:8766`.

Variables nuevas del servidor: `GALATEA_WEB` (carpeta del cliente compilado) y
`GALATEA_ORIGENES` (orígenes con CORS, por defecto `https://localhost`, el de la app Android;
vacío desactiva CORS).

**No corras `galatea experimento` mientras el servidor sirve:** `preparar` vacía
`datos/sesiones/` y barre `datos/comfy/`, y se llevaría las sesiones vivas.
```

- En «Pruebas», añade `nix develop -c npm --prefix web test   # Vitest del cliente`.
- En «Documentos», añade la especificación y el plan de este trabajo.

`DEMO.md`, «Antes de salir de casa», paso 1: `scripts/demo.sh` ya compila el cliente; nada más cambia por ahora (la instalación del certificado sigue vigente para el navegador; el APK llega en la Tarea 8).

`CLAUDE.md`, sección «Pruebas»: añade una línea: «El cliente se prueba con Vitest (`web/src/*.test.js`) para la lógica y con Playwright para el recorrido; los selectores `data-estado`/`data-accion` son el contrato entre ambos.»

- [ ] **Paso 6: Commit**

```bash
git add -A src Cargo.toml Cargo.lock flake.nix recursos tests README.md DEMO.md CLAUDE.md
git commit -m "web: el cliente Svelte reemplaza la página askama"
```

---

### Tarea 6: Toolchain Android en el flake y proyecto Capacitor con APK de depuración

**Files:**
- Modify: `flake.nix`, `web/package.json`, `web/package-lock.json`, `.gitignore`
- Create: `web/capacitor.config.json`, `web/android/**` (generado y commiteado), `scripts/apk.sh`

**Interfaces:**
- Produces: `nix develop` con `ANDROID_HOME`, `JAVA_HOME`, `gradle`, `adb`; `scripts/apk.sh` deja `datos/apk/galatea-debug.apk`. Variable `GALATEA_SERVIDOR_APK` (por defecto `https://192.168.0.10:8444`) fija `VITE_SERVIDOR` en el build del APK.

- [ ] **Paso 1: SDK, JDK y Gradle en el flake**

En `flake.nix`, en `pkgs = import nixpkgs { … }` añade `config = { allowUnfree = true; android_sdk.accept_license = true; };`. Antes de `in`:

```nix
        # Capacitor 8: AGP 8.13 → build-tools 36.0.0, compileSdk 36, JDK 21, Gradle 8.14.
        versionBuildTools = "36.0.0";
        android = pkgs.androidenv.composeAndroidPackages {
          platformVersions = [ "36" ];
          buildToolsVersions = [ versionBuildTools ];
          includeEmulator = true;
          includeSystemImages = true;
          systemImageTypes = [ "google_apis" ];
          abiVersions = [ "x86_64" ];
        };
        sdkAndroid = android.androidsdk;
        casaAndroid = "${sdkAndroid}/libexec/android-sdk";
```

En el `devShell`: añade a `packages` `sdkAndroid pkgs.jdk21 pkgs.gradle_8` y estas variables:

```nix
          ANDROID_HOME = casaAndroid;
          ANDROID_SDK_ROOT = casaAndroid;
          JAVA_HOME = "${pkgs.jdk21}";
          # El aapt2 que baja Gradle no corre en NixOS: se usa el del SDK de nixpkgs.
          GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${casaAndroid}/build-tools/${versionBuildTools}/aapt2";
```

Comprueba (la primera vez descarga el SDK, varios GB):

```bash
nix develop -c sh -c 'echo $ANDROID_HOME && adb version && java -version && gradle --version | head -3 && ls $ANDROID_HOME/build-tools'
```

Esperado: rutas en `/nix/store`, `adb` 37.x, Java 21, Gradle 8.14.x, `36.0.0`.

- [ ] **Paso 2: Capacitor en `web/`**

```bash
cd web
for p in @capacitor/core @capacitor/cli @capacitor/android @capacitor/app; do printf '%s ' "$p"; npm view "$p" version; done
npm install @capacitor/core@^8.5.2 @capacitor/app@^8.1.1
npm install -D @capacitor/cli@^8.5.2 @capacitor/android@^8.5.2
```

`web/capacitor.config.json` (escríbelo a mano; equivale a `npx cap init`):

```json
{
  "appId": "cl.galatea.probador",
  "appName": "Probador Galatea",
  "webDir": "dist",
  "android": {
    "webContentsDebuggingEnabled": true
  }
}
```

`webContentsDebuggingEnabled` permite inspeccionar el WebView desde `chrome://inspect` en el computador; es la forma de ver la consola de la app durante el desarrollo. Queda en `true` porque es una demo; se quita en la primera versión pública.

```bash
npm run build && npx cap add android && npx cap sync android
git status --short android | head -40
```

Revisa `web/android/.gitignore` (lo crea Capacitor). Deben quedar fuera de git: `build/`, `.gradle/`, `local.properties`, `app/src/main/assets/public/`, `app/src/main/assets/capacitor.config.json`, `app/src/main/assets/capacitor.plugins.json`, `capacitor-cordova-android-plugins/`. Si falta alguno, añádelo a `web/android/.gitignore`.

- [ ] **Paso 3: Gradle con el SDK de Nix**

Añade al final de `web/android/gradle.properties`:

```
# El SDK viene de nixpkgs (solo lectura): Gradle no debe intentar instalar componentes.
android.builder.sdkDownload=false
```

Compila:

```bash
cd web/android && gradle --no-daemon assembleDebug
ls -la app/build/outputs/apk/debug/app-debug.apk
```

Si Gradle se queja de la versión del wrapper o de AGP, la referencia es `web/android/build.gradle` (AGP 8.13.0) y `gradle/wrapper/gradle-wrapper.properties` (8.14.3): el `gradle_8` de nixpkgs (8.14.4) los cumple. Si `aapt2` falla, revisa que `GRADLE_OPTS` apunte a un archivo existente: `ls $ANDROID_HOME/build-tools/36.0.0/aapt2`.

- [ ] **Paso 4: `scripts/apk.sh`**

```bash
#!/usr/bin/env bash
# Genera el APK de depuración en datos/apk/galatea-debug.apk.
# Pasos: CA de Caddy → recurso del APK (Tarea 7), cliente con la dirección del servidor,
# cap sync, gradle assembleDebug. Requiere `nix develop` (SDK, JDK, Gradle, Node).
set -euo pipefail
cd "$(dirname "$0")/.."

: "${GALATEA_SERVIDOR_APK:=https://192.168.0.10:8444}"
echo "== servidor dentro del APK: $GALATEA_SERVIDOR_APK"

( cd web
  if [ ! -f node_modules/.package-lock.json ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then npm ci; fi
  VITE_SERVIDOR="$GALATEA_SERVIDOR_APK" npm run build
  npx cap sync android
  cd android && gradle --no-daemon --quiet assembleDebug )

mkdir -p datos/apk
cp web/android/app/build/outputs/apk/debug/app-debug.apk datos/apk/galatea-debug.apk
echo "== APK listo: datos/apk/galatea-debug.apk"
echo "   instalar por USB:  adb install -r datos/apk/galatea-debug.apk"
```

`chmod +x scripts/apk.sh`. Añade `datos/apk/` a nada: `datos/` ya está en `.gitignore`.

```bash
nix develop -c scripts/apk.sh
nix develop -c sh -c '$ANDROID_HOME/build-tools/36.0.0/aapt dump badging datos/apk/galatea-debug.apk' | grep -E "^package|application-label|sdkVersion"
```

Esperado: `package: name='cl.galatea.probador'`, `application-label:'Probador Galatea'`, `sdkVersion:'24'`. Comprueba también que la dirección quedó dentro: `unzip -p datos/apk/galatea-debug.apk 'assets/public/assets/index-*.js' | grep -c 192.168.0.10` → `1` o más.

- [ ] **Paso 5: `npmDepsHash` y check**

Las dependencias cambiaron: recalcula y pega el hash.

```bash
nix run nixpkgs#prefetch-npm-deps -- web/package-lock.json
nix flake check
```

- [ ] **Paso 6: Commit**

```bash
git add flake.nix web/package.json web/package-lock.json web/capacitor.config.json web/android scripts/apk.sh
git commit -m "apk: proyecto Capacitor y toolchain Android desde el flake"
```

Revisa antes con `git status --short | grep -v '^A\|^M'` que no entre nada de `build/` ni `assets/public`.

---

### Tarea 7: Permisos, CA empaquetada, ícono, botón Atrás y aviso de privacidad de la app

**Files:**
- Modify: `web/android/app/src/main/AndroidManifest.xml`, `web/android/app/src/main/res/values/strings.xml`, `web/android/app/src/main/res/values/ic_launcher_background.xml`, `web/src/pantallas/Aviso.svelte`, `web/src/main.js`, `scripts/apk.sh`, `.gitignore`
- Create: `web/android/app/src/main/res/xml/network_security_config.xml`, `web/src/nativo.js`, `scripts/icono.py`, `web/public/icono.png`, `web/android/app/src/main/res/mipmap-*/ic_launcher*.png` (generados)

**Interfaces:**
- Consumes: `estado.js` (`estado`, `cerrarAjustes`, `volverAPrendas`, `otraPrenda`, `terminar`, `descartarFoto`).
- Produces: `nativo.js` exporta `esNativo(): boolean` y `conectarAtras(): void`.

- [ ] **Paso 1: Manifest y seguridad de red**

`web/android/app/src/main/AndroidManifest.xml`: dentro de `<manifest>`, junto al permiso de Internet que ya trae:

```xml
    <uses-permission android:name="android.permission.CAMERA" />
    <uses-feature android:name="android.hardware.camera" android:required="false" />
```

En `<application …>` añade el atributo:

```xml
        android:networkSecurityConfig="@xml/network_security_config"
```

y comprueba que `android:usesCleartextTraffic` no esté en `true` (si está, quítalo: el APK solo habla HTTPS).

`web/android/app/src/main/res/xml/network_security_config.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<!-- La CA raíz de Caddy (LAN) viaja dentro del APK: nadie la instala en el teléfono.
     scripts/apk.sh copia datos/caddy/caddy/pki/authorities/local/root.crt a res/raw/caddy_root.crt. -->
<network-security-config>
    <base-config cleartextTrafficPermitted="false">
        <trust-anchors>
            <certificates src="system" />
            <certificates src="@raw/caddy_root" />
        </trust-anchors>
    </base-config>
</network-security-config>
```

`.gitignore` de la raíz: añade `web/android/app/src/main/res/raw/caddy_root.crt`.

En `scripts/apk.sh`, antes del bloque `( cd web …`:

```bash
CA=datos/caddy/caddy/pki/authorities/local/root.crt
if [ ! -f "$CA" ]; then
  echo "no existe $CA: levanta Caddy una vez (docker compose up -d caddy) para que genere su CA" >&2
  exit 1
fi
mkdir -p web/android/app/src/main/res/raw
cp "$CA" web/android/app/src/main/res/raw/caddy_root.crt
echo "== CA empaquetada: $(openssl x509 -in "$CA" -noout -subject -enddate | tr '\n' ' ')"
```

(`openssl` viene en el shell; si no, añádelo a `packages`.)

- [ ] **Paso 2: Ícono**

`scripts/icono.py`:

```python
#!/usr/bin/env python3
"""Ícono provisional de la demo: luna creciente sobre fondo oscuro, en todas las densidades.

Escribe web/public/icono.png (512) y los mipmap de Android (ic_launcher, ic_launcher_round,
ic_launcher_foreground). Colores de estilo.css: fondo #1b1b2f, acento #c9a7ff.
"""
from pathlib import Path
from PIL import Image, ImageDraw

RAIZ = Path(__file__).resolve().parent.parent
FONDO, ACENTO = (0x1B, 0x1B, 0x2F, 255), (0xC9, 0xA7, 0xFF, 255)
DENSIDADES = {"mdpi": 1, "hdpi": 1.5, "xhdpi": 2, "xxhdpi": 3, "xxxhdpi": 4}


def luna(tamano, con_fondo, margen):
    """Dibuja a 4x y reduce para suavizar bordes. `margen` es la fracción libre alrededor."""
    s = tamano * 4
    img = Image.new("RGBA", (s, s), FONDO if con_fondo else (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    r = s * (0.5 - margen)
    cx = cy = s / 2
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=ACENTO)
    # La sombra que forma la creciente: un círculo del color del fondo desplazado.
    dx = r * 0.45
    d.ellipse((cx - r + dx, cy - r - r * 0.1, cx + r + dx, cy + r - r * 0.1), fill=FONDO if con_fondo else (0, 0, 0, 0))
    return img.resize((tamano, tamano), Image.LANCZOS)


def main():
    publico = RAIZ / "web" / "public"
    publico.mkdir(parents=True, exist_ok=True)
    luna(512, True, 0.12).save(publico / "icono.png")
    res = RAIZ / "web" / "android" / "app" / "src" / "main" / "res"
    for nombre, factor in DENSIDADES.items():
        carpeta = res / f"mipmap-{nombre}"
        carpeta.mkdir(parents=True, exist_ok=True)
        luna(int(48 * factor), True, 0.12).save(carpeta / "ic_launcher.png")
        luna(int(48 * factor), True, 0.12).save(carpeta / "ic_launcher_round.png")
        # Ícono adaptativo: 108 dp con la figura dentro del círculo seguro central (66 dp).
        luna(int(108 * factor), False, 0.30).save(carpeta / "ic_launcher_foreground.png")
    print("ícono escrito en", publico / "icono.png", "y", res / "mipmap-*")


if __name__ == "__main__":
    main()
```

```bash
nix develop -c python3 scripts/icono.py
```

En `web/android/app/src/main/res/values/ic_launcher_background.xml` pon el color `#1B1B2F`. Borra los `ic_launcher*.png`/`.webp` que trajo la plantilla en los mismos `mipmap-*` si son de otro formato (`git rm` de los `.webp` para que no queden dos íconos). En `strings.xml` comprueba `app_name` = «Probador Galatea» y `title_activity_main` igual.

- [ ] **Paso 3: Botón Atrás y detección de app nativa**

`web/src/nativo.js`:

```js
// Lo que solo existe dentro de Capacitor. En la web estas funciones no hacen nada.
import { Capacitor } from "@capacitor/core";
import { App } from "@capacitor/app";
import { get } from "svelte/store";
import { cerrarAjustes, descartarFoto, estado, otraPrenda, terminar, volverAPrendas } from "./estado.js";

export function esNativo() {
  return Capacitor.isNativePlatform();
}

/** Botón físico Atrás: equivale al «volver» de cada pantalla; en Resultado pregunta antes de borrar. */
export function conectarAtras() {
  if (!esNativo()) return;
  App.addListener("backButton", async () => {
    const e = get(estado);
    switch (e.pantalla) {
      case "ajustes": cerrarAjustes(); break;
      case "camara": if (e.foto) descartarFoto(); else otraPrenda(); break;
      case "generando": volverAPrendas(); break;
      case "resultado": if (confirm("¿Terminar y borrar la foto y las imágenes?")) await terminar(); break;
      default: App.exitApp();
    }
  });
}
```

`web/src/main.js`:

```js
import { mount } from "svelte";
import "./estilo.css";
import App from "./App.svelte";
import { conectarAtras } from "./nativo.js";

conectarAtras();
mount(App, { target: document.getElementById("app") });
```

`Aviso.svelte`: el texto del aviso cambia dentro de la app, porque no hay `pagehide` fiable (spec §5.5):

```svelte
<script>
  import { abrirAjustes, entendido } from "../estado.js";
  import { esNativo } from "../nativo.js";
</script>

<h2>Antes de la foto</h2>
<p>La foto sirve solo para mostrar cómo queda la prenda. Se borra al pulsar «Terminar y borrar»{esNativo() ? "" : ", al cerrar la página"} o a los quince minutos, y no se usa para entrenar nada. No se fotografía a menores.</p>
<button class="principal" data-accion="entendido" onclick={entendido}>Entendido</button>
<button class="enlace" data-accion="abrir-ajustes" onclick={abrirAjustes}>Ajustes del servidor</button>
```

Vitest sigue en verde (`nativo.js` no se importa desde `estado.js`; los módulos de Capacitor cargan en Node sin tocar el DOM). Corre `cd web && npm test && npm run build`.

- [ ] **Paso 4: Generar el APK y comprobar lo empaquetado**

```bash
nix develop -c scripts/apk.sh
A=datos/apk/galatea-debug.apk
nix develop -c sh -c "\$ANDROID_HOME/build-tools/36.0.0/aapt dump badging $A" | grep -E "uses-permission|application-label|icon"
unzip -l $A | grep -E "res/raw/caddy_root|res/xml/network_security_config|mipmap-xxxhdpi/ic_launcher"
```

Esperado: `uses-permission: name='android.permission.CAMERA'`, el label, y las tres entradas del zip (el XML puede aparecer compilado con otro nombre dentro de `res/xml/`; basta que `aapt dump xmltree $A AndroidManifest.xml | grep networkSecurityConfig` muestre el atributo).

- [ ] **Paso 5: Playwright y check**

Las siete pruebas siguen en verde (la web no cambia de comportamiento; el texto del aviso sigue conteniendo «quince minutos»):

```bash
nix develop -c npx playwright test && nix flake check
```

- [ ] **Paso 6: Commit**

```bash
git add web/android/app/src/main/AndroidManifest.xml web/android/app/src/main/res web/src scripts/icono.py scripts/apk.sh web/public .gitignore
git commit -m "apk: permiso de cámara, CA de la LAN empaquetada, ícono y botón Atrás"
```

Comprueba con `git show --stat HEAD | grep raw/` que **no** entró `caddy_root.crt`.

---

### Tarea 8: Emulador, checklist del APK y documentación

**Files:**
- Create: `scripts/emulador.sh`, `experimento/2026-09-27-apk.md`, `experimento/2026-09-27-apk-aviso.png`
- Modify: `README.md`, `DEMO.md`, `experimento/2026-09-27-demo-celular.md`, `.gitignore`

**Interfaces:**
- Consumes: `datos/apk/galatea-debug.apk` de la Tarea 7; el servidor real levantado con `scripts/demo.sh` (o, sin GPU, `GALATEA_FLUJO_POR_DEFECTO=fijo scripts/demo.sh`).
- Produces: `scripts/emulador.sh [captura]` deja el emulador arriba con la app instalada y abierta.

- [ ] **Paso 1: `scripts/emulador.sh`**

```bash
#!/usr/bin/env bash
# Emulador Android para ver el APK durante el desarrollo. Crea el AVD en datos/avd la primera
# vez, arranca el emulador (cámara trasera = webcam del computador si existe), instala el APK,
# abre la app, concede el permiso de cámara y, si se pide, guarda una captura.
# Uso: scripts/emulador.sh [ruta-de-captura.png]    (dentro de `nix develop`; necesita /dev/kvm)
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_HOME:?corre esto dentro de nix develop}"
export ANDROID_AVD_HOME="$PWD/datos/avd"
export ANDROID_USER_HOME="$PWD/datos/android"
mkdir -p "$ANDROID_AVD_HOME" "$ANDROID_USER_HOME"
AVD=galatea
IMAGEN="system-images;android-36;google_apis;x86_64"
APK=datos/apk/galatea-debug.apk
PAQUETE=cl.galatea.probador

[ -f "$APK" ] || { echo "falta $APK: corre scripts/apk.sh" >&2; exit 1; }
[ -r /dev/kvm ] || { echo "sin acceso a /dev/kvm: el emulador no arranca" >&2; exit 1; }

if ! avdmanager list avd 2>/dev/null | grep -q "Name: $AVD"; then
  echo "== creando el AVD $AVD"
  # `-d pixel_7` si el perfil existe en esta versión de cmdline-tools; si no, `avdmanager list device` y elige otro.
  echo no | avdmanager create avd -n "$AVD" -k "$IMAGEN" -d pixel_7 >/dev/null
  printf 'hw.gpu.enabled=yes\nhw.gpu.mode=swiftshader_indirect\nhw.keyboard=yes\n' >> "$ANDROID_AVD_HOME/$AVD.avd/config.ini"
fi

CAMARA=none; [ -e /dev/video0 ] && CAMARA=webcam0
if ! adb devices | grep -q '^emulator-.*device$'; then
  echo "== arrancando el emulador (cámara trasera: $CAMARA)"
  "$ANDROID_HOME/emulator/emulator" -avd "$AVD" -no-boot-anim -no-snapshot -camera-back "$CAMARA" ${GALATEA_EMULADOR_FLAGS:-} \
    > datos/android/emulador.log 2>&1 &
  adb wait-for-device
  until [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; do sleep 2; done
fi

echo "== instalando $APK"
adb install -r "$APK" >/dev/null
adb shell pm grant "$PAQUETE" android.permission.CAMERA
adb shell am start -n "$PAQUETE/.MainActivity" >/dev/null
sleep 4
if [ "${1:-}" ]; then adb exec-out screencap -p > "$1"; echo "== captura en $1"; fi
echo "== consola del WebView: chrome://inspect en Chromium del computador (adb ya está conectado)"
echo "== apagar: adb emu kill"
```

`chmod +x scripts/emulador.sh`. En `.gitignore` no hace falta nada (`datos/`).

- [ ] **Paso 2: Correr el emulador y sacar la evidencia**

Levanta el servidor sin GPU para la prueba (no toques ComfyUI):

```bash
scripts/web.sh
GALATEA_WEB=web/dist nix run . -- servir &     # 127.0.0.1:8766; Caddy ya publica https://192.168.0.10:8444
nix develop -c scripts/emulador.sh experimento/2026-09-27-apk-aviso.png
```

Con la app abierta en el emulador: pulsa «Entendido» (`adb shell input tap` sobre el botón, o desde `chrome://inspect`), comprueba que la lista de prendas carga desde `https://192.168.0.10:8444` (la CA empaquetada funciona si aparecen las prendas; si sale «No encuentro el servidor», mira `adb logcat -s chromium` para ver si el fallo es de certificado), abre Ajustes y prueba la dirección. Guarda una segunda captura como `experimento/2026-09-27-apk-prendas.png`.

Si el emulador no llega a `192.168.0.10` (depende de la red del host), usa `GALATEA_SERVIDOR_APK=https://10.0.2.2:8444 scripts/apk.sh` **solo para esta comprobación**: `10.0.2.2` es el host visto desde el emulador, pero la CA de Caddy no cubre ese nombre, así que el certificado fallará; en ese caso la comprobación de la CA queda para el teléfono real y se dice en el informe. Detén el servidor con `kill %1` (o el PID) y el emulador con `adb emu kill`.

- [ ] **Paso 3: Checklist del APK**

`experimento/2026-09-27-apk.md`:

```markdown
# Prueba del APK · 2026-09-27

## En el emulador (hecho por el desarrollador)
- [ ] `scripts/apk.sh` genera `datos/apk/galatea-debug.apk` (fecha y tamaño: …)
- [ ] `scripts/emulador.sh experimento/2026-09-27-apk-aviso.png` arranca y muestra el aviso (captura adjunta)
- [ ] Las prendas cargan desde `https://192.168.0.10:8444` sin instalar la CA (captura `…-apk-prendas.png`)
- [ ] Ajustes: dirección inválida avisa; Probar contra el servidor muestra la versión; Guardar; Volver
- [ ] Botón Atrás en Prendas cierra la app; en Cámara vuelve a Prendas

## En el teléfono (hecho por Alen)
1. Conecta el teléfono por USB con depuración activada, o copia `datos/apk/galatea-debug.apk` y ábrelo.
   `adb install -r datos/apk/galatea-debug.apk`
2. Abre «Probador Galatea» en la misma Wi-Fi que el computador, con `scripts/demo.sh` corriendo.
3. Recorrido completo: Entendido → prenda → permiso de cámara (aceptar) → foto → Usar esta → esperar → resultado → Ver original → Otra prenda → Terminar y borrar.
4. Pausa la app a mitad de la generación y vuelve: el resultado debe aparecer igual.
5. Botón Atrás en el resultado: pregunta antes de borrar.
6. Ajustes: cambia la dirección a una inválida, prueba, vuelve a la correcta, guarda.
7. Cierra la app desde recientes y vuelve a abrirla: parte en el aviso.
8. Niega el permiso de cámara una vez: «Subir foto» debe seguir funcionando.

Anota aquí lo que falló y en qué paso.
```

Marca lo que ejecutaste con las capturas como evidencia. No marques lo que no corriste.

- [ ] **Paso 4: Documentación**

`README.md`: nueva sección «## APK (Android)» después de «Cliente»:

```markdown
## APK (Android)

El mismo cliente empaquetado con Capacitor (`web/android/`, commiteado). La CA de Caddy va
dentro del APK, así que el teléfono no necesita instalar nada; la dirección del servidor se fija
al compilar (`GALATEA_SERVIDOR_APK`, por defecto `https://192.168.0.10:8444`) y se puede cambiar
en la pantalla Ajustes de la app.

```bash
nix develop -c scripts/apk.sh                 # → datos/apk/galatea-debug.apk (firma de depuración)
adb install -r datos/apk/galatea-debug.apk    # teléfono por USB con depuración activada
nix develop -c scripts/emulador.sh captura.png   # emulador Android con la app abierta (necesita /dev/kvm)
```

Si se regenera la CA de Caddy (se borró `datos/caddy`), hay que volver a correr `scripts/apk.sh` y
reinstalar. Gradle baja sus dependencias la primera vez (red); el SDK, el JDK y Gradle vienen del
flake, y `GRADLE_OPTS` apunta al `aapt2` de nixpkgs porque el de Maven no corre en NixOS.
Depurar el WebView: `chrome://inspect` en Chromium con el teléfono o el emulador conectados.
```

`DEMO.md`, «Antes de salir de casa»: el paso 2 (instalar el certificado) pasa a ser la alternativa «si se usa el navegador»; el camino principal es «instala el APK: `adb install -r datos/apk/galatea-debug.apk` o copia el archivo». En «Si algo falla» añade: «La app dice “No encuentro el servidor” → Ajustes en la app, comprobar la dirección; si la CA cambió, regenerar el APK con `scripts/apk.sh`.»

`experimento/2026-09-27-demo-celular.md`: una línea al inicio: «Este checklist es del cliente web en el navegador; el del APK está en `2026-09-27-apk.md`.»

- [ ] **Paso 5: Verificación final y commit**

```bash
nix develop -c cargo test && nix develop -c npm --prefix web test && nix develop -c npx playwright test && nix flake check
git add scripts/emulador.sh experimento/2026-09-27-apk.md experimento/2026-09-27-apk-*.png README.md DEMO.md experimento/2026-09-27-demo-celular.md
git commit -m "apk: emulador, checklist y documentación"
```

Las capturas del emulador no contienen personas (aviso y lista de prendas de prueba); por eso pueden entrar a git.

---

## Autorrevisión del plan (hecha al escribirlo)

- **Cobertura de la especificación:** §4 arquitectura → T1, T2, T5; §5.1–5.4 cliente → T3, T4; §5.5 Capacitor (sin borrado en segundo plano, Atrás) → T7; §5.6 selectores → T4; §6 servidor → T2, T5; §7 Android (manifest, CA, ícono, `appId`, firma de depuración, `apk.sh`, `sdkDownload`, `aapt2`) → T6, T7; §8 Nix (Node 22, `checks.web`, androidenv, JDK, Gradle) → T1, T6; §9 cómo se ve (web, emulador, teléfono) → T4, T8; §10 errores → T3 (`FalloApi`), T4 (mensaje con botón Ajustes), T2 (`preparar`); §11 privacidad → T7 (texto del aviso); §12 pruebas → T2, T3, T4, T8; §13 futuro → sin tarea (queda en la especificación); §14 verificaciones → resueltas al escribir el plan: `/dev/kvm` es `crw-rw-rw-`, `/dev/video0` existe, `androidenv` del nixpkgs fijado trae plataforma 36, build-tools 36.0.0 e imagen `google_apis` x86_64; Capacitor concede `getUserMedia` tras el permiso `CAMERA` (`BridgeWebChromeClient.onPermissionRequest`); el `aapt2` de nixpkgs se comprueba en T6 paso 3.
- **Desviación de la especificación:** §8 decía `packages.emulador` con `androidenv.emulateApp`; el plan usa `scripts/emulador.sh` porque `emulateApp` recrea el AVD en un directorio temporal en cada arranque y no permite fijar `datos/avd`. Mismo resultado, arranques más rápidos.
- **Consistencia de nombres:** `Conf.web`/`Conf.origenes` (T2) se usan en T5; `FalloApi.clase` (T3) en T4 y T7; acciones de `estado.js` (T3) en T4 y `nativo.js` (T7); `GALATEA_SERVIDOR_APK` (T6) en T8; `data-accion="abrir-ajustes"` (T4) en la prueba de Ajustes (T4).
