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
CLASS = "portrait_trial"   # 真部件先放試驗類，整套 70 款齊了才換掉 portrait
OUT = (320, 400)
SCALE = OUT[0] / CANVAS[0]
PAPER = (0xED, 0xE4, 0xCF)
INK = (0x2B, 0x26, 0x20)   # 墨線色，著色器不染

# 疊圖順序（由後往前），與 8.12 的格一致；slot 的 z 序用這裡的下標
# 與佔位目錄同名；order 越大越前。試驗類的臉在衣領之後：臉的頸子畫到衣領底下，衣領蓋住頸根，領口的皮膚由臉露出
SLOTS = [("back_hair", 10, 1), ("face", 20, 1), ("collar", 30, 1), ("wrinkle", 40, 0), ("mouth", 50, 0), ("eyes", 60, 1), ("brows", 70, 1),
         ("beard", 80, 1), ("mark", 90, 0), ("front_hair", 100, 1), ("hat", 110, 0)]
CHANNELS = ["skin", "hair", "robe", "trim", "iris", "lip", "violet", "gold"]   # 與佔位目錄相同


def base_path(sex):
    """送畫的底版：光頭素臉、無內衫、純綠底（由 face 部件的送畫結果定稿）。"""
    return SRC + f"base-face-{sex}.png"


def code_of(key):
    slot, _, style = key.partition("-")
    return f"{CLASS}/{slot}.{style}"


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
        "beard": (lm["head"][0] - 40, lm["mouth"] - 30, lm["head"][1] + 40, lm["chin"] + 140),
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
# 圖 1 是底版（保住），圖 2 是畫風基準。底版是光頭素臉、赤裸頸肩、純綠底（綠色只是去背用，成品沒有它）。
HEAD = "圖1是一張光頭素臉的頭肩半身像底版（背景是純綠色），圖2是畫風基準圖。"
TAIL = ("畫風與圖2一致：賽璐璐水墨平塗，墨線勾邊（外輪廓粗、內部細），底色純平，只加一階硬邊陰影，不用漸層、不用柔光、不要紙紋或顆粒。"
        "除了上面要求新畫的這一件，人物的頭、臉、頸、肩與圖1逐像素相同，只做上面說的這一件事。"
        "背景保持圖1的純綠色（#00FF00），純色無漸層。不加任何文字、印章、邊框，不加其他飾品。")
SAME = "整張圖的構圖、縮放、位置與圖1完全一致：不要放大縮小，不要平移，不要裁切。"
# 髮、鬚、眉走兩段式：先畫灰階稿，顏色之後由遊戲染
TONE = ("【只有這一件畫成灰色】這一件是灰階稿，之後才會上色，所以這一件只用中性灰、完全沒有色相（不要棕、不要黑藍、不要泛藍的白髮）：底色是明亮的中淺灰（約 #B0B0B0，不是黑色、不是深灰），"
        "陰影塊是明顯較深的灰（約 #606060），少量近白的高光塊（約 #F2F2F2），輪廓線與髮束線條才用近黑（約 #151515），輪廓墨線要連續封閉、比色塊暗得多、在 1024 寬的圖上約 3 到 4 像素寬；底色、陰影、高光三階要一眼分得開：髮束與髮束之間、髮的下緣與髻的下側用陰影灰塊明確表示（陰影約占髮面積的三分之一），頭頂與髮束的上緣留幾條高光灰塊，不要把整件塗成同一個灰。"
        "人物其餘的地方——臉、耳、頸、肩、胸的皮膚——必須保持圖1原本的暖膚色，一點都不要變灰、不要加紋理或顆粒，整張圖其他部分與圖1完全一樣")


