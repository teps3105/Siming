"""2D 關節直接寫關鍵姿勢的動作（8.11「肢體動作」）：跪、坐、轉身、執手、奉物與接過等整個身體輪廓大幅變化的動作。

這批動作的重點是剪影：誇大到 64 像素高、不看字卡也分得出哪一種事。3D 骨架只有「前後擺」的關節，畫不出棋子
的跪、坐、背身，所以直接在棋子的 2D 關節座標寫姿勢（身高為 1、腳底在原點、−y 向上、人朝 −x），不經 Blender。
不依賴 bpy：

    python tools/figure/model/poses2d.py game/modules/figure/resources/actions.json

把這批動作合進 actions.json（已有同名的取代，其餘動作原樣不動）；export_poses.py 重建全部動作時也呼叫 build()。

姿勢用 pose() 寫：腰 hip、上身前傾 lean（度，往前為正）、雙臂 f / b 各以兩個角度 A(上臂, 前臂)（0 是垂下、90 是向前水平、
180 是向上）或手的目標 H(x, y)（二節反向運動學，肘朝 bend 方向彎）給。肩的位置與 CultivatorFigure._build 一致
（頸 + 固定偏移），所以手臂長度固定、不會被拉長。關鍵格之間緩入緩出內插，seated、point 以 0.5 為界。
"""
import json
import math
import os
import sys

FPS = 24
STAND_HIP = (0.02, -0.50)
TORSO = (-0.02, -0.34)            # 立姿腰到頸的向量
SHOULDER_F = (-0.067, 0.05)       # 頸到朝前那一肩（CultivatorFigure 取 s = 0.09）
SHOULDER_B = (0.092, 0.058)
UPPER, FORE = 0.13, 0.15
HEAD_RADIUS = 0.5 / 7.0


class A:
    """手臂以角度給：上臂、前臂與垂下方向的夾角（度），往前為正。"""
    def __init__(self, upper, fore):
        self.upper, self.fore = upper, fore


class H:
    """手臂以手的位置給；bend：肘朝哪個方向彎（down 向下、back 向後、front 向前、up 向上）。"""
    def __init__(self, x, y, bend="down"):
        self.x, self.y, self.bend = x, y, bend


def _dir(deg):
    a = math.radians(deg)
    return (-math.sin(a), math.cos(a))


def _arm(shoulder, spec):
    if isinstance(spec, A):
        d1, d2 = _dir(spec.upper), _dir(spec.fore)
        elbow = (shoulder[0] + d1[0] * UPPER, shoulder[1] + d1[1] * UPPER)
        return elbow, (elbow[0] + d2[0] * FORE, elbow[1] + d2[1] * FORE)
    dx, dy = spec.x - shoulder[0], spec.y - shoulder[1]
    d = max(min(math.hypot(dx, dy), UPPER + FORE - 1e-4), abs(UPPER - FORE) + 1e-3)
    ux, uy = dx / math.hypot(dx, dy), dy / math.hypot(dx, dy)
    a = (UPPER * UPPER - FORE * FORE + d * d) / (2 * d)
    h = math.sqrt(max(UPPER * UPPER - a * a, 0.0))
    mid = (shoulder[0] + ux * a, shoulder[1] + uy * a)
    options = [(mid[0] - uy * h, mid[1] + ux * h), (mid[0] + uy * h, mid[1] - ux * h)]
    pick = {"down": lambda p: p[1], "up": lambda p: -p[1], "back": lambda p: p[0], "front": lambda p: -p[0]}[spec.bend]
    elbow = max(options, key=pick)
    return elbow, (shoulder[0] + ux * d, shoulder[1] + uy * d)


