"""補完：被別的部件或底版遮住、切斷的地方，交給畫家畫完整。

    python tools/parts/complete.py <部件> [--try N]

1. 輸入圖＝收件結果放在純綠色底上（範圍之外的底色是中性灰，畫家把它當沒畫完的地方），連同畫風基準圖送畫（paint.paint，同一個佇列）；
   指示取 portrait.COMPLETE[部件]：這是什麼、哪裡沒畫完、要畫成什麼、不許動什麼。
2. 收件：畫家圖與輸入圖以收件結果為錨做仿射對位（畫家常有些微縮放與平移），取與灰底差異大的像素，清碎屑、填小洞，
   寫 complete/<部件>.png（1024×1280 透明底）。範圍外維持畫家畫的外形，不以原圖像素補。
"""
import argparse
import os
import sys

import cv2
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402
import paint as PT  # noqa: E402
from collect import register, load_rgb  # noqa: E402

BG = (6, 186, 0)   # 補完的底色：純綠（灰階稿與灰青道袍都不是這個色）


def build_input(name):
    part = cv2.cvtColor(cv2.imread(P.TMP + f"collect/{name}.png", cv2.IMREAD_UNCHANGED), cv2.COLOR_BGRA2RGBA)
    a = part[..., 3:].astype(np.float32) / 255
    canvas = np.full(part.shape[:2] + (3,), BG, np.float32)
    canvas = canvas * (1 - a) + part[..., :3] * a
    path = P.TMP + f"paint/{name}-complete-in.png"
    cv2.imwrite(path, cv2.cvtColor(canvas.round().astype(np.uint8), cv2.COLOR_RGB2BGR))
    return path, part[..., 3] > 128


def complete(name, n=1, seed=None, thr=26, thick=6.0):
    inp, solid = build_input(name)
    tag = f"{name}-complete-t{n}"
    out = PT.paint(tag, inp, P.STYLE, P.COMPLETE[name], seed or 20261100 + n, record=name)
    a, b = load_rgb(inp), load_rgb(out)
    anchor = cv2.dilate(solid.astype(np.uint8), np.ones((9, 9), np.uint8)).astype(bool)
    try:
        mat, cc = register(b, a, anchor)
    except cv2.error:   # 錨（部件本身）太小時 ECC 不收斂，改用不對位，由人眼核對位置
        mat, cc = np.eye(2, 3, dtype=np.float32), -1.0
    warped = cv2.warpAffine(b, mat, (a.shape[1], a.shape[0]), flags=cv2.INTER_LINEAR + cv2.WARP_INVERSE_MAP, borderValue=BG)
    d = np.abs(cv2.GaussianBlur(warped, (0, 0), 1).astype(int) - np.array(BG)[None, None]).max(2)
    m = (d > thr).astype(np.uint8)
    m = cv2.morphologyEx(m, cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
    count, labels, stats, _ = cv2.connectedComponentsWithStats(m, connectivity=8)
    keep = np.zeros_like(m)
    for i in range(1, count):
        comp = (labels == i).astype(np.uint8)
        if stats[i, cv2.CC_STAT_AREA] >= 250 and cv2.distanceTransform(comp, cv2.DIST_L2, 3).max() >= thick:
            keep[comp > 0] = 1
    cnts, hier = cv2.findContours(keep, cv2.RETR_CCOMP, cv2.CHAIN_APPROX_NONE)
    for i, c in enumerate(cnts):
        if hier[0][i][3] != -1 and cv2.contourArea(c) < 400:
            cv2.drawContours(keep, [c], -1, 1, -1)
    alpha = cv2.GaussianBlur(cv2.erode(keep, np.ones((3, 3), np.uint8)).astype(np.float32), (0, 0), 0.8)
    res = np.dstack([warped, (alpha * 255).round().astype(np.uint8)])
    res[res[..., 3] == 0, :3] = 0
    os.makedirs(P.TMP + "complete", exist_ok=True)
    path = P.TMP + f"complete/{name}.png"
    cv2.imwrite(path, cv2.cvtColor(res, cv2.COLOR_RGBA2BGRA))
    before, after = int(solid.sum()), int(keep.sum())
    ys, xs = np.nonzero(keep)
    P.log(name, "complete", {"from": out, "ecc": round(cc, 5), "pixels_before": before, "pixels_after": after, "bbox": [int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1], "output": path})
    print(name, "補完前", before, "補完後", after)
    return path


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("part")
    ap.add_argument("--try", dest="n", type=int, default=1)
    a = ap.parse_args()
    complete(a.part, a.n)
