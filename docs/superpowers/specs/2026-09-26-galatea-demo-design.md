# Galatea · Diseño de la demo del probador virtual

**Fecha:** 2026-09-26 · **Estado:** aprobado por secciones en conversación, pendiente de
revisión del documento escrito

## 1. Resumen

Galatea es un probador virtual con IA generativa. En esta primera versión es una **demo
presencial**: una persona le saca una foto de cuerpo entero a otra con el celular, elige una
prenda de una lista precargada y, en menos de un minuto, ve a la persona con la prenda puesta.
La imagen se genera en el computador del desarrollador (GPU local) con **ComfyUI en un
contenedor**; el celular es solo cámara y pantalla, por la red de la casa.

El nombre viene de la luna de Neptuno y del mito: la estatua que cobra vida. Es el nombre de
la app, no de una tienda, para que cualquier tienda pueda usarla.

## 2. Decisiones ya tomadas

| Decisión | Elección | Por qué |
|---|---|---|
| Quién usa la demo | Una persona fotografía a otra, en persona | Es la demo que se quiere mostrar primero |
| Dónde se genera | GPU local del desarrollador | Costo cero por imagen, fotos no salen de la casa; API queda para después |
| Motor | **ComfyUI en contenedor**, un flujo JSON por modelo | Cambiar de modelo es cambiar un archivo; permite probar modelos propios entrenados para el celular |
| Cliente | Página web instalable (PWA) por HTTPS en la LAN, no app nativa | Cámara del navegador basta; sin tiendas de apps; reutiliza el Caddy con CA interna de Aylin |
| Servidor | Rust + axum, construido desde el flake de Nix | Convención de Nexo Huella y Crescent |
| Persistencia | Ninguna: sesiones en memoria y archivos temporales | La demo no necesita guardar nada |
| Licencias de modelos | Para la demo se aceptan modelos no comerciales | Es demo; antes de vender se migra a Leffa (MIT) o FLUX.2 klein (Apache 2.0) |

## 3. Alcance

**Dentro de la demo**

- Elegir prenda de una lista precargada; tomar foto; generar; ver resultado; comparar con la
  original; probar otra prenda con la misma foto; terminar y borrar.
- Selector oculto de flujo (motor) para comparar modelos con la misma foto.
- Motor `Fijo` que devuelve imágenes pregeneradas, para ensayar sin GPU o salir del paso si
  algo falla en terreno.
- Borrado de fotos y resultados al terminar, al vencer la sesión o al reiniciar el servidor.

**Fuera de la demo** (anotado en la ficha de la idea; no se cuela aquí)

- WhatsApp, Instagram, cuentas, historial, cobros, catálogo editable desde el celular.
- Modelos genéricas sin foto de la clienta.
- Correr el modelo en el teléfono.
- Medición de tallas (ver sección 14).

## 4. Flujo y pantalla

Una sola página, cuatro estados, más una pantalla inicial de aviso.

0. **Aviso.** Una frase: para qué es la foto, que se borra al terminar o a los quince
   minutos, que no se usa para entrenar, y que no se fotografía a menores. Botón
   "entendido". Se muestra una vez por sesión de navegador.
1. **Elegir prenda.** Cuadrícula de miniaturas con nombre corto. Tocar una la selecciona y
   queda visible arriba durante la foto.
2. **Tomar la foto.** Cámara trasera a pantalla completa con silueta guía superpuesta y tres
   indicaciones fijas: de pie, cuerpo entero, brazos separados del cuerpo. Botón grande de
   captura; luego "usar esta" o "repetir". La foto no se guarda en el celular: se sube al
   servidor y se descarta del navegador.
3. **Generando.** Barra de progreso real (eventos de ComfyUI), posición en cola si la hay y
   tiempo estimado. Si supera el límite (120 s por defecto) o falla, mensaje en español que
   dice qué pasó y botón de reintentar con la misma foto.
4. **Resultado.** Imagen generada grande con botón o deslizador para comparar con la original.
   Marca de agua discreta "imagen generada con IA", aplicada en el servidor. Acciones:
   **otra prenda** (vuelve a 1 conservando la foto), **repetir foto** (vuelve a 2),
   **descargar** (la persona puede quedarse con la imagen) y **terminar** (borra todo en el
   servidor y lo confirma en pantalla).

El selector oculto de flujo se abre con un parámetro en la URL (`?flujos=1`) y muestra, en el
estado 4, un desplegable para regenerar la misma foto y prenda con otro flujo.

## 5. Arquitectura

Tres servicios en `compose.yaml`, GPU por CDI como en Crescent.

```
celular ──HTTPS (LAN)──▶ caddy ──▶ galatea (axum) ──HTTP red compose──▶ comfyui (GPU)
                                       │                                   │
                                  datos/sesiones/                    datos/modelos/ (pesos)
                                  (tmpfs)                            datos/comfy/{input,output} (tmpfs)
```

