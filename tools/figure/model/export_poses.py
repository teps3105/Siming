"""把 3D 骨架的動作逐格取樣，換算成 2D 棋子（CultivatorFigure）的關節姿勢，寫成 actions.json（8.11）。

    blender -b --factory-startup -P tools/figure/model/export_poses.py -- game/modules/figure/resources/actions.json

換算：
- 投影：人物正面三分面。Blender 的 +X 是人物左手邊、−Y 是正面、+Z 向上；看畫的人在人物的左前方（方位 AZIMUTH），
  所以 x2d = X·sin(AZ) + Y·cos(AZ)（往後是 +x，人物朝 −x），y2d = −Z。人物的右手（_r）離看畫的人遠，是 _f；左手（_l）是 _b。
- 以「立」第 0 格為錨：各關節 = 封面立姿（CultivatorFigure.POSES[STAND]）的位置 + 投影後的位移。立第 0 格與封面逐點相同，
  其餘格的位移是 3D 骨架真正動出來的量。手臂的位移乘臂長比（封面的臂比 3D 骨架短：上臂 ARM_K[0]、前臂 ARM_K[1]）。
- head_roll：頭的上軸在畫面上偏離立姿多少弧度（正值是頭往後仰）。
- anchors：定位點（頭頂 crown、腳底 foot、兩掌心 palm_f / palm_b），座標系同關節，以標準身形（7 頭身）算。
數值取小數五位。
"""
import json
import math
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import actions  # noqa: E402
import build  # noqa: E402

AZIMUTH = math.radians(45.0)
ARM_K = (0.8, 0.97)
# CultivatorFigure.POSES[Pose.STAND] 的關節（封面立姿）。
COVER = {"hip": (0.02, -0.50), "neck": (0.0, -0.84), "elbow_f": (-0.06, -0.66), "hand_f": (-0.015, -0.52),
         "elbow_b": (0.09, -0.66), "hand_b": (0.07, -0.51)}
SIDES = {"f": "_r", "b": "_l"}
HEAD_RADIUS = 0.5 / 7.0


def set_azimuth(degrees):
    global AZIMUTH
    AZIMUTH = math.radians(degrees)


def proj(v):
    return (v.x * math.sin(AZIMUTH) + v.y * math.cos(AZIMUTH), -v.z)


def turn(p, angle):
    """與 Godot 的 Vector2.rotated 同向（正值是頭往後、+x 倒）。"""
    c, s = math.cos(angle), math.sin(angle)
    return (p[0] * c - p[1] * s, p[0] * s + p[1] * c)


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def add(a, b, k=1.0):
    return (a[0] + b[0] * k, a[1] + b[1] * k)


def joints_of(rig, rigid=False, floating=0.0):
    """目前姿勢下各關節的投影點（未換算）。rigid：把 root 的俯仰與浮空高度拆掉，關節還原成直立座標，
    另記 roll（root 俯仰，頭往後為正）、float（浮空高度）與 pivot（root 的直立座標點）。"""
    bones = rig.pose.bones
    roll = 0.0
    root = proj(bones["root"].head)
    pivot = (root[0], root[1] + floating)

    def level(v):
        p = proj(v)
        if not rigid:
            return p
        d = turn((p[0] - root[0], p[1] - root[1]), -roll)
        return (d[0] + pivot[0], d[1] + pivot[1])
    if rigid:
        rx, ry = proj(bones["root"].matrix.to_3x3().col[1])
        roll = math.atan2(rx, -ry)
    out = {"hip": level(bones["waist"].head), "neck": level(bones["neck"].head)}
    for tag, bone_tag in SIDES.items():
        out["shoulder_" + tag] = level(bones["upper_arm" + bone_tag].head)
        out["elbow_" + tag] = level(bones["forearm" + bone_tag].head)
        out["hand_" + tag] = level(bones["hand" + bone_tag].head)
        out["foot_" + tag] = level(bones["foot" + bone_tag].head)
    up = bones["head"].matrix.to_3x3().col[1]
    sx, sy = proj(up)
    out["head_up"] = math.atan2(sx, -sy) - roll
    out["roll"], out["float"], out["pivot"] = roll, floating, pivot
    return out


