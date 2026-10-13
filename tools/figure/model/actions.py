"""各動作的關鍵姿勢（8.11 的動作表）。腳本寫關鍵姿勢，每秒 24 格；建動作時關鍵姿勢以四元數球面內插、逐格打關鍵幀。

每個關鍵格是 {骨頭: (前後, 扭轉, 側向)}，單位度；root 另有 lift（身體高低，公尺）。
- 前後：繞世界 X 轉。垂下的肢體（手臂、腿）正值往後擺、負值往前擺；立在關節上方的部位（胸、頸、頭）正值往前傾、負值往後仰。
- 扭轉：繞骨頭自己的軸（世界 Z）轉。
- 側向：左右肢體用「out」——正值離開身體；胸、頸、頭的側向是原始的繞骨頭 Z 軸角度。
腳底要貼地：蹲低時 squat() 同時給大腿、小腿、腳與身體高度。

每個動作 make() 回傳 {frames, loop, loop_start, keys, events}：
- loop_start：循環動作從哪一格開始繞圈（之前的格只播一次，當作進場）；
- events：動作內有意義的時刻（格號），例如 struck 的 hit（雷擊中）、brace 的 raised（舉臂到頂）；
- wind：衣擺被風吹開的程度（CultivatorFigure 的 wind），[(格號, 值), ...] 之間線性內插。
- 選用欄位（新動作才有，舊動作的匯出結果不變）：
  azimuth：匯出投影的方位角（度，預設 45；0 是正側面，前後擺的動作用它）；
  rigid：匯出時把 root 的俯仰（倒地）與 float（浮空高度）拆成整體剛體變換（body_roll、body_lift、body_pivot）；
  feet：匯出雙腳位置（foot_f、foot_b）；point：[(格號, 值)]，大於 0.5 為併指前指；
  float：[(格號, 公尺)]，浮空高度，疊在 root 的 lift 上。
節奏依 sources/figure/direct/<動作>/timing.md 的關鍵時刻；影片標為不可信的部分（特效、失真的袖筒）不抄。
"""
import math

from mathutils import Euler, Quaternion

LIMBS = ("shoulder", "upper_arm", "forearm", "hand", "thigh", "shin", "foot")
THIGH, SHIN = 0.23, 0.22
FPS = 24


def squat(deg):
    """雙膝彎曲 deg 度（大腿往前、小腿往後），腳掌保持平放；回傳 (姿勢, 身體下沉量)。"""
    pose = {}
    for tag in ("_l", "_r"):
        pose["thigh" + tag] = (-deg, 0, 0)
        pose["shin" + tag] = (2 * deg, 0, 0)
        pose["foot" + tag] = (-deg, 0, 0)
    drop = (THIGH + SHIN) * (1 - math.cos(math.radians(deg)))
    return pose, drop


def blend(*poses):
    out = {}
    for pose in poses:
        for bone, value in pose.items():
            out[bone] = tuple(a + b for a, b in zip(out.get(bone, (0, 0, 0)), value))
    return out


def stand_pose(sway=0.0, breath=0.0):
    """立姿：雙臂自然垂下（上臂略離身、前臂微屈），重心隨 sway（−1 到 1）左右微移。"""
    return {"chest": (breath * 1.2, 0, sway * 0.8), "head": (-breath * 0.8, 0, -sway * 0.6),
            "forearm_l": (-6, 0, 0), "forearm_r": (-6, 0, 0),
            "upper_arm_l": (breath * 1.2, 0, -2 + breath * 0.8), "upper_arm_r": (breath * 1.2, 0, -2 + breath * 0.8),
            "thigh_l": (0, 0, 1.5 - sway * 0.8), "thigh_r": (0, 0, 1.5 + sway * 0.8)}