- **comfyui.** `Dockerfile` corto: imagen base fijada por digest, más `git clone` de cada nodo
  personalizado de try-on a un commit fijo, más sus dependencias de Python. Monta
  `datos/modelos/` (pesos, fuera de git) y `datos/comfy/` (entrada y salida, en `tmpfs`).
  Escucha solo en la red de compose; durante el experimento se publica además en
  `127.0.0.1:8188` para usar su interfaz gráfica y exportar flujos.
- **galatea.** El servidor Rust. Sirve la página, maneja sesiones, rellena y envía flujos,
  sigue el progreso, aplica la marca de agua y borra.
- **caddy.** TLS con CA interna para la LAN y sitio auxiliar para instalar el certificado en el
  celular, copiado de `aylin/caddy/`.

Los pesos se descargan con `scripts/descargar-modelos.sh`, que lee `flujos/flujos.toml`, baja
cada archivo declarado a `datos/modelos/<subcarpeta>/` y verifica su suma SHA-256.

## 6. Flujos (motores) como archivos

`flujos/<nombre>.json` es el flujo en **formato API** de ComfyUI, exportado desde su interfaz.
`flujos/flujos.toml` lo describe:

```toml
[[flujo]]
nombre = "catvton"
titulo = "CatVTON"
descripcion = "SD1.5 inpainting, rápido, < 8 GB. Licencia CC BY-NC-SA (solo demo)."
archivo = "catvton.json"
nodo_persona = "12"        # id del nodo LoadImage de la foto
nodo_prenda = "13"         # id del nodo LoadImage de la prenda
nodo_tipo = "20"           # id del nodo que recibe "upper" | "lower" | "overall"; opcional
nodo_salida = "30"         # id del nodo SaveImage
tipos = ["superior", "inferior", "vestido"]
[[flujo.peso]]
url = "https://huggingface.co/.../resolve/main/....safetensors"
destino = "checkpoints/....safetensors"
sha256 = "..."
```

El servidor Rust solo conoce esos identificadores: carga el JSON, escribe el nombre del archivo
subido en `nodo_persona`, el archivo de la prenda en `nodo_prenda`, el tipo traducido en
`nodo_tipo` si existe, y manda el flujo. **Agregar un modelo, incluido uno propio, es exportar
un flujo y agregar una entrada al TOML.** No se toca Rust.

Un flujo con `nombre = "fijo"` es especial: no va a ComfyUI. Devuelve
`flujos/fijo/<prenda>.png` si existe, o `flujos/fijo/generico.png`, tras esperar
`espera_segundos` (por defecto 8) para que la demo se vea realista.

## 7. Prendas

`prendas/prendas.toml`:

```toml
[[prenda]]
id = "polera-amarilla"
nombre = "Polera amarilla"
archivo = "polera-amarilla.jpg"   # en prendas/, fondo liso, ≥ 768 px de alto
tipo = "superior"                 # superior | inferior | vestido
```

Las miniaturas se generan al arrancar (256 px) en `datos/miniaturas/`. Un `prendas.toml`
malformado o un archivo faltante impide arrancar con un mensaje que dice qué línea y qué
archivo.

## 8. Servidor Rust

Un módulo por responsabilidad. Todo el dominio en español.

- `prendas`: `cargar(ruta) -> Result<Vec<Prenda>, ErrorPrendas>`; `miniatura(&Prenda) -> PathBuf`.
- `flujos`: `cargar(ruta) -> Result<Vec<Flujo>, ErrorFlujos>`; `rellenar(&Flujo, &Entradas) ->
  serde_json::Value` (pura, sin E/S; es la función más probada).
- `sesion`: `Sesiones` (mapa en memoria tras un `RwLock`) con `crear(foto: Bytes) -> Sesion`,
  `obtener(id)`, `terminar(id)`, `vencer_antiguas(ahora)`. Cada sesión: `id` aleatorio de 128
  bits, `creada`, carpeta `datos/sesiones/<id>/` con `persona.jpg` y `resultado-<n>.png`.
  Una tarea de fondo llama a `vencer_antiguas` cada minuto. Al arrancar se vacía
  `datos/sesiones/` completo.
- `motor`: `enum Motor { Comfy { url }, Fijo { carpeta } }`. El flujo elegido decide el
  motor: el flujo `fijo` va a `Motor::Fijo`, cualquier otro a `Motor::Comfy`. Un solo método:
  `generar(&self, flujo, persona, prenda) -> impl Stream<Item = Avance>` donde
  `Avance::{EnCola(posicion), Progreso(fraccion), Listo(PathBuf), Error(ErrorMotor)}`.
  Solo `motor::comfy` habla con ComfyUI: `POST /upload/image`, `POST /prompt`, WebSocket
  `/ws?clientId=` para progreso, `GET /view` para bajar la salida, `GET /queue` para la
  posición. Al terminar, borra la entrada y la salida en ComfyUI (`datos/comfy/` está montado
  también en `galatea` para poder borrar por nombre).
