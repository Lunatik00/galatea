#!/usr/bin/env python3
"""Ícono provisional de la demo: luna creciente sobre fondo oscuro, en todas las densidades.

Escribe web/public/icono.png (512) y los mipmap de Android (ic_launcher, ic_launcher_round,
ic_launcher_foreground). Colores de estilo.css: fondo #1b1b2f, acento #c9a7ff.
"""
from pathlib import Path
from PIL import Image, ImageDraw

RAIZ = Path(__file__).resolve().parent.parent
FONDO, ACENTO = (0x1B, 0x1B, 0x2F, 255), (0xC9, 0xA7, 0xFF, 255)
DENSIDADES = {"mdpi": 1, "hdpi": 1.5, "xhdpi": 2, "xxhdpi": 3, "xxxhdpi": 4}


def luna(tamano, con_fondo, margen):
    """Dibuja a 4x y reduce para suavizar bordes. `margen` es la fracción libre alrededor."""
    s = tamano * 4
    img = Image.new("RGBA", (s, s), FONDO if con_fondo else (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    r = s * (0.5 - margen)
    cx = cy = s / 2
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=ACENTO)
    # La sombra que forma la creciente: un círculo del color del fondo desplazado.
    dx = r * 0.45
    d.ellipse((cx - r + dx, cy - r - r * 0.1, cx + r + dx, cy + r - r * 0.1), fill=FONDO if con_fondo else (0, 0, 0, 0))
    return img.resize((tamano, tamano), Image.LANCZOS)


def main():
    publico = RAIZ / "web" / "public"
    publico.mkdir(parents=True, exist_ok=True)
    luna(512, True, 0.12).save(publico / "icono.png")
    res = RAIZ / "web" / "android" / "app" / "src" / "main" / "res"
    for nombre, factor in DENSIDADES.items():
        carpeta = res / f"mipmap-{nombre}"
        carpeta.mkdir(parents=True, exist_ok=True)
        luna(int(48 * factor), True, 0.12).save(carpeta / "ic_launcher.png")
        luna(int(48 * factor), True, 0.12).save(carpeta / "ic_launcher_round.png")
        # Ícono adaptativo: 108 dp con la figura dentro del círculo seguro central (66 dp).
        luna(int(108 * factor), False, 0.30).save(carpeta / "ic_launcher_foreground.png")
    print("ícono escrito en", publico / "icono.png", "y", res / "mipmap-*")


if __name__ == "__main__":
    main()
