"""一致化：縮到入庫尺寸、拆材質子層、轉成明度鍵。

    python tools/parts/key.py <部件>     → .shiftblame/tmp/parts/portrait/key/<部件>.<子層>.png（320×400 RGBA）

1. 讀補完後的部件（有 complete/<部件>.png 用它，否則 collect/<部件>.png），1024×1280 → 320×400：預乘 alpha 後 INTER_AREA 縮，再還原顏色，
   所以邊上只有部件自己的顏色，不帶底版的紙色。
2. 子層（spec["subs"]）：材質不同的區域拆成各自一張。判別法：all＝整件；chroma＝高彩度（石青的瞳）；red＝紅色（朱邊）；rest＝其餘。
   一個像素只屬於一個子層；深色線與暗像素留在 rest（主材質）。
3. 明度鍵（染色子層）：子層內的亮度做 4 群 1D k-means 找出墨線、陰影、底色、高光的中心（底色是非墨群中像素最多的一群），
   再以單調分段線性曲線對到 0x00/0x55/0xAA/0xFF：各階中心附近是平台、平台之間線性過渡，邊緣保留反鋸齒（不做硬量化、不做中值）。
   定色子層（spec["fixed"]）保持原色不轉鍵。
"""
import argparse
import os
import sys

import cv2
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402

INK_MAX, MERGE = 52, 12   # 墨線的亮度上限；相鄰色群中心相差小於此值併成一階
KEY_LEVELS = {"ink": 0, "shadow": 85, "base": 170, "high": 255}   # 0x00、0x55、0xAA、0xFF，與遊戲內圖集一致
PLATEAU = 0.3   # 色塊平台的半寬：到相鄰色群中心距離的這個比例；平台之間線性過渡，所以邊緣與外緣保留反鋸齒


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
    """把 mask 內的像素轉成連續灰階的明度鍵，回傳 (RGBA 圖, 統計)。

    以不透明像素的亮度做 4 群 1D k-means，找出墨線、陰影、底色、高光的中心；然後以單調分段線性曲線把亮度對到 0x00/0x55/0xAA/0xFF：
    每階中心附近是平台（色塊內部乾淨平塗），平台之間線性過渡（交界與外緣保留反鋸齒）。這是色調對應，不是量化。
    """
    L = luma(img[..., :3].astype(np.float32))
    solid = mask & (img[..., 3] > 200)
    vals = L[solid].reshape(-1, 1).astype(np.float32)
    if len(vals) < 30:
        return None, {}
    _, labels, centers = cv2.kmeans(vals, 4, None, (cv2.TERM_CRITERIA_EPS | cv2.TERM_CRITERIA_MAX_ITER, 50, 0.5), 5, cv2.KMEANS_PP_CENTERS)
    order = np.argsort(centers.ravel())
    centers = centers.ravel()[order]
    rank = np.empty(4, int)
    rank[order] = np.arange(4)
    idx = rank[labels.ravel()]
    # 相鄰中心差小於 MERGE 的併成一群；底色是像素最多的一群；比底色暗的群裡，最暗的一群若夠暗（≤ INK_MAX 或不到底色的 0.6）是墨線，其餘暗群是陰影；比底色亮的是高光
    group = np.zeros(4, int)
    g = 0
    for i in range(1, 4):
        if centers[i] - centers[i - 1] >= MERGE:
            g += 1
        group[i] = g
    gid = group[idx]
    counts = np.bincount(gid, minlength=g + 1)
    gmean = {i: float(vals[gid == i].mean()) for i in range(g + 1) if (gid == i).any()}
    base = max(gmean, key=lambda i: counts[i])
    level_of = {}
    darker = sorted(i for i in gmean if i < base)
    for i in gmean:
        if i == base:
            level_of[i] = "base"
        elif i > base:
            level_of[i] = "high"
        else:
            level_of[i] = "shadow"
    if darker and (gmean[darker[0]] <= INK_MAX or gmean[darker[0]] < 0.6 * gmean[base]):
        level_of[darker[0]] = "ink"
    gids = [i for i in range(g + 1) if (gid == i).any()]
    mean = {i: float(vals[gid == i].mean()) for i in gids}
    # 曲線的控制點：平台兩端 (亮度, 輸出灰階)
    xs, ys = [], []
    for k, i in enumerate(gids):
        gaps = []
        if k > 0:
            gaps.append(mean[i] - mean[gids[k - 1]])
        if k < len(gids) - 1:
            gaps.append(mean[gids[k + 1]] - mean[i])
        h = PLATEAU * min(gaps) if gaps else 0.0
        v = KEY_LEVELS[level_of[i]]
        xs += [mean[i] - h, mean[i] + h]
        ys += [v, v]
    xs = [0.0] + xs + [255.0]
    ys = [ys[0]] + ys + [ys[-1]]
    gray = np.interp(L, xs, ys)
    gray = np.where(mask, gray, 0).clip(0, 255).round().astype(np.uint8)
    out = np.dstack([gray, gray, gray, np.where(mask, img[..., 3], 0).astype(np.uint8)])
    stats = {"centers": [round(mean[i], 1) for i in gids], "share": [round(float(counts[i]) / counts.sum(), 3) for i in gids], "levels": [level_of[i] for i in gids]}
    return out, stats


def clip_face(rgba):
    """臉的肩畫到畫面邊緣，比衣領的肩線寬，疊上去會在領外露出一條膚色邊。把「頸柱以外、下巴以下、又不在衣領範圍內」的臉像素去掉：
    切線落在衣領底下或頸與肩的交界上，看不到；頭、耳、頸與領口裡的皮膚不動。這是裁切，不補像素。"""
    for sub in ("complete", "collect"):
        path = P.TMP + f"{sub}/collar-dao_plain.png"
        if os.path.exists(path):
            break
    else:
        return rgba
    collar = cv2.imread(path, cv2.IMREAD_UNCHANGED)[..., 3] > 40
    collar = cv2.dilate(collar.astype(np.uint8), cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (13, 13))) > 0
    lm = P.landmarks("m")
    h, w = rgba.shape[:2]
    yy, xx = np.mgrid[0:h, 0:w]
    neck_l, neck_r = lm["head"][0] + 10, lm["head"][1] - 10   # 頸柱大約與頭同寬再收一點
    outside = (yy > lm["chin"]) & ((xx < neck_l) | (xx > neck_r)) & ~collar
    out = rgba.copy()
    out[outside, 3] = 0
    out[outside, :3] = 0
    return out


def key(name):
    spec = P.PARTS[name]
    src = P.TMP + f"complete/{name}.png"
    if not os.path.exists(src):
        src = P.TMP + f"collect/{name}.png"
    rgba = cv2.cvtColor(cv2.imread(src, cv2.IMREAD_UNCHANGED), cv2.COLOR_BGRA2RGBA)
    if spec["slot"] == "face":
        rgba = clip_face(rgba)
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
