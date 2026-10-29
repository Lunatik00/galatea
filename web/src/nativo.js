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
