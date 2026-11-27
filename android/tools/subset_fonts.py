#!/usr/bin/env python3
"""Polices de l'APK réduites aux caractères du jeu.

Les polices complètes sont dans android/fonts/ (hors APK) ; ce script écrit
dans app/src/main/assets/fonts/ des copies qui ne gardent que :
  - tous les caractères non ASCII des sources (Rust des crates du jeu, Kotlin) ;
  - pour JetBrains Mono, les plages utiles aux textes venus d'ailleurs
    (pseudos, chemins) : latin accentué, ponctuation, flèches, cadres…
Ce qui manque encore passe par la police monospace du système.
GlyphCoverageTest vérifie que chaque caractère du jeu reste couvert.

Usage (fontTools requis : pip install fonttools) :
    python3 android/tools/subset_fonts.py
"""
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
SRC = ROOT / "android" / "fonts"
OUT = ROOT / "android" / "app" / "src" / "main" / "assets" / "fonts"
SOURCES = [
    *(ROOT / "crates" / c / "src" for c in ("domain", "application", "presentation", "android")),
    ROOT / "android" / "app" / "src" / "main" / "java",
]

# Plages gardées pour JetBrains Mono (en plus des caractères du jeu).
TEXT_RANGES = [
    (0x0020, 0x007E),  # ASCII
    (0x00A0, 0x017F),  # Latin-1 et latin étendu A (œ, ł, ž…)
    (0x2000, 0x206F),  # ponctuation générale (’ … — •)
    (0x20A0, 0x20CF),  # monnaies
    (0x2190, 0x21FF),  # flèches
    (0x2200, 0x22FF),  # opérateurs mathématiques
    (0x2500, 0x25FF),  # cadres, blocs, formes géométriques
]


def used_chars():
    chars = set()
    for base in SOURCES:
        for f in base.rglob("*"):
            if f.suffix in (".rs", ".kt"):
                chars.update(c for c in f.read_text(encoding="utf-8") if ord(c) > 0x7E)
    return chars


def subset(name, unicodes):
    out = OUT / name
    subprocess.run(
        [
            "pyftsubset",
            str(SRC / name),
            "--unicodes=" + ",".join(f"U+{u:04X}" for u in sorted(unicodes)),
            "--layout-features=*",
            "--no-hinting",
            "--desubroutinize",
            "--output-file=" + str(out),
        ],
        check=True,
    )
    print(f"{name}: {(SRC / name).stat().st_size} → {out.stat().st_size} octets")


def main():
    used = {ord(c) for c in used_chars()}
    text = {u for a, b in TEXT_RANGES for u in range(a, b + 1)}
    OUT.mkdir(parents=True, exist_ok=True)
    for name in ("JetBrainsMono-Regular.ttf", "JetBrainsMono-Bold.ttf"):
        subset(name, used | text)
    for name in ("NotoSansSymbols2-Regular.ttf", "NotoSansSymbols-Regular.ttf"):
        subset(name, used)


if __name__ == "__main__":
    sys.exit(main())
