#!/usr/bin/env python3
"""標題「司業命緣」的標誌圖（GDD 9.1 封面）：以行書字型的字當骨架，排成一行加朱印。

    python tools/title/build_logo.py              # 寫 sources/title/ 的來源與 game/app/assets/title_logo.png
    python tools/title/build_logo.py --preview 路徑  # 另存一張墊紙色的預覽圖

字取自素材庫的「衡山毛筆フォント行書」（opentype/kouzangyousho），輪廓原樣不改；「緣」用字型的行書寫法「縁」。
VectorCraft 把輪廓排成路徑：墨字一層（帶紙色外暈，疊在山水上看得清）、朱印一層。
版面單位與 TitleLogo 相同（1080 高畫面上的像素）：字帶寬 BAND_W、高 BAND_H，四周留 BLEED 給出格的筆畫。
"""
import argparse
import json
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

from fontTools.pens.basePen import BasePen
from fontTools.pens.boundsPen import BoundsPen
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "sources" / "title"
TEXTURE = ROOT / "game" / "app" / "assets" / "title_logo.png"
CLI = Path("D:/crafting/vectorcraft/target/release/vectorcraft-cli.exe")
FONT_ZIP = Path("D:/godot-art-library/packs/opentype/kouzangyousho/KouzanGyousho.zip")

# 版面（與 game/app/code/title_logo.gd 的常數對應）。
GLYPH = 135.0
GLYPH_GAP = 10.0
SEAL = 56.0
SEAL_GAP = 12.0
BAND_W = 4.0 * GLYPH + 3.0 * GLYPH_GAP + SEAL_GAP + SEAL
BAND_H = GLYPH
BLEED = 40.0
PX = 4.0  # 一個版面單位輸出幾個像素

# 四個字：[字, 墨跡高度相對字帶的倍率, 往下移的量（版面單位）]。大小有節奏，「命」的長豎出格。
LAYOUT = [("司", 0.92, 2.0), ("業", 1.04, 0.0), ("命", 1.16, 6.0), ("縁", 1.02, 0.0)]
SEAL_MARKS = ["司", "命"]
INK = "#2b2620"
PAPER = "#f3ebd8"
SEAL_RED = "#b23a2e"


class PathData(BasePen):
    """把字的輪廓寫成 SVG 路徑資料；二次曲線由 BasePen 升成三次。"""

    def __init__(self, glyphs, scale, dx, dy):
        super().__init__(glyphs)
        self.parts = []
        self.scale, self.dx, self.dy = scale, dx, dy

    def _pt(self, point):
        return f"{self.dx + point[0] * self.scale:.2f} {self.dy - point[1] * self.scale:.2f}"

    def _moveTo(self, point):
        self.parts.append("M" + self._pt(point))

    def _lineTo(self, point):
        self.parts.append("L" + self._pt(point))

    def _curveToOne(self, a, b, c):
        self.parts.append("C" + " ".join(self._pt(p) for p in (a, b, c)))

    def _closePath(self):
        self.parts.append("Z")

    def data(self):
        return " ".join(self.parts)


def outline(font, char, height, center_x, center_y):
    """一個字的路徑資料：墨跡縮放到指定高度，墨跡中心放在指定位置。"""
    glyphs = font.getGlyphSet()
    name = font.getBestCmap()[ord(char)]
    bounds = BoundsPen(glyphs)
    glyphs[name].draw(bounds)
    x0, y0, x1, y1 = bounds.bounds
    scale = height / (y1 - y0)
    pen = PathData(glyphs, scale, center_x - (x0 + x1) * 0.5 * scale, center_y + (y0 + y1) * 0.5 * scale)
    glyphs[name].draw(pen)
    return pen.data()


