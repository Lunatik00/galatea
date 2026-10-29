#!/usr/bin/env bash
# Genera el APK de depuración en datos/apk/galatea-debug.apk.
# Pasos: CA de Caddy → recurso del APK (Tarea 7), cliente con la dirección del servidor,
# cap sync, gradle assembleDebug. Requiere `nix develop` (SDK, JDK, Gradle, Node).
set -euo pipefail
cd "$(dirname "$0")/.."
# GALATEA_IP (la IP del host en la red local) vive en .env, fuera de git.
[ -f .env ] && { set -a; . ./.env; set +a; }
: "${ANDROID_HOME:?corre esto dentro de nix develop}"

if [ -z "${GALATEA_SERVIDOR_APK:-}" ]; then
  : "${GALATEA_IP:?define GALATEA_IP en .env (o GALATEA_SERVIDOR_APK) con la dirección del servidor en la red local}"
  GALATEA_SERVIDOR_APK="https://$GALATEA_IP:8444"
fi
echo "== servidor dentro del APK: $GALATEA_SERVIDOR_APK"

CA=datos/caddy/caddy/pki/authorities/local/root.crt
if [ ! -f "$CA" ]; then
  echo "no existe $CA: levanta Caddy una vez (docker compose up -d caddy) para que genere su CA" >&2
  exit 1
fi
mkdir -p web/android/app/src/main/res/raw
cp "$CA" web/android/app/src/main/res/raw/caddy_root.crt
echo "== CA empaquetada: $(openssl x509 -in "$CA" -noout -subject -enddate | tr '\n' ' ')"

( cd web
  if [ ! -f node_modules/.package-lock.json ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then npm ci; fi
  VITE_SERVIDOR="$GALATEA_SERVIDOR_APK" npm run build
  npx cap sync android
  cd android && gradle --no-daemon --quiet assembleDebug )

mkdir -p datos/apk
cp web/android/app/build/outputs/apk/debug/app-debug.apk datos/apk/galatea-debug.apk
echo "== APK listo: datos/apk/galatea-debug.apk"
echo "   instalar por USB:  adb install -r datos/apk/galatea-debug.apk"
