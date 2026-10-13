"""組裝圖：把 key/ 裡的部件依疊圖順序疊起來，供驗收讀圖。

    python tools/parts/montage.py <輸出.png> <部件>[,<部件>…] [--scale 2] [--faces m0,m1,m2]

每列一種臉型，每欄一組染色（淺／中／深膚色 × 黑／白髮色，共六欄）。部件名是 portrait.PARTS 的鍵（如 front_hair-topknot）；
臉不用列，由 --faces 指定。另有函式 render(names, pal) 給其他腳本取單張。
"""
import argparse
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402
import check as C  # noqa: E402

SKINS = [(238, 208, 184), (222, 177, 130), (107, 74, 52)]
HAIRS = [(46, 42, 40), (228, 228, 222)]
IRISES = [(59, 42, 34), (47, 59, 54), (58, 58, 74), (85, 64, 42)]   # 與 PortraitRecipe.IRISES 相同：深褐與黑為主
LIP_BASE, LIP_MIX, LIP_DARK = (189, 76, 71), 0.4, 0.88   # 與 PortraitRecipe.lip_of 相同：唇色往膚色靠、略暗


def lip_of(skin):
    return tuple(int(round((s + (b - s) * LIP_MIX) * LIP_DARK)) for s, b in zip(skin, LIP_BASE))


ROBE = [(92, 112, 128), (150, 118, 82), (86, 128, 108)]


def palette(skin, hair, k=0):
    return {"skin": skin, "hair": hair, "robe": ROBE[k % 3], "trim": (178, 58, 46), "iris": IRISES[k % len(IRISES)], "violet": (107, 74, 142),
            "gold": (201, 162, 70), "lip": lip_of(skin)}


def render(names, pal, bg=C.PAPER):
    """names 含臉；回傳 (400,320,3) uint8。"""
    order = {sid: o for sid, o, _ in P.SLOTS}
    canvas = np.zeros((P.OUT[1], P.OUT[0], 3), np.float32)
    canvas[:] = bg
    layers = []
    for n in names:
        spec = P.PARTS[n]
        for k, sub in enumerate(spec["subs"]):
            f = P.TMP + f"key/{n}.{sub}.png"
            if not os.path.exists(f):
                continue
            im = np.asarray(Image.open(f).convert("RGBA"))
            rgba = im if sub in spec.get("fixed", []) else C.tint(im, pal[spec["dye"][sub]])
            layers.append((order[spec["slot"]] + k * 0.1 + (0.05 if sub == "trim" else 0), rgba))
    for _, rgba in sorted(layers, key=lambda t: t[0]):
        C.paste(canvas, rgba)
    return canvas.round().astype(np.uint8)


def grid(names, faces=("m0", "m1", "m2"), scale=2):
    rows = []
    for fi, fc in enumerate(faces):
        row = []
        for si, skin in enumerate(SKINS):
            for hi, hair in enumerate(HAIRS):
                row.append(render([f"face-{fc}"] + list(names), palette(skin, hair, si)))
        rows.append(np.concatenate(row, 1))
    im = Image.fromarray(np.concatenate(rows, 0))
    return im.resize((im.width * scale // 2, im.height * scale // 2), Image.LANCZOS if scale < 2 else Image.NEAREST) if scale != 2 else im


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("names")
    ap.add_argument("--scale", type=int, default=2)
    ap.add_argument("--faces", default="m0,m1,m2")
    a = ap.parse_args()
    grid(a.names.split(",") if a.names else [], a.faces.split(","), a.scale).save(a.out)
    print(a.out)
