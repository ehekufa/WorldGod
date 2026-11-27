#!/usr/bin/env python3
"""Генерирует иконки приложения для Android (res/mipmap-*).

Рисуем иконку в большом масштабе и уменьшаем с Lanczos — так края остаются
аккуратными. Скрипт использует только Pillow.

    pip install pillow
    python3 tools/make_icons.py
"""

from __future__ import annotations

import math
import os
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RES = os.path.join(ROOT, "crates", "worldgod_app", "res")

SIZES = {
    "mipmap-mdpi": 48,
    "mipmap-hdpi": 72,
    "mipmap-xhdpi": 96,
    "mipmap-xxhdpi": 144,
    "mipmap-xxxhdpi": 192,
}

BASE = 512  # рисуем в 512×512, потом уменьшаем


def rounded_mask(size: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=255)
    return mask


def lerp(a: tuple[int, int, int], b: tuple[int, int, int], t: float) -> tuple[int, int, int]:
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))  # type: ignore[return-value]


def draw_icon() -> Image.Image:
    size = BASE
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    # Океан: вертикальный градиент.
    top = (10, 30, 48)
    bottom = (16, 58, 92)
    for y in range(size):
        color = lerp(top, bottom, y / (size - 1))
        draw.line([(0, y), (size, y)], fill=color + (255,))

    # Волны.
    for index, (offset, alpha) in enumerate(((0.34, 46), (0.46, 34), (0.58, 26))):
        y = int(size * offset)
        amplitude = 9 - index * 2
        points = []
        for x in range(0, size + 8, 8):
            wave = math.sin(x / 46 + index * 1.7) * amplitude
            points.append((x, y + wave))
        draw.line(points, fill=(150, 220, 228, alpha), width=5 + index * 2)

    # Остров: песчаная кайма и зелень.
    cx, cy = size * 0.5, size * 0.60
    draw.ellipse(
        (cx - 150, cy - 96, cx + 150, cy + 96),
        fill=(214, 196, 138, 255),
    )
    draw.ellipse(
        (cx - 132, cy - 82, cx + 132, cy + 82),
        fill=(92, 148, 78, 255),
    )
    # Холмы.
    draw.ellipse((cx - 96, cy - 46, cx + 12, cy + 52), fill=(74, 124, 64, 255))
    draw.ellipse((cx + 8, cy - 30, cx + 108, cy + 58), fill=(104, 162, 86, 255))

    # Деревья.
    for tx, ty, r in ((cx - 74, cy - 6, 20), (cx - 34, cy + 18, 16), (cx + 62, cy + 6, 18)):
        draw.rectangle((tx - 4, ty + r - 6, tx + 4, ty + r + 14), fill=(98, 70, 44, 255))
        draw.ellipse((tx - r, ty - r, tx + r, ty + r), fill=(38, 84, 52, 255))
        draw.ellipse((tx - r + 5, ty - r + 3, tx + r - 8, ty + r - 9), fill=(70, 130, 72, 255))

    # Дом с красной крышей.
    hx, hy = cx + 6, cy + 34
    draw.rectangle((hx - 26, hy - 10, hx + 26, hy + 30), fill=(176, 138, 92, 255))
    draw.polygon(
        [(hx - 34, hy - 10), (hx + 34, hy - 10), (hx, hy - 44)],
        fill=(166, 84, 64, 255),
    )
    draw.rectangle((hx - 7, hy + 6, hx + 7, hy + 30), fill=(96, 68, 42, 255))

    # Человечек.
    px, py = cx - 44, cy + 40
    draw.ellipse((px - 7, py - 30, px + 7, py - 16), fill=(238, 204, 162, 255))
    draw.rectangle((px - 9, py - 15, px + 9, py + 8), fill=(214, 96, 77, 255))
    draw.rectangle((px - 8, py + 8, px - 2, py + 24), fill=(48, 54, 66, 255))
    draw.rectangle((px + 2, py + 8, px + 8, py + 24), fill=(48, 54, 66, 255))

    # Мягкая виньетка сверху, чтобы иконка не сливалась с фоном.
    image.putalpha(rounded_mask(size, int(size * 0.22)))
    return image


def main() -> None:
    icon = draw_icon()
    for folder, size in SIZES.items():
        target = os.path.join(RES, folder)
        os.makedirs(target, exist_ok=True)
        resized = icon.resize((size, size), Image.LANCZOS)
        path = os.path.join(target, "ic_launcher.png")
        resized.save(path, "PNG")
        print("saved", os.path.relpath(path, ROOT))


if __name__ == "__main__":
    main()
