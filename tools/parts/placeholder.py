#!/usr/bin/env python3
"""修士頭像的佔位部件（GDD 8.12）：照部件表的 12 格 70 款，用程式畫簡單的明度鍵形狀，
排成圖集 atlas.png 與目錄 catalog.json，放進 game/modules/parts/resources/portrait/。

這只是讓疊圖節點與人物卡先能跑的臨時資料：每一款的形狀看得出不同，左上角印著代號。
圖集與目錄的格式就是日後真部件要用的格式（見 8.12「目錄」），真部件由 tools/parts/pack.py 以同一格式輸出。

用法：python tools/parts/placeholder.py
需要 Pillow。輸出是確定的：同一份程式兩次輸出逐位元組相同。
"""
import json
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "game" / "modules" / "parts" / "resources" / "portrait"

W, H = 320, 400
S = 2  # 超取樣倍率
PIVOT = [160, 250]
HALO = {"center": [160, 170], "radius": 92, "step": 8, "width": 3}

# 明度鍵的四階灰：墨線、陰影、底色、高光。
INK, SHADOW, BASE, LIGHT = 0x00, 0x55, 0xAA, 0xFF
INK_RGB = (0x2B, 0x26, 0x20)
SCLERA = (250, 248, 240)

CHANNELS = ["skin", "hair", "robe", "trim", "iris", "lip", "violet", "gold"]
# 由後往前；count 與 8.12 的部件表一致。
SLOTS = [
    ("back_hair", 8), ("collar", 12), ("face", 6), ("wrinkle", 2), ("mouth", 6), ("eyes", 8),
    ("brows", 6), ("beard", 5), ("mark", 3), ("front_hair", 8), ("hat", 6),
]
HAIR_NAMES = ["topknot", "crown", "loose", "hat", "twin_buns", "high_bun", "shaven", "scholar_cap"]
SCHOOLS = ["none", "ru", "shi", "dao"]
WEALTH = ["poor", "plain", "rich"]
BEARDS = ["moustache", "goatee", "three", "five", "full"]
HATS = ["straw0", "straw1", "crown0", "crown1", "scholar0", "scholar1"]


# ── 畫布 ──────────────────────────────────────────────────

class Layer:
    """一張子層：顏色與透明度各一張圖；keyed 的顏色是灰階。"""

    def __init__(self, tint, mode):
        self.tint = tint
        self.mode = mode
        self.rgb = Image.new("RGB", (W * S, H * S), (0, 0, 0))
        self.alpha = Image.new("L", (W * S, H * S), 0)

    def paint(self, mask, color):
        if isinstance(color, int):
            color = (color, color, color)
        self.rgb.paste(color, mask=mask)
        self.alpha.paste(255, mask=mask)

    def solid(self, mask, shade=(-9, -7)):
        """底色平塗、一階陰影、墨線勾邊（外粗內細不做，單線）。"""
        self.paint(mask, BASE)
        if shade:
            shifted = ImageChops.offset(mask, shade[0] * S, shade[1] * S)
            self.paint(ImageChops.subtract(mask, shifted), SHADOW)
        ring = ImageChops.subtract(mask.filter(ImageFilter.MaxFilter(7)), mask.filter(ImageFilter.MinFilter(7)))
        self.paint(ring, INK)

    def ink(self, mask):
        self.paint(mask, INK)


def _new():
    return Image.new("L", (W * S, H * S), 0)


def ell(cx, cy, rx, ry):
    m = _new()
    ImageDraw.Draw(m).ellipse([(cx - rx) * S, (cy - ry) * S, (cx + rx) * S, (cy + ry) * S], fill=255)
    return m


def poly(pts):
    m = _new()
    ImageDraw.Draw(m).polygon([(x * S, y * S) for x, y in pts], fill=255)
    return m


def box(x0, y0, x1, y1):
    m = _new()
    ImageDraw.Draw(m).rounded_rectangle([x0 * S, y0 * S, x1 * S, y1 * S], radius=6 * S, fill=255)
    return m


def stroke(pts, w):
    m = _new()
    d = ImageDraw.Draw(m)
    d.line([(x * S, y * S) for x, y in pts], fill=255, width=int(w * S), joint="curve")
    for x, y in (pts[0], pts[-1]):
        d.ellipse([(x - w / 2) * S, (y - w / 2) * S, (x + w / 2) * S, (y + w / 2) * S], fill=255)
    return m


def union(*masks):
    out = masks[0]
    for m in masks[1:]:
        out = ImageChops.lighter(out, m)
    return out


