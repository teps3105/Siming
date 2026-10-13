"""修士頭像的部件表與指示模板（8.12「修士頭像」）。其他腳本都從這裡取部件代號、格、指示與定位。

部件代號：portrait/<格>.<款>；PARTS 的鍵是「格-款」（檔名用）。
座標：畫家畫布 1024×1280（CANVAS），入庫尺寸 320×400（OUT），比例 0.3125。
"""
import json
import os
import time

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))).replace("\\", "/")
SRC = ROOT + "/sources/parts/portrait/"
TMP = ROOT + "/.shiftblame/tmp/parts/portrait/"
STYLE = SRC + "style.png"
CANVAS = (1024, 1280)
OUT = (320, 400)
SCALE = OUT[0] / CANVAS[0]
PAPER = (0xED, 0xE4, 0xCF)
INK = (0x2B, 0x26, 0x20)   # 墨線色，著色器不染

# 疊圖順序（由後往前），與 8.12 的格一致；slot 的 z 序用這裡的下標
SLOTS = ["hair_back", "collar", "face", "wrinkle", "mouth", "eyes", "brows", "beard", "mark", "hair_front", "cap"]


def base_path(sex):
    return SRC + f"base-{sex}.png"


def log(part, kind, entry):
    """把一筆紀錄追加到 sources/parts/portrait/<部件>.json 的 kind 清單。"""
    path = SRC + part + ".json"
    data = json.load(open(path, encoding="utf-8")) if os.path.exists(path) else {"part": part}
    entry = {**entry, "time": time.strftime("%Y-%m-%d %H:%M:%S")}
    data.setdefault(kind, []).append(entry)
    json.dump(data, open(path, "w", encoding="utf-8"), ensure_ascii=False, indent=1)


def landmarks(sex):
    """底版定位點（畫家畫布 1024×1280 座標）。"""
    pts = json.load(open(SRC + "base.json", encoding="utf-8"))["sex"][sex]["points"]
    g = lambda k: pts[k]["canvas"]
    return {"cx": g("cx"), "top": g("top_y"), "eye": g("eye_y"), "nose": g("nose_y"), "mouth": g("mouth_y"), "chin": g("chin_y"), "neck": g("neck_y"),
            "head": pts["head_w"]["canvas"], "ear_top": pts["ears"]["l"]["canvas"][1], "ear_bot": pts["ears"]["l"]["canvas"][2]}


def zone_box(name, sex):
    """各部件的收件範圍（畫布座標 x0,y0,x1,y1）：只收範圍內畫家新畫的東西。"""
    lm = landmarks(sex)
    cx, W, H = lm["cx"], CANVAS[0], CANVAS[1]
    return {
        "head_top": (150, 0, W - 150, lm["ear_bot"] - 40),
        "eyes": (cx - 190, lm["eye"] - 70, cx + 190, lm["eye"] + 60),
        "brows": (cx - 200, lm["eye"] - 140, cx + 200, lm["eye"] + 5),
        "beard": (lm["head"][0] - 40, lm["mouth"] - 30, lm["head"][1] + 40, lm["chin"] + 280),
        "collar": (0, lm["chin"] - 40, W, H),
    }[name]


def zone_mask(name, sex, shape):
    import numpy as np
    x0, y0, x1, y1 = zone_box(name, sex)
    m = np.zeros(shape, bool)
    m[max(y0, 0):y1, max(x0, 0):x1] = True
    return m


def anchor_mask(name, sex, shape):
    """對位用：底版沒被動到的區域（收件範圍之外；領是頭部上半）。"""
    import numpy as np
    lm = landmarks(sex)
    if name == "collar":
        m = np.zeros(shape, bool)
        m[lm["top"] - 20:lm["chin"] - 60, lm["head"][0] - 60:lm["head"][1] + 60] = True
        return m
    return ~zone_mask(name, sex, shape)


# ---- 指示模板 ----
# 通用開頭與結尾：圖 1 是底版（保住），圖 2 是畫風基準。
HEAD = "圖1是一張光頭素臉的頭肩半身像底版，圖2是畫風基準圖。"
TAIL = ("畫風與圖2一致：賽璐璐水墨平塗，墨線勾邊（外輪廓粗、內部細，線色近墨色），底色純平，只加一階硬邊陰影，不用漸層、不用柔光、不要紙紋或顆粒。"
        "除了上面要求新畫或改動的這一件，人物的頭、臉、頸、背景與圖1逐像素相同，只做上面說的這一件事。"
        "背景保持圖1的純色宣紙色。不加任何文字、印章、邊框，不加其他飾品。")

SAME = ("整張圖的構圖、縮放、位置與圖1完全一致：不要放大縮小，不要平移，不要裁切。")
# 染色部件的色調：不畫成黑色，畫成中等明度的暖灰褐，使墨線、陰影、底色、高光四階分得開
TONE = "顏色不要畫成黑色：底色用淺中度的灰褐色（約 #8C7B6A，一眼看去是灰褐而不是黑），一階較深的硬邊陰影塊（約 #5E5145），少量淺色高光塊（約 #B5A692）；只有輪廓線與髮束線條用近黑的墨色"

