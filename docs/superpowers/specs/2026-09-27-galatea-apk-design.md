# Galatea · Diseño del cliente Svelte y del APK con Capacitor

**Fecha:** 2026-09-27 · **Estado:** aprobado por secciones en conversación, pendiente de
revisión del documento escrito · **Diseño anterior:** `2026-09-26-galatea-demo-design.md`
(la demo web; todo lo que no se menciona aquí sigue vigente).

## 1. Resumen

La demo de Galatea hoy es una página web servida por el servidor Rust y abierta desde el
navegador del celular por HTTPS en la LAN. Funciona, pero usarla exige instalar la CA de Caddy
en el teléfono y escribir una dirección. El usuario quiere **un APK** por dos motivos:

1. **Facilidad de uso** para quien atiende la tienda: instalar un ícono y abrir, sin certificados ni
   direcciones.
2. **Prepararse para lo nativo**: medir tallas con ARCore y, más adelante, correr el modelo en
   el teléfono.

Este diseño convierte el cliente en una **app Svelte** compilada con Vite, que se usa de dos
formas con el mismo build: servida por el servidor Rust para el navegador, y empaquetada
dentro de un APK con **Capacitor**. El servidor y los motores no cambian salvo en cómo sirven
la página y en aceptar peticiones desde el origen de la app.

## 2. Decisiones ya tomadas

| Decisión | Elección | Por qué |
|---|---|---|
| Cómo se hace el APK | **Capacitor** envolviendo el cliente web | No se bota nada de lo construido; el APK sale en días; los plugins nativos se escriben en Kotlin, que es el lenguaje de ARCore |
| Framework del cliente | **Svelte 5 + Vite**, ahora y no después | El cliente tiene ~300 líneas; migrarlo hoy cuesta poco y Capacitor necesita de todos modos un proyecto web con build |
| Cámara en la app | La misma API del navegador (`getUserMedia`) dentro del WebView | Vista previa en vivo idéntica en web y app; Capacitor concede el permiso de cámara al WebView; sin plugin de cámara |
| Confianza en la CA de la LAN | La CA raíz de Caddy va **dentro del APK** como recurso, declarada en la configuración de seguridad de red de Android | Nadie instala nada en el teléfono |
| Dirección del servidor | Fijada al compilar el APK, modificable en una pantalla Ajustes | Si el servidor cambia de IP no hay que recompilar |
| Firma | Clave de depuración | Es demo; la clave de release llega con la distribución |
| Alternativas para después de la demo | React Native o Dioxus (Rust completo) quedan anotadas; **Flutter descartado** por la experiencia con la cámara | Decisión del usuario el 2026-09-27; se revisa cuando la parte nativa pese más que la web |
| Rama | `demo` fusionada en `master`; este trabajo va en la rama `apk` | Cierre limpio de la etapa anterior |

## 3. Alcance

**Dentro**

- Cliente Svelte con las mismas pantallas y el mismo recorrido de la demo, más una pantalla
  Ajustes con la dirección del servidor.
- Servidor Rust sirviendo la carpeta compilada por Vite y con CORS configurable.
- Proyecto Android generado por Capacitor y commiteado, con permiso de cámara, CA empaquetada,
  nombre e ícono.
- Scripts para compilar el cliente y para generar el APK de depuración.
- Node, SDK de Android, JDK y emulador dentro del flake (`nix develop`); el build del cliente
  dentro de `nix flake check`.
- Correcciones aparcadas de la demo: límite de sondeo del cliente (120 s → cubre los 240 s del
  servidor) y nota en el README de no correr `galatea experimento` mientras se sirve.
- Pruebas: Rust (estáticos y CORS), Vitest (`api.js` y store), Playwright (las seis existentes
  adaptadas más Ajustes), checklist manual del APK.

**Fuera (anotado en §13)**

- ARCore, modelo en el teléfono, iOS, Play Store, clave de release, actualizaciones
  automáticas de la app, modo sin conexión, cuentas de usuario.

## 4. Arquitectura