def brace_pose(reach=0.0, breath=0.0):
    """迎雷：仰首，雙腳站開，人物的右手高舉向天（上臂外展約 150°）、左手垂在身側微離。reach 是右臂再往上伸的角度。"""
    legs, drop = squat(13)
    pose = blend(legs, {
        "chest": (-9 + breath * 1.2, 0, 0), "neck": (-8, 0, 0), "head": (-14 - breath * 1.5, 0, 0),
        "upper_arm_r": (-14, 0, 148 + reach), "forearm_r": (-10, 0, 6), "hand_r": (-8, 0, 0),
        "upper_arm_l": (-6, 0, 22), "forearm_l": (-20, 0, 0),
        "thigh_l": (0, 0, 6), "thigh_r": (0, 0, 6), "waist": (-2, 0, 0)})
    return pose, -drop


def _stand():
    keys = {}
    for frame, sway, breath in ((0, 0.0, 0.0), (12, 0.6, 0.5), (24, 0.0, 1.0), (36, -0.6, 0.5), (48, 0.0, 0.0)):
        keys[frame] = (stand_pose(sway, breath), -0.003 * breath)
    return {"frames": 48, "loop": True, "loop_start": 0, "keys": keys, "events": {}, "wind": [(0, 0.05)]}


def _brace():
    """迎雷（timing.md）：0–4 立姿；發力 4–12：右臂經水平（8）舉到近垂直（12），頭後仰、膝微彎、雙腳站開；
    12 之後停住循環（臂與頭極微小的晃動疊上呼吸）。進場 12 格（0.5 秒），循環主段 48 格（2 秒）。"""
    keys = {0: (stand_pose(), 0.0)}
    ready = blend(stand_pose(), {"head": (-4, 0, 0), "chest": (-2, 0, 0), "upper_arm_r": (0, 0, 24)})
    keys[4] = (ready, -0.004)
    mid = blend(brace_pose(0)[0], {"upper_arm_r": (0, 0, -58), "head": (8, 0, 0), "chest": (5, 0, 0)})
    keys[8] = (mid, -0.02)
    for frame, reach, breath in ((12, 0, 0.0), (24, 3, 0.6), (36, 0, 1.0), (48, -3, 0.6), (60, 0, 0.0)):
        pose, drop = brace_pose(reach, breath)
        keys[frame] = (pose, drop)
    return {"frames": 60, "loop": True, "loop_start": 12, "keys": keys, "events": {"raised": 12},
            "wind": [(0, 0.05), (12, 0.08), (60, 0.08)]}


