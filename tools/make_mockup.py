#!/usr/bin/env python3
"""Рисует макет интерфейса WorldGod для README.

Это именно макет (иллюстрация компоновки и палитры), а не скриншот: настоящие
скриншоты делает `cargo run -p worldgod_preview`, который использует тот же
рендер, что и игра.

Запуск:
    python3 tools/make_mockup.py [docs/mockup-ui.png]

Требуется Pillow:  pip install pillow
"""

from __future__ import annotations

import math
import random
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONT_PATH = ROOT / "crates/worldgod_app/assets/fonts/DejaVuSans.ttf"

# --- палитра (совпадает с crates/worldgod_app/src/theme.rs) -----------------
BACKGROUND = (10, 16, 24)
SURFACE = (21, 27, 35)
SURFACE_ALT = (31, 39, 49)
ACCENT = (96, 172, 255)
ACCENT_DARK = (26, 66, 106)
TEXT = (232, 236, 242)
TEXT_DIM = (148, 160, 176)
SUCCESS = (124, 210, 134)
DANGER = (255, 112, 100)
WARNING = (240, 190, 92)

# --- палитра мира (совпадает с crates/worldgod_render/src/palette.rs) -------
DEEP_WATER = (18, 42, 74)
OCEAN = (26, 60, 100)
SHALLOW = (42, 92, 132)
FOAM = (198, 226, 238)
BEACH = (226, 208, 152)
BEACH_LIGHT = (238, 222, 174)
GRASS = (86, 146, 78)
GRASS_DARK = (68, 124, 64)
FOREST = (48, 104, 60)
FOREST_DARK = (38, 86, 50)
TREE = (34, 78, 46)
TREE_LIGHT = (62, 116, 62)
HILLS = (122, 122, 106)
MOUNTAIN = (140, 142, 148)
SNOW = (238, 242, 246)
WOOD = (138, 100, 66)
WOOD_DARK = (96, 68, 44)
ROOF = (176, 84, 72)
SKIN = (232, 190, 152)
WOLF = (96, 100, 108)
ZOMBIE = (128, 158, 110)
EMBER = (255, 168, 72)
FLAME = (255, 214, 110)

SCALE = 2  # рисуем в 2× и сохраняем как есть (ретина)


def s(value: float) -> int:
    """Логические пиксели → пиксели макета."""
    return int(round(value * SCALE))


def load_font(size: float) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(str(FONT_PATH), s(size))


class Canvas:
    def __init__(self, width: int, height: int) -> None:
        self.image = Image.new("RGB", (s(width), s(height)), BACKGROUND)
        self.draw = ImageDraw.Draw(self.image)

    def rect(self, x, y, w, h, color, radius=0.0):
        if radius <= 0:
            self.draw.rectangle([s(x), s(y), s(x + w) - 1, s(y + h) - 1], fill=color)
        else:
            self.draw.rounded_rectangle(
                [s(x), s(y), s(x + w) - 1, s(y + h) - 1], radius=s(radius), fill=color
            )

    def pixel(self, x, y, size, color):
        self.draw.rectangle([x, y, x + size - 1, y + size - 1], fill=color)

    def rect_px(self, x, y, w, h, color):
        """Прямоугольник в пикселях макета (не в логических)."""
        self.draw.rectangle([x, y, x + w - 1, y + h - 1], fill=color)

    def text(self, x, y, value, size, color, anchor="lt"):
        self.draw.text((s(x), s(y)), value, font=load_font(size), fill=color, anchor=anchor)

    def textlen(self, value: str, size: float) -> float:
        """Ширина строки в логических пикселях."""
        return self.draw.textlength(value, font=load_font(size)) / SCALE


# --- генерация мира ---------------------------------------------------------
WATER_LEVEL = 320
MAX_ELEVATION = 1000


def elevation_at(nx: float, ny: float, rng: random.Random) -> int:
    """Псевдоостров: несколько лопастей + небольшой горный хребет на севере."""
    lobes = max(
        math.exp(-((nx / 1.10) ** 2 + (ny / 0.86) ** 2) * 1.7),
        0.92 * math.exp(-(((nx - 0.44) / 0.52) ** 2 + ((ny - 0.30) / 0.62) ** 2) * 2.2),
        0.88 * math.exp(-(((nx + 0.48) / 0.46) ** 2 + ((ny + 0.42) / 0.54) ** 2) * 2.3),
        0.76 * math.exp(-(((nx - 0.08) / 0.55) ** 2 + ((ny + 0.56) / 0.42) ** 2) * 2.6),
    )
    detail = (
        0.10 * math.sin(nx * 5.1 + 1.3)
        + 0.08 * math.sin(ny * 6.7 - 0.8)
        + 0.06 * math.sin((nx + ny) * 9.4 + 2.1)
        + 0.05 * math.sin((nx - ny) * 13.2)
    )
    base = lobes * 470 + detail * lobes * 260
    ridge = math.exp(-(((nx + 0.10) ** 2) / 0.012 + ((ny + 0.30) ** 2) / 0.018))
    peak = ridge * 330 * lobes
    value = base + peak + rng.uniform(-6.0, 6.0)
    return int(max(0, min(MAX_ELEVATION, value)))


