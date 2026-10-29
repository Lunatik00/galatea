# Galatea

Probador virtual con IA generativa para una demo presencial: alguien le saca una foto a otra
persona desde el celular, elige una prenda y ve cómo le quedaría. La foto se borra al terminar
la sesión (o a los 15 minutos). Sin base de datos.

## Tres procesos

| Proceso | Dónde | Dirección |
|---|---|---|
| Galatea (Rust + axum): sirve el cliente Svelte (`web/dist`), sesiones, marca de agua | host, `nix run . -- servir` | `127.0.0.1:8766` |
| ComfyUI con la GPU: corre los flujos (FASHN VTON 1.5 por defecto; Leffa, CatVTON y FLUX.2 klein) | contenedor `galatea-comfyui` | `127.0.0.1:8188` |
| Caddy: HTTPS en la red local para que el celular permita la cámara | contenedor `galatea-caddy` | `https://$GALATEA_IP:8444`, certificado en `http://$GALATEA_IP:8091/caddy-root.crt` |

## Levantar la demo

Primero, la IP del host en la red local (Caddy escucha ahí y el celular entra por ella).
Va en `.env`, que está fuera de git:

```bash
echo GALATEA_IP=192.168.0.10 > .env    # cambia por la IP de tu máquina (ip -4 addr)
```

```bash
scripts/descargar-modelos.sh fashn     # una vez: pesos en datos/modelos/ (2.3 GB)
scripts/descargar-modelos.sh leffa     # opcional, el flujo anterior por defecto (8.7 GB)
scripts/demo.sh                        # detiene lo que usa la GPU, sube ComfyUI y Caddy, arranca Galatea
```

El flujo por defecto es **FASHN VTON 1.5** (sin máscara: no redibuja brazos ni cara;
Apache-2.0 salvo el parser humano; ~8 GB de VRAM según FASHN). Sus segundos en la 3090 y la
primera carga están **por medir**. Además de `datos/modelos/fashn-vton/`, el nodo carga el
parser `fashn-ai/fashn-human-parser` (256 MB) desde la caché de Hugging Face,
`datos/hf-cache/`; si falta, lo baja en la primera generación (hace falta red). Para dejarlo
puesto antes:

```bash
docker run --rm --user 1000:100 --entrypoint python -e HOME=/home/comfy \
  -v "$PWD/datos/hf-cache:/home/comfy/.cache/huggingface" galatea-comfyui:0.35.0-2 \
  -c 'from huggingface_hub import snapshot_download as s; print(s("fashn-ai/fashn-human-parser", allow_patterns=["*.json","model.safetensors"]))'
```

Los otros se eligen con `?flujos=1` en la página:

| Flujo | Licencia | VRAM | Segundos (en caliente) | Pesos |
|---|---|---|---|---|
| `fashn` (FASHN VTON 1.5, **por defecto**) | Apache 2.0 (parser humano: no comercial) | ~3.5 GB | ~18 (65 la primera); salida 576×864 | 2.3 GB |
| `leffa` | MIT | ~7.4 GB | ~17 | 8.7 GB |
| `catvton` | CC BY-NC-SA (solo demo) | ~4.5 GB | ~40 | `descargar-modelos.sh catvton` |
| `klein` (FLUX.2 klein 4B + LoRA) | Apache 2.0 + segformer no comercial | ~15 GB | ~22, salida de ~1 MP | 16.2 GB |

**Solo `fashn` admite pantalones y vestidos** (`tipos = ["superior", "inferior", "vestido"]`);
los otros declaran `tipos = ["superior"]` y con un vestido o un pantalón de
`prendas/prendas.toml` muestran un aviso en la página y no generan.

Cambiar de flujo en plena demo recarga modelos (hasta 4 minutos con klein). Las cifras y
los veredictos, IDM-VTON descartado incluido, están en
[experimento/2026-09-27-nodos.md](experimento/2026-09-27-nodos.md).

El guion completo, con la instalación del certificado en el celular, está en [DEMO.md](DEMO.md).
Las prendas van en `prendas/` (imágenes fuera de git) y se declaran en `prendas/prendas.toml`.

## Agregar un flujo (un motor)

1. En ComfyUI, arma el flujo y expórtalo en **formato API** a `flujos/<nombre>.json`.
2. Declara una entrada `[[flujo]]` en `flujos/flujos.toml` con los identificadores de nodo
   (`nodo_persona`, `nodo_prenda`, `nodo_salida`), los `tipos` que admite, la licencia en
   `descripcion` y sus `[[flujo.peso]]`.