def pose(hip=STAND_HIP, lean=0.0, f=A(-3, -18), b=A(0, 8), head=None, wind=0.05, seated=0, point=0, flip=None, neck=None, feet=None):
    """一個姿勢：{hip, neck, elbow_f, hand_f, elbow_b, hand_b, head_roll, wind, seated, point[, flip]}。
    feet 是 (foot_f, foot_b) 雙腳相對立姿的位移（動作中每一格都要給，不然內插缺鍵）；neck 不給就由腰與前傾算出；head 是頭的後仰（弧度，前低為負），不給就跟著上身前傾。"""
    t = math.radians(lean)
    vx, vy = TORSO
    top = (vx * math.cos(t) + vy * math.sin(t), -vx * math.sin(t) + vy * math.cos(t))
    neck = neck or (hip[0] + top[0], hip[1] + top[1])
    out = {"hip": hip, "neck": neck}
    for tag, off, spec in (("f", SHOULDER_F, f), ("b", SHOULDER_B, b)):
        out["elbow_" + tag], out["hand_" + tag] = _arm((neck[0] + off[0], neck[1] + off[1]), spec)
    out["head_roll"] = -t * 0.9 if head is None else head
    out["wind"], out["seated"], out["point"] = wind, seated, point
    if flip is not None:
        out["flip"] = flip
    if feet is not None:
        out["foot_f"], out["foot_b"] = feet
    return out


def _smooth(t):
    return t * t * (3 - 2 * t)


def _mix(a, b, t):
    if isinstance(a, (tuple, list)):
        return tuple(x + (y - x) * t for x, y in zip(a, b))
    return a + (b - a) * t


def sample(keys, frame):
    """關鍵格 [(格號, 姿勢)] 之間緩入緩出；超出取端點。"""
    keys = sorted(keys, key=lambda k: k[0])
    if frame <= keys[0][0]:
        return dict(keys[0][1])
    for (fa, pa), (fb, pb) in zip(keys, keys[1:]):
        if fa <= frame <= fb:
            t = _smooth((frame - fa) / (fb - fa)) if fb != fa else 0.0
            return {k: _mix(pa[k], pb[k], t) for k in pa}
    return dict(keys[-1][1])


def anchors_of(p):
    roll = p["head_roll"]
    top = (-0.014, -HEAD_RADIUS * 2.12)
    c, s = math.cos(roll), math.sin(roll)
    out = {"crown": (p["neck"][0] + top[0] * c - top[1] * s, p["neck"][1] + top[0] * s + top[1] * c), "foot": (p["hip"][0], 0.0)}
    for tag in ("f", "b"):
        ex, ey = p["elbow_" + tag]
        hx, hy = p["hand_" + tag]
        length = math.hypot(hx - ex, hy - ey) or 1.0
        out["palm_" + tag] = (hx + (hx - ex) * 0.05 / length, hy + (hy - ey) * 0.05 / length)
    return out


def _r5(v):
    if isinstance(v, dict):
        return {k: _r5(x) for k, x in v.items()}
    if isinstance(v, (tuple, list)):
        return [_r5(x) for x in v]
    if isinstance(v, float):
        return round(v, 5)
    return v


def bake(spec):
    frames = spec["frames"]
    poses = []
    for index in range(frames + 1):
        p = sample(spec["keys"], index)
        p["seated"] = bool(p["seated"] > 0.5)
        p["point"] = bool(p["point"] > 0.5)
        p["anchors"] = anchors_of(p)
        poses.append(_r5(p))
    return {"loop": False, "loop_start": 0, "length": frames, "events": spec["events"], "poses": poses}


# ── 動作 ──────────────────────────────────────────────────────

def _stand():
    return pose()


def kneel():
    """跪拜叩首：0–14 屈膝跪下（髖降到 −0.27、雙手合在胸前）；14–22 上身往前撲下、雙手按地、臀高頭低；
    22–36 叩首停住（額近地，極輕的起伏）；36–44 直起上身回到跪；44–54 站起。down 是額頭落下，up 是開始直起。"""
    kneeling = pose(hip=(0.06, -0.27), lean=0, f=H(-0.15, -0.46), b=H(-0.10, -0.44), head=0.05)
    down = pose(hip=(0.14, -0.26), lean=78, f=H(-0.27, -0.19, "up"), b=H(-0.22, -0.2, "up"), head=-1.2, wind=0.07)
    down2 = pose(hip=(0.14, -0.255), lean=82, f=H(-0.27, -0.19, "up"), b=H(-0.22, -0.2, "up"), head=-1.3, wind=0.07)
    keys = [(0, _stand()), (8, pose(hip=(0.03, -0.38), lean=4, f=A(22, 40), b=A(20, 38))), (14, kneeling), (22, down),
            (30, down2), (36, down), (44, kneeling), (49, pose(hip=(0.03, -0.40), lean=4, f=A(22, 40), b=A(20, 38))), (54, _stand())]
    return {"frames": 54, "keys": keys, "events": {"down": 22, "up": 36}}


