import { beforeEach, describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

vi.mock("./api.js", async importar => {
  const real = await importar();
  return {
    ...real,
    prendas: vi.fn(async () => [{ id: "roja", nombre: "Roja", miniatura: "/prendas/roja/miniatura.png" }]),
    crearSesion: vi.fn(async () => ({ id: "s1" })),
    generar: vi.fn(async () => ({ trabajo: "t1", resultado: 1 })),
    avance: vi.fn(() => ({ cerrar: vi.fn() })),
    buscarResultado: vi.fn(async () => "listo"),
    terminar: vi.fn(async () => true),
    flujos: vi.fn(async () => [{ nombre: "fijo", titulo: "Fijo", disponible: true }]),
  };
});

import * as api from "./api.js";
import * as acciones from "./estado.js";

const { estado } = acciones;
const foto = new Blob(["x"], { type: "image/jpeg" });
const prenda = { id: "roja", nombre: "Roja", miniatura: "/prendas/roja/miniatura.png" };

describe("estado", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    estado.set(acciones.inicial());
  });

  it("arranca en el aviso y pasa a prendas", () => {
    expect(get(estado).pantalla).toBe("aviso");
    acciones.entendido();
    expect(get(estado).pantalla).toBe("prendas");
  });

  it("elegir prenda sin foto abre la cámara; con sesión y foto, genera directo", async () => {
    acciones.elegirPrenda(prenda);
    expect(get(estado).pantalla).toBe("camara");
    expect(get(estado).prenda).toEqual(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    expect(api.crearSesion).toHaveBeenCalledWith(foto);
    expect(api.generar).toHaveBeenCalledWith("s1", "roja", "");
    expect(get(estado).pantalla).toBe("generando");
    expect(get(estado).sesion).toEqual({ id: "s1", trabajo: "t1", resultado: 1 });
    // Segunda prenda con la misma foto: no se crea otra sesión.
    acciones.otraPrenda();
    expect(get(estado).pantalla).toBe("prendas");
    await acciones.elegirPrenda({ ...prenda, id: "azul" });
    expect(api.crearSesion).toHaveBeenCalledTimes(1);
    expect(api.generar).toHaveBeenLastCalledWith("s1", "azul", "");
  });

  it("el avance actualiza el progreso y «listo» muestra el resultado", async () => {
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    const oyentes = api.avance.mock.calls[0][2];
    oyentes.encola(2);
    expect(get(estado).progreso.texto).toBe("Hay 2 delante en la cola");
    oyentes.progreso(0.5);
    expect(get(estado).progreso).toEqual({ valor: 0.5, texto: "50 %" });
    oyentes.listo(1);
    expect(get(estado).pantalla).toBe("resultado");
    expect(get(estado).sesion.resultado).toBe(1);
  });

  it("si el SSE se corta sin datos, va a buscar el resultado", async () => {
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    const oyentes = api.avance.mock.calls[0][2];
    await oyentes.error(null);
    expect(api.buscarResultado).toHaveBeenCalledWith("s1", 1, expect.any(Object));
    expect(get(estado).pantalla).toBe("resultado");
  });

  it("si el servidor falla, queda el mensaje y los botones de reintentar/volver", async () => {
    api.generar.mockRejectedValueOnce(new api.FalloApi("http", 503, "motor apagado"));
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    expect(get(estado).pantalla).toBe("generando");
    expect(get(estado).mensaje).toEqual({ texto: "motor apagado", error: true });
    expect(get(estado).fallo).toBe(true);
    acciones.volverAPrendas();
    expect(get(estado).pantalla).toBe("prendas");
    expect(get(estado).sesion.id).toBe("s1"); // la sesión y la foto se conservan
  });

  it("404 al generar = sesión vencida: vuelve a la cámara sin foto y con el aviso", async () => {
    api.generar.mockRejectedValueOnce(new api.FalloApi("vencida", 404, api.VENCIDA));
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    const e = get(estado);
    expect(e.pantalla).toBe("camara");
    expect(e.foto).toBeNull();
    expect(e.sesion.id).toBeNull();
    expect(e.mensaje).toEqual({ texto: api.VENCIDA, error: true });
  });

  it("terminar borra la sesión y lo dice; si no se confirma, lo dice como error", async () => {
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    await acciones.terminar();
    expect(api.terminar).toHaveBeenCalledWith("s1");
    expect(get(estado).pantalla).toBe("prendas");
    expect(get(estado).sesion.id).toBeNull();
    expect(get(estado).mensaje.texto).toContain("se borraron");
    api.terminar.mockResolvedValueOnce(false);
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    await acciones.terminar();
    expect(get(estado).mensaje).toMatchObject({ error: true });
    expect(get(estado).mensaje.texto).toContain("No se pudo confirmar");
  });

  it("repetir foto termina la sesión y vuelve a la cámara", async () => {
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    await acciones.repetirFoto();
    expect(api.terminar).toHaveBeenCalledWith("s1");
    expect(get(estado).pantalla).toBe("camara");
    expect(get(estado).foto).toBeNull();
  });

  it("elegir flujo vuelve a generar con ese flujo", async () => {
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    await acciones.elegirFlujo("leffa");
    expect(api.generar).toHaveBeenLastCalledWith("s1", "roja", "leffa");
  });

  it("ajustes recuerda a dónde volver", () => {
    acciones.entendido();
    acciones.abrirAjustes();
    expect(get(estado).pantalla).toBe("ajustes");
    acciones.cerrarAjustes();
    expect(get(estado).pantalla).toBe("prendas");
  });

  it("un fallo de red al elegir prenda deja el mensaje con la dirección y el botón de ajustes", async () => {
    api.crearSesion.mockRejectedValueOnce(new api.FalloApi("red", null, "No encuentro el servidor en https://x (fetch)."));
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    expect(get(estado).pantalla).toBe("camara");
    expect(get(estado).mensaje).toEqual({ texto: "No encuentro el servidor en https://x (fetch).", error: true, ajustes: true });
  });
  it("si se vuelve a las prendas mientras se busca el resultado, el sondeo no saca a la persona de ahí", async () => {
    let resolver;
    api.buscarResultado.mockImplementationOnce(() => new Promise(r => { resolver = r; }));
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    await acciones.usarFoto();
    const oyentes = api.avance.mock.calls[0][2];
    const pendiente = oyentes.error(null);
    expect(api.buscarResultado).toHaveBeenCalledTimes(1);
    const { vigente } = api.buscarResultado.mock.calls[0][2];
    expect(vigente()).toBe(true);
    acciones.volverAPrendas();
    expect(vigente()).toBe(false);
    resolver("listo");
    await pendiente;
    expect(get(estado).pantalla).toBe("prendas");
    expect(get(estado).mensaje).toBeNull();
  });

  it("si se vuelve a las prendas mientras espera el trabajo, el trabajo que llega se ignora", async () => {
    let resolver;
    api.generar.mockImplementationOnce(() => new Promise(r => { resolver = r; }));
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    const enCurso = acciones.usarFoto();
    await vi.waitFor(() => expect(api.generar).toHaveBeenCalledTimes(1));
    expect(get(estado).pantalla).toBe("generando");
    acciones.volverAPrendas();
    resolver({ trabajo: "t1", resultado: 1 });
    await enCurso;
    expect(api.avance).not.toHaveBeenCalled();
    expect(get(estado).pantalla).toBe("prendas");
  });

  it("un fallo que llega después de volver a las prendas no pinta el error ahí", async () => {
    let rechazar;
    api.generar.mockImplementationOnce(() => new Promise((_, r) => { rechazar = r; }));
    acciones.elegirPrenda(prenda);
    acciones.fotoLista(foto);
    const enCurso = acciones.usarFoto();
    await vi.waitFor(() => expect(api.generar).toHaveBeenCalledTimes(1));
    acciones.volverAPrendas();
    rechazar(new api.FalloApi("http", 503, "motor apagado"));
    await enCurso;
    expect(get(estado).pantalla).toBe("prendas");
    expect(get(estado).mensaje).toBeNull();
    expect(get(estado).fallo).toBe(false);
  });
});
