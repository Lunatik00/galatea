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
  it("un 2xx que no es JSON es un fallo: no es un servidor Galatea", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => respuesta(200)));
    await expect(api.salud()).rejects.toMatchObject({
      clase: "http", estado: 200, message: "El servidor respondió algo que no es Galatea",
    });
    await expect(api.prendas()).rejects.toMatchObject({ clase: "http", estado: 200 });
  });

  it("salud acepta otra base sin tocar el ajuste guardado", async () => {
    const fetch = vi.fn(async () => respuesta(200, { programa: "galatea", version: "0.1.0" }));
    vi.stubGlobal("fetch", fetch);
    const s = await api.salud("https://192.168.0.30:8444");
    expect(s).toEqual({ programa: "galatea", version: "0.1.0" });
    expect(fetch.mock.calls[0][0]).toBe("https://192.168.0.30:8444/salud");
    await api.salud();
    expect(fetch.mock.calls[1][0]).toBe("/salud");
  });
});
