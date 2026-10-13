"""各動作的關鍵姿勢（8.11 的動作表）。腳本直接寫關鍵姿勢，每秒 24 格，播放時內插。

每個關鍵格是 {骨頭: (前後, 扭轉, 側向)}，單位度；root 另有 lift（身體高低，公尺）。
- 前後：繞世界 X 轉。垂下的肢體（手臂、腿）正值往後擺、負值往前擺；立在關節上方的部位（胸、頸、頭）正值往前傾、負值往後仰。
- 扭轉：繞骨頭自己的軸（世界 Z）轉。
- 側向：左右肢體用「out」——正值離開身體；胸、頸、頭的側向是原始的繞骨頭 Z 軸角度。
腳底要貼地：蹲低時 squat() 同時給大腿、小腿、腳與身體高度。
"""
import math

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
    """立姿：雙手自然垂下略外張，重心隨 sway（−1 到 1）左右微移。"""
    return {"chest": (breath * 1.2, 0, sway * 0.8), "head": (-breath * 0.8, 0, -sway * 0.6),
            "forearm_l": (-7, 0, 0), "forearm_r": (-7, 0, 0),
            "upper_arm_l": (breath * 1.5, 0, 3 + breath), "upper_arm_r": (breath * 1.5, 0, 3 + breath),
            "thigh_l": (0, 0, 1.5 - sway * 0.8), "thigh_r": (0, 0, 1.5 + sway * 0.8)}


def brace_pose(reach=0.0, breath=0.0):
    """迎雷：仰首，雙腳站開，人物的右手高舉向天、左手垂在身側微握。reach 是右臂再往上伸的角度。"""
    legs, drop = squat(7)
    pose = blend(legs, {
        "chest": (-9 + breath * 1.2, 0, 0), "neck": (-12, 0, 0), "head": (-22 - breath * 1.5, 0, 0),
        "upper_arm_r": (-14, 0, 140 + reach), "forearm_r": (-12, 0, 6), "hand_r": (-10, 0, 0),
        "upper_arm_l": (-6, 0, 24), "forearm_l": (-22, 0, 0),
        "thigh_l": (0, 0, 6), "thigh_r": (0, 0, 6), "waist": (-2, 0, 0)})
    pose["root"] = (0, 0, 0)
    return pose, -drop


# 每個動作：(格數, 是否循環, {格號: (姿勢, 身體高低)})；循環動作的最後一格等於第 0 格。
def _stand():
    keys = {}
    for frame, sway, breath in ((0, 0.0, 0.0), (12, 0.6, 0.5), (24, 0.0, 1.0), (36, -0.6, 0.5), (48, 0.0, 0.0)):
        keys[frame] = (stand_pose(sway, breath), -0.003 * breath)
    return 48, True, keys


def _brace():
    keys = {}
    for frame, reach, breath in ((0, 0, 0.0), (12, 6, 0.6), (24, 0, 1.0), (36, -5, 0.6), (48, 0, 0.0)):
        pose, drop = brace_pose(reach, breath)
        keys[frame] = (pose, drop)
    return 48, True, keys


def _struck():
    """受雷：由迎雷姿起，被擊中身體向後猛仰、雙臂甩開、雙膝一軟，再撐住，回到立姿。0.75 秒。"""
    start, start_drop = brace_pose()
    legs, drop = squat(16)
    hit = blend(legs, {"chest": (-26, 0, 0), "neck": (-14, 0, 0), "head": (-30, 0, 0), "waist": (-8, 0, 0),
                       "upper_arm_r": (-8, 0, 70), "forearm_r": (-22, 0, 0), "upper_arm_l": (-10, 0, 62), "forearm_l": (-24, 0, 0),
                       "thigh_l": (0, 0, 8), "thigh_r": (0, 0, 8)})
    legs2, drop2 = squat(24)
    recoil = blend(legs2, {"chest": (14, 0, 0), "neck": (6, 0, 0), "head": (10, 0, 0), "waist": (6, 0, 0),
                           "upper_arm_r": (-16, 0, 36), "forearm_r": (-34, 0, 0), "upper_arm_l": (-12, 0, 30), "forearm_l": (-30, 0, 0),
                           "thigh_l": (0, 0, 6), "thigh_r": (0, 0, 6)})
    legs3, drop3 = squat(10)
    brace_again = blend(legs3, {"chest": (4, 0, 0), "head": (-6, 0, 0), "upper_arm_r": (-6, 0, 16), "forearm_r": (-18, 0, 0),
                                "upper_arm_l": (-4, 0, 14), "forearm_l": (-16, 0, 0), "thigh_l": (0, 0, 5), "thigh_r": (0, 0, 5)})
    keys = {0: (start, start_drop), 3: (hit, -drop - 0.012), 8: (recoil, -drop2), 13: (brace_again, -drop3), 18: (stand_pose(), 0.0)}
    return 18, False, keys


ACTIONS = {"stand": _stand, "brace": _brace, "struck": _struck}


def apply(rig, pose, lift, frame, all_bones):
    """把一個姿勢寫成一格關鍵格（所有骨頭每格都鍵一次，匯出後每根骨頭的軌道等長）。"""
    for bone in all_bones:
        pitch, twist, side = pose.get(bone.name, (0, 0, 0))
        if bone.name.endswith("_l") and bone.name.split("_")[0] in LIMBS + ("upper",):
            side = side
        elif bone.name.endswith("_r") and bone.name.split("_")[0] in LIMBS + ("upper",):
            side = -side
        bone.rotation_mode = "XYZ"
        bone.rotation_euler = (math.radians(pitch), math.radians(twist), math.radians(side))
        bone.keyframe_insert("rotation_euler", frame=frame)
    root = rig.pose.bones["root"]
    root.location = (0.0, lift, 0.0)   # 骨頭朝 +Z，骨頭自己的 Y 是世界的 Z
    root.keyframe_insert("location", frame=frame)


def build(rig):
    """建全部動作，每個動作一條 NLA 軌（匯出時以軌名當動作名）。"""
    import bpy
    data = rig.animation_data_create()
    bones = list(rig.pose.bones)
    for name, make in ACTIONS.items():
        frames, loop, keys = make()
        action = bpy.data.actions.new(name)
        action.use_fake_user = True
        data.action = action
        for frame in sorted(keys):
            pose, lift = keys[frame]
            apply(rig, pose, lift, frame, bones)
        action["loop"], action["frames"] = loop, frames
        for fcurve_owner in _fcurves(action):
            for point in fcurve_owner.keyframe_points:
                point.interpolation = "BEZIER"
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