def bless():
    """撫頂（對跪著的人）：10 上身前傾 20°、遠手舉過頭；16 舉到最高；22 touch 上身前傾 40°、遠手落到身前最遠處（頭頂高度）按下，
    近手甩向身後；拍兩下；40 release 收手；54 立姿。"""
    up = pose(hip=(0.05, -0.49), lean=22, f=A(172, 178), b=A(-70, -80), head=-0.2, wind=0.08)
    top = pose(hip=(0.07, -0.48), lean=30, f=A(176, 178), b=A(-78, -88), head=-0.3, wind=0.09)
    reach = pose(hip=(0.10, -0.46), lean=42, f=H(-0.46, -0.52), b=A(-82, -90), head=-0.55, wind=0.1)
    pat = pose(hip=(0.10, -0.46), lean=42, f=H(-0.46, -0.58), b=A(-82, -90), head=-0.55, wind=0.1)
    keys = [(0, _stand()), (8, up), (14, top), (22, reach), (26, pat), (30, reach), (34, pat), (40, pose(lean=14, f=A(50, 60), b=A(-30, -45), head=-0.1)), (54, _stand())]
    return {"frames": 54, "keys": keys, "events": {"touch": 22, "release": 40}}


def _seated(neck_dy=0.0):
    p = {"hip": (0.02, -0.10), "neck": (0.01, -0.47 + neck_dy), "elbow_f": (-0.13, -0.27), "hand_f": (-0.05, -0.15),
         "elbow_b": (0.15, -0.27), "hand_b": (0.07, -0.15), "head_roll": 0.0, "wind": 0.02, "seated": 1, "point": 0}
    return p


def sit():
    """盤坐運功：0–16 沉身坐下（14 格前仍是站著的袍，15 格起換成盤腿）；16–46 停住調息（頸極輕的起伏）；
    46–58 起身；64 立姿，接得上 rise。settle 是坐定，rise 是開始起身。"""
    keys = [(0, _stand()), (8, pose(hip=(0.03, -0.30), lean=6, f=A(20, 40), b=A(18, 36))), (13, pose(hip=(0.03, -0.20), lean=8, f=A(28, 52), b=A(26, 48))),
            (14, _seated()), (16, _seated(0.0)), (24, _seated(0.01)), (32, _seated(-0.004)), (40, _seated(0.008)), (46, _seated(0.0)),
            (47, pose(hip=(0.03, -0.20), lean=8, f=A(28, 52), b=A(26, 48))), (55, pose(hip=(0.03, -0.32), lean=6, f=A(20, 40), b=A(18, 36))), (64, _stand())]
    return {"frames": 64, "keys": keys, "events": {"settle": 16, "rise": 46}}


def slump():
    """盤坐後垂首不動（壽終）：0–16 坐下；16–26 坐定；26–46 頭慢慢垂下、上身塌向前、雙手滑進腿上；46 起不動。
    droop 是頭開始垂，still 是完全不動。"""
    slumped = {"hip": (0.03, -0.10), "neck": (-0.07, -0.43), "elbow_f": (-0.12, -0.24), "hand_f": (-0.06, -0.10),
               "elbow_b": (0.13, -0.24), "hand_b": (0.06, -0.10), "head_roll": -1.0, "wind": 0.02, "seated": 1, "point": 0}
    keys = [(0, _stand()), (8, pose(hip=(0.03, -0.30), lean=6, f=A(20, 40), b=A(18, 36))), (13, pose(hip=(0.03, -0.20), lean=8, f=A(28, 52), b=A(26, 48))),
            (14, _seated()), (16, _seated()), (26, _seated()), (36, dict(slumped, neck=(-0.045, -0.45), head_roll=-0.5)), (46, slumped), (60, dict(slumped, neck=(-0.10, -0.41), head_roll=-1.25))]
    return {"frames": 60, "keys": keys, "events": {"droop": 26, "still": 46}}