def mirror(pts):
    return [(W - x, y) for x, y in reversed(pts)]


# ── 各格的畫法 ────────────────────────────────────────────

def back_hair(i):
    cap = ell(160, 150, 68, 84)
    extra = {
        0: ell(160, 68, 24, 24),
        1: box(128, 56, 192, 92),
        2: union(poly([(84, 150), (126, 150), (120, 345), (78, 335)]), poly([(236, 150), (194, 150), (200, 345), (242, 335)])),
        3: ell(160, 140, 50, 40),
        4: union(ell(106, 84, 25, 25), ell(214, 84, 25, 25)),
        5: ell(160, 48, 34, 34),
        6: None,
        7: box(104, 54, 216, 112),
    }[i]
    if i == 6:
        cap = ell(160, 150, 63, 80)
    layer = Layer("hair", "keyed")
    layer.solid(union(cap, extra) if extra is not None else cap)
    return [layer]


def front_hair(i):
    layer = Layer("hair", "keyed")
    fringe = ell(160, 104, 64, 26)
    shapes = {
        0: fringe,
        1: union(fringe, box(116, 70, 204, 96)),
        2: union(fringe, poly([(94, 110), (120, 110), (116, 230), (92, 226)]), poly([(226, 110), (200, 110), (204, 230), (228, 226)])),
        3: ell(160, 110, 54, 14),
        4: union(ell(160, 100, 66, 22), poly([(150, 100), (170, 100), (160, 140)])),
        5: ell(160, 104, 66, 20),
        6: ell(160, 98, 20, 9),
        7: box(98, 86, 222, 108),
    }
    layer.solid(shapes[i], shade=(-6, -5))
    return [layer]


def collar(i):
    school, wealth = divmod(i, 3)
    notch = {
        0: [(128, 290), (144, 304), (160, 310)],
        1: [(128, 288), (160, 352)],
        2: [(120, 290), (148, 340), (160, 346)],
        3: [(134, 284), (160, 284)],
    }[school]
    left = [(14, 400), (28, 330), (92, 292)] + notch
    body = poly(left + mirror(left))
    robe = Layer("robe", "keyed")
    robe.solid(body, shade=(-10, -10))
    if school == 1:  # 儒：交領的兩道斜線
        robe.ink(stroke([(112, 288), (160, 352), (208, 288)], 3))
    if school == 2:  # 釋：袈裟的斜帶
        robe.ink(stroke([(70, 300), (250, 400)], 3))
    if school == 3:  # 道：中縫
        robe.ink(stroke([(160, 284), (160, 400)], 3))
    trim = Layer("trim", "keyed")
    width = [3, 7, 13][wealth]
    line = stroke([(92, 292)] + notch, width)
    both = union(line, line.transpose(Image.FLIP_LEFT_RIGHT))
    both = ImageChops.multiply(both, body)
    trim.solid(both, shade=None)
    if wealth == 2:
        trim.solid(ImageChops.multiply(box(14, 380, 306, 396), body), shade=None)
    return [robe, trim]


def face(i):
    male = i < 3
    k = i % 3
    rx, ry = ([(62, 78), (58, 82), (66, 74)] if male else [(54, 76), (56, 80), (52, 72)])[k]
    nw = 26 if male else 20
    head = union(ell(160, 168, rx, ry), box(160 - nw, 200, 160 + nw, 302), ell(160 - rx, 175, 8, 14), ell(160 + rx, 175, 8, 14))
    layer = Layer("skin", "keyed")
    layer.solid(head, shade=(-8, -6))
    return [layer]


def wrinkle(i):
    layer = Layer(None, "fixed")
    ink = (INK_RGB[0], INK_RGB[1], INK_RGB[2])
    marks = [
        union(stroke([(120, 112), (160, 108), (200, 112)], 2), stroke([(124, 122), (160, 118), (196, 122)], 2),
              stroke([(128, 196), (136, 224)], 2), stroke([(192, 196), (184, 224)], 2)),
        union(stroke([(112, 150), (100, 142)], 2), stroke([(112, 156), (98, 158)], 2), stroke([(208, 150), (220, 142)], 2),
              stroke([(208, 156), (222, 158)], 2), stroke([(130, 114), (190, 114)], 2)),
    ][i]
    layer.paint(marks, ink)
    return [layer]


