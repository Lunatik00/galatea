import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests/e2e",
  timeout: 60_000,
  use: { baseURL: "http://127.0.0.1:8767", viewport: { width: 390, height: 844 } },
  webServer: {
    command: "sh -c 'scripts/web.sh && cargo run --quiet'",
    url: "http://127.0.0.1:8767/salud",
    reuseExistingServer: false,
    timeout: 300_000,
    env: {
      GALATEA_PUERTO: "8767",
      GALATEA_PRENDAS: "pruebas/prendas",
      GALATEA_FLUJOS: "pruebas/flujos",
      GALATEA_DATOS: "target/e2e-datos",
      GALATEA_MODELOS: "target/e2e-datos/modelos",
      GALATEA_WEB: "web/dist",
      RUST_LOG: "warn",
    },
  },
});
