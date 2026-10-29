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
// Cada `generar()` es un intento con su número. Salir de «generando» sube el contador, así que
// lo que llegue después (la respuesta de generar, el SSE, el sondeo) ya no toca la pantalla.
let intento = 0;
function abandonarIntento() {
  intento++;
  cerrarFuente();
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
  abandonarIntento();
  const mio = intento;
  const actual = () => mio === intento && get(estado).pantalla === "generando";
  cambiar({ pantalla: "generando", progreso: { valor: 0, texto: "Preparando" }, fallo: false, mensaje: null });
  const { sesion, prenda, flujo } = get(estado);
  try {
    const { trabajo, resultado } = await api.generar(sesion.id, prenda.id, flujo);
    if (!actual()) return;
    cambiar({ sesion: { ...sesion, trabajo, resultado } });
    seguir(sesion.id, trabajo, resultado, actual);
  } catch (e) {
    if (!actual()) return;
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

/** `actual()` dice si este intento sigue siendo el de la pantalla «generando». */
function seguir(id, trabajo, resultado, actual) {
  fuente = api.avance(id, trabajo, {
    encola: n => {
      if (actual()) cambiar({ progreso: { valor: 0, texto: n === 0 ? "Empezando" : `Hay ${n} delante en la cola` } });
    },
    progreso: v => { if (actual()) cambiar({ progreso: { valor: v, texto: `${Math.round(v * 100)} %` } }); },
    listo: n => { if (!actual()) return; fuente = null; mostrarResultado(n); },
    error: async texto => {
      if (!actual()) return;
      fuente = null;
      if (texto) { fallar(new api.FalloApi("http", null, texto)); return; }
      // Sin datos se cortó la conexión (celular bloqueado, cambio de red): el servidor igual
      // termina y sella, así que se va a buscar el resultado.
      cambiar({ progreso: { valor: 0, texto: "Se cortó la conexión; esperando el resultado" } });
      const vigente = () => {
        const s = get(estado).sesion;
        return actual() && s.id === id && s.resultado === resultado;
      };
      const fin = await api.buscarResultado(id, resultado, { vigente });
      if (!vigente()) return;
      if (fin === "listo") mostrarResultado(resultado);
      else if (fin === "agotado") fallar(new api.FalloApi("red", null, "Se perdió la conexión con el servidor"));
    },
  });
}

function mostrarResultado(n) {
  estado.update(e => ({ ...e, sesion: { ...e.sesion, resultado: n }, pantalla: "resultado", mensaje: null }));
}

export function volverAPrendas() {
  abandonarIntento();
  mostrar("prendas");
}
export function otraPrenda() {
  abandonarIntento();
  mostrar("prendas");
}

export async function elegirFlujo(flujo) {
  cambiar({ flujo });
  await generar();
}

async function cerrarSesion() {
  const { sesion } = get(estado);
  abandonarIntento();
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