```
web/  (Svelte + Vite)  ──vite build──▶  web/dist/
                                          │
              ┌───────────────────────────┴───────────────────────────┐
              ▼                                                       ▼
   Servidor Rust (axum) sirve web/dist            Capacitor copia web/dist al APK
   en 127.0.0.1:8766, detrás de Caddy             origen interno: https://localhost
   https://192.168.0.10:8444  (origen A)         │
              ▲                                   │ fetch / EventSource
              └───── misma origen ────────────────┘ hacia el origen A, con CORS
```

- Un solo código de cliente; la única diferencia entre web y app es la **dirección base** de
  la API: vacía en web (mismo origen) y `https://192.168.0.10:8444` en la app (fijada al
  compilar, editable en Ajustes).
- La app carga sus archivos desde el servidor interno de Capacitor (`https://localhost`) y
  habla con el servidor Rust por la LAN. Para eso el servidor responde CORS a ese origen.
- Las sesiones se identifican por id en la URL, sin cookies, así que CORS no necesita
  credenciales.
- Caddy y ComfyUI no cambian. La CA raíz de Caddy (`datos/caddy/caddy/pki/authorities/local/root.crt`)
  se copia al APK al compilarlo.

## 5. Cliente Svelte (`web/`)

### 5.1 Estructura

```
web/
├── package.json            svelte, vite, @sveltejs/vite-plugin-svelte, vitest,
│                           @capacitor/core, @capacitor/cli, @capacitor/android, @capacitor/app
├── package-lock.json       fijado; lo usa buildNpmPackage
├── vite.config.js          salida en dist/; en desarrollo, proxy de la API a 127.0.0.1:8766
├── capacitor.config.json   appId, appName, webDir "dist"
├── index.html
├── src/
│   ├── main.js
│   ├── App.svelte          elige la pantalla según el store
│   ├── estado.js           store central (pantalla, prenda, flujo, sesión, mensaje)
│   ├── api.js              URL base + llamadas HTTP + SSE + sondeo; sin Svelte
│   ├── ajustes.js          lee/guarda la dirección base (localStorage)
│   ├── estilo.css          el actual, movido
│   └── pantallas/
│       ├── Aviso.svelte
│       ├── Prendas.svelte
│       ├── Camara.svelte
│       ├── Generando.svelte
│       ├── Resultado.svelte
│       └── Ajustes.svelte
├── public/icono.png        ícono de la app, generado por scripts/icono.py
└── android/                proyecto Capacitor (ver §7)
```

Se eliminan `plantillas/`, `estaticos/` y `askama.toml` del repo. La marca de agua
`estaticos/marca.png` la embebe el servidor con `include_bytes!` y no la usa el cliente: se
mueve a `recursos/marca.png` y se actualiza la ruta en `src/marca.rs`.

### 5.2 Estado central (`estado.js`)

Un store con la forma actual de `app.js`, hecho explícito:

```js
{ pantalla: "aviso" | "prendas" | "camara" | "generando" | "resultado" | "ajustes",
  prenda: null | { nombre, titulo, tipo },
  flujo: "" | nombre,
  sesion: { id: null, trabajo: null, resultado: null },
  mensaje: { texto: "", error: false },
  volverA: "aviso" | "prendas" | ...      // a dónde regresa Ajustes
}
```

Las acciones (`elegirPrenda`, `usarFoto`, `generar`, `otraPrenda`, `repetirFoto`,
`terminar`, `abrirAjustes`, ...) viven junto al store y son las únicas que lo modifican. Los
componentes leen el store y llaman acciones. La transición de estados es la de la demo, sin
cambios funcionales.

### 5.3 API (`api.js`)

- `base()` devuelve la dirección base de `ajustes.js`; vacía en web.
- Funciones: `prendas()`, `flujos()`, `crearSesion(foto)`, `generar(id, prenda, flujo)`,
  `avance(id, trabajo, alRecibir)` (EventSource), `buscarResultado(id, n, {cada, limite})`
  (HEAD periódico), `resultadoUrl(id, n)`, `personaUrl(id)`, `terminar(id, {keepalive})`,
  `salud()`.
- `SONDEO_LIMITE_MS` sube a **300 000** (5 min), por encima del `GALATEA_LIMITE_SEG=240` que
  exporta `demo.sh`. Cierra el pendiente aparcado de la demo.
- Errores de red y HTTP se convierten a un tipo `FalloApi { clase: "red" | "http" | "vencida",
  estado, texto }` que las pantallas traducen a mensajes.

