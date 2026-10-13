"""驗收的量測與圖：單層量測、紙色與墨色底的單層圖、疊回並排圖（三組染色）。

    python tools/parts/check.py [部件…]     不給部件就檢全部已有 key/ 的

輸出（.shiftblame/tmp/parts/portrait/check/）：
    metrics.json            每個子層的量測
    layers-paper.png / layers-ink.png   每個子層以預設染色，放在紙色（#EDE4CF）與墨色（#2B2620）底上
    stack.png               已有的部件（含臉），三組不同的膚色、髮色、衣色並排
量測（入庫尺寸 320×400）：
    bare_edge   外緣上離最近的墨線（鍵 0 階，或定色層暗於 80）超過 2 像素的像素數——「邊上沒有輪廓線」；hidden 的邊要人工扣
    bare_ratio  bare_edge 占外緣長度
    specks      面積小於 40 像素的連通塊數；blobs 為連通塊總數
    holes       部件內的洞數（面積 ≥ 4）
    halo        外緣像素中亮度高於同層平均且偏紙色的比例（亮邊）
"""
import json
import os
import sys

import cv2
import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402
from key import KEY_LEVELS  # noqa: E402

PAPER, INKC = (0xED, 0xE4, 0xCF), (0x2B, 0x26, 0x20)
INK_MAX_KEY = 42   # 鍵圖灰階在此以下算墨線（0 與 0x55 的中點）
# 三組染色：膚色、髮色、衣色、緣邊色、瞳色（欄位名與目錄 channels 相同）
PALETTES = [
    {"skin": (222, 177, 130), "hair": (96, 84, 76), "robe": (92, 112, 128), "trim": (178, 58, 46), "iris": (70, 110, 150)},
    {"skin": (238, 208, 184), "hair": (225, 225, 220), "robe": (150, 118, 82), "trim": (60, 52, 46), "iris": (110, 84, 58)},
    {"skin": (176, 126, 88), "hair": (122, 76, 50), "robe": (86, 128, 108), "trim": (201, 162, 70), "iris": (58, 120, 96)},
]
for _p in PALETTES:
    _p.setdefault("gold", (201, 162, 70))   # 金色通道，遊戲內是 PartTint.GOLD


def tint(layer_rgba, color):
    """與著色器同一條公式：灰階 → 墨、陰影、底色、高光四個停點，線性分段；陰影與高光由底色導出（part_tint.gd 的 ramp）。"""
    c = np.array(color, np.float32)
    ink = np.array(INKC, np.float32)
    stops = [ink, c + (ink - c) * 0.28, c, c + (np.array([0xFB, 0xF8, 0xEE], np.float32) - c) * 0.38]
    p = np.clip(layer_rgba[..., 0].astype(np.float32) / 255, 0, 1) * 3
    i = np.minimum(p.astype(int), 2)
    f = (p - i)[..., None]
    st = np.stack(stops)
    out = st[i] * (1 - f) + st[i + 1] * f
    return np.dstack([out.clip(0, 255), layer_rgba[..., 3]]).round().astype(np.uint8)


def paste(canvas, rgba):
    a = rgba[..., 3:].astype(np.float32) / 255
    canvas[...] = canvas * (1 - a) + rgba[..., :3] * a
    return canvas


def layer_files(names):
    out = []
    for n in names:
        for sub in P.PARTS[n]["subs"]:
            f = P.TMP + f"key/{n}.{sub}.png"
            if os.path.exists(f):
                out.append((n, sub, f))
    return out


def measure_union(items):
    """一個部件的各子層合起來量（墨線在主材質層、緣邊層本身沒有墨線時，單層量會誤報）。"""
    solid = ink = None
    for _, _, f, fixed in items:
        im = np.asarray(Image.open(f).convert("RGBA"))
        sd = im[..., 3] > 128
        ik = sd & ((cv2.cvtColor(im[..., :3], cv2.COLOR_RGB2GRAY) < 80) if fixed else (im[..., 0] <= INK_MAX_KEY))
        solid = sd if solid is None else solid | sd
        ink = ik if ink is None else ink | ik
    return _measure(solid, ink, None, False)


def measure(name, sub, path, fixed):
    im = np.asarray(Image.open(path).convert("RGBA"))
    solid = im[..., 3] > 128
    ink = (im[..., 0] <= INK_MAX_KEY) if not fixed else (cv2.cvtColor(im[..., :3], cv2.COLOR_RGB2GRAY) < 80)
    return _measure(solid, ink, im, fixed)