def _struck():
    """受雷（timing.md，影片 8–44 格排成 36 格）：自迎雷姿起；0–2 受擊（hit 在第 2 格）、身體挺直；
    6 雙臂甩到水平、10–12 往下甩到 45° 外展而胸後仰約 28°；14–18 後仰極點停住（雙臂外展約 70°、膝彎約 20°、臀坐後）；
    20 開始收；24 前傾約 15°、頭低、雙臂折回身前、蹲得最低（約 30°）；28 開始站直、一手垂一手在胸前；36 立姿。"""
    start, start_drop = brace_pose()
    shock = blend(start, {"chest": (3, 0, 0), "head": (-6, 0, 0), "upper_arm_r": (0, 0, 4)})
    legs1, drop1 = squat(14)
    swing = blend(legs1, {"chest": (-14, 0, 0), "neck": (-6, 0, 0), "head": (-14, 0, 0), "waist": (-4, 0, 0),
                          "upper_arm_r": (6, 0, 92), "forearm_r": (-14, 0, 0), "upper_arm_l": (4, 0, 92), "forearm_l": (-14, 0, 0),
                          "thigh_l": (0, 0, 8), "thigh_r": (0, 0, 8)})
    legs2, drop2 = squat(18)
    throw = blend(legs2, {"chest": (-22, 0, 0), "neck": (-4, 0, 0), "head": (-14, 0, 6), "waist": (-6, 0, 0),
                          "upper_arm_r": (16, 0, 48), "forearm_r": (-18, 0, 0), "upper_arm_l": (14, 0, 46), "forearm_l": (-18, 0, 0),
                          "thigh_l": (0, 0, 9), "thigh_r": (0, 0, 9)})
    legs3, drop3 = squat(20)
    peak = blend(legs3, {"chest": (-24, 0, 0), "neck": (-4, 0, 0), "head": (-16, 0, 9), "waist": (-6, 0, 0),
                         "upper_arm_r": (10, 0, 70), "forearm_r": (-20, 0, 0), "upper_arm_l": (8, 0, 68), "forearm_l": (-20, 0, 0),
                         "thigh_l": (0, 0, 9), "thigh_r": (0, 0, 9)})
    legs4, drop4 = squat(24)
    gather = blend(legs4, {"chest": (-6, 0, 0), "neck": (0, 0, 0), "head": (-6, 0, 3), "waist": (0, 0, 0),
                           "upper_arm_r": (-22, 0, 36), "forearm_r": (-50, 0, 0), "upper_arm_l": (-20, 0, 34), "forearm_l": (-48, 0, 0),
                           "thigh_l": (0, 0, 8), "thigh_r": (0, 0, 8)})
    legs5, drop5 = squat(30)
    low = blend(legs5, {"chest": (15, 0, 0), "neck": (8, 0, 0), "head": (14, 0, 0), "waist": (6, 0, 0),
                        "upper_arm_r": (-40, 0, 18), "forearm_r": (-82, 0, 0), "upper_arm_l": (-38, 0, 16), "forearm_l": (-80, 0, 0),
                        "thigh_l": (0, 0, 7), "thigh_r": (0, 0, 7)})
    legs6, drop6 = squat(14)
    rise = blend(legs6, {"chest": (6, 0, 0), "neck": (2, 0, 0), "head": (4, 0, 0),
                         "upper_arm_r": (-4, 0, 8), "forearm_r": (-10, 0, 0), "upper_arm_l": (-34, 0, 14), "forearm_l": (-70, 0, 0),
                         "thigh_l": (0, 0, 5), "thigh_r": (0, 0, 5)})
    keys = {0: (start, start_drop), 2: (shock, start_drop - 0.004), 6: (swing, -drop1), 10: (throw, -drop2 - 0.01),
            14: (peak, -drop3 - 0.012), 18: (peak, -drop3 - 0.012), 21: (gather, -drop4), 24: (low, -drop5),
            28: (rise, -drop6), 36: (stand_pose(), 0.0)}
    return {"frames": 36, "loop": False, "loop_start": 0, "keys": keys, "events": {"hit": 2, "peak": 14, "lowest": 24},
            "wind": [(0, 0.08), (6, 0.14), (14, 0.12), (24, 0.08), (36, 0.05)]}


def curve_at(points, frame):
    """[(格號, 值), ...] 之間線性內插；超出範圍取端點。"""
    if frame <= points[0][0]:
        return points[0][1]
    for (a, va), (b, vb) in zip(points, points[1:]):
        if a <= frame <= b:
            return va + (vb - va) * ((frame - a) / (b - a) if b != a else 0.0)
    return points[-1][1]


def eased(points, frames):
    """關鍵 (格號, 值) 之間緩入緩出，展開成每格一個值的曲線。"""
    return [(frame, _between(points, frame)) for frame in range(frames + 1)]


def _between(points, frame):
    if frame <= points[0][0]:
        return points[0][1]
    for (a, va), (b, vb) in zip(points, points[1:]):
        if a <= frame <= b:
            return va + (vb - va) * (_smooth((frame - a) / (b - a)) if b != a else 0.0)
    return points[-1][1]


HIP_Z, ANKLE_Z = 0.50, 0.05


def _leg_foot(thigh_deg, shin_deg):
    """一條腿（大腿 thigh_deg、小腿相對再彎 shin_deg，正值往後）的腳踝相對髖的 (往後, 高度)。"""
    t, u = math.radians(thigh_deg), math.radians(thigh_deg + shin_deg)
    return THIGH * math.sin(t) + SHIN * math.sin(u), HIP_Z - THIGH * math.cos(t) - SHIN * math.cos(u)