# 臉：從原底版 base-m.png 去掉內衫、補完頸肩、背景換純綠；之後定稿成 base-face-m.png
FACE_PROMPT = ("圖1是一張光頭素臉的頭肩半身像，身上穿著圓領內衫，背景是米色；圖2是畫風基準圖。請做三件事：一、把圓領內衫整件去掉，"
               "露出赤裸的頸、鎖骨與雙肩，肩線沿著圖1內衫的肩線向左右畫到畫面邊緣、向下一路畫到畫面下緣，胸肩的皮膚完整連續，沒有任何衣服、衣領或領口線；"
               "皮膚用與頭臉相同的底色加一階硬邊陰影，頸與鎖骨的陰影畫法照圖1。二、把整個背景改成純綠色（#00FF00），純色、沒有漸層、沒有紙紋，人物外輪廓以外全部是這個綠。"
               "三、頭、臉、五官、耳朵、頸與圖1完全相同，位置與大小都不動，外輪廓墨線保持封閉、粗細與圖1一致。不加衣服、不加頭髮、不加文字。" + SAME)

PARTS = {
    # key = <格>-<款>。slot、sex、what、how、zone（收件範圍）、subs（子層：名稱 → 判別方法）、dye（子層 → 染色欄位）
    "face-m0": {
        "slot": "face", "sex": "m", "zone": None, "base": "orig", "prompt": FACE_PROMPT,
        "subs": {"skin": "all"}, "dye": {"skin": "skin"}},
    "front_hair-topknot": {
        "slot": "front_hair", "sex": "m", "zone": "head_top",
        "what": "請在圖1的頭上畫「前髮」：貼著頭皮的整齊頭髮，覆蓋頭頂與前額，髮際線在額頭上方三分之一處，兩鬢收在耳朵前上方，耳朵完整露出。",
        "how": "只畫貼著頭皮的這一層髮，不畫髮髻、不畫髮簪、不畫垂到頭外面去的髮。髮束用少數幾條乾淨的線表示，不畫細碎髮絲。整片頭髮的外緣用墨線包成一圈封閉的形狀。" + TONE + "。眉眼口鼻、耳朵、臉與頸不許被蓋住。",
        "subs": {"hair": "all"}, "dye": {"hair": "hair"}},
    "back_hair-topknot": {
        "slot": "back_hair", "sex": "m", "zone": "head_top",
        "what": "請在圖1的頭後面畫「後髮」：頭頂偏後挽起的一個圓髻，髻上橫插一支簡單的簪。髻從頭頂後方露出來，頭頂以上整個髻都看得見。",
        "how": "頭本身（光頭的頭皮、額、臉、耳）完全不要被蓋住、不要畫髮際線，仍然是光頭；頭髮只畫在頭的後面、頭的輪廓線之外。髮束用少數幾條乾淨的線表示。" + TONE + "；簪也用灰階（中灰帶一階陰影），畫在頭後面。",
        "subs": {"hair": "all"}, "dye": {"hair": "hair"}},
    "eyes-m0": {
        "slot": "eyes", "sex": "m", "zone": "eyes",
        "what": "請在圖1的臉上畫一雙眼睛，位置在鼻樑兩側、略高於耳朵中段的眼線上，兩眼左右對稱、與鼻樑等距。",
        "how": "細長的鳳眼，上眼瞼墨線較粗、下眼瞼有一條細線，眼白是淺米白色，瞳孔是一個石青色的圓，中間一點深色、一小塊白色高光。畫法大幅概括簡化，每隻眼只用少數幾條乾淨的線，不畫睫毛叢、不畫眼影、不畫眉毛、不畫眼袋。臉的其他部分不許動。",
        "rescue": True, "subs": {"iris": "chroma", "rest": "rest"}, "fixed": ["rest"], "dye": {"iris": "iris"}},
    "brows-0": {
        "slot": "brows", "sex": "m", "zone": "brows",
        "what": "請在圖1的臉上畫一對眉毛，位置在眼睛上方約一指寬處、額頭下緣，左右對稱。",
        "how": "略斜向上揚的劍眉，粗細均勻、末端收尖，是兩個封閉的色塊形狀，外緣用細墨線包住。" + TONE + "。不畫眼睛，臉的其他部分不許動。",
        "subs": {"brow": "all"}, "dye": {"brow": "hair"}},
    "beard-goatee": {
        "slot": "beard", "sex": "m", "zone": "beard",
        "what": "請在圖1的臉上只畫「一件」山羊鬚：從下唇正下方起，順著下巴中線往下垂的一綹短鬚，連成一整塊，上寬下尖，末端只比下巴下緣低約一個鼻子的高度。嘴唇與嘴線完全不許動、不許被蓋住，上唇上方與兩頰不畫任何鬍鬚。",
        "how": "只有這一綹，是一個封閉、連續、乾淨的色塊形狀，外緣用墨線包住，內部只用一兩條線表示髮束走向，不畫細碎鬍鬚，不要分成好幾塊、不要有散開的鬚尖。" + TONE + "。鼻子、嘴線、下巴輪廓與頸不許動。",
        "subs": {"beard": "all"}, "dye": {"beard": "hair"}},
    "collar-dao_plain": {
        "slot": "collar", "sex": "m", "zone": "collar", "extract": "color",
        "what": "請給圖1的人穿上一件道士的交領右衽道袍：衣身是灰青色，交領在胸前交疊成一個深的V形，領口、衣襟與袖口肩線滾一道朱紅色細邊。",
        "how": "道袍蓋住頸根以下的肩與胸，肩線一路畫到畫面左右邊緣與下緣；道袍用兩階明暗（底色加一階硬邊陰影），衣褶只用少數幾條粗而乾淨的線。領口的內緣貼著頸子，沿著整條貼頸的邊都要有一圈完整的墨線。頭、臉、耳與頸子的皮膚不許動、不許被蓋住；V形領口露出的是頸子與鎖骨的皮膚，保持原樣。",
        "subs": {"robe": "rest", "trim": "red"}, "dye": {"robe": "robe", "trim": "trim"}},
}


