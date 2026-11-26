"""Vérifie qu'un APK est compatible avec les pages mémoire de 16 Ko (Android 15+,
exigé par le Play Store) : chaque bibliothèque native a des segments LOAD
alignés sur au moins 16 Ko, et est stockée non compressée à un décalage
multiple de 16 Ko dans l'archive.

Usage : python check_16k.py app-release.apk
"""

import struct
import sys
import zipfile

PAGE = 16 * 1024


def load_alignment(elf: bytes) -> int:
    """Plus petit alignement des segments PT_LOAD d'un ELF 64 bits."""
    if elf[:4] != b"\x7fELF" or elf[4] != 2:
        raise ValueError("pas un ELF 64 bits")
    phoff = struct.unpack_from("<Q", elf, 0x20)[0]
    entsize, count = struct.unpack_from("<HH", elf, 0x36)
    aligns = []
    for i in range(count):
        off = phoff + i * entsize
        if struct.unpack_from("<I", elf, off)[0] == 1:  # PT_LOAD
            aligns.append(struct.unpack_from("<Q", elf, off + 48)[0])
    return min(aligns)


def main(path: str) -> int:
    errors = []
    with zipfile.ZipFile(path) as z, open(path, "rb") as raw:
        libs = [i for i in z.infolist() if i.filename.endswith(".so")]
        for info in libs:
            align = load_alignment(z.read(info))
            raw.seek(info.header_offset + 26)
            name_len, extra_len = struct.unpack("<HH", raw.read(4))
            data_off = info.header_offset + 30 + name_len + extra_len
            ok = align >= PAGE and (info.compress_type != 0 or data_off % PAGE == 0)
            print(f"{'ok ' if ok else 'KO '} {info.filename} : segments alignés sur {align // 1024} Ko")
            if not ok:
                errors.append(info.filename)
    if not libs:
        print("aucune bibliothèque native trouvée")
        return 1
    if errors:
        print(f"{len(errors)} bibliothèque(s) incompatible(s) avec les pages de 16 Ko")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