def turn():
    """拂袖轉身：0–6 近手抬到胸前（袖攏起）；6–14 甩袖向後上方、整個人翻面（flip 1 → −1，經過 0 時是側薄的一線）；
    14–34 袖垂回、風量退去；34 起背對原方向站定。sweep 是甩袖最遠，turned 是翻面完成。"""
    wind_up = pose(lean=-2, f=A(-5, -20), b=H(-0.10, -0.72, "back"), head=0.05, wind=0.08)
    sweep = pose(lean=-3, f=A(-5, -20), b=A(-120, -150), head=0.0, wind=0.2, flip=0.0)
    end = pose(f=A(-3, -18), b=A(-45, -60), wind=0.2, flip=-1.0)
    keys = [(0, pose(flip=1.0)), (6, dict(wind_up, flip=1.0)), (10, dict(sweep, flip=0.6)), (12, dict(sweep, flip=0.0)), (14, pose(lean=-2, f=A(-5, -20), b=A(-108, -140), head=0.0, wind=0.2, flip=-1.0)),
            (22, pose(f=A(-4, -20), b=A(-60, -80), wind=0.2, flip=-1.0)), (34, pose(flip=-1.0)), (40, pose(flip=-1.0))]
    return {"frames": 40, "keys": keys, "events": {"sweep": 12, "turned": 14}}


def away():
    """背過身：6 抬手預備；12 甩袖橫掃（一袖向後上、一袖向前、衣擺飄起）整個人翻面（flip 1 → −1）；20 turned 翻面完成，
    低頭、遠手掩面、近手抱在胸前、肩線下沉；34 放鬆；40 站定（背對原方向）。turned 是翻面完成。"""
    wind_up = pose(lean=-2, f=A(-5, -20), b=H(-0.10, -0.72, "back"), head=0.05, wind=0.08)
    sweep = pose(lean=-4, f=A(75, 95), b=A(-125, -155), head=0.0, wind=0.2, flip=0.0)
    covered = pose(hip=(0.05, -0.43), lean=26, f=H(-0.25, -0.82, "front"), b=H(-0.22, -0.56, "back"), head=-0.85, wind=0.05, flip=-1.0)
    keys = [(0, pose(flip=1.0)), (6, dict(wind_up, flip=1.0)), (10, dict(sweep, flip=0.5)), (12, sweep), (14, dict(sweep, flip=-0.5)),
            (20, covered), (30, covered), (36, pose(lean=4, head=-0.2, flip=-1.0)), (40, pose(flip=-1.0))]
    return {"frames": 40, "keys": keys, "events": {"turned": 20}}


def _z(**kw):
    kw.setdefault("feet", ((0.0, 0.0), (0.0, 0.0)))
    return pose(**kw)


def clasp():
    """執手：兩人各跨一大步（雙腳前後分開、髖在前腳上方）、上身前傾、雙臂向前平伸到兩人中間（遠手掌心落在身前 0.32 = PAIR_GAP）相接；
    8–20 跨步伸手；20 meet 兩手相接；20–36 握住；36 release 鬆手；36–50 收手收步；56 立姿。"""
    step = _z(hip=(0.03, -0.47), lean=10, f=A(55, 75), b=A(35, 60), head=-0.05, feet=((-0.12, 0.0), (0.06, 0.0)))
    meet = _z(hip=(0.08, -0.45), lean=32, f=H(-0.28, -0.70), b=H(-0.24, -0.62), head=-0.35, wind=0.1, feet=((-0.30, 0.0), (0.28, 0.0)))
    keys = [(0, _z()), (8, step), (20, meet), (30, dict(meet, wind=0.06)), (36, meet),
            (42, _z(hip=(0.03, -0.47), lean=10, f=A(55, 75), b=A(35, 60), feet=((-0.12, 0.0), (0.06, 0.0)))), (50, _z(lean=2, f=A(12, 30), b=A(2, 12))), (56, _z())]
    return {"frames": 56, "keys": keys, "events": {"meet": 20, "release": 36}}


