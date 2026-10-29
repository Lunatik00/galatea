import { test, expect } from "@playwright/test";

test("recorrido completo con el flujo fijo y borrado al terminar", async ({ page, request }) => {
  await page.goto("/");
  await expect(page.locator('[data-estado="aviso"]')).toBeVisible();
  await page.click('[data-accion="entendido"]');

  await expect(page.locator('[data-prenda="roja"]')).toBeVisible();
  await page.click('[data-prenda="roja"]');

  await expect(page.locator('[data-estado="camara"]')).toBeVisible();
  await page.setInputFiles('input[data-accion="archivo"]', "pruebas/imagenes/persona-prueba.jpg");
  await expect(page.locator('[data-accion="usar"]')).toBeVisible();
  await page.click('[data-accion="usar"]');

  await expect(page.locator('[data-estado="resultado"]')).toBeVisible({ timeout: 30_000 });
  const resultado = page.locator('img[data-imagen="resultado"]');
  await expect(resultado).toBeVisible();
  const url = await resultado.getAttribute("src");
  expect(url).toMatch(/\/sesiones\/[0-9a-f]{32}\/resultado\/1\.png/);
  expect((await request.get(url)).status()).toBe(200);

  await page.click('[data-accion="comparar"]');
  await expect(page.locator('img[data-imagen="original"]')).toBeVisible();

  await page.click('[data-accion="terminar"]');
  await expect(page.locator('[data-estado="prendas"]')).toBeVisible();
  await expect(page.locator("[data-mensaje]")).toContainText("se borraron");
  expect((await request.get(url)).status()).toBe(404);
});

test("el selector de flujos aparece solo con ?flujos=1", async ({ page }) => {
  await page.goto("/?flujos=1");
  await page.click('[data-accion="entendido"]');
  await page.click('[data-prenda="azul"]');
  await page.setInputFiles('input[data-accion="archivo"]', "pruebas/imagenes/persona-prueba.jpg");
  await page.click('[data-accion="usar"]');
  await expect(page.locator("[data-selector-flujos]")).toBeVisible({ timeout: 30_000 });
  const opciones = page.locator("select[data-flujos] option");
  await expect(opciones).toHaveCount(2);
  // toBeDisabled() no sirve aquí: Playwright redirige un elemento dentro de <label> al control
  // del label (el <select>, habilitado). Se mira la propiedad de cada <option>.
  await expect(opciones.nth(0)).toHaveJSProperty("disabled", false); // «fijo» siempre disponible
  await expect(opciones.nth(1)).toHaveJSProperty("disabled", true); // «prueba» sin pesos
});

async function hastaUsarEsta(page, prenda = "roja") {
  await page.goto("/");
  await page.click('[data-accion="entendido"]');
  await page.click(`[data-prenda="${prenda}"]`);
  await page.setInputFiles('input[data-accion="archivo"]', "pruebas/imagenes/persona-prueba.jpg");
  await page.click('[data-accion="usar"]');
}

test("si el servidor no confirma el borrado, no se dice que se borró", async ({ page }) => {
  await hastaUsarEsta(page);
  await expect(page.locator('[data-estado="resultado"]')).toBeVisible({ timeout: 30_000 });
  await page.route("**/sesiones/*", ruta =>
    ruta.request().method() === "DELETE" ? ruta.fulfill({ status: 500, body: "" }) : ruta.continue());
  await page.click('[data-accion="terminar"]');
  await expect(page.locator('[data-estado="prendas"]')).toBeVisible();
  const mensaje = page.locator("[data-mensaje]");
  await expect(mensaje).toContainText("No se pudo confirmar");
  await expect(mensaje).not.toContainText("se borraron");
  await expect(mensaje).toHaveClass(/error/);
});

test("si la generación falla, se puede volver a las prendas", async ({ page }) => {
  await page.route("**/sesiones/*/generar", ruta =>
    ruta.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({ error: "motor apagado" }) }));
  await hastaUsarEsta(page);
  await expect(page.locator('[data-estado="generando"]')).toBeVisible();
  await expect(page.locator("[data-mensaje]")).toContainText("motor apagado");
  await expect(page.locator('[data-accion="reintentar"]')).toBeVisible();
  const volver = page.locator('[data-accion="volver"]');
  await expect(volver).toBeVisible();
  await volver.click();
  await expect(page.locator('[data-estado="prendas"]')).toBeVisible();
  await expect(page.locator('[data-prenda="azul"]')).toBeVisible();
});

test("si la sesión venció, generar da 404 y la página pide una foto nueva", async ({ page }) => {
  let sesionesCreadas = 0;
  page.on("request", r => { if (r.method() === "POST" && new URL(r.url()).pathname === "/sesiones") sesionesCreadas++; });
  await page.route("**/sesiones/*/generar", ruta =>
    ruta.fulfill({ status: 404, contentType: "application/json", body: JSON.stringify({ error: "esa sesión no existe o ya venció (15 minutos); toma una foto nueva" }) }));
  await hastaUsarEsta(page);
  await expect(page.locator('[data-estado="camara"]')).toBeVisible();
  const mensaje = page.locator("[data-mensaje]");
  await expect(mensaje).toContainText("Pasaron más de 15 minutos; la foto ya se borró. Toma una nueva.");
  await expect(mensaje).toHaveClass(/error/);
  expect(sesionesCreadas).toBe(1);
  // La foto vieja se olvidó: para seguir hay que subir una nueva, que crea otra sesión.
  await expect(page.locator('[data-accion="usar"]')).toBeHidden();
  await page.setInputFiles('input[data-accion="archivo"]', "pruebas/imagenes/persona-prueba.jpg");
  await page.click('[data-accion="usar"]');
  await expect.poll(() => sesionesCreadas).toBe(2);
});

test("si se corta el SSE, la página va a buscar el resultado igual", async ({ page }) => {
  // El avance nunca llega: EventSource da un error sin datos, como con el celular bloqueado.
  await page.route("**/sesiones/*/avance/*", ruta => ruta.abort("connectionreset"));
  await hastaUsarEsta(page);
  await expect(page.locator('[data-estado="resultado"]')).toBeVisible({ timeout: 20_000 });
  await expect(page.locator('img[data-imagen="resultado"]')).toHaveAttribute("src", /\/resultado\/1\.png/);
});

test("ajustes: prueba la dirección, guarda y vuelve", async ({ page, baseURL }) => {
  await page.goto("/");
  await page.click('[data-accion="abrir-ajustes"]');
  await expect(page.locator('[data-estado="ajustes"]')).toBeVisible();
  await page.fill('[data-campo="servidor"]', "http://inseguro.local");
  await page.click('[data-accion="probar"]');
  await expect(page.locator("[data-aviso-ajustes]")).toContainText("https://");
  // Vacía = mismo origen: Probar consulta /salud y muestra la versión.
  await page.fill('[data-campo="servidor"]', "");
  await page.click('[data-accion="probar"]');
  await expect(page.locator("[data-aviso-ajustes]")).toContainText("responde");
  await page.click('[data-accion="guardar"]');
  await expect(page.locator("[data-aviso-ajustes]")).toContainText("Guardado");
  await page.click('[data-accion="cerrar-ajustes"]');
  await expect(page.locator('[data-estado="aviso"]')).toBeVisible();
});