def _walk():
    """步行（timing.md）：一步 11 格、兩步一循環 22 格；極點在第 0 與 11 格（一腿前一腿後，步幅最大，腰最低），
    過渡在第 5.5、16.5 格（腰最高）。支撐腿的腳底相對身體等速往後（線性），所以相對地面不動、不滑；
    擺動腿屈膝收腳、餘弦曲線往前，在極點輕輕落下。對側臂與腿反向擺；胸前傾約 4°，頭穩定。
    身體高低由兩腿同時著地推算（支撐腳恆貼地）。"""
    keys = {}
    amp = 34.0
    reach = math.sin(math.radians(amp)) * (THIGH + SHIN)
    for frame in range(22):
        phi = 2 * math.pi * frame / 22
        half = (frame % 11) / 11.0
        # 兩腿各一個「支撐段」：右腿 0–11 格、左腿 11–22 格；支撐腳往後（+）等速 −reach → +reach。
        stance_x = -reach + 2 * reach * half
        stance_t = math.degrees(math.asin(stance_x / (THIGH + SHIN)))
        swing_x = reach * math.cos(math.pi * half)
        swing_t = math.degrees(math.asin(swing_x / (THIGH + SHIN)))
        swing_s = 42 * math.sin(math.pi * half)
        if frame < 11:
            t_r, s_r, t_l, s_l = stance_t, 0.0, swing_t, swing_s
        else:
            t_l, s_l, t_r, s_r = stance_t, 0.0, swing_t, swing_s
        z_r = _leg_foot(t_r, s_r)[1]
        z_l = _leg_foot(t_l, s_l)[1]
        lift = ANKLE_Z - min(z_r, z_l)
        c = math.cos(phi)
        pose = blend(stand_pose(), {
            "thigh_r": (t_r, 0, 0), "shin_r": (s_r, 0, 0), "foot_r": (-(t_r + s_r), 0, 0),
            "thigh_l": (t_l, 0, 0), "shin_l": (s_l, 0, 0), "foot_l": (-(t_l + s_l), 0, 0),
            "upper_arm_l": (-34 * c + 4, 0, 0), "forearm_l": (-8 - 22 * max(0.0, c), 0, 0),
            "upper_arm_r": (34 * c + 4, 0, 0), "forearm_r": (-8 - 22 * max(0.0, -c), 0, 0),
            "waist": (4, 0, 0), "chest": (4 - 1.5 * c * c, 0, 0), "head": (-5, 0, 0)})
        keys[frame] = (pose, lift)
    keys[22] = keys[0]
    return {"frames": 22, "loop": True, "loop_start": 0, "keys": keys, "events": {"step_a": 0, "step_b": 11},
            "wind": [(0, 0.06)], "azimuth": 0, "feet": True}


def _fall():
    """倒地（timing.md）：從立姿起。4 失力（重心後移、膝微彎、頭微仰）；8 膝軟（膝彎約 35°、胸後仰）；
    12–15 臀坐低、整個人繞腳跟往後倒（root 後仰）、雙臂往前上方甩；18 背落地（平躺）；20 彈起一點、22 落定；
    22 起不動（雙臂放在身側、雙腿伸直）。倒地後的靜止至 36 格。"""
    pose0 = stand_pose()

    def lie(arm_f, fore, chest, head, legs, roll):
        sq, drop = squat(legs) if legs else ({}, 0.0)
        pose = blend(pose0, sq, {"chest": (chest, 0, 0), "head": (head, 0, 0), "neck": (head * 0.3, 0, 0),
                                 "upper_arm_l": (arm_f, 0, 0), "upper_arm_r": (arm_f, 0, 0),
                                 "forearm_l": (fore, 0, 0), "forearm_r": (fore, 0, 0), "root": (roll, 0, 0)})
        return pose, -drop
    keys = {0: (pose0, 0.0),
            4: lie(-8, -10, -4, -8, 10, -2),
            8: lie(-30, -14, -12, -12, 34, -8),
            12: lie(-62, -16, -20, -16, 24, -26),
            15: lie(-78, -12, -18, -18, 12, -55),
            18: lie(-70, -10, -8, -10, 0, -88),
            20: lie(-50, -10, -4, -6, 0, -82),
            22: lie(-5, -4, 0, 0, 0, -90),
            36: lie(-5, -4, 0, -2, 0, -90)}
    floats = eased([(0, 0.0), (12, 0.0), (18, 0.085), (20, 0.115), (22, 0.088), (36, 0.088)], 36)
    return {"frames": 36, "loop": False, "loop_start": 0, "keys": keys, "events": {"buckle": 8, "land": 18, "still": 22},
            "wind": [(0, 0.05), (12, 0.12), (18, 0.14), (24, 0.06), (36, 0.05)], "azimuth": 0, "rigid": True,
            "float": floats}