def offer():
    """雙手奉物：跨出一步、雙手高舉過胸、上身前傾約 30°、頭深低。12 雙手端到胸前；24–38 雙臂向前上方高托過下巴；
    30 give 把東西遞出；38–50 手收回；60 立姿。"""
    cup = _z(hip=(0.02, -0.48), lean=12, f=H(-0.16, -0.64), b=H(-0.10, -0.62), head=-0.2, feet=((-0.06, 0.0), (0.03, 0.0)))
    full = _z(hip=(0.04, -0.45), lean=32, f=H(-0.40, -0.90), b=H(-0.30, -0.88), head=-0.7, wind=0.08, feet=((-0.20, 0.0), (0.12, 0.0)))
    keys = [(0, _z()), (12, cup), (24, full), (38, full), (46, cup), (60, _z())]
    return {"frames": 60, "keys": keys, "events": {"give": 30, "retreat": 38}}


def receive():
    """接過：屈膝半蹲、雙腳分開、雙手高舉過頭去接；22 take 接住；28–44 收下來抱在胸前、頭低下看；60 立姿。"""
    catch = _z(hip=(0.05, -0.23), lean=-10, f=A(150, 172), b=A(125, 156), head=0.4, wind=0.08, feet=((-0.26, 0.0), (0.26, 0.0)))
    held = _z(hip=(0.03, -0.47), lean=14, f=H(-0.10, -0.58), b=H(-0.06, -0.55), head=-0.45)
    keys = [(0, _z()), (8, _z(hip=(0.04, -0.40), lean=-4, f=A(70, 110), b=A(55, 95), head=0.1, feet=((-0.08, 0.0), (0.08, 0.0)))), (22, catch), (28, catch),
            (40, held), (50, held), (60, _z())]
    return {"frames": 60, "keys": keys, "events": {"take": 22, "hold": 40}}


def salute():
    """抱拳：8 抬手到胸前、雙肘向兩側張開成一個菱形（拳掌相抱）、雙腳分開；22 salute 前傾 24° 深深一揖；
    22–34 停住；34 release 起身；54 立姿。"""
    ready = _z(hip=(0.03, -0.47), lean=6, f=H(-0.16, -0.70, "up"), b=H(-0.12, -0.68, "back"), head=0.0, feet=((-0.08, 0.0), (0.08, 0.0)))
    full = _z(hip=(0.02, -0.34), lean=10, f=H(-0.20, -0.74, "up"), b=H(-0.12, -0.72, "back"), head=-0.6, wind=0.1, feet=((-0.32, 0.0), (0.32, 0.0)))
    keys = [(0, _z()), (10, ready), (22, full), (34, full), (42, ready), (54, _z())]
    return {"frames": 54, "keys": keys, "events": {"salute": 22, "release": 34}}


def hail():
    """振臂向天：8 屈膝蓄力、雙臂收向身後；24 raise 雙臂伸直舉向天空成 V 形、仰頭、挺胸、雙腳分開；24–44 停住；56 放下回立姿。"""
    gather = _z(hip=(0.04, -0.40), lean=12, f=A(-30, -40), b=A(-40, -50), head=-0.1, feet=((-0.10, 0.0), (0.10, 0.0)))
    sky = _z(hip=(0.01, -0.46), lean=-10, f=A(140, 150), b=A(-140, -150), head=0.7, wind=0.12, feet=((-0.24, 0.0), (0.24, 0.0)))
    keys = [(0, _z()), (10, gather), (24, sky), (44, sky), (56, _z())]
    return {"frames": 56, "keys": keys, "events": {"raise": 24, "release": 44}}