- `marca`: `aplicar(png: &[u8], texto) -> Vec<u8>`: texto pequeño en la esquina inferior.
- `web`: rutas y plantillas askama + htmx; JavaScript solo para la cámara (`getUserMedia`),
  la captura a JPEG y el SSE.

Rutas:

| Método y ruta | Qué hace |
|---|---|
| `GET /` | Página única |
| `GET /prendas` | Fragmento con la cuadrícula |
| `POST /sesiones` | Recibe `persona.jpg` (multipart), devuelve `{ id }` |
| `POST /sesiones/{id}/generar` | Cuerpo `{ prenda, flujo }`; devuelve `{ trabajo }` |
| `GET /sesiones/{id}/avance/{trabajo}` | SSE con `EnCola`, `Progreso`, `Listo`, `Error` |
| `GET /sesiones/{id}/resultado/{n}.png` | Imagen con marca de agua |
| `GET /sesiones/{id}/persona.jpg` | Original, para comparar |
| `DELETE /sesiones/{id}` | Termina y borra |
| `GET /flujos` | Lista de flujos, para el selector oculto |
| `GET /salud` | Estado: ComfyUI alcanzable, flujos con pesos presentes |

Un solo trabajo activo a la vez en `galatea` (semáforo de 1); si llega otro, responde
`EnCola(n)` y el celular lo muestra.

Configuración por variables de entorno con prefijo `GALATEA_`: `PUERTO`, `COMFY_URL`,
`DATOS`, `FLUJO_POR_DEFECTO`, `VENCIMIENTO_MIN` (15), `LIMITE_SEG` (120).

## 9. Errores

Todos los mensajes que ve una persona están en español, dicen qué pasó y qué hacer:

- ComfyUI no responde → "El motor de imágenes no está encendido. Levántalo con `docker
  compose up comfyui` o cambia al flujo Fijo."
- Faltan pesos de un flujo → se detecta en `/salud` al arrancar y el flujo aparece
  deshabilitado en el selector con el nombre del archivo faltante.
- Foto sin cuerpo entero o sin persona → por ahora no se detecta; el resultado sale malo y la
  guía de la cámara es la mitigación. Detectar pose queda para después.
- Tiempo excedido → cancela el trabajo en ComfyUI (`POST /interrupt`) y ofrece reintentar.
- Sesión vencida → "Pasaron más de 15 minutos; la foto ya se borró. Toma una nueva."

## 10. Privacidad

- Aviso inicial (sección 4, estado 0). Sin firma ni datos personales: es una demo presencial
  con voluntarias y quien sostiene el celular explica.
- Fotos y resultados solo en `datos/sesiones/` y `datos/comfy/`, ambos en `tmpfs`; se borran al
  terminar, al vencer, y al reiniciar.
- La bitácora registra tiempos, flujo usado y errores; nunca imágenes ni el identificador
  completo de sesión (solo sus primeros 6 caracteres).
- Nada se manda fuera de la red local en esta versión.
- Base legal a la vista para lo que viene: Ley 21.719 (vigente 2026-12-01); ver el anexo de la
  ficha de la idea.

## 11. Pruebas

- **Unitarias (Rust):** `prendas::cargar` y `flujos::cargar` con TOML válidos y malformados;
  `flujos::rellenar` con un flujo de ejemplo (verifica los tres nodos y que el resto no
  cambie); `sesion::vencer_antiguas` con reloj inyectado; `marca::aplicar` sobre un PNG de
  prueba (dimensiones intactas, píxeles de la esquina cambiados).
- **Ruta web con motor `Fijo`:** crear sesión, generar, leer el SSE hasta `Listo`, bajar el
  PNG, terminar, verificar 404 después. Sin GPU.
- **Integración con ComfyUI real**, marcada `#[ignore]` y ejecutada solo con
  `GALATEA_COMFY_URL` definida y el contenedor arriba: sube la foto de prueba, corre el flujo
  más liviano y verifica que vuelve un PNG.
- **Playwright** (como en Aylin): cámara simulada con una imagen fija
  (`--use-fake-device-for-media-stream`), recorrido completo con `Fijo`, y comprobación de que
  tras terminar el servidor ya no sirve la imagen.

## 12. Experimento y métrica

Primera tarea real, antes de pulir pantalla alguna:

1. Levantar ComfyUI en el contenedor y hacer correr, uno por uno, **CatVTON**, **IDM-VTON**,
   **Leffa** y **FLUX.2 klein 4B + LoRA de try-on**. El que no compile, no tenga nodo
   funcional o no quepa en VRAM se descarta y se anota por qué.
