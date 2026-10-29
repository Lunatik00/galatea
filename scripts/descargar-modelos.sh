#!/usr/bin/env bash
# Baja los pesos declarados en flujos/flujos.toml a datos/modelos/<destino> y verifica sha256.
# Uso: scripts/descargar-modelos.sh [nombre-de-flujo]   (sin argumento: todos)
# GALATEA_MODELOS cambia la carpeta de destino (por defecto datos/modelos del checkout
# donde está el script). Sirve para bajar desde un worktree a la carpeta que monta el
# contenedor: GALATEA_MODELOS=/ruta/al/checkout/datos/modelos scripts/descargar-modelos.sh fashn
set -euo pipefail
cd "$(dirname "$0")/.."
python3 - "$@" <<'PY'
import hashlib, os, pathlib, subprocess, sys, tomllib
flujos = tomllib.loads(pathlib.Path("flujos/flujos.toml").read_text())["flujo"]
pedidos = set(sys.argv[1:])
base = pathlib.Path(os.environ.get("GALATEA_MODELOS") or "datos/modelos")

def sha256(ruta):
    h = hashlib.sha256()
    with open(ruta, "rb") as f:
        for bloque in iter(lambda: f.read(1 << 20), b""):
            h.update(bloque)
    return h.hexdigest()

for f in flujos:
    if pedidos and f["nombre"] not in pedidos:
        continue
    for p in f.get("peso", []):
        destino = base / p["destino"]
        destino.parent.mkdir(parents=True, exist_ok=True)
        if destino.exists():
            print(f"ya está: {destino}")
        else:
            print(f"bajando {p['url']} → {destino}")
            parcial = destino.with_name(destino.name + ".parcial")
            subprocess.run(["curl", "-L", "--fail", "--progress-bar", "-o", str(parcial), p["url"]], check=True)
            parcial.rename(destino)
        h = sha256(destino)
        if p["sha256"] and h != p["sha256"]:
            sys.exit(f"sha256 distinto para {destino}: esperado {p['sha256']}, obtenido {h}. "
                     f"Borra el archivo y vuelve a correr el script; si se repite, el repositorio "
                     f"de origen cambió y hay que revisar flujos/flujos.toml.")
        print(f"ok: {destino.name} {h[:12]}")
PY