### 5.4 Ajustes (`ajustes.js` y `Ajustes.svelte`)

- Guarda `servidor` en `localStorage`. Valor inicial: `import.meta.env.VITE_SERVIDOR` (vacío
  en el build web; la IP de la LAN en el build del APK, la pone `scripts/apk.sh`).
- La pantalla muestra el campo de dirección, un botón **Probar** que llama a `salud()` y
  muestra la versión del servidor o el error, y **Guardar**. Se llega desde un enlace discreto
  en la pantalla Aviso y desde el mensaje de error cuando el servidor no responde.
- En web la pantalla existe igual (útil para apuntar el navegador del computador a otro
  servidor), pero con la base vacía todo es mismo origen, como hoy.

### 5.5 Comportamiento dentro de Capacitor

- Se detecta con `Capacitor.isNativePlatform()`.
- **Borrado de sesión al salir.** En web sigue `pagehide` con `keepalive`. En la app no hay
  `pagehide` fiable cuando Android mata el proceso, y borrar al pasar a segundo plano haría
  perder el resultado por cambiar de app un momento. Decisión: en la app **no se borra por
  segundo plano**; la sesión muere por el botón *Terminar y borrar* o por el vencimiento del
  servidor (15 min), que ya garantiza la privacidad. Se anota en el aviso de privacidad de la
  app: "las fotos se borran al terminar o a los 15 minutos".
- Botón físico *Atrás* (`@capacitor/app`, evento `backButton`): en Prendas o Aviso cierra la
  app; en las demás pantallas equivale al botón "volver" de esa pantalla, y en Resultado
  pregunta antes de terminar y borrar.

### 5.6 Selectores para pruebas

Se conservan los atributos `data-estado` en la sección visible y `data-accion` en los botones,
con los mismos valores que la demo, para que las pruebas Playwright se adapten y no se
reescriban. Ajustes agrega `data-estado="ajustes"` y `data-accion="probar" | "guardar" |
"cerrar-ajustes"`.

## 6. Servidor Rust

- **Sale** `askama`, `askama.toml`, `src/web/pagina.rs`, `src/web/estaticos.rs` y sus rutas.
- **Entra** `tower-http` con `ServeDir` (con `fallback` a `index.html`) montado como
  `fallback_service` del router, así las rutas de la API siguen ganando. La carpeta se lee de
  `GALATEA_WEB` (por defecto `web/dist`). Si no existe al arrancar, `preparar` falla con el
  mensaje: "no existe la carpeta del cliente web; compílala con scripts/web.sh".
- **CORS** con `CorsLayer`: orígenes desde `GALATEA_ORIGENES`, lista separada por comas, por
  defecto `https://localhost` (el origen interno de Capacitor en Android). Métodos GET, HEAD,
  POST, DELETE; cabecera `content-type`; sin credenciales; `max-age` de una hora. Sin
  orígenes configurados (`GALATEA_ORIGENES=`) no se emiten cabeceras CORS.
- `Conf` gana los campos `web` y `origenes`. `README.md` documenta ambas variables y la nota
  de no correr `galatea experimento` mientras el servidor sirve (pendiente aparcado).
- El filtro de fuentes del flake deja de incluir `plantillas` y `estaticos`.

## 7. Proyecto Android (`web/android/`)

- Generado con `npx cap add android` y **commiteado**, como recomienda Capacitor, porque lleva
  cambios manuales:
  - `AndroidManifest.xml`: permiso `CAMERA`, `usesCleartextTraffic` en falso,
    `networkSecurityConfig` apuntando a `res/xml/network_security_config.xml`.
  - `res/xml/network_security_config.xml`: confía en `@raw/caddy_root` además del sistema.
  - `res/raw/caddy_root.crt`: **git-ignored**; `scripts/apk.sh` lo copia desde
    `datos/caddy/caddy/pki/authorities/local/root.crt` antes de compilar.
  - `strings.xml`: nombre "Probador Galatea". Ícono: `scripts/icono.py` (Pillow, ya en el
    shell) dibuja una luna creciente sobre fondo oscuro y escribe `web/public/icono.png` y
    los `mipmap-*` de Android en todas las densidades. Es un ícono provisional de demo.