def terrain_of(elevation: int, moisture: float) -> str:
    if elevation < WATER_LEVEL - 90:
        return "deep"
    if elevation < WATER_LEVEL - 20:
        return "ocean"
    if elevation < WATER_LEVEL:
        return "shallow"
    if elevation < WATER_LEVEL + 55:
        return "beach"
    if elevation < 560:
        return "forest" if moisture > 0.58 else "grass"
    if elevation < 700:
        return "hills"
    return "snow" if elevation > 880 else "mountain"


def moisture_at(nx: float, ny: float) -> float:
    return 0.5 + 0.32 * math.sin(nx * 3.7 + 0.4) + 0.2 * math.cos(ny * 4.9 - 1.1)


def draw_world(canvas: Canvas, left, top, width, height, tile_logical=4.0):
    rng = random.Random(7)
    tile = s(tile_logical)
    cols = int(s(width) // tile)
    rows = int(s(height) // tile)
    colors = {
        "deep": DEEP_WATER,
        "ocean": OCEAN,
        "shallow": SHALLOW,
        "beach": BEACH,
        "grass": GRASS,
        "forest": FOREST,
        "hills": HILLS,
        "mountain": MOUNTAIN,
        "snow": SNOW,
    }

    grid = []
    for row in range(rows):
        line = []
        for col in range(cols):
            nx = ((col + 0.5) / cols - 0.5) * 2.0 * 0.82
            ny = ((row + 0.5) / rows - 0.40) * 2.0 * 0.82
            elevation = elevation_at(nx, ny, rng)
            line.append((terrain_of(elevation, moisture_at(nx, ny)), elevation))
        grid.append(line)

    origin_x, origin_y = s(left), s(top)

    for row in range(rows):
        for col in range(cols):
            terrain, elevation = grid[row][col]
            color = colors[terrain]
            if terrain in ("deep", "ocean", "shallow"):
                # «Волны»: редкие светлые пиксели, как в рендере.
                sparkle = math.sin(col * 12.9898 + row * 78.233) * 43758.5453
                if sparkle - math.floor(sparkle) > 0.94:
                    color = tuple(min(255, c + 18) for c in color)
            canvas.pixel(origin_x + col * tile, origin_y + row * tile, tile, color)

            # Пена у берега.
            if terrain in ("deep", "ocean", "shallow"):
                near_land = False
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    nc, nr = col + dx, row + dy
                    if 0 <= nc < cols and 0 <= nr < rows:
                        if grid[nr][nc][0] in ("beach", "grass", "forest"):
                            near_land = True
                if near_land:
                    canvas.pixel(origin_x + col * tile, origin_y + row * tile, max(2, tile // 3), FOAM)

    # Детали: деревья, камни, травинки.
    for row in range(rows):
        for col in range(cols):
            terrain, _elevation = grid[row][col]
            x = origin_x + col * tile
            y = origin_y + row * tile
            if terrain == "forest" and (col * 7 + row * 3) % 5 == 0:
                canvas.pixel(x + tile // 3, y + tile // 3, max(2, tile // 2), TREE)
                canvas.pixel(x + tile // 3, y + tile // 3 - 1, max(1, tile // 3), TREE_LIGHT)
            elif terrain == "grass" and (col * 5 + row) % 11 == 0:
                canvas.pixel(x + tile // 2, y + tile // 2, max(1, tile // 3), GRASS_DARK)
            elif terrain == "beach" and (col + row * 3) % 9 == 0:
                canvas.pixel(x + tile // 3, y + tile // 2, max(1, tile // 3), BEACH_LIGHT)
            elif terrain in ("mountain", "snow", "hills") and (col * 3 + row * 5) % 7 == 0:
                canvas.pixel(x + tile // 2, y + tile // 3, max(1, tile // 3), SNOW if terrain == "snow" else MOUNTAIN)

    return grid, origin_x, origin_y, tile, cols, rows


def draw_territory(canvas: Canvas, grid, origin_x, origin_y, tile, cols, rows, center, radius, color):
    """Полупрозрачная подсветка владений вокруг столицы (круг, как в ядре)."""
    overlay = Image.new("RGBA", canvas.image.size, (0, 0, 0, 0))
    pen = ImageDraw.Draw(overlay)
    border = tuple(min(255, int(c * 1.3)) for c in color)

    def inside(col: int, row: int) -> bool:
        return math.hypot(col - center[0], row - center[1]) <= radius

    def land(col: int, row: int) -> bool:
        if not (0 <= col < cols and 0 <= row < rows):
            return False
        return grid[row][col][0] in ("grass", "forest", "beach", "hills", "mountain", "snow")

    for row in range(rows):
        for col in range(cols):
            if not inside(col, row) or not land(col, row):
                continue
            x = origin_x + col * tile
            y = origin_y + row * tile
            pen.rectangle([x, y, x + tile - 1, y + tile - 1], fill=color + (64,))
            edge = (
                not inside(col - 1, row)
                or not inside(col + 1, row)
                or not inside(col, row - 1)
                or not inside(col, row + 1)
                or not land(col - 1, row)
                or not land(col + 1, row)
                or not land(col, row - 1)
                or not land(col, row + 1)
            )
            if edge:
                pen.rectangle(
                    [x, y, x + tile - 1, y + tile - 1],
                    outline=border + (210,),
                    width=max(1, tile // 4),
                )
    merged = Image.alpha_composite(canvas.image.convert("RGBA"), overlay).convert("RGB")
    canvas.image.paste(merged, (0, 0))


def draw_person(canvas: Canvas, x, y, tile, cloth, head=SKIN):
    unit = max(2, tile // 2)
    canvas.pixel(x + unit, y, unit, head)
    canvas.pixel(x, y + unit, unit * 3, cloth)
    canvas.pixel(x + unit, y + unit * 2, unit, cloth)
    canvas.pixel(x, y + unit * 3, unit, cloth)
    canvas.pixel(x + unit * 2, y + unit * 3, unit, cloth)


def draw_house(canvas: Canvas, x, y, tile):
    unit = max(2, tile // 2)
    canvas.rect_px(x, y + unit, unit * 5, unit, ROOF)
    canvas.rect_px(x + unit, y, unit * 3, unit, ROOF)
    canvas.rect_px(x, y + unit * 2, unit * 5, unit * 3, WOOD)
    canvas.rect_px(x + unit * 2, y + unit * 3, unit, unit * 2, WOOD_DARK)


def draw_fire(canvas: Canvas, x, y, tile):
    unit = max(2, tile // 2)
    canvas.pixel(x + unit, y, unit, FLAME)
    canvas.rect_px(x, y + unit, unit * 3, unit, EMBER)
    canvas.rect_px(x, y + unit * 2, unit * 3, max(1, unit // 2), WOOD_DARK)


def draw_wolf(canvas: Canvas, x, y, tile):
    unit = max(2, tile // 2)
    canvas.rect_px(x, y + unit, unit * 3, unit, WOLF)
    canvas.rect_px(x + unit * 2, y, unit, unit, WOLF)
    canvas.rect_px(x, y + unit * 2, unit, unit, WOLF)
    canvas.rect_px(x + unit * 2, y + unit * 2, unit, unit, WOLF)


# --- элементы интерфейса ----------------------------------------------------
def draw_chip(canvas: Canvas, x, y, label, value, color) -> float:
    value_w = canvas.textlen(value, 12.5)
    label_w = canvas.textlen(label, 10.5)
    width = 7 + 7 + 4 + value_w + 4 + label_w + 7
    canvas.rect(x, y, width, 26, SURFACE_ALT, radius=8)
    canvas.draw.ellipse([s(x + 7), s(y + 9.5), s(x + 14), s(y + 16.5)], fill=color)
    canvas.text(x + 18, y + 6, value, 12.5, TEXT)
    canvas.text(x + 18 + value_w + 4, y + 8, label, 10.5, TEXT_DIM)
    return width


def draw_icon_button(canvas: Canvas, x, y, size=34) -> None:
    canvas.rect(x, y, size, size, SURFACE_ALT, radius=10)


def draw_play(canvas: Canvas, x, y, size=34, color=ACCENT) -> None:
    cx, cy = x + size / 2, y + size / 2
    canvas.draw.polygon(
        [ (s(cx - 4), s(cy - 7)), (s(cx + 7), s(cy)), (s(cx - 4), s(cy + 7)) ], fill=color
    )


def draw_menu(canvas: Canvas, x, y, size=34, color=TEXT) -> None:
    for offset in (-6, 0, 6):
        canvas.rect(x + size / 2 - 7, y + size / 2 + offset - 1, 14, 2, color, radius=1)


def draw_tool(canvas: Canvas, x, y, label, icon, selected=False) -> None:
    canvas.rect(x, y, 56, 66, ACCENT_DARK if selected else SURFACE_ALT, radius=12)
    color = ACCENT if selected else TEXT_DIM
    cx, cy = x + 28, y + 27
    if icon == "raise":
        canvas.draw.polygon([(s(cx - 11), s(cy + 9)), (s(cx - 3), s(cy - 8)), (s(cx + 6), s(cy + 9))], fill=color)
    elif icon == "lower":
        for k in (-7, 0, 7):
            canvas.draw.line([(s(cx - 10), s(cy + k)), (s(cx + 10), s(cy + k - 3))], fill=color, width=s(2))
    elif icon == "forest":
        canvas.rect(cx - 1.5, cy + 3, 3, 7, color)
        canvas.draw.polygon([(s(cx), s(cy - 11)), (s(cx + 10), s(cy + 2)), (s(cx - 10), s(cy + 2))], fill=color)
    elif icon == "sand":
        canvas.draw.line(
            [(s(cx - 11), s(cy + 6)), (s(cx - 3), s(cy - 2)), (s(cx + 3), s(cy + 5)), (s(cx + 11), s(cy - 4))],
            fill=color,
            width=s(3),
        )
    elif icon == "mountain":
        canvas.draw.polygon([(s(cx - 12), s(cy + 9)), (s(cx - 3), s(cy - 10)), (s(cx + 7), s(cy + 9))], fill=color)
        canvas.draw.polygon([(s(cx + 2), s(cy + 9)), (s(cx + 8), s(cy - 2)), (s(cx + 14), s(cy + 9))], fill=color)
    elif icon == "human":
        canvas.draw.ellipse([s(cx - 4), s(cy - 12), s(cx + 4), s(cy - 4)], fill=color)
        canvas.draw.polygon([(s(cx - 7), s(cy + 10)), (s(cx), s(cy - 2)), (s(cx + 7), s(cy + 10))], fill=color)
    elif icon == "soldier":
        canvas.draw.ellipse([s(cx - 5), s(cy - 12), s(cx + 3), s(cy - 4)], fill=color)
        canvas.draw.polygon([(s(cx - 7), s(cy + 10)), (s(cx - 1), s(cy - 1)), (s(cx + 5), s(cy + 10))], fill=color)
        canvas.draw.line([(s(cx + 1), s(cy + 6)), (s(cx + 12), s(cy - 10))], fill=color, width=s(2))
    elif icon == "deer":
        canvas.draw.polygon([(s(cx - 7), s(cy + 8)), (s(cx), s(cy - 4)), (s(cx + 8), s(cy + 8))], fill=color)
        canvas.draw.line([(s(cx - 2), s(cy - 3)), (s(cx - 7), s(cy - 11))], fill=color, width=s(2))
        canvas.draw.line([(s(cx + 2), s(cy - 3)), (s(cx + 7), s(cy - 11))], fill=color, width=s(2))
    elif icon == "wolf":
        canvas.draw.polygon(
            [
                (s(cx - 11), s(cy + 4)),
                (s(cx - 8), s(cy - 8)),
                (s(cx - 2), s(cy - 1)),
                (s(cx + 3), s(cy - 1)),
                (s(cx + 9), s(cy - 8)),
                (s(cx + 12), s(cy + 4)),
                (s(cx), s(cy + 10)),
            ],
            fill=color,
        )
    elif icon == "zombie":
        canvas.draw.ellipse([s(cx - 4), s(cy - 11), (s(cx + 4)), s(cy - 3)], fill=color)
        canvas.draw.polygon([(s(cx - 6), s(cy + 10)), (s(cx), s(cy - 1)), (s(cx + 6), s(cy + 10))], fill=color)
        canvas.draw.line([(s(cx - 4), s(cy)), (s(cx - 11), s(cy + 6))], fill=color, width=s(2))
        canvas.draw.line([(s(cx + 4), s(cy)), (s(cx + 11), s(cy + 6))], fill=color, width=s(2))
    elif icon == "inspect":
        canvas.draw.ellipse([s(cx - 9), s(cy - 11), s(cx + 3), s(cy + 1)], outline=color, width=s(2))
        canvas.draw.line([(s(cx + 1), s(cy + 2)), (s(cx + 10), s(cy + 11))], fill=color, width=s(3))

    canvas.text(x + 28, y + 52, label, 10.5, ACCENT if selected else TEXT_DIM, anchor="mt")


def draw_hud(canvas: Canvas, width: float, height: float) -> None:
    # Верхняя панель.
    canvas.rect(0, 0, width, 86, SURFACE)
    x = 8
    for label, value, color in (
        ("люди", "12", SUCCESS),
        ("страны", "1", ACCENT),
        ("монстры", "3", DANGER),
        ("суша", "21 %", WARNING),
    ):
        x += draw_chip(canvas, x, 6, label, value, color) + 5

    draw_icon_button(canvas, width - 42, 46)
    draw_menu(canvas, width - 42, 46)
    draw_icon_button(canvas, width - 82, 46)
    draw_play(canvas, width - 82, 46)
    canvas.text(width - 90, 56, "02:14", 11.5, TEXT_DIM, anchor="rt")

    canvas.text(10, 62, "Велдор: построен новый дом", 11, TEXT_DIM)

    # Карточка существа.
    card_top = height - 196
    canvas.rect(0, card_top, width, 72, SURFACE)
    canvas.text(10, card_top + 8, "Человек", 14.5, TEXT)
    canvas.text(10 + canvas.textlen("Человек", 14.5) + 8, card_top + 11, "Велдор", 11.5, (232, 168, 74))
    canvas.text(width - 10, card_top + 12, "Работа · убийств: 0", 10.5, TEXT_DIM, anchor="rt")
    canvas.text(10, card_top + 30, "Жизнь  82 / 100", 11, TEXT)
    canvas.text(width - 10, card_top + 30, "Сытость 61 %", 11, TEXT_DIM, anchor="rt")
    canvas.rect(10, card_top + 50, width - 20, 6, SURFACE_ALT, radius=3)
    canvas.rect(10, card_top + 50, (width - 20) * 0.82, 6, SUCCESS, radius=3)

    # Лоток инструментов.
    tray_top = height - 120
    canvas.rect(0, tray_top, width, 120, SURFACE)
    tools = [
        ("Суша", "raise", True),
        ("Вода", "lower", False),
        ("Лес", "forest", False),
        ("Песок", "sand", False),
        ("Горы", "mountain", False),
        ("Человек", "human", False),
        ("Воин", "soldier", False),
    ]
    x = 6
    for label, icon, selected in tools:
        draw_tool(canvas, x, tray_top + 6, label, icon, selected)
        x += 62
    canvas.text(10, tray_top + 80, "Радиус", 11.5, TEXT_DIM)
    canvas.draw.rectangle([s(58), s(tray_top + 86), s(188), s(tray_top + 90)], fill=SURFACE_ALT)
    canvas.draw.rectangle([s(58), s(tray_top + 86), s(58 + 50), s(tray_top + 90)], fill=ACCENT)
    canvas.draw.ellipse([s(104), s(tray_top + 81), s(112), s(tray_top + 95)], fill=ACCENT)
    canvas.text(196, tray_top + 80, "Суша · радиус 4", 11, TEXT_DIM)

    # Подпись «это макет» — поверх нижнего края карты.
    canvas.text(width - 8, card_top - 6, "макет интерфейса", 9.5, (120, 132, 148), anchor="rb")


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "docs/mockup-ui.png"
    width, height = 360, 640
    canvas = Canvas(width, height)

    map_top, map_height = 86.0, height - 86 - 196
    grid, origin_x, origin_y, tile, cols, rows = draw_world(canvas, 0, map_top, width, map_height)

    # Столица и дома на острове.
    center = (cols // 2, rows // 2 + 2)
    draw_territory(canvas, grid, origin_x, origin_y, tile, cols, rows, center, 9, (232, 168, 74))

    capital_x = origin_x + center[0] * tile
    capital_y = origin_y + center[1] * tile
    draw_fire(canvas, capital_x, capital_y, tile)
    for dx, dy in ((4, 1), (-3, 2), (2, -3), (-4, -2)):
        draw_house(canvas, origin_x + (center[0] + dx) * tile, origin_y + (center[1] + dy) * tile, tile)
    for dx, dy in ((1, 3), (-2, -1), (5, -2), (-5, 3), (3, 4)):
        draw_person(canvas, origin_x + (center[0] + dx) * tile, origin_y + (center[1] + dy) * tile, tile, (232, 168, 74))
    for dx, dy in ((11, 6), (-10, -7)):
        draw_wolf(canvas, origin_x + (center[0] + dx) * tile, origin_y + (center[1] + dy) * tile, tile)

    draw_hud(canvas, width, height)

    out.parent.mkdir(parents=True, exist_ok=True)
    canvas.image.save(out)
    print(f"готово: {out} ({canvas.image.width}×{canvas.image.height})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