def _measure(solid, ink, im, fixed):
    edge = solid & ~cv2.erode(solid.astype(np.uint8), np.ones((3, 3), np.uint8)).astype(bool)
    near = cv2.dilate((ink & solid).astype(np.uint8), cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5))).astype(bool)
    bare = edge & ~near
    n, lab, st, _ = cv2.connectedComponentsWithStats(solid.astype(np.uint8), connectivity=8)
    areas = st[1:, cv2.CC_STAT_AREA]
    inv = (~solid).astype(np.uint8)
    nh, lh, sh, _ = cv2.connectedComponentsWithStats(inv, connectivity=4)
    holes = sum(1 for i in range(1, nh) if sh[i, cv2.CC_STAT_AREA] >= 4 and not (lh[i] == lh[0, 0]).any() and sh[i, 0] > 0 and sh[i, 1] > 0
                and sh[i, 0] + sh[i, 2] < im.shape[1] and sh[i, 1] + sh[i, 3] < im.shape[0])
    ys, xs = np.nonzero(edge)
    hal = 0
    if im is not None and not fixed and len(xs):
        hal = float(((im[..., 0] >= 255 - 10)[edge]).mean())
    return {"edge_len": int(edge.sum()), "bare_edge": int(bare.sum()), "bare_ratio": round(float(bare.sum() / max(edge.sum(), 1)), 3), "blobs": int(len(areas)),
            "specks": int((areas < 40).sum()), "holes": holes, "high_on_edge": round(hal, 3), "pixels": int(solid.sum())}


def layers_sheet(files, bg, path):
    tiles = []
    for n, sub, f in files:
        fixed = sub in P.PARTS[n].get("fixed", [])
        im = np.asarray(Image.open(f).convert("RGBA"))
        rgba = im if fixed else tint(im, PALETTES[0][P.PARTS[n]["dye"][sub]])
        canvas = np.zeros(im.shape[:2] + (3,), np.float32)
        canvas[:] = bg
        tiles.append(paste(canvas, rgba).round().astype(np.uint8))
    sheet = np.concatenate(tiles, 1) if tiles else np.zeros((1, 1, 3), np.uint8)
    Image.fromarray(sheet).save(path)


def stack(names, path):
    """已有的部件依疊圖順序疊起，三組染色並排（臉是部件，沒有底版）。"""
    order = {sid: o for sid, o, _ in P.SLOTS}
    cols = []
    for pal in PALETTES:
        canvas = np.zeros((P.OUT[1], P.OUT[0], 3), np.float32)
        canvas[:] = PAPER
        layers = []
        for n in names:
            spec = P.PARTS[n]
            for k, sub in enumerate(spec["subs"]):
                f = P.TMP + f"key/{n}.{sub}.png"
                if not os.path.exists(f):
                    continue
                im = np.asarray(Image.open(f).convert("RGBA"))
                rgba = im if sub in spec.get("fixed", []) else tint(im, pal[spec["dye"][sub]])
                layers.append((order[spec["slot"]] + k * 0.1 + (0.05 if sub == "trim" else 0), rgba))
        for _, rgba in sorted(layers, key=lambda t: t[0]):
            paste(canvas, rgba)
        cols.append(canvas.round().astype(np.uint8))
    Image.fromarray(np.concatenate(cols, 1)).resize((P.OUT[0] * 3 * 2, P.OUT[1] * 2), Image.NEAREST).save(path)


def main(names):
    os.makedirs(P.TMP + "check", exist_ok=True)
    files = layer_files(names)
    metrics = {}
    for n, sub, f in files:
        metrics[f"{n}.{sub}"] = measure(n, sub, f, sub in P.PARTS[n].get("fixed", []))
        print(f"{n}.{sub}", metrics[f"{n}.{sub}"])
    for n in names:
        items = [(n, sub, f, sub in P.PARTS[n].get("fixed", [])) for n2, sub, f in files if n2 == n]
        if len(items) > 1:
            metrics[f"{n}.union"] = measure_union(items)
            print(f"{n}.union", metrics[f"{n}.union"])
    json.dump(metrics, open(P.TMP + "check/metrics.json", "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    layers_sheet(files, PAPER, P.TMP + "check/layers-paper.png")
    layers_sheet(files, INKC, P.TMP + "check/layers-ink.png")
    stack(names, P.TMP + "check/stack.png")
    for n in names:
        P.log(n, "check", {"metrics": {k: v for k, v in metrics.items() if k.startswith(n + ".")}})


if __name__ == "__main__":
    main(sys.argv[1:] or [n for n in P.PARTS if layer_files([n])])
