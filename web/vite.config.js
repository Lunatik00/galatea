import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// En desarrollo (`npm run dev`) la API se reenvía al servidor Rust local.
const API = ["/prendas", "/flujos", "/sesiones", "/salud"];

export default defineConfig({
  plugins: [svelte()],
  build: { outDir: "dist", emptyOutDir: true },
  server: { proxy: Object.fromEntries(API.map(r => [r, "http://127.0.0.1:8766"])) },
  test: { environment: "node", include: ["src/**/*.test.js"] },
});