3. Descarga los pesos: `scripts/descargar-modelos.sh <nombre>`.
4. Un flujo puede pedir `encuadre = "3:4"` en su `[[flujo]]`: Galatea le manda la foto de la
   persona encuadrada a 3:4 con relleno blanco y recorta el resultado de vuelta al tamaño
   original, así que el flujo debe devolver la imagen con la misma proporción 3:4 (si no, el
   servidor la deja sin recortar y lo avisa en el log). Sirve para nodos que trabajan en un
   marco fijo y estiran el resultado, como Leffa (768×1024). Sin el campo, la foto va tal cual
   y el resultado no se toca.

Rust no sabe qué modelo hay detrás: solo lee el TOML.

## Comparar flujos

```bash
GALATEA_COMFY_URL=http://127.0.0.1:8188 nix run . -- experimento \
  --personas <carpeta-con-fotos> --prendas prendas \
  --flujos fashn,catvton,leffa,klein --salida experimento/$(date +%F)-<nombre>
```

Corre personas × prendas × flujos con los mismos motores que la página y deja
`tabla.csv` (`flujo,persona,prenda,segundos,estado,archivo`) y `hoja.html` para calificar a
ojo. Las imágenes salen **sin marca de agua** (son para evaluar, no para mostrar) y
`experimento/*/` no entra a git.

**Al terminar de calificar, borra `experimento/<fecha>-<nombre>/` y la carpeta que pasaste en
`--personas`:** guardan fotos de voluntarias y salidas sin marca de agua, y nada las borra solo.

## Cliente

El cliente vive en `web/` (Svelte 5 + Vite). `scripts/web.sh` lo compila en `web/dist`, que el
servidor sirve como fallback (`GALATEA_WEB`, por defecto `./web/dist`). Para iterar:
`cd web && npm run dev` abre Vite en `http://localhost:5173` con la API reenviada a `127.0.0.1:8766`.

Variables nuevas del servidor: `GALATEA_WEB` (carpeta del cliente compilado) y
`GALATEA_ORIGENES` (orígenes con CORS, por defecto `https://localhost`, el de la app Android;
vacío desactiva CORS).

**No corras `galatea experimento` mientras el servidor sirve:** `preparar` vacía
`datos/sesiones/` y barre `datos/comfy/`, y se llevaría las sesiones vivas.

## APK (Android)

El mismo cliente empaquetado con Capacitor (`web/android/`, commiteado). La CA de Caddy va
dentro del APK, así que el teléfono no necesita instalar nada; la dirección del servidor se fija
al compilar (`GALATEA_SERVIDOR_APK`, por defecto `https://$GALATEA_IP:8444`) y se puede cambiar
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
Caddy deja pasar las peticiones con el origen de la app, `https://localhost`; cualquier otro origen
«cross-site» sigue recibiendo 403.
Las `<img>` llevan `crossorigin="anonymous"` para que también manden ese `Origin`; sin él, Caddy les responde 403.
Límite actual: «Descargar» no guarda nada en el APK, porque el WebView de Android no maneja descargas sin
código nativo. En la web funciona. Falta además el recorrido en un teléfono real (`experimento/2026-09-27-apk.md`).

## Pruebas

```bash
nix develop -c cargo test                 # unidad e integración (motor Fijo y ComfyUI falso)
nix develop -c npx playwright test        # navegador, con el servidor real
nix develop -c npm --prefix web test      # Vitest del cliente
nix flake check                           # la autoridad antes de dar algo por terminado
# contra el ComfyUI real, con el contenedor arriba:
# (GALATEA_FLUJO_REAL elige el flujo; por omisión catvton)
GALATEA_FLUJO_REAL=leffa GALATEA_COMFY_URL=http://127.0.0.1:8188 nix develop -c cargo test --test comfy_real -- --ignored --nocapture
```

## Documentos

- Especificación: [docs/superpowers/specs/2026-09-26-galatea-demo-design.md](docs/superpowers/specs/2026-09-26-galatea-demo-design.md)
- Plan de implementación: [docs/superpowers/plans/2026-09-26-galatea-demo.md](docs/superpowers/plans/2026-09-26-galatea-demo.md)
- Especificación del cliente y el APK: [docs/superpowers/specs/2026-09-27-galatea-apk-design.md](docs/superpowers/specs/2026-09-27-galatea-apk-design.md)
- Plan de implementación del cliente y el APK: [docs/superpowers/plans/2026-09-27-galatea-apk.md](docs/superpowers/plans/2026-09-27-galatea-apk.md)
- Convenciones del código: [CLAUDE.md](CLAUDE.md)

## Licencia

El código es [MIT](LICENSE). Los modelos que cargan los flujos de ComfyUI tienen su propia
licencia, indicada en la `descripcion` de cada flujo en `flujos/flujos.toml`; algunos no
permiten uso comercial.
