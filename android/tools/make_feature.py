#!/usr/bin/env python3
"""Bannière du Play Store (1024 × 500) : le trou noir de l'icône et le nom du jeu.

python3 make_feature.py <hole-512.png> <police Bold .ttf> <sortie.png>
"""
import math
import random
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

HOLE, FONT, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
W, H = 1024, 500


def lerp(a, b, t):
    t = max(0.0, min(1.0, t))
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


img = Image.new("RGB", (W, H))
px = img.load()
for y in range(H):
    for x in range(W):
        d = math.hypot(x - W * 0.3, y - H / 2) / (W * 0.8)
        px[x, y] = lerp((40, 18, 72), (4, 3, 12), d)
rnd = random.Random(5)
d = ImageDraw.Draw(img)
for _ in range(220):
    x, y = rnd.uniform(0, W), rnd.uniform(0, H)
    r = rnd.choice([0.8, 1, 1.5, 2])
    b = rnd.randint(90, 255)
    d.ellipse([x - r, y - r, x + r, y + r], fill=(b, b, min(255, b + 25)))

# Le trou noir, éclairé additivement sur le fond.
hole = Image.open(HOLE).convert("RGB").resize((560, 560), Image.LANCZOS)
from PIL import ImageChops  # noqa: E402

layer = Image.new("RGB", (W, H))
layer.paste(hole, (20, (H - 560) // 2))
img = ImageChops.lighter(img, layer)
# L'horizon reste noir (le mélange de lumière laisserait voir le fond).
cx, cy, r = 20 + 280, H // 2, 560 * 0.15 * 0.97
horizon = Image.new("L", (W, H), 0)
ImageDraw.Draw(horizon).ellipse([cx - r, cy - r, cx + r, cy + r], fill=255)
img.paste((0, 0, 0), (0, 0), horizon.filter(ImageFilter.GaussianBlur(2)))
# Le disque repasse devant l'horizon : on recolle sa bande lumineuse.
band = Image.new("L", (W, H), 0)
ImageDraw.Draw(band).rectangle([cx - r, cy - 2, cx + r, cy + r * 0.32], fill=255)
img.paste(ImageChops.lighter(img, layer), (0, 0), band.filter(ImageFilter.GaussianBlur(3)))


def gradient_text(img, text, size, y, x0):
    font = ImageFont.truetype(FONT, size)
    mask = Image.new("L", (W, H), 0)
    md = ImageDraw.Draw(mask)
    md.text((x0, y), text, font=font, fill=255)
    w = md.textlength(text, font=font)
    halo = mask.filter(ImageFilter.MaxFilter(9)).filter(ImageFilter.GaussianBlur(8))
    img.paste((0, 0, 0), (0, 0), halo)
    grad = Image.new("RGB", (W, H))
    gd = ImageDraw.Draw(grad)
    for x in range(W):
        t = (x - x0) / max(1, w)
        c = lerp((170, 110, 255), (255, 110, 200), t * 2) if t < 0.5 else lerp((255, 110, 200), (255, 210, 90), (t - 0.5) * 2)
        gd.line([(x, 0), (x, H)], fill=c)
    img.paste(grad, (0, 0), mask)


gradient_text(img, "FONDERIE", 84, 150, 520)
gradient_text(img, "STELLAIRE", 74, 245, 520)
font = ImageFont.truetype(FONT, 28)
ImageDraw.Draw(img).text((524, 345), "un idle game cosmique", font=font, fill=(160, 160, 190))
img.save(OUT, optimize=True)
