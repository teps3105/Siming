"""排圖集、寫目錄：把 key/ 裡所有部件子層裁邊後排進圖集，寫 game/modules/parts/resources/portrait/{atlas0.png,catalog.json}。

    python tools/parts/pack.py

目錄格式（catalog.json，與 parts-runtime 尚未對齊，主對話合併時以這份欄位為準）：
{
  "version": 1, "class": "portrait",
  "size": [320, 400],                     # 入庫畫布尺寸；每個部件子層的 offset 是它在這張畫布上的左上角
  "ink": "#2B2620",                       # 墨線色，著色器不染（明度鍵 0 階）
  "key_levels": {"ink": 0, "shadow": 90, "base": 170, "high": 255},  # 鍵圖 R 通道的四階；著色器以 45/130/212 為界歸階
  "slots": ["hair_back", "collar", ...],  # 疊圖順序，由後往前（下標即 z）
  "anchors": {"m": {...}, "f": {...}},    # 底版定位點（入庫座標）
  "atlases": [{"file": "atlas0.png", "size": [w, h]}],
  "parts": {
    "hair_front.bun": {                   # 部件代號是 portrait/<此鍵>
      "slot": "hair_front", "sex": "m",
      "layers": [ {"sub": "hair", "mode": "key", "dye": "髮色",   # mode: key＝明度鍵要染色；fixed＝定色，著色器照原色畫
                   "atlas": 0, "rect": [x, y, w, h],            # 在圖集上的矩形（不含邊距）
                   "offset": [ox, oy]} ]                          # 該矩形在 320×400 畫布上的左上角
    }
  }
}
鍵圖與定色圖都是 RGBA，alpha 為邊緣羽化；鍵圖 RGB 三通道相同（灰階）。
"""
import json
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402
from key import KEY_LEVELS  # noqa: E402

DEST = P.ROOT + "/game/modules/parts/resources/portrait/"
PAD = 2
ATLAS_W = 1024


def trim(im):
    a = np.asarray(im)[..., 3]
    ys, xs = np.nonzero(a > 0)
    box = (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)
    return im.crop(box), box[:2]


def main():
    os.makedirs(DEST, exist_ok=True)
    items = []
    for name, spec in P.PARTS.items():
        for sub in spec["subs"]:
            path = P.TMP + f"key/{name}.{sub}.png"
            if not os.path.exists(path):
                continue
            im, off = trim(Image.open(path).convert("RGBA"))
            items.append((name, sub, im, off))
    items.sort(key=lambda it: -it[2].height)
    x = y = row = 0
    place = []
    for name, sub, im, off in items:   # 逐列排（由高到矮）
        w, h = im.size
        if x + w + PAD > ATLAS_W:
            x, y, row = 0, y + row + PAD, 0
        place.append((name, sub, im, off, x, y))
        x, row = x + w + PAD, max(row, h)
    H = 1
    while H < y + row:
        H *= 2
    atlas = Image.new("RGBA", (ATLAS_W, H), (0, 0, 0, 0))
    parts = {}
    for name, sub, im, off, ax, ay in place:
        atlas.paste(im, (ax, ay))
        spec = P.PARTS[name]
        slot, _, style = name.partition("-")
        entry = parts.setdefault(f"{slot}.{style}", {"slot": slot, "sex": spec["sex"], "layers": []})
        entry["layers"].append({"sub": sub, "mode": "fixed" if sub in spec.get("fixed", []) else "key", "dye": spec["dye"].get(sub, "定色"),
                                "atlas": 0, "rect": [ax, ay, im.width, im.height], "offset": list(off)})
    atlas.save(DEST + "atlas0.png", optimize=True)
    base = json.load(open(P.SRC + "base.json", encoding="utf-8"))
    anchors = {s: {k: ({side: w["out"] for side, w in v.items()} if k == "ears" else v["out"]) for k, v in e["points"].items()} for s, e in base["sex"].items()}
    catalog = {"version": 1, "class": "portrait", "size": list(P.OUT), "ink": "#2B2620", "key_levels": KEY_LEVELS, "slots": P.SLOTS, "anchors": anchors,
               "atlases": [{"file": "atlas0.png", "size": list(atlas.size)}], "parts": parts}
    json.dump(catalog, open(DEST + "catalog.json", "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print("圖集", atlas.size, os.path.getsize(DEST + "atlas0.png"), "bytes；目錄", os.path.getsize(DEST + "catalog.json"), "bytes；部件", len(parts))


if __name__ == "__main__":
    main()
