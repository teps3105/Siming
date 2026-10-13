"""收件：從畫家的整張輸出取出這一件部件。畫家把底版連同新畫的部件整張重畫，位置有些微縮放與平移，所以：

    1. 以底版沒被動到的區域（收件範圍之外）估出仿射對位，把畫家圖變換回底版座標（一致性修正，不改畫家畫的內容）；
    2. 與底版相減得到「畫家新畫的地方」，限制在收件範圍內，清掉對位殘留的細線與碎屑，填洞；
    3. 範圍內的像素取畫家圖的顏色，邊緣內縮一像素再羽化，避免帶進底版的紙色亮邊。

    python tools/parts/collect.py <部件> [--try N] [--thr 38]     → .shiftblame/tmp/parts/portrait/collect/<部件>.png（1024×1280 透明底）

沒有畫家的拆件、語義拆件（See-through）：實測見 8.12 回報——它不補畫被遮住的髮、把髮髻與髮際分成怪的兩層、每次 10 分鐘，不採用。
"""
import argparse
import json
import os
import sys

import cv2
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402


def load_rgb(path):
    return cv2.cvtColor(cv2.imread(path, cv2.IMREAD_COLOR), cv2.COLOR_BGR2RGB)


def register(painted, base, anchor):
    """估出 painted→base 的仿射（cv2.warpAffine 的 WARP_INVERSE_MAP 用法）。anchor 是底版沒被動到的區域遮罩。"""
    def gray(im, sigma):
        return cv2.GaussianBlur(cv2.cvtColor(im, cv2.COLOR_RGB2GRAY), (0, 0), sigma).astype(np.float32)
    w = np.eye(2, 3, dtype=np.float32)
    cc = 0.0
    for scale, sigma in ((0.25, 1.5), (0.5, 2.0), (1.0, 2.5)):
        size = (int(base.shape[1] * scale), int(base.shape[0] * scale))
        gp, gb = cv2.resize(gray(painted, sigma), size), cv2.resize(gray(base, sigma), size)
        mk = cv2.resize(anchor.astype(np.uint8) * 255, size, interpolation=cv2.INTER_NEAREST)
        w2 = w.copy()
        w2[:, 2] *= scale
        cc, w2 = cv2.findTransformECC(gb, gp, w2, cv2.MOTION_AFFINE, (cv2.TERM_CRITERIA_EPS | cv2.TERM_CRITERIA_COUNT, 300, 1e-6), mk, 5)
        w = w2.copy()
        w[:, 2] /= scale
    return w, float(cc)


def bg_color(img):
    """純色底的顏色：四個角的中位數。"""
    pts = np.concatenate([img[:20, :20].reshape(-1, 3), img[:20, -20:].reshape(-1, 3)])
    return np.median(pts, 0).astype(int)


