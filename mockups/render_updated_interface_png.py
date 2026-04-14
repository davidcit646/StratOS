#!/usr/bin/env python3
"""
Generate a lightweight UI mockup PNG without external image libraries.
Output: stratterm-and-settings-mockup-v2.png
"""

import struct
import zlib
from pathlib import Path


WIDTH = 1600
HEIGHT = 950


def rgb(hex_color: str) -> tuple[int, int, int]:
    hex_color = hex_color.lstrip("#")
    return tuple(int(hex_color[i : i + 2], 16) for i in (0, 2, 4))


PALETTE = {
    "bg": rgb("#cfd9e6"),
    "window": rgb("#e9eef4"),
    "chrome": rgb("#d8e1ec"),
    "panel": rgb("#f1f5fa"),
    "panel_dark": rgb("#0d223d"),
    "line": rgb("#afbed0"),
    "text": rgb("#2f4a67"),
    "text_soft": rgb("#607d9b"),
    "ok": rgb("#9ed2b1"),
    "chip": rgb("#bfd6c8"),
    "accent": rgb("#bcd0e8"),
}


FONT_3X5 = {
    "A": ["010", "101", "111", "101", "101"],
    "B": ["110", "101", "110", "101", "110"],
    "C": ["011", "100", "100", "100", "011"],
    "D": ["110", "101", "101", "101", "110"],
    "E": ["111", "100", "110", "100", "111"],
    "F": ["111", "100", "110", "100", "100"],
    "G": ["011", "100", "101", "101", "011"],
    "H": ["101", "101", "111", "101", "101"],
    "I": ["111", "010", "010", "010", "111"],
    "J": ["001", "001", "001", "101", "010"],
    "K": ["101", "101", "110", "101", "101"],
    "L": ["100", "100", "100", "100", "111"],
    "M": ["101", "111", "111", "101", "101"],
    "N": ["101", "111", "111", "111", "101"],
    "O": ["010", "101", "101", "101", "010"],
    "P": ["110", "101", "110", "100", "100"],
    "Q": ["010", "101", "101", "111", "011"],
    "R": ["110", "101", "110", "101", "101"],
    "S": ["011", "100", "010", "001", "110"],
    "T": ["111", "010", "010", "010", "010"],
    "U": ["101", "101", "101", "101", "111"],
    "V": ["101", "101", "101", "101", "010"],
    "W": ["101", "101", "111", "111", "101"],
    "X": ["101", "101", "010", "101", "101"],
    "Y": ["101", "101", "010", "010", "010"],
    "Z": ["111", "001", "010", "100", "111"],
    "0": ["111", "101", "101", "101", "111"],
    "1": ["010", "110", "010", "010", "111"],
    "2": ["110", "001", "111", "100", "111"],
    "3": ["110", "001", "011", "001", "110"],
    "4": ["101", "101", "111", "001", "001"],
    "5": ["111", "100", "111", "001", "111"],
    "6": ["011", "100", "111", "101", "111"],
    "7": ["111", "001", "001", "010", "010"],
    "8": ["111", "101", "111", "101", "111"],
    "9": ["111", "101", "111", "001", "110"],
    " ": ["000", "000", "000", "000", "000"],
    ".": ["000", "000", "000", "000", "010"],
    "/": ["001", "001", "010", "100", "100"],
    "-": ["000", "000", "111", "000", "000"],
    ">": ["100", "010", "001", "010", "100"],
    ":": ["000", "010", "000", "010", "000"],
}