def _rise():
    """浮空（timing.md 的 rise，飛升用）：0–8 立姿、重心微沉；12 腳跟離地；12–22 上升（約 0.13 身高）、雙臂外展到約 40°；
    24–36 升到最高停住（極輕的上下晃動）；36–46 慢慢降低、袖回垂；46 腳尖著地；50 立姿。不循環，終點是立姿。"""
    base = stand_pose()

    def arms(out, head):
        return blend(base, {"upper_arm_r": (-2, 0, out), "upper_arm_l": (-2, 0, out),
                            "forearm_r": (-6, 0, 0), "forearm_l": (-6, 0, 0), "head": (head, 0, 0), "chest": (-head * 0.3, 0, 0)})
    dip, drop = squat(9)
    keys = {0: (base, 0.0),
            8: (blend(arms(6, 0), dip), -drop),
            12: (arms(10, -1), 0.0),
            17: (arms(36, -3), 0.0),
            22: (arms(52, -5), 0.0),
            27: (arms(56, -6), 0.0),
            32: (arms(48, -5), 0.0),
            36: (arms(54, -5), 0.0),
            41: (arms(32, -2), 0.0),
            46: (arms(8, 0), 0.0),
            50: (base, 0.0)}
    floats = eased([(0, 0.0), (11, 0.0), (22, 0.12), (27, 0.133), (32, 0.123), (36, 0.13), (41, 0.07), (46, 0.0), (50, 0.0)], 50)
    return {"frames": 50, "loop": False, "loop_start": 0, "keys": keys, "events": {"liftoff": 12, "apex": 24, "touch": 46},
            "wind": [(0, 0.05), (12, 0.1), (24, 0.24), (36, 0.24), (46, 0.1), (50, 0.05)], "azimuth": 45, "rigid": True,
            "float": floats}


def _strike():
    """出手（並指成劍、一道劍光；節奏照 timing.md 的格數）：0–8 預備、右臂往後引；8–26 身體後坐蓄勢（剪影縮小：
    膝彎、胸後仰、右肘後拉、手收在腰後）；16 draw 是蓄勢開始吃緊、26 raise 是蓄勢到頂（停一下）；
    26→32 出手（6 格，整組最快）：右臂水平前刺到最遠（32 hit）、上身前傾、後腳蹬直、衣擺與飄帶向後甩（wind 到頂）、
    並指（point）；32–34 停住，38 收勢，44 立姿。劍光不畫；匯出時在 palm_f 與 aim_f 標出劍光的起點與方向。
    與筆記的出入：筆記是拔劍、舉劍過頭、劈下，這裡改成後引與前刺（姿勢照上面，格數與 draw/raise/hit 的節奏照筆記）。
    右臂是畫面遠側的 _f 臂；az=0 的正側面，前刺是完整的前後向。"""
    base = stand_pose()

    def at(ua, fa, chest=0.0, head=0.0, legs=0, left=(6, -14), waist=0.0, spread=0.0):
        sq, drop = squat(legs) if legs else ({}, 0.0)
        pose = blend(base, sq, {"upper_arm_r": (ua, 0, 0), "forearm_r": (fa, 0, 0),
                                "upper_arm_l": (left[0], 0, 0), "forearm_l": (left[1], 0, 0),
                                "chest": (chest, 0, 0), "head": (head, 0, 0), "neck": (head * 0.3, 0, 0), "waist": (waist, 0, 0),
                                "thigh_l": (0, 0, spread), "thigh_r": (0, 0, spread)})
        return pose, -drop
    keys = {0: (base, 0.0),
            8: at(34, -52, -3, -2, 10, (10, -22), -2, 3),
            16: at(52, -38, -7, -4, 22, (14, -26), -4, 5),
            26: at(64, -30, -10, -5, 30, (16, -30), -6, 7),
            32: at(-92, -2, 14, 8, 16, (22, -8), 8, 6),
            34: at(-92, -2, 14, 8, 16, (22, -8), 8, 6),
            38: at(-40, -30, 6, 3, 6, (10, -16), 3, 3),
            44: (base, 0.0)}
    return {"frames": 44, "loop": False, "loop_start": 0, "keys": keys, "events": {"draw": 16, "raise": 26, "hit": 32},
            "wind": [(0, 0.05), (26, 0.06), (32, 0.32), (34, 0.32), (44, 0.05)], "azimuth": 0, "aim": True,
            "point": [(0, 0), (27, 0), (28, 1), (36, 1), (37, 0)]}