def collect_face(key="face-m0", n=1):
    """臉：畫家把底版去掉內衫、補完頸肩、背景換純綠；以背景色解混去背，邊緣像素還原成前景色，不留亮邊。
    也把畫家原圖存成之後所有部件的底版 sources/parts/portrait/base-face-m.png。"""
    src = P.TMP + f"paint/{key}-t{n}.png"
    img = load_rgb(src)
    G = bg_color(img).astype(np.float32)
    ink = np.array(P.INK, np.float32)
    f = img.astype(np.float32)
    d = np.linalg.norm(f - G, axis=2)
    # 到底色的距離 → 不透明度：離底色夠遠（>90）算完全不透明；近底色處以「墨色與底色的混色」估算
    axis = ink - G
    a_mix = np.clip(((f - G) @ axis) / float(axis @ axis), 0, 1)
    a = np.where(d > 90, 1.0, np.clip(a_mix, 0, 1))
    a = np.where(d < 12, 0.0, a)
    fg = np.where(a[..., None] > 0.02, (f - (1 - a[..., None]) * G) / np.maximum(a[..., None], 0.02), 0).clip(0, 255)
    # 最大的一塊才是人；其餘（背景裡的雜訊）丟掉
    solid = (a > 0.5).astype(np.uint8)
    cnt, lab, st, _ = cv2.connectedComponentsWithStats(solid, connectivity=8)
    big = 1 + int(np.argmax(st[1:, cv2.CC_STAT_AREA]))
    a = np.where(cv2.dilate((lab == big).astype(np.uint8), np.ones((5, 5), np.uint8)) > 0, a, 0)
    out = np.dstack([fg, a * 255]).round().astype(np.uint8)
    out[out[..., 3] == 0, :3] = 0
    os.makedirs(P.TMP + "collect", exist_ok=True)
    path = P.TMP + f"collect/{key}.png"
    cv2.imwrite(path, cv2.cvtColor(out, cv2.COLOR_RGBA2BGRA))
    import shutil
    shutil.copy(src, P.base_path(P.PARTS[key]["sex"]))
    info = {"from": f"paint/{key}-t{n}.png", "bg": G.astype(int).tolist(), "pixels": int((a > 0.5).sum()), "output": path}
    P.log(key, "collect", info)
    print(key, info)
    return path


