#!/usr/bin/env python3
"""Génère l'icône (trou noir + nom du jeu) : python3 make_icon.py <police Bold .ttf> <dossier de sortie>."""
import math, random, sys
from PIL import Image, ImageDraw, ImageFilter, ImageFont

FONT = sys.argv[1]
OUT = sys.argv[2]
S = 1024

def lerp(a, b, t):
    t = max(0.0, min(1.0, t))
    return tuple(int(a[i] + (b[i]-a[i])*t) for i in range(len(a)))

def add(a, b):
    """Mélange additif (lumière)."""
    from PIL import ImageChops
    return ImageChops.add(a, b)

def background():
    img = Image.new("RGB", (S, S))
    px = img.load()
    for y in range(S):
        for x in range(S):
            d = math.hypot(x-S/2, y-S/2) / (S*0.72)
            px[x, y] = lerp((40, 18, 72), (4, 3, 12), d)
    rnd = random.Random(11)
    d = ImageDraw.Draw(img)
    for _ in range(300):
        x, y = rnd.uniform(0, S), rnd.uniform(0, S)
        r = rnd.choice([1, 1, 1.5, 2, 2.5])
        b = rnd.randint(90, 255)
        d.ellipse([x-r, y-r, x+r, y+r], fill=(b, b, min(255, b+25)))
    return img

def glow_ellipse(box, color, width, blur, alpha=255, start=None, end=None):
    layer = Image.new("RGB", (S, S), (0, 0, 0))
    d = ImageDraw.Draw(layer)
    c = tuple(int(v*alpha/255) for v in color)
    if start is None:
        d.ellipse(box, outline=c, width=width)
    else:
        d.arc(box, start, end, fill=c, width=width)
    return layer.filter(ImageFilter.GaussianBlur(blur)) if blur else layer

def black_hole(img):
    cx, cy = S/2, S/2
    R = S*0.15
    light = Image.new("RGB", (S, S), (0, 0, 0))
    # Disque arrière (moitié haute de l'ellipse), très aplati.
    for i in range(60):
        t = i/59
        rx = S*0.47 - t*(S*0.47 - R*1.15)
        ry = rx*0.16
        col = lerp((90, 30, 140), (255, 170, 90), t**1.4)
        light = add(light, glow_ellipse([cx-rx, cy-ry, cx+rx, cy+ry], col, 3, 4, int(70+120*t), 180, 360))
    # Lumière du disque déviée au-dessus et au-dessous de l'horizon.
    for i in range(24):
        t = i/23
        r = R*1.75 - t*R*0.6
        col = lerp((200, 70, 160), (255, 205, 130), t)
        light = add(light, glow_ellipse([cx-r, cy-r*0.92, cx+r, cy+r*0.92], col, 3, 6, int(50+150*t)))
    # Anneau de photons : fin et éclatant.
    light = add(light, glow_ellipse([cx-R*1.06, cy-R*1.06, cx+R*1.06, cy+R*1.06], (255, 240, 210), 6, 3))
    light = add(light, glow_ellipse([cx-R*1.06, cy-R*1.06, cx+R*1.06, cy+R*1.06], (255, 170, 120), 16, 14))
    img = add(img, light)
    # Horizon des événements.
    hole = Image.new("L", (S, S), 0)
    ImageDraw.Draw(hole).ellipse([cx-R, cy-R, cx+R, cy+R], fill=255)
    hole = hole.filter(ImageFilter.GaussianBlur(3))
    img.paste((0, 0, 0), (0, 0), hole)
    # Disque avant (moitié basse), qui passe devant l'horizon.
    front = Image.new("RGB", (S, S), (0, 0, 0))
    for i in range(60):
        t = i/59
        rx = S*0.47 - t*(S*0.47 - R*1.15)
        ry = rx*0.16
        col = lerp((120, 40, 170), (255, 225, 170), t**1.3)
        front = add(front, glow_ellipse([cx-rx, cy-ry, cx+rx, cy+ry], col, 3, 3, int(80+170*t), 0, 180))
    return add(img, front)

def text(img, line, size, y, spacing=0):
    font = ImageFont.truetype(FONT, size)
    mask = Image.new("L", (S, S), 0)
    md = ImageDraw.Draw(mask)
    w = md.textlength(line, font=font)
    md.text(((S-w)/2, y), line, font=font, fill=255, anchor="la")
    # Halo sombre puis texte en dégradé violet → rose → or.
    halo = mask.filter(ImageFilter.MaxFilter(15)).filter(ImageFilter.GaussianBlur(12))
    img.paste((0, 0, 0), (0, 0), halo)
    grad = Image.new("RGB", (S, S))
    gd = ImageDraw.Draw(grad)
    x0, x1 = (S-w)/2, (S+w)/2
    for x in range(S):
        t = (x-x0)/max(1, x1-x0)
        c = lerp((170, 110, 255), (255, 110, 200), t*2) if t < 0.5 else lerp((255, 110, 200), (255, 210, 90), (t-0.5)*2)
        gd.line([(x, 0), (x, S)], fill=c)
    img.paste(grad, (0, 0), mask)
    return img

def compose(with_text=True, transparent=False):
    img = background() if not transparent else Image.new("RGB", (S, S), (0, 0, 0))
    img = black_hole(img)
    if with_text:
        img = text(img, "FONDERIE", 118, S*0.5 - 128)
        img = text(img, "STELLAIRE", 104, S*0.5 + 22)
    return img

full = compose()
full.save(f"{OUT}/icon2-1024.png")
full.resize((512, 512), Image.LANCZOS).save(f"{OUT}/icon2-512.png")
compose(with_text=False).resize((512, 512), Image.LANCZOS).save(f"{OUT}/hole2-512.png")