class Canvas:
    def __init__(self, width: int, height: int, color: tuple[int, int, int]) -> None:
        self.width = width
        self.height = height
        self.pixels = bytearray(color * (width * height))

    def _idx(self, x: int, y: int) -> int:
        return (y * self.width + x) * 3

    def set_px(self, x: int, y: int, color: tuple[int, int, int]) -> None:
        if 0 <= x < self.width and 0 <= y < self.height:
            i = self._idx(x, y)
            self.pixels[i : i + 3] = bytes(color)

    def fill_rect(self, x: int, y: int, w: int, h: int, color: tuple[int, int, int]) -> None:
        x0 = max(0, x)
        y0 = max(0, y)
        x1 = min(self.width, x + w)
        y1 = min(self.height, y + h)
        for yy in range(y0, y1):
            row = (yy * self.width + x0) * 3
            self.pixels[row : row + (x1 - x0) * 3] = bytes(color) * (x1 - x0)

    def rect(self, x: int, y: int, w: int, h: int, line: tuple[int, int, int], t: int = 1) -> None:
        self.fill_rect(x, y, w, t, line)
        self.fill_rect(x, y + h - t, w, t, line)
        self.fill_rect(x, y, t, h, line)
        self.fill_rect(x + w - t, y, t, h, line)

    def draw_text(
        self,
        x: int,
        y: int,
        text: str,
        color: tuple[int, int, int],
        scale: int = 3,
        spacing: int = 1,
    ) -> None:
        cx = x
        for ch in text.upper():
            glyph = FONT_3X5.get(ch, FONT_3X5[" "])
            for gy, row in enumerate(glyph):
                for gx, bit in enumerate(row):
                    if bit == "1":
                        self.fill_rect(
                            cx + gx * scale,
                            y + gy * scale,
                            scale,
                            scale,
                            color,
                        )
            cx += 3 * scale + spacing * scale

    def save_png(self, path: Path) -> None:
        raw = bytearray()
        stride = self.width * 3
        for y in range(self.height):
            raw.append(0)
            start = y * stride
            raw.extend(self.pixels[start : start + stride])

        def chunk(kind: bytes, data: bytes) -> bytes:
            return (
                struct.pack(">I", len(data))
                + kind
                + data
                + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
            )

        png = bytearray(b"\x89PNG\r\n\x1a\n")
        ihdr = struct.pack(">IIBBBBB", self.width, self.height, 8, 2, 0, 0, 0)
        png.extend(chunk(b"IHDR", ihdr))
        png.extend(chunk(b"IDAT", zlib.compress(bytes(raw), level=9)))
        png.extend(chunk(b"IEND", b""))
        path.write_bytes(png)


def chip(c: Canvas, x: int, y: int, text: str, fill: str = "chip") -> None:
    w = max(80, len(text) * 8 + 24)
    c.fill_rect(x, y, w, 30, PALETTE[fill])
    c.rect(x, y, w, 30, PALETTE["line"], 1)
    c.draw_text(x + 10, y + 9, text, PALETTE["text"], scale=2)