- `appId`: `cl.galatea.probador` (provisional; para Play Store debe corresponder a un dominio propio del proyecto, que aún no existe). `appName`: "Probador Galatea".
- El `gradlew` del proyecto queda, pero se usa el Gradle y el JDK del flake para que el build
  no dependa de descargas del wrapper; Gradle sí baja dependencias de Maven la primera vez.
- Ajustes de Gradle para SDK de Nix (solo lectura): `android.builder.sdkDownload=false`,
  `android.aapt2FromMavenOverride` apuntando al `aapt2` de nixpkgs. Se documentan en el
  README con el motivo.
- `scripts/apk.sh`: copia la CA, `VITE_SERVIDOR=https://192.168.0.10:8444` (o el valor de
  `GALATEA_SERVIDOR_APK` si está definido) → `npm run build` → `npx cap sync android` →
  `gradle assembleDebug` → copia el APK a `datos/apk/galatea-debug.apk` e imprime la ruta y
  el comando `adb install -r`.

## 8. Herramientas y Nix

- **Node 22** y npm del flake. Las dependencias de `web/` se fijan en `package-lock.json`.
- **`nix flake check`** gana `checks.web`: `buildNpmPackage` con `npmDepsHash` fijo que corre
  `vite build` y `vitest run`. Sin red. Actualizar el hash es parte de tocar dependencias.
- **Android** desde `androidenv.composeAndroidPackages`: platform-tools, build-tools y
  plataforma 35, emulador e imagen de sistema x86_64 con Google APIs. Requiere
  `android_sdk.accept_license = true` en la configuración de nixpkgs del flake. Entra al
  `devShell` con `ANDROID_HOME`/`ANDROID_SDK_ROOT` y `JAVA_HOME` (JDK 21) exportados.
- **Emulador**: `packages.emulador` con `androidenv.emulateApp`, que crea el AVD y lo arranca.
  Necesita KVM (`/dev/kvm` accesible al usuario). Cámara trasera del emulador: la webcam del
  host si existe, si no la escena virtual.
- El build del APK **no** entra a `flake check` (Gradle necesita red).
- Playwright sigue en la raíz (`tests/e2e/`, `playwright.config.js`, `package.json` raíz con
  el enlace a `playwright-test` de nixpkgs); el proyecto `web/` tiene su propio `package.json`.

## 9. Cómo se ve durante el desarrollo

| Destino | Cómo | Qué se obtiene |
|---|---|---|
| Web | Playwright contra el servidor Rust sirviendo `web/dist` (como hoy); `vite dev` con proxy para iterar | Pruebas automáticas y capturas |
| APK en emulador | `nix run .#emulador`, `adb install -r datos/apk/galatea-debug.apk`, `adb exec-out screencap -p` | Capturas del APK real; el emulador llega al servidor por la IP de la LAN |
| Teléfono real | Prueba humana con el checklist; si el teléfono está por USB con depuración, también `adb install` y capturas | Verificación final |

## 10. Errores

| Situación | Qué ve la persona | Qué hace la app |
|---|---|---|
| Servidor inalcanzable (red, DNS, puerto) | "No encuentro el servidor en `<dirección>`" con botón **Ajustes** y **Reintentar** | `FalloApi.clase = "red"` |
| Certificado no confiable (CA distinta a la del APK) | Mismo mensaje; el README explica que hay que regenerar el APK con `apk.sh` cuando cambia la CA | El WebView lo reporta como fallo de red; no se distingue |
| Cámara denegada | Se ofrece **Subir foto** (galería o cámara del sistema) | Como en la demo |
| Sesión vencida (404) | Vuelve a la cámara con el mensaje de la demo | Como en la demo |
| Dirección mal escrita en Ajustes | **Probar** muestra el error antes de guardar; Guardar exige que sea `https://` o vacía | Validación local |
| `web/dist` ausente al arrancar el servidor | Error de arranque con instrucción | `preparar` falla |

## 11. Privacidad

Sin cambios de fondo: fotos y resultados solo en memoria y carpetas temporales del servidor,
borrados al terminar, al vencer (15 min) o al reiniciar. Cambios de esta etapa:

- La app no manda nada a ningún servicio salvo al servidor configurado. Sin analítica, sin
  permisos más allá de cámara e internet.