def token():
    """高舉信物（傳承）：0–14 跪下、雙手捧在身前去接；16 receive 接住；16–28 停住低頭；28–40 起身，一臂向上直直舉過頭；
    40 lift 舉到最高、仰頭；40–54 停住；72 放下回立姿。"""
    kneeling = _z(hip=(0.06, -0.27), lean=0, f=H(-0.28, -0.50), b=H(-0.22, -0.46), head=-0.2)
    held = _z(hip=(0.06, -0.27), lean=14, f=H(-0.14, -0.58), b=H(-0.10, -0.54), head=-0.5)
    rising = _z(hip=(0.03, -0.40), lean=4, f=A(60, 90), b=A(-10, -20), head=0.0, feet=((-0.08, 0.0), (0.08, 0.0)))
    up = _z(hip=(0.06, -0.44), lean=-20, f=A(172, 178), b=A(-85, -90), head=0.6, wind=0.1, feet=((-0.22, 0.0), (0.18, 0.0)))
    keys = [(0, _z()), (8, _z(hip=(0.03, -0.38), lean=4, f=A(22, 40), b=A(20, 38))), (14, kneeling), (16, kneeling), (24, held), (30, held), (36, rising), (40, up),
            (54, up), (72, _z())]
    return {"frames": 72, "keys": keys, "events": {"receive": 16, "lift": 40}}


def writhe():
    """抱頭掙扎跪倒（入魔）：0–10 雙手抱頭、雙肘向兩側張開；10–30 搖晃掙扎（身體前後打擺、頭甩動）；30 clutch 最劇烈；
    30–44 腿軟屈膝；44 collapse 跪倒、上身伏下、雙手仍抱著頭；64 停住。終點是跪伏姿勢，不接立姿。"""
    clutch = _z(hip=(0.02, -0.48), lean=-6, f=H(-0.07, -0.88, "up"), b=H(0.0, -0.90, "back"), head=0.3, wind=0.1, feet=((-0.10, 0.0), (0.10, 0.0)))
    sway_a = _z(hip=(0.08, -0.44), lean=18, f=H(-0.15, -0.80, "up"), b=H(-0.07, -0.82, "back"), head=-0.5, wind=0.12, feet=((-0.16, 0.0), (0.14, 0.0)))
    sway_b = _z(hip=(-0.06, -0.44), lean=-24, f=H(-0.05, -0.92, "up"), b=H(0.03, -0.94, "back"), head=0.7, wind=0.12, feet=((-0.06, 0.0), (0.16, 0.0)))
    knees = _z(hip=(0.06, -0.30), lean=14, f=H(-0.14, -0.62, "up"), b=H(-0.08, -0.62, "back"), head=-0.4, wind=0.08, feet=((-0.08, 0.0), (0.08, 0.0)))
    down = _z(hip=(0.12, -0.26), lean=62, f=H(-0.22, -0.45, "up"), b=H(-0.14, -0.46, "back"), head=-1.1, wind=0.06)
    keys = [(0, _z()), (8, clutch), (14, sway_a), (20, sway_b), (26, sway_a), (30, clutch), (38, knees), (44, down), (64, dict(down, hip=(0.12, -0.25)))]
    return {"frames": 64, "keys": keys, "events": {"clutch": 30, "collapse": 44}}


def cough():
    """掩口踉蹌（瘟疫）：6 一手掩口、身子前傾；10 cover；10–30 咳得身體前後打擺、腳步亂踏、另一臂張開找平衡；30 stagger 踉蹌最大一步；
    38–48 直回立姿（接 fall）。"""
    cover = _z(hip=(0.05, -0.47), lean=16, f=H(-0.17, -0.90, "up"), b=A(-50, -60), head=-0.5, feet=((-0.08, 0.0), (0.04, 0.0)))
    lurch_a = _z(hip=(0.09, -0.46), lean=26, f=H(-0.20, -0.84, "up"), b=A(-85, -90), head=-0.7, wind=0.1, feet=((-0.18, 0.0), (0.12, 0.0)))
    lurch_b = _z(hip=(-0.03, -0.49), lean=-6, f=H(-0.13, -0.92, "up"), b=A(-70, -50), head=0.1, wind=0.1, feet=((0.04, 0.0), (0.14, 0.0)))
    stagger = _z(hip=(-0.07, -0.42), lean=-26, f=H(-0.10, -0.94, "up"), b=A(-110, -100), head=0.5, wind=0.12, feet=((-0.04, 0.0), (0.26, 0.0)))
    keys = [(0, _z()), (6, cover), (10, cover), (16, lurch_a), (22, lurch_b), (26, lurch_a), (30, stagger), (38, dict(cover, wind=0.07)), (48, _z())]
    return {"frames": 48, "keys": keys, "events": {"cover": 10, "stagger": 30}}


