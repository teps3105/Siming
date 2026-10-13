"""排圖集、寫目錄：把 key/ 裡所有部件子層裁邊後排進圖集，寫 game/modules/parts/resources/portrait_trial/{atlas.png,catalog.json}。

    python tools/parts/pack.py

格式與遊戲內現有的相同（8.12「圖集與目錄的格式」、PartCatalog._read）：
- atlas.png：RGBA，RGB 預乘 alpha；明度鍵子層的 RGB 是連續灰階（墨 0x00、陰影 0x55、底色 0xAA、高光 0xFF 為平台，之間線性過渡）；子層之間留 4 px。
- catalog.json：version／class／canvas／pivot／halo／atlas／channels／slots／parts；
  部件 {anchor, layers:[{rect, offset, mode: keyed|fixed, tint}]}；代號 <類>/<格>.<款>。
  class 寫 portrait_trial（真部件先放試驗類）；不寫 placeholder。
"""
import json
import os
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402

DEST = P.ROOT + f"/game/modules/parts/resources/{P.CLASS}/"
PAD = 4
ATLAS_W = 1024


def trim(im):
    a = np.asarray(im)[..., 3]
    ys, xs = np.nonzero(a > 0)
    box = (int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1)
    return im.crop(box), box[:2]


def premultiply(im):
    a = np.asarray(im).astype(np.float32)
    a[..., :3] = a[..., :3] * a[..., 3:] / 255
    return Image.fromarray(a.round().astype(np.uint8))


def main():
    os.makedirs(DEST, exist_ok=True)
    items = []
    for name, spec in P.PARTS.items():
        if name in P.RETIRED:
            continue
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
        atlas.paste(premultiply(im), (ax, ay))
        spec = P.PARTS[name]
        fixed = sub in spec.get("fixed", [])
        entry = parts.setdefault(P.code_of(name), {"anchor": None, "layers": [], "_boxes": []})
        entry.setdefault("_subs", []).append(list(spec["subs"]).index(sub))
        entry["layers"].append({"rect": [ax, ay, im.width, im.height], "offset": list(off), "mode": "fixed" if fixed else "keyed",
                                "tint": None if fixed else spec["dye"][sub]})
        entry["_boxes"].append((off[0], off[1], off[0] + im.width, off[1] + im.height))
    for key, fits in P.HAIRS.items():   # 冠帽可搭配的髮式
        parts[P.code_of(key)]["hairs"] = list(fits)
    for e in parts.values():   # 對位參考點：各子層外框聯集的中心
        b = e.pop("_boxes")
        e["anchor"] = [round((min(t[0] for t in b) + max(t[2] for t in b)) / 2), round((min(t[1] for t in b) + max(t[3] for t in b)) / 2)]
    # 順序在字典裡由後往前：子層依 SLOT 順序已由 PARTS 宣告順序決定（同一款的子層：主材質在前、緣邊在後）
    for code, e in parts.items():
        order_of = e.pop("_subs")
        if P.PARTS[next(k for k in P.PARTS if P.code_of(k) == code)].get("extract") == "line":   # 細線件：依 subs 宣告順序（唇在下、墨線在上）
            e["layers"] = [l for _, l in sorted(zip(order_of, e["layers"]), key=lambda t: t[0])]
        else:
            e["layers"].sort(key=lambda l: {"trim": 1, "gold": 2}.get(l["tint"], 0))
    lm = json.load(open(P.SRC + "base.json", encoding="utf-8"))["sex"]["m"]["points"]
    cx, top, chin = lm["cx"]["out"], lm["top_y"]["out"], lm["chin_y"]["out"]
    counts = {}
    for code in parts:
        slot = code.split("/")[1].split(".")[0]
        counts[slot] = counts.get(slot, 0) + 1
    catalog = {
        "version": 1, "class": P.CLASS, "canvas": list(P.OUT),
        "pivot": [cx, chin - 8],   # 縮放的中心：下巴略上方的頸線（佔位目錄的 [160,250] 同一位置，依真底版的下巴 261 換算）
        "halo": {"center": [cx, round((top + chin) / 2)], "radius": 92, "step": 8, "width": 3},   # 中心取頭的中心；半徑、圈距、線寬沿用佔位目錄
        "atlas": "atlas.png", "channels": P.CHANNELS,
        "slots": [{"id": sid, "order": order, "count": counts.get(sid, 0)} for sid, order, _ in P.SLOTS if counts.get(sid, 0) > 0],
        "parts": parts,
    }
    atlas.save(DEST + "atlas.png", optimize=True)
    json.dump(catalog, open(DEST + "catalog.json", "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print("圖集", atlas.size, os.path.getsize(DEST + "atlas.png"), "bytes；目錄", os.path.getsize(DEST + "catalog.json"), "bytes；部件", len(parts))


if __name__ == "__main__":
    main()
