"""並排圖：每款一欄、三種膚色三列，只放大臉中段。

    python tools/parts/side.py <輸出.png> <部件>[,<部件>…]
"""
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import montage as M  # noqa: E402

out, parts = sys.argv[1], sys.argv[2].split(",")
rows = []
for si, skin in enumerate(M.SKINS):
    row = [M.render(["face-m0", p], M.palette(skin, M.HAIRS[0], si))[90:290, 60:260] for p in parts]
    rows.append(np.concatenate(row, 1))
im = Image.fromarray(np.concatenate(rows, 0))
im.resize((im.width * 2, im.height * 2), Image.NEAREST).save(out)