def collect(key, n=1, thr=38, min_area=250, thick=6.0, keep_skin=False):
    spec = P.PARTS[key]
    sex = spec["sex"]
    base = load_rgb(P.input_path(key))
    painted = load_rgb(P.TMP + f"paint/{key}-t{n}.png")
    h, wd = base.shape[:2]
    zone = P.zone_mask(spec["zone"], sex, (h, wd))
    anchor = P.anchor_mask(spec["zone"], sex, (h, wd))
    mat, cc = register(painted, base, anchor)
    warped = cv2.warpAffine(painted, mat, (wd, h), flags=cv2.INTER_LINEAR + cv2.WARP_INVERSE_MAP, borderMode=cv2.BORDER_REPLICATE)
    d = np.abs(cv2.GaussianBlur(warped, (0, 0), 1).astype(int) - cv2.GaussianBlur(base, (0, 0), 1).astype(int)).max(2)
    raw = (d > thr) & zone
    if not keep_skin:   # 對位殘留會把底版膚色的弧線與碎邊算進來；本件不是膚色的東西，膚色像素直接不收
        hsv = cv2.cvtColor(warped, cv2.COLOR_RGB2HSV).astype(np.float32)
        hue, sat, val = hsv[..., 0] * 2, hsv[..., 1] / 255, hsv[..., 2] / 255
        raw &= ~((hue > 10) & (hue < 45) & (sat > 0.18) & (val > 0.55))
    if spec.get("extract") == "color":
        # 新畫的東西與底版被它蓋住的東西明度色相相近時（灰青道袍蓋灰青內衫），相減會留洞；改取「不是膚色、不是紙色」的像素，
        # 以 7×7 開運算切掉頸輪廓之類的細線，再向外長 4 像素與原遮罩取交集，把部件自己的墨邊找回來。
        hsv = cv2.cvtColor(warped, cv2.COLOR_RGB2HSV).astype(np.float32)
        hue, sat, val = hsv[..., 0] * 2, hsv[..., 1] / 255, hsv[..., 2] / 255
        skin = (hue > 16) & (hue < 45) & (sat > 0.18) & (sat < 0.62) & (val > 0.5)
        paper = np.abs(warped.astype(int) - bg_color(base)[None, None]).max(2) < 60   # 純綠底
        cand = (zone & ~skin & ~paper)
        core = cv2.morphologyEx(cand.astype(np.uint8), cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (9, 9)))
        grown = cv2.dilate(core, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (9, 9)))
        raw = cand & (grown > 0)
    m = cv2.morphologyEx(raw.astype(np.uint8), cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
    m = cv2.morphologyEx(m, cv2.MORPH_CLOSE, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
    count, labels, stats, _ = cv2.connectedComponentsWithStats(m, connectivity=8)
    keep = np.zeros_like(m)
    for i in range(1, count):
        if stats[i, cv2.CC_STAT_AREA] >= min_area:
            comp = (labels == i).astype(np.uint8)
            if cv2.distanceTransform(comp, cv2.DIST_L2, 3).max() >= thick:   # 對位殘留的細弧線最粗只有幾像素，真的部件有一塊實心
                keep[comp > 0] = 1
    if spec.get("rescue"):
        # 下眼瞼之類的細線過不了開運算與厚度門檻：與實心塊相距 10 像素內的細元件一併收回（只用在眼）
        near = cv2.dilate(keep, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (21, 21)))
        rc, rl, rs, _ = cv2.connectedComponentsWithStats(raw.astype(np.uint8), connectivity=8)
        for i in range(1, rc):
            if rs[i, cv2.CC_STAT_AREA] >= 25 and (near[rl == i] > 0).all():
                keep[rl == i] = 1
    # 填洞：外輪廓內全部算部件（髮內的高光、眼白之類），但本來露出底版的洞（領口 V）留著
    holes = np.zeros_like(keep)
    cnts, hier = cv2.findContours(keep, cv2.RETR_CCOMP, cv2.CHAIN_APPROX_NONE)
    for i, c in enumerate(cnts):
        if hier[0][i][3] != -1 and cv2.contourArea(c) < 400:   # 小洞填掉，大洞保留
            cv2.drawContours(holes, [c], -1, 1, -1)
    keep = np.maximum(keep, holes)
    inner = cv2.erode(keep, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3)))
    alpha = cv2.GaussianBlur(inner.astype(np.float32), (0, 0), 0.8)
    # 外緣一圈的顏色混著底版（綠或膚色），改取最近的內部像素的顏色（墨線部件的外緣就是墨色），不帶亮邊也不帶綠邊
    core = cv2.erode(keep, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
    if core.any():
        _, lab = cv2.distanceTransformWithLabels((core == 0).astype(np.uint8), cv2.DIST_L2, 3, labelType=cv2.DIST_LABEL_PIXEL)
        ys0, xs0 = np.nonzero(core)
        pal = np.zeros((lab.max() + 1, 3), np.uint8)
        pal[lab[ys0, xs0]] = warped[ys0, xs0]
        warped = np.where((core > 0)[..., None], warped, pal[lab])
    out = np.dstack([warped, (alpha * 255).round().astype(np.uint8)])
    out[out[..., 3] == 0, :3] = 0
    os.makedirs(P.TMP + "collect", exist_ok=True)
    path = P.TMP + f"collect/{key}.png"
    cv2.imwrite(path, cv2.cvtColor(out, cv2.COLOR_RGBA2BGRA))
    ys, xs = np.nonzero(keep)
    info = {"from": f"paint/{key}-t{n}.png", "affine": mat.round(5).tolist(), "ecc": round(cc, 5), "thr": thr, "min_area": min_area, "thick": thick, "keep_skin": keep_skin,
            "pixels": int(keep.sum()), "bbox": [int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1] if len(xs) else None,
            "components": int(cv2.connectedComponents(keep, connectivity=8)[0] - 1), "output": path}
    P.log(key, "collect", info)
    print(key, {k: v for k, v in info.items() if k != "affine"}, "scale", round(float(np.sqrt(abs(np.linalg.det(mat[:, :2])))), 4), "shift", mat[:, 2].round(1))
    return path


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("part")
    ap.add_argument("--try", dest="n", type=int, default=1)
    ap.add_argument("--thr", type=int, default=38)
    ap.add_argument("--thick", type=float, default=6.0)
    ap.add_argument("--keep-skin", action="store_true")
    a = ap.parse_args()
    if P.PARTS[a.part].get("zone") is None:
        collect_face(a.part, a.n)
    else:
        collect(a.part, a.n, a.thr, thick=a.thick, keep_skin=a.keep_skin)