def _seal():
    """結印（timing.md）：4–8 兩手略向前、左手（近側）提到胸前；10–16 右臂（遠側 _f）抬到肩前再向前伸直；
    16 成訣（右手併指前指）；18–34 停住（右臂水平前伸、左手豎掌在胸前、胸前探約 5°、極輕的呼吸）；36 起右臂略落；
    38–40 雙臂向兩側放開；44 立姿。前伸是緩慢推出，沒有爆發。"""
    base = stand_pose()

    def at(ur, fr, ul, fl, chest=0.0, head=0.0):
        return blend(base, {"upper_arm_r": (ur, 0, 0), "forearm_r": (fr, 0, 0), "upper_arm_l": (ul, 0, 0), "forearm_l": (fl, 0, 0),
                            "chest": (chest, 0, 0), "head": (head, 0, 0)}), 0.0
    keys = {0: (base, 0.0),
            5: at(-14, -22, -14, -22, 0, 0),
            8: at(-30, -70, -22, -98, 1, 0),
            12: at(-58, -52, -22, -104, 2, 1),
            16: at(-88, -6, -22, -106, 4, 2),
            24: at(-90, -4, -22, -108, 5.5, 2.5),
            30: at(-89, -5, -22, -107, 4.5, 2),
            34: at(-88, -6, -22, -106, 5, 2),
            36: at(-74, -12, -22, -100, 4, 1),
            38: at(-42, -16, -14, -50, 2, 0),
            40: at(-12, -12, -8, -24, 1, 0),
            44: (base, 0.0)}
    return {"frames": 44, "loop": False, "loop_start": 0, "keys": keys, "events": {"form": 16, "release": 36},
            "wind": [(0, 0.05), (16, 0.08), (34, 0.08), (44, 0.05)], "azimuth": 0, "point": [(0, 0), (15, 0), (16, 1), (35, 1), (36, 0)]}


def _bow():
    """行禮（timing.md）：6–10 雙臂向前向內抬、雙手在腹前相對；12–14 彎腰（12 約 15°、14 約 30°，頭低）；
    14–34 停住（手合在胸前，極輕的呼吸起伏）；36–40 起身（36 約 15°）、雙臂放下；44 立姿。腰是彎曲的軸，雙腿站直。"""
    base = stand_pose()

    def at(bend, ua, fa, breath=0.0):
        return blend(base, {"waist": (bend * 0.6 + breath, 0, 0), "chest": (bend * 0.25, 0, 0), "neck": (bend * 0.1, 0, 0), "head": (bend * 0.2, 0, 0),
                            "upper_arm_l": (ua, 0, 0), "forearm_l": (fa, 0, 0), "upper_arm_r": (ua, 0, 0), "forearm_r": (fa, 0, 0)}), 0.0
    keys = {0: (base, 0.0),
            6: at(0, -10, -14),
            8: at(0, -38, -50),
            10: at(2, -48, -88),
            12: at(14, -56, -96),
            14: at(30, -62, -96),
            24: at(30, -62, -96, 1.2),
            34: at(30, -62, -96),
            36: at(15, -44, -64),
            40: at(2, -12, -24),
            44: (base, 0.0)}
    return {"frames": 44, "loop": False, "loop_start": 0, "keys": keys, "events": {"bend": 14, "rise": 36},
            "wind": [(0, 0.05), (14, 0.06), (44, 0.05)], "azimuth": 0}


