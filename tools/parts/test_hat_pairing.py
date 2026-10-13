"""冠帽與髮式搭配的量測測試：戴冠帽時，帽外露出的前髮像素不超過門檻（8.12「冠帽與髮式的搭配」）。

    python tools/parts/test_hat_pairing.py

儒巾配髮髻：前髮的左鬢角仍有約 300 像素在帽緣下、耳前露出（每款前髮的鬢角都在，換髮式也解不掉；要根治得另畫沒有鬢角的前髮），門檻定在現況，不得更差。
"""
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402

LIMIT = {("hat-scholar0", "topknot"): 320}   # 小冠與斗笠的前髮本來就垂在帽外，只量遮住頭頂的儒巾


def solid(name, sub):
    return np.asarray(Image.open(P.TMP + f"key/{name}.{sub}.png"))[..., 3] > 60


def main():
    for hat, styles in P.HAIRS.items():
        cover = solid(hat, "hat")
        for style in styles:
            if (hat, style) not in LIMIT:
                continue
            out = int((solid(f"front_hair-{style}", "hair") & ~cover).sum())
            print(hat, style, "帽外前髮像素", out)
            assert out <= LIMIT[(hat, style)], f"{hat} 配 {style}：帽外前髮 {out} 像素超過門檻 {LIMIT[(hat, style)]}"
    print("搭配測試通過")


if __name__ == "__main__":
    main()