2. **Comprobación de flujo con imágenes Creative Commons** (sección 13): una prenda y una
   persona, por cada flujo que sobrevivió. El objetivo aquí es solo que la tubería funcione de
   punta a punta: subir, generar, bajar, borrar.
3. Luego, cinco prendas de una tienda por tres voluntarias: quince fotos por flujo. Los
   resultados van a `experimento/AAAA-MM-DD/` con una tabla: flujo, segundos por imagen, VRAM
   pico, y la nota de quien vende "la mostraría a una clienta: sí / no".
4. El flujo con más "sí" pasa a `GALATEA_FLUJO_POR_DEFECTO`. Los demás quedan disponibles en
   el selector oculto.

**Métrica de la demo:** al menos 10 de 15 imágenes que la tienda mostraría, en menos de 45
segundos cada una en la GPU local. Si no se llega, la demo se hace igual con el mejor flujo, se
anota la brecha, y la decisión pasa a ser esperar la próxima generación de modelos o usar una
API.

**VRAM.** La GPU tiene ~24 GB y la comparten Ollama (Crescent) y vLLM (Aylin). Durante el
experimento y la demo se apagan; `scripts/demo.sh` lo recuerda y verifica con `nvidia-smi`
antes de levantar ComfyUI.

## 13. Imágenes de prueba (Creative Commons)

Para la comprobación de flujo, sin fotos de personas reales del entorno. Se descargan con
`scripts/descargar-pruebas.sh` a `pruebas/imagenes/` (fuera de git) y la atribución vive en
`pruebas/imagenes/LICENCIAS.md` (dentro de git). Confirmadas visualmente el 2026-09-26:

| Uso | Archivo en Commons | Licencia | Autor / fuente |
|---|---|---|---|
| Prenda superior, plana sobre fondo liso | `T-shirt (AM 2002.94.1.1-1).jpg` (1680×1696) https://commons.wikimedia.org/wiki/File:T-shirt_(AM_2002.94.1.1-1).jpg | CC BY 4.0 | Auckland War Memorial Museum |
| Persona de pie, cuerpo entero, fondo liso | `Gordon Cooper posing in suit.jpg` (3022×4096) https://commons.wikimedia.org/wiki/File:Gordon_Cooper_posing_in_suit.jpg | Dominio público (NASA) | NASA |

La persona lleva traje espacial, lo que es un caso difícil a propósito: si el flujo funciona
con eso, funciona con ropa normal. Si algún modelo falla solo por el traje, la tarea del
experimento agrega una segunda persona con estos criterios: fotografía (no dibujo ni pintura),
de pie, cuerpo entero, ropa ajustada y lisa, fondo liso, licencia CC0, CC BY o dominio público,
sin menores. Y una segunda prenda: vestido o pantalón, plano o en maniquí, fondo liso, misma
regla de licencia. Fuentes que sirvieron: la API de búsqueda de Wikimedia Commons con
`filetype:bitmap` y las colecciones del Auckland Museum; Openverse dio poco.

## 14. Ideas futuras (anotadas, no se hacen ahora)

- **Talla, no solo apariencia.** Con ARCore (o ARKit) medir a la persona (altura, hombros,
  pecho, cintura, cadera) y tener las medidas de cada prenda en `prendas.toml` (largo, ancho de
  pecho, cintura, por talla). La app diría "te ves así **y** la M te queda; la S te aprieta en
  el pecho". Esto ataca el problema real de las devoluciones y es lo que ninguna API de try-on
  hace. Requiere app nativa o WebXR con medición, y una tabla de medidas por prenda que la
  tienda ya maneja al confeccionar. Anotado el 2026-09-26 a pedido del desarrollador.
- **Correr en el celular** con calidad menor: modelos chicos entrenados o destilados, probados
  primero como flujo de ComfyUI (por eso ComfyUI) y luego portados.
- **Modelos genéricas** sin foto de la clienta, precalculadas por prenda.
- **Motor `Api`** (FASHN, Vertex) para usar la demo fuera de la red de la casa.
- **Canal WhatsApp** para la clienta sola, con consentimiento registrado.

## 15. Verificaciones pendientes para el plan

- Qué nodos de ComfyUI existen y funcionan hoy para CatVTON, IDM-VTON y Leffa, y a qué commit
  fijarlos; klein 4B es nativo en ComfyUI y su LoRA de try-on se carga como cualquier LoRA.
- Imagen base de ComfyUI para el `Dockerfile` (con CUDA compatible con el driver de la
  máquina) y su digest.
- VRAM real de cada flujo a 768×1024 en esta GPU.
- Si `tmpfs` para `datos/comfy/` cabe en RAM con las salidas del experimento (poner límite).
