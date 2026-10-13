"""驗收的量測與圖：單層量測、紙色與墨色底的單層圖、疊回並排圖（三組染色）。

    python tools/parts/check.py [部件…]     不給部件就檢全部已有 key/ 的

輸出（.shiftblame/tmp/parts/portrait/check/）：
    metrics.json            每個子層的量測
    layers-paper.png / layers-ink.png   每個子層以預設染色，放在紙色（#EDE4CF）與墨色（#2B2620）底上
    stack.png               底版加已有的部件，三組不同的膚色、髮色、衣色並排
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
# 三組染色：膚色、髮色、衣色、緣邊色、瞳色
PALETTES = [
    {"skin": (222, 177, 130), "hair": (96, 84, 76), "cloth": (92, 112, 128), "trim": (178, 58, 46), "iris": (70, 110, 150)},
    {"skin": (238, 208, 184), "hair": (205, 205, 200), "cloth": (150, 118, 82), "trim": (60, 52, 46), "iris": (110, 84, 58)},
    {"skin": (176, 126, 88), "hair": (122, 76, 50), "cloth": (86, 128, 108), "trim": (201, 162, 70), "iris": (58, 120, 96)},
]


def tint(layer_rgba, color):
    """把鍵圖染色：墨線維持 ink，陰影、底色、高光換成 color 的三階。"""
    c = np.array(color, np.float32)
    ramp = {KEY_LEVELS["shadow"]: c * 0.62, KEY_LEVELS["base"]: c, KEY_LEVELS["high"]: c + (255 - c) * 0.4}
    g = layer_rgba[..., 0]
    out = np.zeros(layer_rgba.shape[:2] + (3,), np.float32)
    out[:] = INKC
    for lv, col in ramp.items():
        out[g > lv - 40] = col   # 由低到高逐階覆蓋，等同以 45/130/212 為界
    # 上面的寫法依字典順序（90、170、255）逐階覆蓋；ink 是 g<=50
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
        ik = sd & ((cv2.cvtColor(im[..., :3], cv2.COLOR_RGB2GRAY) < 80) if fixed else (im[..., 0] <= 40))
        solid = sd if solid is None else solid | sd
        ink = ik if ink is None else ink | ik
    return _measure(solid, ink, None, False)


def measure(name, sub, path, fixed):
    im = np.asarray(Image.open(path).convert("RGBA"))
    solid = im[..., 3] > 128
    ink = (im[..., 0] <= 40) if not fixed else (cv2.cvtColor(im[..., :3], cv2.COLOR_RGB2GRAY) < 80)
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
        pal = PALETTES[0]
        rgba = im if fixed else tint(im, pal.get(sub if sub in pal else {"hair": "hair", "brow": "hair", "beard": "hair", "cloth": "cloth", "trim": "trim", "iris": "iris"}.get(sub, "hair")))
        canvas = np.zeros(im.shape[:2] + (3,), np.float32)
        canvas[:] = bg
        tiles.append(paste(canvas, rgba).round().astype(np.uint8))
    sheet = np.concatenate(tiles, 1) if tiles else np.zeros((1, 1, 3), np.uint8)
    Image.fromarray(sheet).save(path)


def skin_retint(base, src_skin, dst_skin):
    hsv = cv2.cvtColor(base, cv2.COLOR_RGB2HSV).astype(np.float32)
    h, s, v = hsv[..., 0] * 2, hsv[..., 1] / 255, hsv[..., 2] / 255
    skin = (h > 10) & (h < 45) & (s > 0.18) & (v > 0.35)
    ratio = np.array(dst_skin, np.float32) / np.array(src_skin, np.float32)
    out = base.astype(np.float32)
    out[skin] *= ratio
    return out.clip(0, 255).astype(np.uint8)


def paper_alpha(base):
    d = np.abs(base.astype(int) - np.array(PAPER)[None, None]).max(2)
    return (d > 14).astype(np.uint8)


def stack(names, path, sex="m"):
    base = np.asarray(Image.open(P.base_path(sex)).convert("RGB").resize(P.OUT, Image.LANCZOS))
    skin_alpha = cv2.GaussianBlur(paper_alpha(base).astype(np.float32), (0, 0), 0.6)
    skin_alpha = np.where(skin_alpha > 0.5, 1.0, 0.0).astype(np.float32)
    order = {s: i for i, s in enumerate(P.SLOTS)}
    cols = []
    for pal in PALETTES:
        canvas = np.zeros(base.shape[:2] + (3,), np.float32)
        canvas[:] = PAPER
        face = skin_retint(base, PALETTES[0]["skin"], pal["skin"])
        face_rgba = np.dstack([face, (skin_alpha * 255).astype(np.uint8)])
        layers = []
        for n in names:
            slot = P.PARTS[n]["slot"]
            for sub in P.PARTS[n]["subs"]:
                f = P.TMP + f"key/{n}.{sub}.png"
                if not os.path.exists(f):
                    continue
                im = np.asarray(Image.open(f).convert("RGBA"))
                if sub in P.PARTS[n].get("fixed", []):
                    rgba = im
                else:
                    key = {"hair": "hair", "brow": "hair", "beard": "hair", "cloth": "cloth", "trim": "trim", "iris": "iris"}[sub]
                    rgba = tint(im, pal[key])
                layers.append((order[slot], rgba))
        layers.append((order["hair_back"] + 0.5, face_rgba))   # 底版（臉加內衫）緊接在後髮之上，領壓在它上面
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