def chop():
    """揮劍劈下：8 雙手合握舉過頭頂、身後仰、雙腳分開；16 raise 舉到最高；24 hit 雙臂劈到身前低處、前傾 36°、前腳弓出；
    24–32 停住；32–44 收回；44 立姿。"""
    wind_up = _z(hip=(0.03, -0.48), lean=-8, f=A(150, 170), b=A(140, 165), head=0.3, feet=((-0.06, 0.0), (0.10, 0.0)))
    high = _z(hip=(0.04, -0.47), lean=-16, f=A(172, 178), b=A(165, 176), head=0.6, wind=0.1, feet=((-0.04, 0.0), (0.18, 0.0)))
    cut = _z(hip=(0.10, -0.38), lean=50, f=H(-0.34, -0.08, "up"), b=H(-0.30, -0.06, "up"), head=-0.9, wind=0.12, feet=((-0.34, 0.0), (0.30, 0.0)))
    keys = [(0, _z()), (8, wind_up), (16, high), (20, dict(high, wind=0.1)), (24, cut), (32, cut), (44, _z())]
    return {"frames": 44, "keys": keys, "events": {"raise": 16, "hit": 24}}


def palm():
    """掌擊：6 雙臂收到腰側、屈膝沉身；12 charge 蓄滿；22 hit 近手拉回腰側、遠手向前平推到最遠、前腳弓步、上身前傾；22–30 停住；30–40 收回；40 立姿。"""
    charge = _z(hip=(0.05, -0.42), lean=-6, f=A(-30, -20), b=A(-50, -40), head=0.1, wind=0.08, feet=((0.02, 0.0), (0.14, 0.0)))
    push = _z(hip=(0.06, -0.41), lean=4, f=H(-0.40, -0.76), b=A(-60, -50), head=-0.1, wind=0.12, feet=((-0.34, 0.0), (0.28, 0.0)))
    keys = [(0, _z()), (6, _z(hip=(0.04, -0.46), lean=-2, f=A(-20, -10), b=A(-40, -30), feet=((0.0, 0.0), (0.06, 0.0)))), (12, charge), (22, push), (30, push), (40, _z())]
    return {"frames": 40, "keys": keys, "events": {"charge": 12, "hit": 22}}


def guard():
    """後退格擋：被打中的那一瞬（hit）起播：0 站著；6 block 雙臂交叉擋在面前、身子後仰、後腳退一大步；14–24 腳步繼續後滑；32 放下；36 立姿。"""
    block = _z(hip=(0.12, -0.46), lean=-14, f=H(-0.14, -0.92, "up"), b=H(-0.12, -0.86, "up"), head=0.2, wind=0.1, feet=((0.0, 0.0), (0.28, 0.0)))
    slide = _z(hip=(0.18, -0.46), lean=-12, f=H(-0.10, -0.92, "up"), b=H(-0.08, -0.86, "up"), head=0.2, wind=0.12, feet=((0.08, 0.0), (0.34, 0.0)))
    keys = [(0, _z()), (6, block), (14, slide), (24, slide), (32, _z(hip=(0.10, -0.48), lean=-4, f=A(30, 50), b=A(20, 40), feet=((0.04, 0.0), (0.16, 0.0)))), (36, _z())]
    return {"frames": 36, "keys": keys, "events": {"block": 6}}


ACTIONS = {"kneel": kneel, "bless": bless, "sit": sit, "slump": slump, "turn": turn, "away": away, "clasp": clasp, "offer": offer, "receive": receive,
           "salute": salute, "hail": hail, "token": token, "writhe": writhe, "cough": cough, "chop": chop, "palm": palm, "guard": guard}


def build():
    return {name: bake(make()) for name, make in ACTIONS.items()}


def merge(path):
    with open(path, encoding="utf-8") as handle:
        data = json.load(handle)
    data["actions"].update(build())
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(data, handle, ensure_ascii=False, separators=(",", ":"))
    print("poses2d:", {n: a["length"] for n, a in build().items()})


if __name__ == "__main__":
    merge(sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), "..", "..", "..", "game", "modules", "figure", "resources", "actions.json"))
