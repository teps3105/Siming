"""一致化：縮到入庫尺寸、拆材質子層、轉成明度鍵。

    python tools/parts/key.py <部件>     → .shiftblame/tmp/parts/portrait/key/<部件>.<子層>.png（320×400 RGBA）

1. 讀補完後的部件（有 complete/<部件>.png 用它，否則 collect/<部件>.png），1024×1280 → 320×400：預乘 alpha 後 INTER_AREA 縮，再還原顏色，
   所以邊上只有部件自己的顏色，不帶底版的紙色。
2. 子層（spec["subs"]）：材質不同的區域拆成各自一張。判別法：all＝整件；chroma＝高彩度（石青的瞳）；red＝紅色（朱邊）；rest＝其餘。
   一個像素只屬於一個子層；深色線與暗像素留在 rest（主材質）。
3. 明度鍵（染色子層）：子層內的亮度做 4 群 1D k-means，依順序對應 墨線、陰影、底色、高光 四階灰（KEY_LEVELS）；
   底色是非墨群中像素最多的一群，比它暗的是陰影，比它亮的是高光；最後以 3×3 中值去掉單像素的雜點與過渡色。
   定色子層（spec["fixed"]）保持原色不轉鍵。
"""
import argparse
import os
import sys

import cv2
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402

INK_MAX, MERGE = 52, 18   # 墨線的亮度上限；相鄰色群中心相差小於此值併成一階
KEY_LEVELS = {"ink": 0, "shadow": 90, "base": 170, "high": 255}


def shrink(rgba):
    a = rgba[..., 3:].astype(np.float32) / 255
    pre = np.dstack([rgba[..., :3].astype(np.float32) * a, a * 255])
    small = cv2.resize(pre, P.OUT, interpolation=cv2.INTER_AREA)
    alpha = small[..., 3:] / 255
    rgb = small[..., :3] / np.maximum(alpha, 1e-3)
    return np.dstack([rgb.clip(0, 255), small[..., 3]]).round().astype(np.uint8)


def luma(rgb):
    return rgb[..., 0] * 0.299 + rgb[..., 1] * 0.587 + rgb[..., 2] * 0.114


def split_subs(img, subs):
    """回傳 {子層: 遮罩}。"""
    solid = img[..., 3] > 8
    hsv = cv2.cvtColor(img[..., :3], cv2.COLOR_RGB2HSV).astype(np.float32)
    hue, sat, val = hsv[..., 0] * 2, hsv[..., 1] / 255, hsv[..., 2] / 255
    masks, taken = {}, np.zeros_like(solid)
    for name, how in subs.items():
        if how == "all":
            m = solid
        elif how == "chroma":
            m = solid & (sat > 0.28) & (val > 0.3) & (hue > 150) & (hue < 260)   # 石青、石綠
        elif how == "red":
            m = solid & (sat > 0.45) & (val > 0.35) & ((hue < 22) | (hue > 340))
        else:
            continue
        m = m & ~taken
        # 紅邊、瞳是小塊，去掉面積過小的雜點；剩下的歸 rest
        if how in ("chroma", "red"):
            n, lab, st, _ = cv2.connectedComponentsWithStats(m.astype(np.uint8), connectivity=8)
            for i in range(1, n):
                if st[i, cv2.CC_STAT_AREA] < 6:
                    m[lab == i] = False
        masks[name] = m
        taken |= m
    for name, how in subs.items():
        if how == "rest":
            masks[name] = solid & ~taken
    return masks


def to_key(img, mask):
    """把 mask 內的像素轉成四階灰，回傳 (RGBA 圖, 統計)。"""
    L = luma(img[..., :3].astype(np.float32))
    vals = L[mask].reshape(-1, 1).astype(np.float32)
    if len(vals) < 30:
        return None, {}
    _, labels, centers = cv2.kmeans(vals, 4, None, (cv2.TERM_CRITERIA_EPS | cv2.TERM_CRITERIA_MAX_ITER, 50, 0.5), 5, cv2.KMEANS_PP_CENTERS)
    order = np.argsort(centers.ravel())
    centers = centers.ravel()[order]
    rank = np.empty(4, int)
    rank[order] = np.arange(4)
    idx = rank[labels.ravel()]
    # 分群：中心亮度 ≤ INK_MAX 的歸墨線；其餘相鄰中心差小於 MERGE 的併成一群（均勻色塊被 k-means 硬切成四份時不當成四階）
    group = np.zeros(4, int)
    g = 0
    for i in range(4):
        if centers[i] <= INK_MAX:
            group[i] = 0
        else:
            if i > 0 and centers[i - 1] > INK_MAX and centers[i] - centers[i - 1] < MERGE:
                group[i] = g
            else:
                g += 1
                group[i] = g
    gid = group[idx]
    counts = np.bincount(gid, minlength=g + 1)
    ids = [i for i in range(1, g + 1) if counts[i] > 0]
    base = max(ids, key=lambda i: counts[i]) if ids else None
    level_of = {0: "ink"}
    for i in ids:
        level_of[i] = "shadow" if i < base else ("base" if i == base else "high")
    gray = np.full(L.shape, -1, np.int16)
    gray[mask] = np.array([KEY_LEVELS[level_of.get(i, "base")] for i in gid])
    centers_g = [round(float(vals[gid == i].mean()), 1) if (gid == i).any() else None for i in range(g + 1)]
    counts_all = counts
    lv = np.where(mask, gray, 0).astype(np.uint8)
    lv_med = cv2.medianBlur(lv, 3)
    gray = np.where(mask, lv_med, 0).astype(np.uint8)
    out = np.dstack([gray, gray, gray, np.where(mask, img[..., 3], 0).astype(np.uint8)])
    stats = {"centers": centers_g, "share": [round(float(c) / counts_all.sum(), 3) for c in counts_all], "levels": [level_of.get(i) for i in range(g + 1)]}
    return out, stats


def key(name):
    spec = P.PARTS[name]
    src = P.TMP + f"complete/{name}.png"
    if not os.path.exists(src):
        src = P.TMP + f"collect/{name}.png"
    rgba = cv2.cvtColor(cv2.imread(src, cv2.IMREAD_UNCHANGED), cv2.COLOR_BGRA2RGBA)
    img = shrink(rgba)
    masks = split_subs(img, spec["subs"])
    os.makedirs(P.TMP + "key", exist_ok=True)
    report = {"source": os.path.relpath(src, P.ROOT).replace("\\", "/"), "levels": KEY_LEVELS, "subs": {}}
    for sub, m in masks.items():
        fixed = sub in spec.get("fixed", [])
        if fixed:
            out = img.copy()
            out[..., 3] = np.where(m, img[..., 3], 0)
            out[out[..., 3] == 0, :3] = 0
            stats = {"fixed": True}
        else:
            out, stats = to_key(img, m)
            if out is None:
                continue
        path = P.TMP + f"key/{name}.{sub}.png"
        cv2.imwrite(path, cv2.cvtColor(out, cv2.COLOR_RGBA2BGRA))
        report["subs"][sub] = {"file": os.path.relpath(path, P.ROOT).replace("\\", "/"), "pixels": int(m.sum()), "dye": spec["dye"].get(sub, "定色"), **stats}
        print(name, sub, report["subs"][sub])
    P.log(name, "key", report)


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("part")
    key(ap.parse_args().part)
