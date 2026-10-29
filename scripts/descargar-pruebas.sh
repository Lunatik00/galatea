#!/usr/bin/env bash
# Baja las imágenes Creative Commons de la comprobación de flujo. Atribución en
# pruebas/imagenes/LICENCIAS.md. No son fotos de nadie del entorno del proyecto.
set -euo pipefail
cd "$(dirname "$0")/.."
destino=pruebas/imagenes
mkdir -p "$destino"
# Wikimedia pide un User-Agent con contacto: GALATEA_CONTACTO="tu@correo".
ua="galatea-dev/0.1 (contacto: ${GALATEA_CONTACTO:-sin contacto})"
bajar() { # nombre url
  [ -s "$destino/$1" ] && { echo "ya está: $1"; return; }
  curl -fsSL -A "$ua" -o "$destino/$1" "$2" && echo "bajado: $1"
}
bajar prenda-polera-cc.jpg "https://commons.wikimedia.org/w/index.php?title=Special:FilePath/T-shirt_(AM_2002.94.1.1-1).jpg&width=1024"
bajar persona-cc.jpg "https://commons.wikimedia.org/w/index.php?title=Special:FilePath/Gordon_Cooper_posing_in_suit.jpg&width=1024"
file "$destino"/*.jpg
