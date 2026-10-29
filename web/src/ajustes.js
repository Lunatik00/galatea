// Dirección base del servidor. Sin nada guardado se usa VITE_SERVIDOR; si también está vacía,
// el mismo origen (la web servida por Rust). En el APK el build fija VITE_SERVIDOR y la persona
// puede cambiarla en Ajustes; guardarla vacía vuelve al valor por defecto.
const CLAVE = "galatea.servidor";

export function porDefecto() {
  return normalizarServidor(import.meta.env.VITE_SERVIDOR ?? "");
}

export function normalizarServidor(valor) {
  return String(valor ?? "").trim().replace(/\/+$/, "");
}

/** "" si es válida; si no, el mensaje para la persona. Vacía es válida (valor por defecto). */
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
    // Vacía no se guarda como "": en el APK eso apuntaría la app a su propio paquete.
    if (v === "" || v === porDefecto()) localStorage.removeItem(CLAVE);
    else localStorage.setItem(CLAVE, v);
  } catch { /* sin almacenamiento: vale solo hasta cerrar la app */ }
}
