#!/usr/bin/env bash
# Compila el cliente Svelte en web/dist. Lo sirve el servidor Rust (GALATEA_WEB) y lo
# empaqueta Capacitor. Hace `npm ci` solo si node_modules falta o el lock cambió.
set -euo pipefail
cd "$(dirname "$0")/../web"
if [ ! -f node_modules/.package-lock.json ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
  npm ci
fi
npm run build
