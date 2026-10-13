"""底版：把 gpt-image 生成的底版原圖（1122×1402）縮成畫家畫布 1024×1280，並寫 sources/parts/portrait/base.json（定位點）。

    python tools/parts/base.py

原圖在 .shiftblame/tmp/parts/portrait/base-<m|f>-c.png（不入庫；提示詞與雜湊記在 base.json）。
定位點是對原圖逐一目視量出的座標（原圖 1122×1402），這裡換算成 1024×1280 與入庫尺寸 320×400。
eye_y 是眼線，nose_y 是鼻底，mouth_y 是口，chin_y 是下巴，neck_y 是頸根（領口最高處），top_y 是頭頂，ears 是左右耳的 [x, 上, 下]。
"""
import hashlib
import json
import os
import sys

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402

RAW = (1122, 1402)
# 原圖座標（目視量測）
LANDMARKS = {
    "m": {"cx": 560, "top_y": 345, "eye_y": 650, "nose_y": 740, "mouth_y": 792, "chin_y": 915, "neck_y": 1085,
          "ears": {"l": [345, 580, 740], "r": [775, 580, 740]}, "head_w": [360, 765]},
    "f": {"cx": 560, "top_y": 335, "eye_y": 650, "nose_y": 728, "mouth_y": 778, "chin_y": 890, "neck_y": 990,
          "ears": {"l": [352, 600, 725], "r": [770, 600, 725]}, "head_w": [367, 755]},
}
PROMPTS = {
    "m": "見 sources/parts/portrait/base.json 的 prompts（三輪：a 初版、b 去眼窩投影擺正、c 縮小留頂部空白）",
    "f": "同上",
}


def conv(v, axis, size):
    return round(v * size / RAW[axis])


def main():
    out = {"canvas": list(P.CANVAS), "out": list(P.OUT), "scale": P.SCALE, "raw_size": list(RAW), "sex": {}, "prompts": PROMPTS_FULL}
    for sex, lm in LANDMARKS.items():
        raw = P.TMP + f"base-{sex}-c.png"
        im = Image.open(raw).convert("RGB").resize(P.CANVAS, Image.LANCZOS)
        im.save(P.base_path(sex))
        entry = {"raw": os.path.relpath(raw, P.ROOT).replace("\\", "/"), "raw_sha256": hashlib.sha256(open(raw, "rb").read()).hexdigest(),
                 "file": f"base-{sex}.png", "sha256": hashlib.sha256(open(P.base_path(sex), "rb").read()).hexdigest(), "points": {}}
        def cv(v, axis):   # 原圖座標 → 畫布座標與入庫座標
            return {"canvas": conv(v, axis, P.CANVAS[axis]), "out": conv(v, axis, P.OUT[axis])}
        for key, v in lm.items():
            if key == "cx":
                entry["points"][key] = cv(v, 0)
            elif key == "head_w":
                entry["points"][key] = {"canvas": [cv(x, 0)["canvas"] for x in v], "out": [cv(x, 0)["out"] for x in v]}
            elif key == "ears":
                entry["points"][key] = {side: {"canvas": [cv(p[0], 0)["canvas"], cv(p[1], 1)["canvas"], cv(p[2], 1)["canvas"]],
                                               "out": [cv(p[0], 0)["out"], cv(p[1], 1)["out"], cv(p[2], 1)["out"]]} for side, p in v.items()}
            else:
                entry["points"][key] = cv(v, 1)
        out["sex"][sex] = entry
    json.dump(out, open(P.SRC + "base.json", "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    print("base.json 已寫")


PROMPTS_FULL = {
    "a_m": "圖是畫風基準(style.png)。請照同一畫風…畫一張「底版」：壯年男子的光頭頭肩半身像，直幅4:5，…完全光頭…不畫眉毛與眼睛（眼眶位置只有平坦的皮膚）；身穿最簡單的素色圓領貼身內衫，頸根與鎖骨清楚，肩線一直畫到畫面下緣。五官大幅概括簡化。背景純平宣紙色#EDE4CF，不要紙紋。（edit_image，參照 style.png，codex 後端，1024x1280 提示、實得 1122x1402）",
    "b": "照 a 重畫：臉上不要有跨過眼窩與額頭的大塊投影，陰影只留在顱頂右側邊緣、耳後、下巴底下與頸側；頭擺正，雙耳露出。",
    "c": "照 b 重畫：整個人縮小，頭頂離上緣約20%（留出畫髮髻與冠帽的空白），肩膀被左右邊緣切掉，肩與衣畫到下緣，頸根在畫面高約72%。女版同理（a 以 style-girl.png 為參照，內衫月白色）。",
}

if __name__ == "__main__":
    main()