def run(steps, source=None, exports=()):
    cmd = [str(CLI), "run"]
    if source:
        cmd += ["--in", str(source)]
    for step, params in steps:
        cmd += ["--cmd", step, "--params", json.dumps(params, ensure_ascii=False)]
    for path in exports:
        cmd += ["--export", str(path)]
    done = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", timeout=600)
    lines = [json.loads(line) for line in done.stdout.splitlines() if line.startswith("{")]
    failed = [line for line in lines if line.get("ok") is False or "error" in line]
    if done.returncode != 0 or failed:
        print(done.stdout[-3000:], done.stderr[-3000:], file=sys.stderr)
        raise SystemExit("VectorCraft 執行失敗")
    return lines


def main():
    ap = argparse.ArgumentParser(description="排標題的標誌圖")
    ap.add_argument("--font", type=Path, default=FONT_ZIP, help="字型的壓縮檔")
    ap.add_argument("--preview", type=Path, help="另存一張墊紙色的預覽圖")
    args = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    TEXTURE.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as temp:
        archive = zipfile.ZipFile(args.font)
        ttf = Path(temp) / "font.ttf"
        ttf.write_bytes(archive.read(archive.infolist()[0]))
        font = TTFont(ttf)
        width = (BAND_W + 2.0 * BLEED) * PX
        height = (BAND_H + 2.0 * BLEED) * PX
        middle = (BLEED + BAND_H * 0.5) * PX
        marks = []
        for index, (char, ratio, drop) in enumerate(LAYOUT):
            center_x = (BLEED + index * (GLYPH + GLYPH_GAP) + GLYPH * 0.5) * PX
            marks.append((char, INK, outline(font, char, BAND_H * ratio * PX, center_x, middle + drop * PX)))
        seal_x = (BLEED + 4.0 * GLYPH + 3.0 * GLYPH_GAP + SEAL_GAP) * PX
        seal_y = (BLEED + GLYPH - SEAL * 0.9) * PX
        side = SEAL * PX
        stamps = []
        for index, char in enumerate(SEAL_MARKS):
            stamps.append(("印文" + char, PAPER, outline(font, char, side * 0.42, seal_x + side * 0.5, seal_y + side * (0.27 + 0.46 * index))))
    # 路徑資料太長，放不進命令列：先寫成 SVG 交給 VectorCraft 開啟，再由它上效果、存原生檔與輸出。
    skeleton = OUT / "logo-skeleton.svg"
    rows = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0f}" height="{height:.0f}" viewBox="0 0 {width:.0f} {height:.0f}">', '<g id="墨">']
    rows += [f'<path id="{name}" fill="{color}" d="{data}"/>' for name, color, data in marks]
    rows += ["</g>", '<g id="印">', f'<rect id="印底" x="{seal_x:.1f}" y="{seal_y:.1f}" width="{side:.1f}" height="{side:.1f}" rx="{side * 0.07:.1f}" fill="{SEAL_RED}"/>']
    rows += [f'<path id="{name}" fill="{color}" d="{data}"/>' for name, color, data in stamps]
    rows += ["</g>", "</svg>"]
    skeleton.write_text("\n".join(rows), encoding="utf-8")
    work = OUT / "logo.vectorcraft"
    tree = run([("document.inspect", {})], source=skeleton)
    layers = {layer["name"]: layer["id"] for layer in tree[-1]["result"]["layers"]}
    if "墨" not in layers:
        raise SystemExit(f"骨架裡找不到墨字那一層：{sorted(layers)}")
    steps = [
        ("effect.apply", {"effect": "stylize.outerGlow", "ids": [layers["墨"]], "params": {"mode": "normal", "opacity": 70.0, "blur": 5.0 * PX, "color": PAPER}}),
        ("file.saveAs", {"path": str(work)}),
    ]
    run(steps, source=skeleton, exports=[OUT / "logo.svg", OUT / "logo.psd", TEXTURE])
    if args.preview:
        from PIL import Image
        art = Image.open(TEXTURE).convert("RGBA")
        paper = Image.new("RGBA", art.size, "#8a8f86")
        paper.alpha_composite(art)
        paper.convert("RGB").save(args.preview)
    print(f"已寫入 {work}、{OUT / 'logo.svg'}、{OUT / 'logo.psd'}、{TEXTURE}")


if __name__ == "__main__":
    main()