def mouth(i):
    nose = Layer("skin", "keyed")
    nose_shapes = [
        poly([(160, 160), (150, 196), (170, 196)]),
        poly([(160, 150), (152, 200), (168, 200)]),
        poly([(160, 170), (140, 200), (180, 200)]),
        ell(160, 192, 12, 10),
        poly([(160, 160), (148, 190), (160, 204), (172, 190)]),
        box(146, 186, 174, 200),
    ]
    nose.solid(nose_shapes[i], shade=(-4, -4))
    lip = Layer("lip", "keyed")
    lips = [
        stroke([(140, 226), (180, 226)], 4),
        ell(160, 228, 20, 8),
        stroke([(140, 222), (160, 232), (180, 222)], 4),
        ell(160, 228, 9, 9),
        box(130, 222, 190, 232),
        stroke([(140, 232), (160, 222), (180, 232)], 4),
    ]
    lip.solid(lips[i], shade=None)
    return [nose, lip]


def eyes(i):
    female = i >= 4
    k = i % 4
    gap = [24, 28, 31, 26][k]
    rx, ry = [(13, 8), (15, 6), (11, 10), (14, 9)][k]
    white = Layer(None, "fixed")
    iris = Layer("iris", "keyed")
    for sx in (-1, 1):
        cx = 160 + sx * gap
        white.paint(ell(cx, 150, rx, ry), SCLERA)
        iris.solid(ell(cx, 150, ry - 1, ry - 1), shade=None)
        iris.ink(stroke([(cx - rx - 2, 150 - ry + (0 if k != 2 else 4)), (cx + rx + 2, 150 - ry - (2 if female else 0))], 2.5 if not female else 4))
    return [white, iris]


def brows(i):
    layer = Layer("hair", "keyed")
    spec = [
        ([(122, 126), (140, 122), (150, 126)], 6),
        ([(120, 128), (150, 128)], 3),
        ([(120, 130), (136, 118), (152, 128)], 5),
        ([(120, 122), (152, 130)], 7),
        ([(126, 128), (146, 126)], 8),
        ([(118, 132), (134, 124), (152, 124)], 4),
    ][i]
    left = stroke(spec[0], spec[1])
    layer.solid(union(left, left.transpose(Image.FLIP_LEFT_RIGHT)), shade=None)
    return [layer]


def beard(i):
    layer = Layer("hair", "keyed")
    mous = poly([(160, 210), (126, 214), (118, 224), (160, 218), (202, 224), (194, 214)])
    chin = poly([(148, 244), (172, 244), (166, 296), (154, 296)])
    cheeks = union(poly([(104, 190), (118, 190), (122, 250), (110, 250)]), poly([(216, 190), (202, 190), (198, 250), (210, 250)]))
    jaw = ImageChops.subtract(ell(160, 170, 68, 84), ell(160, 150, 52, 62))
    jaw = ImageChops.multiply(jaw, box(90, 180, 230, 300))
    shape = [mous, chin, union(chin, cheeks), union(chin, cheeks, mous), jaw][i]
    layer.solid(shape, shade=(-5, -5))
    return [layer]


def mark(i):
    layer = Layer("violet", "keyed")
    shapes = [
        union(poly([(160, 100), (148, 126), (172, 126)]), stroke([(112, 170), (126, 214)], 5), stroke([(208, 170), (194, 214)], 5)),
        union(stroke([(120, 100), (130, 140), (118, 170)], 5), stroke([(200, 100), (190, 140), (202, 170)], 5)),
        union(stroke([(160, 96), (150, 130), (168, 150), (160, 182)], 5), stroke([(110, 200), (130, 216)], 5), stroke([(210, 200), (190, 216)], 5)),
    ]
    layer.solid(shapes[i], shade=None)
    return [layer]


def hat(i):
    kind, rich = divmod(i, 2)
    robe = Layer("robe", "keyed")
    gold = Layer("gold", "keyed")
    if kind == 0:
        robe.solid(poly([(36, 112), (160, 8), (284, 112), (160, 124)]), shade=(-10, -8))
        gold.solid(stroke([(62, 106), (258, 106)], 5 + 4 * rich), shade=None)
    elif kind == 1:
        robe.solid(box(112, 24, 208, 96), shade=(-8, -6))
        gold.solid(union(ell(160, 20, 8 + 4 * rich, 8 + 4 * rich), stroke([(112, 92), (208, 92)], 4 + 4 * rich)), shade=None)
    else:
        robe.solid(union(box(104, 32, 216, 92), poly([(104, 40), (60, 70), (104, 80)]), poly([(216, 40), (260, 70), (216, 80)])), shade=(-8, -6))
        gold.solid(stroke([(104, 88), (216, 88)], 3 + 4 * rich), shade=None)
    return [robe, gold]