def main() -> None:
    c = Canvas(WIDTH, HEIGHT, PALETTE["bg"])

    # Left main app window.
    lx, ly, lw, lh = 48, 55, 940, 820
    c.fill_rect(lx, ly, lw, lh, PALETTE["window"])
    c.rect(lx, ly, lw, lh, PALETTE["line"], 2)
    c.fill_rect(lx, ly, lw, 36, PALETTE["chrome"])
    c.draw_text(lx + 16, ly + 10, "STRAT TERMINAL", PALETTE["text"], scale=3)
    c.draw_text(lx + 290, ly + 12, "UPDATED UI MOCKUP", PALETTE["text_soft"], scale=2)

    c.fill_rect(lx + 18, ly + 52, 530, 30, PALETTE["accent"])
    c.draw_text(
        lx + 28,
        ly + 61,
        "PATH /HOME/DCITARELLI/PROJECTS/STRATOS",
        PALETTE["text"],
        scale=2,
    )

    chip(c, lx + 600, ly + 52, "TREE ON")
    chip(c, lx + 710, ly + 52, "FLAT OFF")
    chip(c, lx + 816, ly + 52, "QUEUE 6")

    c.fill_rect(lx + 18, ly + 96, lw - 36, 30, PALETTE["panel"])
    c.rect(lx + 18, ly + 96, lw - 36, 30, PALETTE["line"], 1)
    c.draw_text(
        lx + 30,
        ly + 106,
        "/ HOME / DCITARELLI / PROJECTS / STRATOS",
        PALETTE["text_soft"],
        scale=2,
    )

    # File pane.
    fx, fy, fw, fh = lx + 18, ly + 136, 395, 430
    c.fill_rect(fx, fy, fw, fh, PALETTE["panel"])
    c.rect(fx, fy, fw, fh, PALETTE["line"], 1)
    c.draw_text(fx + 10, fy + 10, "FILES", PALETTE["text"], scale=2)
    file_lines = [
        "DIR .. GO UP",
        "DIR APPS/",
        "DIR CONFIG/",
        "DIR STRATTERM/",
        "FILE TALKING-2.MD",
        "FILE README.MD",
    ]
    yy = fy + 38
    for i, line in enumerate(file_lines):
        if i == 3:
            c.fill_rect(fx + 6, yy - 6, fw - 12, 26, PALETTE["accent"])
        c.draw_text(fx + 12, yy, line, PALETTE["text_soft"], scale=2)
        yy += 30

    # Preview pane.
    px, py, pw, ph = fx, fy + fh + 14, fw, 170
    c.fill_rect(px, py, pw, ph, PALETTE["panel"])
    c.rect(px, py, pw, ph, PALETTE["line"], 1)
    c.draw_text(px + 10, py + 10, "PREVIEW", PALETTE["text"], scale=2)
    c.draw_text(px + 10, py + 42, "FOLDER STRATTERM", PALETTE["text_soft"], scale=2)
    c.draw_text(px + 10, py + 70, "CONTAINS 4 FOLDERS 9 FILES", PALETTE["text_soft"], scale=2)
    c.draw_text(px + 10, py + 98, "DOUBLE CLICK TO NAVIGATE", PALETTE["text_soft"], scale=2)

    # Terminal pane.
    tx, ty, tw, th = lx + 425, ly + 136, lw - 443, 614
    c.fill_rect(tx, ty, tw, th, PALETTE["panel_dark"])
    c.rect(tx, ty, tw, th, PALETTE["line"], 1)
    c.draw_text(tx + 14, ty + 12, "TERMINAL", PALETTE["window"], scale=2)
    term_lines = [
        "> PWD",
        "/HOME/DCITARELLI/PROJECTS/STRATOS",
        "> LS",
        "APPS CONFIG DOCS SCRIPTS STRATTERM",
        "> CD -S STRAT INST",
        "GHOST STRAT INSTALL RIPGREP",
        "> MAKE -C STRATTERM RUN",
        "INDEXER QUEUE ACTIVE 6",
    ]
    yy = ty + 44
    for line in term_lines:
        c.draw_text(tx + 16, yy, line, rgb("#dbe8f9"), scale=2)
        yy += 30

    # Prompt bar.
    pbx, pby, pbw, pbh = lx + 18, ly + lh - 58, lw - 36, 38
    c.fill_rect(pbx, pby, pbw, pbh, PALETTE["panel"])
    c.rect(pbx, pby, pbw, pbh, PALETTE["line"], 1)
    c.draw_text(pbx + 10, pby + 12, "> CD -S STRAT INST", PALETTE["text"], scale=2)
    c.draw_text(pbx + 380, pby + 12, "GHOST STRAT INSTALL RIPGREP", PALETTE["ok"], scale=2)

    # Right settings window.
    rx, ry, rw, rh = 1015, 95, 540, 730
    c.fill_rect(rx, ry, rw, rh, PALETTE["window"])
    c.rect(rx, ry, rw, rh, PALETTE["line"], 2)
    c.fill_rect(rx, ry, rw, 34, PALETTE["chrome"])
    c.draw_text(rx + 16, ry + 10, "STRAT SETTINGS", PALETTE["text"], scale=3)
    c.draw_text(rx + 16, ry + 42, "INDEXER SETTINGS PANEL", PALETTE["text_soft"], scale=2)

    row_y = ry + 70
    rows = [
        "ENABLE INDEXING              ON",
        "START INDEXER ON BOOT        ON",
        "ENABLE UI INDEXER            ON",
        "DAEMON FREQUENCY MS          1200",
        "RESCAN SECONDS               180",
        "BATCH LIMIT                  96",
        "ROOTS                        /HOME /CONFIG /APPS",
        "EXCLUDE PREFIXES             NONE",
        "TOOLTIPS                     ENABLED",
    ]
    for line in rows:
        c.fill_rect(rx + 16, row_y, rw - 32, 40, PALETTE["panel"])
        c.rect(rx + 16, row_y, rw - 32, 40, PALETTE["line"], 1)
        c.draw_text(rx + 24, row_y + 14, line, PALETTE["text_soft"], scale=2)
        row_y += 50

    c.fill_rect(rx + 16, ry + rh - 110, 150, 36, PALETTE["accent"])
    c.rect(rx + 16, ry + rh - 110, 150, 36, PALETTE["line"], 1)
    c.draw_text(rx + 62, ry + rh - 98, "RELOAD", PALETTE["text"], scale=2)

    c.fill_rect(rx + 180, ry + rh - 110, 150, 36, PALETTE["ok"])
    c.rect(rx + 180, ry + rh - 110, 150, 36, PALETTE["line"], 1)
    c.draw_text(rx + 236, ry + rh - 98, "SAVE", PALETTE["text"], scale=2)

    c.fill_rect(rx + 16, ry + rh - 62, rw - 32, 40, PALETTE["panel"])
    c.rect(rx + 16, ry + rh - 62, rw - 32, 40, PALETTE["line"], 1)
    c.draw_text(
        rx + 24,
        ry + rh - 48,
        "SAVED SETTINGS RESTART INDEXER TO APPLY",
        PALETTE["ok"],
        scale=2,
    )

    out = Path(__file__).with_name("stratterm-and-settings-mockup-v2.png")
    c.save_png(out)
    print(out)


if __name__ == "__main__":
    main()