PARTS = {
    # key: slot、sex、what（畫什麼）、how（怎麼畫）、zone（收件範圍，底版定位點的相對框）、
    #      subs（子層：名稱 → 判別方法，染色欄位）
    "hair_front-bun": {
        "slot": "hair_front", "sex": "m", "zone": "head_top",
        "what": "請在圖1的頭上畫「前髮」：貼著頭皮的整齊頭髮，覆蓋頭頂與前額，髮際線在額頭上方三分之一處，兩鬢收在耳朵前上方，耳朵完整露出。",
        "how": "只畫貼著頭皮的這一層髮，不畫髮髻、不畫髮簪、不畫垂到頭外面去的髮。髮束用少數幾條乾淨的線表示，不畫細碎髮絲。整片頭髮的外緣用墨線包成一圈封閉的形狀。" + TONE + "。眉眼口鼻、耳朵、臉與頸不許被蓋住。",
        "subs": {"hair": "all"}, "dye": {"hair": "髮色"}},
    "hair_back-bun": {
        "slot": "hair_back", "sex": "m", "zone": "head_top",
        "what": "請在圖1的頭後面畫「後髮」：頭頂偏後挽起的一個圓髻，髻上橫插一支簡單的木簪。髻從頭頂後方露出來，頭頂以上整個髻都看得見。",
        "how": "頭本身（光頭的頭皮、額、臉、耳）完全不要被蓋住、不要畫髮際線，仍然是光頭；頭髮只畫在頭的後面、頭的輪廓線之外。髮束用少數幾條乾淨的線表示。" + TONE + "；髮簪用深木色，也畫在頭後面。",
        "subs": {"hair": "all"}, "dye": {"hair": "髮色"}},
    "eyes-a": {
        "slot": "eyes", "sex": "m", "zone": "eyes",
        "what": "請在圖1的臉上畫一雙眼睛，位置在鼻樑兩側、略高於耳朵中段的眼線上，兩眼左右對稱、與鼻樑等距。",
        "how": "細長的鳳眼，上眼瞼墨線較粗、下眼瞼線細，眼白是淺米白色，瞳孔是一個石青色的圓，中間一點深色、一小塊白色高光。畫法大幅概括簡化，每隻眼只用少數幾條乾淨的線，不畫睫毛叢、不畫眼影、不畫眉毛、不畫眼袋。臉的其他部分不許動。",
        "subs": {"iris": "chroma", "rest": "rest"}, "fixed": ["rest"], "dye": {"iris": "瞳色"}},
    "brows-a": {
        "slot": "brows", "sex": "m", "zone": "brows",
        "what": "請在圖1的臉上畫一對眉毛，位置在眼睛上方約一指寬處、額頭下緣，左右對稱。",
        "how": "略斜向上揚的劍眉，粗細均勻、末端收尖，是兩個封閉的色塊形狀，外緣用細墨線包住。" + TONE + "。不畫眼睛，臉的其他部分不許動。",
        "subs": {"brow": "all"}, "dye": {"brow": "髮色"}},
    "beard-3": {
        "slot": "beard", "sex": "m", "zone": "beard",
        "what": "請在圖1的臉上畫「三綹長鬚」：下唇正下方一綹較長的中鬚，兩頰近下頷處各一綹較短的側鬚，三綹都順著下巴往下垂，三綹都很短：最長的中鬚末端只比下巴下緣低約一個鼻子的高度，側鬚只到下頷角附近。上唇上方與人中不畫任何鬍鬚（不要八字鬚），嘴唇完全露出。",
        "how": "每一綹是一個封閉、乾淨的色塊形狀（尖端收細），外緣用墨線包住，內部只用一兩條線表示髮束走向，不畫細碎鬍鬚；三綹之間露出下巴的皮膚。" + TONE + "。嘴唇與鼻子不許被蓋住，頸與衣服不許動。",
        "subs": {"beard": "all"}, "dye": {"beard": "髮色"}},
    "collar-dao-mid": {
        "slot": "collar", "sex": "m", "zone": "collar", "extract": "color",
        "what": "請把圖1身上的素色圓領內衫換成一件道士的交領右衽道袍：衣身是灰青色，交領在胸前交疊成一個深的V形，領口、衣襟與袖口肩線滾一道朱紅色細邊。",
        "how": "衣服覆蓋圖1原本內衫的全部範圍，肩線一路畫到畫面左右邊緣與下緣；道袍用兩階明暗（底色加一階硬邊陰影），衣褶只用少數幾條粗而乾淨的線。頭、臉、耳與頸子的皮膚不許動、不許被蓋住；V形領口露出的是頸子與鎖骨的皮膚，保持原樣。",
        "subs": {"cloth": "rest", "trim": "red"}, "dye": {"cloth": "衣色", "trim": "緣邊色"}},
}


def prompt(key):
    s = PARTS[key]
    return HEAD + s["what"] + s["how"] + SAME + TAIL


# ---- 補完指示（步驟 6）：輸入是收件結果放在灰 128 底上 ----
COMPLETE_HEAD = "圖1是人物頭像的一個部件，單獨放在中性灰色背景上；圖2是畫風基準圖。"
COMPLETE_TAIL = ("畫風與圖2一致：賽璐璐水墨平塗，墨線勾邊，底色純平，一階硬邊陰影，不要漸層、紙紋或顆粒。保留圖1已畫好的部分原樣（位置、大小、顏色都不動），"
                 "只畫沒畫完的地方；灰色背景保持純灰，不加頭、臉、身體或任何其他東西。")
COMPLETE = {
    "hair_back-bun": COMPLETE_HEAD + "圖1是一個髮髻加一支木簪。髻的下半部原本被頭擋住，所以下緣是被切平的，邊上沒有輪廓線，還帶著一條細弧線的殘邊。"
                     "請把它畫成完整獨立的一件：髻的下方接著一塊圓潤的髮根，往下收成一個圓弧底，整個髮髻的外緣用墨線包成一圈封閉的形狀；簪完整、穿過髻；"
                     "去掉那條細弧線和所有碎屑與斑點。" + COMPLETE_TAIL,
}
