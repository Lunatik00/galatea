#!/usr/bin/env bash
# Emulador Android para ver el APK durante el desarrollo. Crea el AVD en datos/avd la primera
# vez, arranca el emulador (cámara trasera = webcam del computador si existe), instala el APK,
# abre la app, concede el permiso de cámara y, si se pide, guarda una captura.
# Uso: scripts/emulador.sh [ruta-de-captura.png]    (dentro de `nix develop`; necesita /dev/kvm)
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_HOME:?corre esto dentro de nix develop}"
export ANDROID_AVD_HOME="$PWD/datos/avd"
export ANDROID_USER_HOME="$PWD/datos/android"
mkdir -p "$ANDROID_AVD_HOME" "$ANDROID_USER_HOME"
AVD=galatea
IMAGEN="system-images;android-36;google_apis;x86_64"
APK=datos/apk/galatea-debug.apk
PAQUETE=cl.galatea.probador

[ -f "$APK" ] || { echo "falta $APK: corre scripts/apk.sh" >&2; exit 1; }
[ -r /dev/kvm ] || { echo "sin acceso a /dev/kvm: el emulador no arranca" >&2; exit 1; }

if ! avdmanager list avd 2>/dev/null | grep -q "Name: $AVD"; then
  echo "== creando el AVD $AVD"
  # `-d pixel_7` si el perfil existe en esta versión de cmdline-tools; si no, `avdmanager list device` y elige otro.
  echo no | avdmanager create avd -n "$AVD" -k "$IMAGEN" -d pixel_7 >/dev/null
  printf 'hw.gpu.enabled=yes\nhw.gpu.mode=swiftshader_indirect\nhw.keyboard=yes\n' >> "$ANDROID_AVD_HOME/$AVD.avd/config.ini"
fi

CAMARA=none; [ -e /dev/video0 ] && CAMARA=webcam0
if ! adb devices | grep -q '^emulator-.*device$'; then
  echo "== arrancando el emulador (cámara trasera: $CAMARA)"
  "$ANDROID_HOME/emulator/emulator" -avd "$AVD" -no-boot-anim -no-snapshot -camera-back "$CAMARA" ${GALATEA_EMULADOR_FLAGS:-} \
    > datos/android/emulador.log 2>&1 &
  adb wait-for-device
  # Tope de 300 s: si el emulador muere al arrancar, el bucle no espera para siempre.
  limite=$((SECONDS + 300))
  until [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; do
    if [ "$SECONDS" -ge "$limite" ]; then
      echo "el emulador no terminó de arrancar en 300 s; mira datos/android/emulador.log y apágalo con: adb emu kill" >&2
      exit 1
    fi
    sleep 2
  done
fi

echo "== instalando $APK"
adb install -r "$APK" >/dev/null
adb shell pm grant "$PAQUETE" android.permission.CAMERA
adb shell am start -n "$PAQUETE/.MainActivity" >/dev/null
sleep 10   # en el primer arranque el WebView tarda más de 4 s en pasar del splash al aviso
if [ "${1:-}" ]; then adb exec-out screencap -p > "$1"; echo "== captura en $1"; fi
echo "== consola del WebView: chrome://inspect en Chromium del computador (adb ya está conectado)"
echo "== apagar: adb emu kill"