ACTIONS = {"stand": _stand, "brace": _brace, "struck": _struck, "walk": _walk, "fall": _fall, "rise": _rise,
           "strike": _strike, "seal": _seal, "bow": _bow}


def _smooth(t):
    return t * t * (3 - 2 * t)


def pose_at(keys, frame, bones):
    """關鍵格之間的姿勢：每根骨頭的旋轉以四元數球面內插（段內緩入緩出），身體高低同樣緩入緩出。
    回傳 ({骨頭: 四元數}, lift)。"""
    marks = sorted(keys)
    last = marks[-1]
    frame = min(max(frame, marks[0]), last)
    index = max(i for i, m in enumerate(marks) if m <= frame)
    a = marks[index]
    b = marks[min(index + 1, len(marks) - 1)]
    t = 0.0 if b == a else _smooth((frame - a) / (b - a))
    pose_a, lift_a = keys[a]
    pose_b, lift_b = keys[b]
    out = {}
    for bone in bones:
        qa = _quat(bone, pose_a.get(bone, (0, 0, 0)))
        qb = _quat(bone, pose_b.get(bone, (0, 0, 0)))
        if qa.dot(qb) < 0:
            qb = -qb
        out[bone] = qa.slerp(qb, t)
    return out, lift_a + (lift_b - lift_a) * t


def _quat(bone, value):
    pitch, twist, side = value
    if bone.endswith("_r") and bone.split("_")[0] in LIMBS + ("upper",):
        side = -side
    return Euler((math.radians(pitch), math.radians(twist), math.radians(side)), "XYZ").to_quaternion()


def apply(rig, quats, lift, frame):
    """把一格的姿勢寫成關鍵幀（所有骨頭每格都鍵一次，四元數旋轉）。"""
    for bone in rig.pose.bones:
        bone.rotation_mode = "QUATERNION"
        bone.rotation_quaternion = quats.get(bone.name, Quaternion())
        bone.keyframe_insert("rotation_quaternion", frame=frame)
    root = rig.pose.bones["root"]
    root.location = (0.0, lift, 0.0)   # 骨頭朝 +Z，骨頭自己的 Y 是世界的 Z
    root.keyframe_insert("location", frame=frame)


def make_action(rig, name):
    """建一個動作：逐格（每秒 24 格）打關鍵幀。回傳 (action, 動作描述)。動作留在 rig 的 animation_data.action 上。"""
    import bpy
    spec = ACTIONS[name]()
    data = rig.animation_data_create()
    data.action = None
    action = bpy.data.actions.new(name)
    action.use_fake_user = True
    data.action = action
    names = [bone.name for bone in rig.pose.bones]
    for frame in range(spec["frames"] + 1):
        quats, lift = pose_at(spec["keys"], frame, names)
        if spec.get("float"):
            lift += curve_at(spec["float"], frame)
        apply(rig, quats, lift, frame)
    for curve in _fcurves(action):
        for point in curve.keyframe_points:
            point.interpolation = "LINEAR"   # 每格都有鍵，格與格之間線性即可
    action["loop"], action["frames"] = spec["loop"], spec["frames"]
    return action, spec


def build(rig):
    """建全部動作，每個動作一條 NLA 軌（供在 Blender 裡檢視；取樣匯出見 export_poses.py）。"""
    data = rig.animation_data_create()
    for name in ACTIONS:
        action, _ = make_action(rig, name)
        slot = data.action_slot
        data.action = None
        track = data.nla_tracks.new()
        track.name = name
        strip = track.strips.new(name, 0, action)
        if slot is not None and hasattr(strip, "action_slot"):
            strip.action_slot = slot
    for track in data.nla_tracks:
        track.mute = False


def _fcurves(action):
    if hasattr(action, "fcurves"):
        return list(action.fcurves)
    out = []
    for layer in action.layers:
        for strip in layer.strips:
            for bag in strip.channelbags:
                out.extend(bag.fcurves)
    return out