- El borrado al salir en la app depende del botón o del vencimiento (§5.5); el aviso lo dice.
- La dirección del servidor guardada en `localStorage` no es dato personal.
- Los registros del servidor siguen redactados (`sesion::redactar`); las cabeceras CORS no
  agregan información a los registros.

## 12. Pruebas

**Rust (`tests/web.rs` y nuevo `tests/estaticos.rs`)**

- `GET /` devuelve `index.html` desde `GALATEA_WEB`; `GET /assets/<archivo>` devuelve el
  archivo con su `content-type`; una ruta desconocida sin extensión devuelve `index.html`
  (fallback); una ruta de la API sigue respondiendo la API.
- CORS: `OPTIONS /prendas` con `Origin: https://localhost` responde `access-control-allow-origin`
  y los métodos; con un origen ajeno no hay cabecera; con `GALATEA_ORIGENES=` vacío tampoco.
- `preparar` falla con mensaje si `GALATEA_WEB` no existe.

**Cliente (Vitest, `web/src/*.test.js`)**

- `api.js`: URL con base vacía y con base remota; `buscarResultado` termina por límite;
  conversión de errores a `FalloApi` (red, 404 → vencida, 500 → http).
- `estado.js`: transiciones (aviso → prendas → cámara → generando → resultado → otra prenda
  vuelve a prendas con la misma foto; terminar limpia la sesión); `abrirAjustes` recuerda a
  dónde volver.
- `ajustes.js`: valor por defecto desde `VITE_SERVIDOR`, guardado y validación.

**Playwright (`tests/e2e/`)**

- Las seis pruebas de la demo adaptadas al cliente Svelte (mismos selectores).
- Nueva: abrir Ajustes desde Aviso, poner una dirección inválida, ver el error de Probar,
  poner la del servidor de prueba, ver la versión, guardar, volver.

**APK (manual, `experimento/2026-09-27-apk.md`)**

- Checklist: instalar, abrir, permiso de cámara, recorrido completo contra el servidor real,
  cambiar la dirección en Ajustes y volver, botón Atrás, cerrar y reabrir. Se guarda una
  captura del emulador como evidencia en `experimento/`.

## 13. Ideas futuras y alternativas evaluadas

- **ARCore para tallas** (pedido el 2026-09-26): entra como plugin Kotlin de Capacitor que
  abre una actividad nativa de medición y devuelve medidas a la web. Requiere teléfono con
  soporte ARCore; el emulador no sirve para eso.
- **Modelo en el teléfono**: mismo camino, plugin Kotlin con LiteRT u ONNX Runtime; el
  cliente pasaría de "cámara y pantalla" a generar localmente con menor calidad.
- **Alternativas de cliente para después de la demo**, en el orden que el usuario las
  considera: React Native (cámara y ARCore con librerías maduras, JS compartido con la web
  solo en la lógica) y Dioxus (Rust de punta a punta; cámara y ARCore por JNI a mano, soporte
  móvil más joven). **Flutter descartado** por mala experiencia previa del usuario con la
  cámara. La señal para migrar: cuando los plugins Kotlin pesen más que el cliente web.
- **Svelte también sirve para la web pública** si Galatea se ofrece como servicio a otras
  tiendas; el mismo build ya es una SPA.

## 14. Verificaciones pendientes para el plan

Cosas que el plan debe comprobar en su primera tarea antes de apoyarse en ellas:

1. Que `androidenv.composeAndroidPackages` en el nixpkgs fijado ofrece plataforma 35, imagen
   de sistema x86_64 con Google APIs y emulador, y que `emulateApp` sigue existiendo con esa
   firma.
2. Que `/dev/kvm` es accesible al usuario (si no, el emulador queda fuera y el APK se ve solo
   en el teléfono).
3. Que Capacitor 8 concede `getUserMedia` en el WebView con el permiso `CAMERA` declarado, sin
   plugin adicional (documentado en `BridgeWebChromeClient`; verificar en el emulador).
4. Que el `aapt2` de nixpkgs y la propiedad `android.aapt2FromMavenOverride` funcionan con
   la versión de AGP que trae Capacitor 8.
5. El `npmDepsHash` real de `web/package-lock.json` una vez fijadas las dependencias.
