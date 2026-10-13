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


ACTIONS = {"stand": _stand, "brace": _brace, "struck": _struck}


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
