"""戴任一髮式時，頭頂不露頭皮：每一欄裡，臉（膚色）高過該欄最高的髮像素者算露頭皮；剃度款例外（試驗類尚無剃度）。

    python tools/parts/test_scalp.py
"""
import glob
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402

SEAM = 12   # 髮髻與前髮之間夾著膚色的像素上限
LIMIT = 60   # 像素；超過就是頭頂外面露出一弧膚色。短髮前髮送畫三次，第二次最好（瘦長臉 58），第三次更差（151），取第二次


def solid(name, sub):
    f = P.TMP + f"key/{name}.{sub}.png"
    return np.asarray(Image.open(f))[..., 3] > 60 if os.path.exists(f) else None


def exposed(face, style):
    parts = [m for m in (solid(f"front_hair-{style}", "hair"), solid(f"back_hair-{style}", "hair")) if m is not None]
    hair = np.logical_or.reduce(parts)
    cols = np.nonzero(hair.any(0))[0]
    top = np.where(hair.any(0), hair.argmax(0), 10 ** 6)
    ys = np.arange(face.shape[0])[:, None]
    over = face & (ys < top[None, :])
    over[:, : cols.min()] = False
    over[:, cols.max() + 1:] = False
    return int(over.sum())


def seam(face, style, rows=130):
    """髮縫：同一欄裡上面有髮、下面（眼線以上）也有髮，中間夾著膚色——髮髻與前髮之間的一弧膚色。"""
    parts = [m for m in (solid(f"front_hair-{style}", "hair"), solid(f"back_hair-{style}", "hair")) if m is not None]
    hair = np.logical_or.reduce(parts)[:rows]
    above = np.maximum.accumulate(hair, 0)
    below = np.maximum.accumulate(hair[::-1], 0)[::-1]
    return int((face[:rows] & ~hair & above & below).sum())


def main():
    styles = sorted(f.split("front_hair-")[1].split(".")[0] for f in glob.glob(P.TMP + "key/front_hair-*.hair.png"))
    styles = [s for s in styles if f"front_hair-{s}" not in P.RETIRED and "shaven" not in s]
    bad = []
    for face in sorted(f.split("/")[-1].split("\\")[-1].split(".")[0] for f in glob.glob(P.TMP + "key/face-m*.skin.png")):
        fm = solid(face, "skin")
        for s in styles:
            n = exposed(fm, s)
            g = seam(fm, s)
            print(face, s, "露頭皮像素", n, "髮縫像素", g)
            if g > SEAM:
                bad.append((face, s, "髮縫", g))
            if n > LIMIT:
                bad.append((face, s, n))
    assert not bad, f"頭頂露頭皮超過 {LIMIT} 像素：{bad}"
    print("頭頂測試通過")


if __name__ == "__main__":
    main()