def prompt(key):
    s = PARTS[key]
    if "prompt" in s:
        return s["prompt"]
    return HEAD + s["what"] + s["how"] + SAME + TAIL


def input_path(key):
    s = PARTS[key]
    return SRC + f"base-{s['sex']}.png" if s.get("base") == "orig" else base_path(s["sex"])


# ---- 補完指示（步驟 6）：輸入是收件結果放在灰 128 底上 ----
COMPLETE_HEAD = "圖1是人物頭像的一個部件，單獨放在純綠色（#00FF00）背景上；圖2是畫風基準圖。"
COMPLETE_TAIL = ("畫風與圖2一致：賽璐璐水墨平塗，墨線勾邊，底色純平，一階硬邊陰影，不要漸層、紙紋或顆粒。保留圖1已畫好的部分原樣（位置、大小、顏色都不動），"
                 "只畫沒畫完的地方；綠色背景保持純綠，不加頭、臉、身體或任何其他東西。")
COMPLETE = {
    "back_hair-topknot": COMPLETE_HEAD + "圖1是一個髮髻加一支簪，是灰階稿（之後才上色，維持中性灰，不加任何色相）。髻的下半部原本被頭擋住，所以下緣是被切平的，邊上沒有輪廓線，還可能帶著細弧線的殘邊。"
                         "請把它畫成完整獨立的一件：髻的下方接著一塊圓潤的髮根，往下收成一個圓弧底，整個髮髻的外緣用墨線包成一圈封閉的形狀；簪完整、穿過髻；"
                         "去掉細弧線和所有碎屑與斑點。" + COMPLETE_TAIL,
    "collar-dao_plain": COMPLETE_HEAD + "圖1是一件道袍的衣領與衣身。領口貼著頸子的那一圈邊原本被頸子擋住，邊上沒有墨線或線斷斷續續，領口內側也可能有缺口。"
                        "請把它畫成完整獨立的一件：領口內緣（V形開口與貼頸的弧邊）整圈補上連續的墨線，朱紅滾邊沿著領口完整連續，衣身的其他部分原樣不動；"
                        "V形開口裡面保持綠色背景（不要畫頸、皮膚或內衫）。" + COMPLETE_TAIL,
}
