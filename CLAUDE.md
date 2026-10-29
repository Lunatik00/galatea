# Convenciones de Galatea

Probador virtual con IA generativa. Demo presencial: una persona le saca una foto a otra
desde el celular y ve la prenda puesta. Rust + axum en el host, ComfyUI en contenedor con
la GPU, Caddy para HTTPS en la red local. **Sin base de datos:** sesiones en memoria y
archivos temporales que se borran.

## Dependencias: siempre la versión más reciente

**Regla dura.** Al añadir o tocar una dependencia, usa la **última versión estable
publicada**, comprobándola contra crates.io en ese momento. No escribas versiones de
memoria: el conocimiento de los modelos va meses o años por detrás.

```bash
curl -s https://crates.io/api/v1/crates/<crate> -H 'User-Agent: crescent-dev' \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['crate']['max_stable_version'])"
```

Lo mismo para el toolchain de Rust y para los *inputs* del flake. Si una versión nueva
rompe la compilación, **arréglalo**; no vuelvas a la vieja sin decirlo en el informe y
explicar por qué.

## Idioma

El dominio se escribe **en español**: `caso`, `nota`, `campo`, `redaccion`, `desde_fila`.
Solo las palabras propias de Rust y de las bibliotecas quedan en inglés. Quien lea este
código en cinco años habla español.

Los mensajes de error que ve una persona se escriben completos y en español, diciendo
qué pasó y cómo arreglarlo.

## Informar: solo lo que ejecutaste

**Regla dura.** Un criterio no está cumplido porque el código parezca hacerlo: está
cumplido cuando lo ejecutaste y viste la salida. Al informar, pega la salida real del
comando que lo comprueba.

Prohibido en un informe:

- El condicional: «*fallaría*», «*debería devolver*», «*se aplican en transacción*».
  Eso describe el código, no lo prueba.
- «CUMPLE por inspección». Si el criterio pide una prueba automática, la prueba tiene
  que existir y correr.
- Dar por bueno que algo arranca porque compila. Compilar y arrancar son dos cosas
  distintas.

**Un criterio no cumplido y reportado vale más que uno dado por bueno sin ejecutar.**
Si algo no se puede verificar todavía, dilo y explica por qué; nadie te va a pedir que
inventes.

Cuando escribas una prueba nueva que protege algo importante, **comprueba que falla
cuando debe**: rompe a propósito lo que protege, mira que la prueba se ponga roja, y
deshaz el cambio. Una prueba que pasa siempre no protege nada.

## Trampas ya vistas en otros proyectos del mismo autor

- **No dupliques el sistema para probarlo.** El ayudante de pruebas levanta el router
  real con `construir_router`. Una copia de las rutas dentro del ayudante deja de
  probar el sistema en cuanto las dos divergen.
- **No inventes valores para satisfacer una restricción.** Un identificador escrito a
  pelo viola una clave foránea en cuanto esa fila no existe. Si un dato obligatorio no
  está disponible, di que falta; no lo rellenes.
- **Lo que es público, se usa.** Un método que dice en un comentario «solo para uso
  interno» pero es `pub` acabará usado desde fuera. Restringe la visibilidad de verdad
  (`pub(super)`, `pub(crate)`).
- **`nix flake check` es la autoridad, no `cargo` en el devShell.** No des una tarea por
  terminada sin que `nix flake check` pase.
- **Los symlinks de nix se llaman `result`, `result-1`, `result-2`…** Ya están en
  `.gitignore`; no los añadas al repositorio.

## Git

Mensajes de commit **planos**. Nunca añadas `Co-Authored-By`, `Claude-Session` ni
ninguna otra firma o *trailer*.

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

El cliente se prueba con Vitest (`web/src/*.test.js`) para la lógica y con Playwright para el
recorrido; los selectores `data-estado`/`data-accion` son el contrato entre ambos.

## GPU compartida

La RTX 3090 la usan también `crescent-ollama` y los contenedores de Aylin. `scripts/demo.sh`
los detiene antes de levantar ComfyUI y lo dice por pantalla. No los mates a mano con
`pkill`: usa `docker stop <nombre>`.