def pose_of(now, ref, wind, rigid=False, feet=False):
    """一格的 CultivatorFigure 關節字典。"""
    pose = {}
    pose["hip"] = add(COVER["hip"], sub(now["hip"], ref["hip"]))
    pose["neck"] = add(COVER["neck"], sub(now["neck"], ref["neck"]))
    for tag in ("f", "b"):
        shoulder = sub(now["shoulder_" + tag], ref["shoulder_" + tag])
        upper = sub(sub(now["elbow_" + tag], now["shoulder_" + tag]), sub(ref["elbow_" + tag], ref["shoulder_" + tag]))
        fore = sub(sub(now["hand_" + tag], now["elbow_" + tag]), sub(ref["hand_" + tag], ref["elbow_" + tag]))
        elbow = add(add(COVER["elbow_" + tag], shoulder), upper, ARM_K[0])
        pose["elbow_" + tag] = elbow
        pose["hand_" + tag] = add(add(COVER["hand_" + tag], shoulder), add(add((0, 0), upper, ARM_K[0]), fore, ARM_K[1]))
    pose["head_roll"] = now["head_up"] - ref["head_up"]
    pose["wind"] = wind
    if feet:
        for tag in ("f", "b"):
            hip_dx = now["hip"][0] - ref["hip"][0]
            pose["foot_" + tag] = (now["foot_" + tag][0] - ref["foot_" + tag][0] - hip_dx, now["foot_" + tag][1] - ref["foot_" + tag][1])
    if rigid:
        pose["body_roll"] = now["roll"]
        pose["body_lift"] = now["float"]
        pose["body_pivot"] = add(COVER["hip"], sub(now["pivot"], ref["hip"]))
    return pose


def body_move(pose, point):
    """整體剛體變換（與 CultivatorFigure 相同）：繞 body_pivot 轉 body_roll，再上移 body_lift。"""
    if "body_roll" not in pose:
        return point
    pivot = pose["body_pivot"]
    moved = turn(sub(point, pivot), pose["body_roll"])
    return (pivot[0] + moved[0], pivot[1] + moved[1] - pose["body_lift"])


def anchors_of(pose):
    roll = pose["head_roll"]
    top = (-0.014, -HEAD_RADIUS * 2.12)
    crown = (pose["neck"][0] + top[0] * math.cos(roll) - top[1] * math.sin(roll), pose["neck"][1] + top[0] * math.sin(roll) + top[1] * math.cos(roll))
    out = {"crown": crown, "foot": (pose["hip"][0], 0.0)}
    for tag in ("f", "b"):
        along = sub(pose["hand_" + tag], pose["elbow_" + tag])
        length = math.hypot(*along) or 1.0
        out["palm_" + tag] = add(pose["hand_" + tag], along, 0.05 / length)
    if "body_roll" in pose:
        out = {k: body_move(pose, v) for k, v in out.items()}
    return out


def wind_at(keys, frame):
    for (a, va), (b, vb) in zip(keys, keys[1:]):
        if a <= frame <= b:
            return va + (vb - va) * ((frame - a) / (b - a) if b != a else 0.0)
    return keys[-1][1]


def r5(value):
    if isinstance(value, dict):
        return {k: r5(v) for k, v in value.items()}
    if isinstance(value, (tuple, list)):
        return [r5(v) for v in value]
    if isinstance(value, float):
        return round(value, 5)
    return value


def sample(rig, name, azimuth=None):
    scene = bpy.context.scene
    action, spec = actions.make_action(rig, name)
    rig.animation_data.action = action
    set_azimuth(spec.get("azimuth", 45) if azimuth is None else azimuth)
    frames = []
    for frame in range(spec["frames"] + 1):
        scene.frame_set(frame)
        bpy.context.view_layer.update()
        floating = actions.curve_at(spec["float"], frame) if spec.get("float") else 0.0
        frames.append(joints_of(rig, spec.get("rigid", False), floating))
    return spec, frames


def main(path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.fps = actions.FPS
    rig = build.build_rig(scene)
    stand_spec, stand_frames = sample(rig, "stand")
    refs = {45: stand_frames[0]}
    result = {"version": 1, "fps": actions.FPS, "azimuth_deg": round(math.degrees(AZIMUTH), 1), "actions": {}}
    for name in actions.ACTIONS:
        spec, frames = (stand_spec, stand_frames) if name == "stand" else sample(rig, name)
        azimuth = spec.get("azimuth", 45)
        if azimuth not in refs:
            refs[azimuth] = sample(rig, "stand", azimuth)[1][0]
        ref = refs[azimuth]
        poses = []
        for index, now in enumerate(frames):
            pose = pose_of(now, ref, wind_at(spec["wind"], index), spec.get("rigid", False), spec.get("feet", False))
            pose["seated"] = False
            pose["point"] = bool(actions.curve_at(spec["point"], index) > 0.5) if spec.get("point") else False
            pose["anchors"] = anchors_of(pose)
            poses.append(r5(pose))
        result["actions"][name] = {"loop": spec["loop"], "loop_start": spec["loop_start"], "length": spec["frames"],
                                   "events": spec["events"], "poses": poses}
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(result, handle, ensure_ascii=False, separators=(",", ":"))
    print("actions.json:", {n: a["length"] for n, a in result["actions"].items()})


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1])