DRAW = {
    "back_hair": back_hair, "collar": collar, "face": face, "wrinkle": wrinkle, "mouth": mouth, "eyes": eyes,
    "brows": brows, "beard": beard, "mark": mark, "front_hair": front_hair, "hat": hat,
}


def tokens(slot):
    if slot in ("back_hair", "front_hair"):
        return HAIR_NAMES
    if slot == "collar":
        return [f"{s}_{w}" for s in SCHOOLS for w in WEALTH]
    if slot == "face":
        return [f"{sex}{k}" for sex in "mf" for k in range(3)]
    if slot == "eyes":
        return [f"{sex}{k}" for sex in "mf" for k in range(4)]
    if slot == "beard":
        return BEARDS
    if slot == "hat":
        return HATS
    return [str(k) for k in range(dict(SLOTS)[slot])]


# ── 輸出 ──────────────────────────────────────────────────

def _font(size):
    for name in ("arial.ttf", "DejaVuSans.ttf", "Arial.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default()


def finish(layer, label, row):
    """超取樣縮回原尺寸，轉成預乘 alpha 的 RGBA，並取出有內容的外框。"""
    if label:
        font = _font(11 * S)
        ImageDraw.Draw(layer.rgb).text((4 * S, (4 + 12 * row) * S), label, fill=INK_RGB if layer.mode == "fixed" else (0, 0, 0), font=font)
        mask = _new()
        ImageDraw.Draw(mask).text((4 * S, (4 + 12 * row) * S), label, fill=255, font=font)
        layer.alpha = ImageChops.lighter(layer.alpha, mask)
    straight = layer.rgb.convert("RGBA")
    straight.putalpha(layer.alpha)
    small = straight.resize((W, H), Image.LANCZOS)
    r, g, b, a = small.split()
    small = Image.merge("RGBA", (ImageChops.multiply(r, a), ImageChops.multiply(g, a), ImageChops.multiply(b, a), a))
    box_ = a.getbbox()
    return small.crop(box_), box_


def build():
    OUT.mkdir(parents=True, exist_ok=True)
    entries = []  # (code, layer index, 圖, 外框, tint, mode)
    parts = {}
    for row, (slot, count) in enumerate(SLOTS):
        names = tokens(slot)
        assert len(names) == count, slot
        for k, name in enumerate(names):
            layers = DRAW[slot](k)
            code = f"portrait/{slot}.{name}"
            meta = []
            for n, layer in enumerate(layers):
                label = f"{slot}.{name}" if n == len(layers) - 1 else None
                image, bbox = finish(layer, label, row)
                entries.append((code, n, image, bbox))
                meta.append({"tint": layer.tint, "mode": layer.mode, "offset": [bbox[0], bbox[1]]})
            parts[code] = {"anchor": [160, 168], "layers": meta}
    # 貨架式排版：高的先排，兩圖之間留 4 px 空白（縮小取樣時不滲色）。
    pad = 4
    atlas_w = 2048
    order = sorted(entries, key=lambda e: (-e[2].height, e[0], e[1]))
    x = y = shelf = 0
    place = {}
    for code, n, image, _ in order:
        if x + image.width + pad > atlas_w:
            x, y, shelf = 0, y + shelf + pad, 0
        place[(code, n)] = (x, y)
        x += image.width + pad
        shelf = max(shelf, image.height)
    atlas_h = y + shelf + pad
    atlas = Image.new("RGBA", (atlas_w, atlas_h), (0, 0, 0, 0))
    for code, n, image, _ in entries:
        px, py = place[(code, n)]
        atlas.paste(image, (px, py))
        layer_meta = parts[code]["layers"][n]
        layer_meta["rect"] = [px, py, image.width, image.height]
    atlas.save(OUT / "atlas.png", optimize=True)
    catalog = {
        "version": 1,
        "class": "portrait",
        "placeholder": True,
        "canvas": [W, H],
        "pivot": PIVOT,
        "halo": HALO,
        "atlas": "atlas.png",
        "channels": CHANNELS,
        "slots": [{"id": slot, "order": (n + 1) * 10, "count": count} for n, (slot, count) in enumerate(SLOTS)],
        "parts": parts,
    }
    (OUT / "catalog.json").write_text(json.dumps(catalog, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"{len(parts)} 款、{len(entries)} 張子層，圖集 {atlas_w}×{atlas_h} -> {OUT}")


if __name__ == "__main__":
    build()
