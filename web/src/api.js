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

const NO_ES_JSON = Symbol("no es JSON");

/** `base` es la dirección del servidor; por omisión, la de Ajustes. */
async function pedir(ruta, opciones = {}, base = servidor()) {
  let r;
  try {
    r = await fetch(base + ruta, opciones);
  } catch (e) {
    throw new FalloApi("red", null, `No encuentro el servidor en ${base || "esta dirección"} (${e.message}).`);
  }
  if (r.status === 204) return null;
  const cuerpo = await r.json().catch(() => NO_ES_JSON);
  if (r.status === 404) throw new FalloApi("vencida", 404, VENCIDA);
  if (cuerpo === NO_ES_JSON) {
    // Un 2xx que no es JSON es otra cosa en esa dirección (en el APK, su propio paquete).
    throw new FalloApi("http", r.status, r.ok ? "El servidor respondió algo que no es Galatea" : `error ${r.status}`);
  }
  if (!r.ok) throw new FalloApi("http", r.status, cuerpo?.error || `error ${r.status}`);
  return cuerpo;
}

export const prendas = () => pedir("/prendas");
export const flujos = () => pedir("/flujos");
/** Sin `base` pregunta al servidor guardado; con `base`, a esa dirección (Probar en Ajustes). */
export const salud = base => pedir("/salud", {}, base ?? servidor());

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
