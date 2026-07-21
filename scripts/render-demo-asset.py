#!/usr/bin/env python3
from __future__ import annotations

import textwrap
from pathlib import Path

import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[1]
ASSET_DIR = ROOT / "docs" / "assets"
GIF_PATH = ASSET_DIR / "plainlink-demo.gif"
MP4_PATH = ASSET_DIR / "plainlink-demo.mp4"
FONT = "/System/Library/Fonts/SFNS.ttf"
MONO_FONT = "/System/Library/Fonts/SFNSMono.ttf"

WIDTH = 960
HEIGHT = 540
FPS = 8
DURATION_SECONDS = 12

DIRTY_URL = "https://example.com/article?id=42&utm_source=newsletter&fbclid=abc"
CLEAN_URL = "https://example.com/article?id=42"


def font(size: int, mono: bool = False) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(MONO_FONT if mono else FONT, size)


TITLE = font(34)
BODY = font(23)
SMALL = font(17)
MONO = font(22, mono=True)
MONO_SMALL = font(17, mono=True)


def ease(value: float) -> float:
    value = max(0.0, min(1.0, value))
    return value * value * (3 - 2 * value)


def draw_wrapped(draw: ImageDraw.ImageDraw, xy: tuple[int, int], text: str, max_chars: int, fill: str) -> None:
    x, y = xy
    for line in textwrap.wrap(text, width=max_chars, break_long_words=True):
        draw.text((x, y), line, font=MONO_SMALL, fill=fill)
        y += 27


def draw_panel(
    draw: ImageDraw.ImageDraw,
    box: tuple[int, int, int, int],
    label: str,
    body: str,
    accent: str,
    alpha: float = 1.0,
) -> None:
    fill = tuple(int(c * alpha + 16 * (1 - alpha)) for c in (250, 248, 239))
    outline = tuple(int(c * alpha + 44 * (1 - alpha)) for c in (215, 222, 214))
    draw.rounded_rectangle(box, radius=18, fill=fill, outline=outline, width=2)
    x1, y1, x2, _ = box
    draw.rounded_rectangle((x1 + 24, y1 + 22, x1 + 88, y1 + 48), radius=13, fill=accent)
    draw.text((x1 + 102, y1 + 22), label, font=BODY, fill="#16201d")
    draw_wrapped(draw, (x1 + 24, y1 + 76), body, max_chars=27, fill="#26342f")
    draw.line((x1 + 24, y1 + 158, x2 - 24, y1 + 158), fill="#d3ddd4", width=1)


def draw_frame(index: int) -> Image.Image:
    t = index / FPS
    image = Image.new("RGB", (WIDTH, HEIGHT), "#0d1513")
    draw = ImageDraw.Draw(image)

    draw.rounded_rectangle((46, 36, WIDTH - 46, HEIGHT - 34), radius=28, fill="#f5f2e9")
    draw.rounded_rectangle((46, 36, WIDTH - 46, 88), radius=28, fill="#1e2c29")
    draw.rectangle((46, 62, WIDTH - 46, 88), fill="#1e2c29")
    draw.ellipse((70, 56, 84, 70), fill="#ff6b5f")
    draw.ellipse((94, 56, 108, 70), fill="#ffc857")
    draw.ellipse((118, 56, 132, 70), fill="#61d394")
    draw.text((160, 52), "PlainLink demo", font=SMALL, fill="#d9efe8")
    draw.text((WIDTH - 244, 52), "local clipboard cleaner", font=SMALL, fill="#9dbcb2")

    draw.text((78, 120), "Clean copied links before you share them.", font=TITLE, fill="#10201b")
    draw.text((80, 164), "Copy any link. PlainLink removes known tracking parameters before paste.", font=SMALL, fill="#54655f")

    copy_progress = ease((t - 0.5) / 2.5)
    clean_progress = ease((t - 6.2) / 2.0)

    dirty_visible = DIRTY_URL[: max(1, int(len(DIRTY_URL) * copy_progress))]
    clean_visible = CLEAN_URL[: max(0, int(len(CLEAN_URL) * clean_progress))]

    draw_panel(draw, (78, 216, 416, 410), "Copy", dirty_visible, "#c6f068")
    draw_panel(draw, (544, 216, 882, 410), "Paste", clean_visible, "#5bd7c8", alpha=0.74 + 0.26 * clean_progress)

    arrow_y = 314
    arrow_alpha = ease((t - 3.2) / 2.0)
    arrow_color = tuple(int(c * arrow_alpha + 190 * (1 - arrow_alpha)) for c in (21, 78, 68))
    draw.line((438, arrow_y, 520, arrow_y), fill=arrow_color, width=7)
    draw.polygon((520, arrow_y, 498, arrow_y - 14, 498, arrow_y + 14), fill=arrow_color)

    if t >= 3.8:
        badge_alpha = ease((t - 3.8) / 1.4)
        badge_fill = tuple(int(c * badge_alpha + 245 * (1 - badge_alpha)) for c in (30, 44, 41))
        draw.rounded_rectangle((312, 438, 648, 482), radius=22, fill=badge_fill)
        draw.text((334, 449), "Removed: utm_source, fbclid", font=SMALL, fill="#d8f5e7")

    if t >= 8.4:
        draw.rounded_rectangle((302, 492, 658, 518), radius=13, fill="#e1f6ec")
        draw.text((326, 497), "Unknown parameters are preserved.", font=SMALL, fill="#1b4d42")

    return image


def main() -> None:
    ASSET_DIR.mkdir(parents=True, exist_ok=True)
    frames = [draw_frame(index) for index in range(FPS * DURATION_SECONDS)]

    frames[0].save(
        GIF_PATH,
        save_all=True,
        append_images=frames[1:],
        duration=int(1000 / FPS),
        loop=0,
        optimize=True,
    )

    writer = cv2.VideoWriter(
        str(MP4_PATH),
        cv2.VideoWriter_fourcc(*"mp4v"),
        FPS,
        (WIDTH, HEIGHT),
    )
    for frame in frames:
        writer.write(cv2.cvtColor(np.asarray(frame), cv2.COLOR_RGB2BGR))
    writer.release()

    print(f"Wrote {GIF_PATH}")
    print(f"Wrote {MP4_PATH}")


if __name__ == "__main__":
    main()
