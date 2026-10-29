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
    // En el APK el valor por defecto es remoto: vacío no puede apuntar la app a su propio paquete.
    vi.stubEnv("VITE_SERVIDOR", "https://192.168.0.10:8444");
    guardarServidor("https://192.168.0.20:8444");
    expect(servidor()).toBe("https://192.168.0.20:8444");
    guardarServidor("");
    expect(servidor()).toBe("https://192.168.0.10:8444");
    guardarServidor("  ");
    expect(servidor()).toBe("https://192.168.0.10:8444");
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
