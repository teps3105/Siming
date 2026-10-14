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


def pose(hip=STAND_HIP, lean=0.0, f=A(-3, -18), b=A(0, 8), head=None, wind=0.05, seated=0, point=0, flip=None, neck=None):
    """一個姿勢：{hip, neck, elbow_f, hand_f, elbow_b, hand_b, head_roll, wind, seated, point[, flip]}。
    neck 不給就由腰與前傾算出；head 是頭的後仰（弧度，前低為負），不給就跟著上身前傾。"""
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
    """撫頂（對跪著的人）：上身深深前傾約 28°、遠手向前下方伸到對方頭頂（手落在身前 0.30、高 −0.50），近手甩向身後上方作平衡；
    8–20 彎身伸手；22 touch 掌落在頂上，拍兩下；40 release 收手；54 立姿。"""
    reach = pose(hip=(0.04, -0.49), lean=28, f=H(-0.30, -0.50), b=A(-62, -75), head=-0.35, wind=0.08)
    pat = pose(hip=(0.04, -0.49), lean=28, f=H(-0.30, -0.55), b=A(-62, -75), head=-0.35, wind=0.08)
    keys = [(0, _stand()), (10, pose(lean=14, f=A(60, 70), b=A(-40, -55), head=-0.1, wind=0.07)), (20, reach), (22, reach), (26, pat), (30, reach),
            (34, pat), (40, pose(lean=12, f=A(50, 60), b=A(-30, -45), head=-0.1)), (54, _stand())]
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
    """背過身（不甩袖）：頭先低下、再整個人翻面，雙手垂下；轉定後肩略垮。turned 是翻面完成。"""
    keys = [(0, pose(flip=1.0)), (8, pose(lean=4, head=-0.25, f=A(2, -10), b=A(4, 10), flip=1.0)), (14, pose(lean=4, head=-0.25, flip=0.0)),
            (20, pose(lean=4, head=-0.25, flip=-1.0)), (34, pose(lean=2, head=-0.12, f=A(0, -12), b=A(2, 8), flip=-1.0)), (40, pose(lean=2, head=-0.12, f=A(0, -12), b=A(2, 8), flip=-1.0))]
    return {"frames": 40, "keys": keys, "events": {"turned": 20}}


def clasp():
    """執手：兩人各伸遠手到兩人中間（掌心落在身前 0.32，與 PAIR_GAP 相等）相接，另一臂向後張開、屈膝前傾。
    8–20 遠手前伸；20 meet 兩手相接；20–36 握住；36–50 鬆手收回；56 立姿。release 是開始鬆手。"""
    meet = pose(hip=(0.05, -0.46), lean=14, f=H(-0.28, -0.62), b=A(-48, -40), head=-0.1, wind=0.09)
    keys = [(0, _stand()), (8, pose(hip=(0.03, -0.48), lean=8, f=A(45, 66), b=A(-20, -25), head=-0.05)), (20, meet), (30, dict(meet, wind=0.06)), (36, meet),
            (42, pose(lean=8, f=A(52, 70), b=A(-20, -25))), (50, pose(lean=2, f=A(12, 30), b=A(2, 12))), (56, _stand())]
    return {"frames": 56, "keys": keys, "events": {"meet": 20, "release": 36}}


def offer():
    """雙手奉物：雙手高舉過胸、上身前傾約 24°、頭深低。12 雙手端到胸前；24–38 雙臂向前上方平托到下巴高度；
    30 give 把東西遞出；38–50 手收回；60 立姿。"""
    cup = pose(hip=(0.02, -0.48), lean=12, f=H(-0.16, -0.64), b=H(-0.10, -0.62), head=-0.2)
    full = pose(hip=(0.01, -0.47), lean=24, f=H(-0.32, -0.80), b=H(-0.24, -0.78), head=-0.6, wind=0.08)
    keys = [(0, _stand()), (12, cup), (24, full), (38, full), (46, cup), (60, _stand())]
    return {"frames": 60, "keys": keys, "events": {"give": 30, "retreat": 38}}


def receive():
    """雙手捧接：身略後仰、雙手在腹前低低攤開。10–22 雙手攤開向前；22 take 接住；28–44 收到胸前、頭低下看；60 立姿。"""
    open_hand = pose(hip=(0.03, -0.43), lean=8, f=H(-0.31, -0.46), b=H(-0.20, -0.42), head=-0.05)
    held = pose(hip=(0.03, -0.48), lean=10, f=H(-0.10, -0.58), b=H(-0.06, -0.55), head=-0.45)
    keys = [(0, _stand()), (10, pose(lean=0, f=A(40, 62), b=A(20, 40), head=0.0)), (22, open_hand), (28, open_hand), (40, held), (50, held), (60, _stand())]
    return {"frames": 60, "keys": keys, "events": {"take": 22, "hold": 40}}


ACTIONS = {"kneel": kneel, "bless": bless, "sit": sit, "slump": slump, "turn": turn, "away": away, "clasp": clasp, "offer": offer, "receive": receive}


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
