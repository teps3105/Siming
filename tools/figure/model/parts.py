"""人物部件表（8.11）：骨架的骨頭表、每個部件蓋住身體哪裡、形狀、綁在哪根骨頭。

座標：成人站立身高 1，腳底在原點，Blender 的 −Y 是正面、+Z 向上、+X 是人物的左手邊。
所有骨頭朝 +Z、不扭轉，所以骨頭自己的 X 軸就是世界的 X 軸、Y 軸是世界的 Z 軸、Z 軸是世界的 −Y 軸
（匯出成 glTF 後三軸與模型空間重合）。

頂點色（COLOR_0）：R 是擺動權重（袖口、衣擺下緣、髮帶尾端為 1），G 是緣邊遮罩（衣領、袖口、衣擺；身體上是鞋），
B 是腰帶遮罩。UV 是圓柱展開：u 繞垂直軸一圈、接縫放在背後，v 是部件內由下到上 0–1。
"""
import math

import bmesh
import bpy
from mathutils import Vector

# ── 骨架 ──────────────────────────────────────────────────────

ARM_ANGLE = math.radians(12.0)   # 站立時手臂往外張開的角度（A 字姿）
SHOULDER = Vector((0.105, 0.0, 0.77))
UPPER, FORE, HAND = 0.17, 0.15, 0.075


def _along(origin, length):
    return origin + Vector((math.sin(ARM_ANGLE), 0.0, -math.cos(ARM_ANGLE))) * length


ELBOW = _along(SHOULDER, UPPER)
WRIST = _along(ELBOW, FORE)
PALM = _along(WRIST, 0.045)
FINGERS = _along(WRIST, HAND)
HAND_END = _along(WRIST, 0.10)
HIP_X = 0.05
KNEE_Z, ANKLE_Z, HIP_Z = 0.27, 0.05, 0.50
HEAD_C = Vector((0.0, 0.0, 0.9285))
HEAD_R = Vector((0.066, 0.072, 0.0715))

# 名稱 → (關節位置, 父骨頭)；左（_l）在 +X。總數 32，規格上限 40。
BONES = {"root": ((0, 0, 0.0), None), "waist": ((0, 0, HIP_Z), "root"), "chest": ((0, 0, 0.62), "waist"),
         "neck": ((0, 0, 0.80), "chest"), "head": ((0, 0, 0.855), "neck")}
for _side, _tag in ((1, "_l"), (-1, "_r")):
    def _m(v):
        return (v.x * _side, v.y, v.z)
    BONES.update({
        "shoulder" + _tag: ((0.04 * _side, 0, 0.78), "chest"),
        "upper_arm" + _tag: (_m(SHOULDER), "shoulder" + _tag),
        "forearm" + _tag: (_m(ELBOW), "upper_arm" + _tag),
        "hand" + _tag: (_m(WRIST), "forearm" + _tag),
        "prop" + _tag: (_m(PALM), "hand" + _tag),
        "sleeve_1" + _tag: (_m(_along(SHOULDER, UPPER * 0.9)), "upper_arm" + _tag),
        "sleeve_2" + _tag: (_m(_along(ELBOW, FORE * 0.95)), "forearm" + _tag),
        "thigh" + _tag: (( HIP_X * _side, 0, HIP_Z), "root"),
        "shin" + _tag: ((HIP_X * _side, 0, KNEE_Z), "thigh" + _tag),
        "foot" + _tag: ((HIP_X * _side, 0, ANKLE_Z), "shin" + _tag),
    })
BONES.update({"hem_f": ((0, -0.09, 0.16), "waist"), "hem_b": ((0, 0.09, 0.16), "waist"),
              "hem_l": ((0.12, 0, 0.16), "waist"), "hem_r": ((-0.12, 0, 0.16), "waist"),
              "hair_1": ((0, 0.035, 1.0), "head"), "hair_2": ((0, 0.09, 0.96), "hair_1"), "hair_3": ((0, 0.14, 0.91), "hair_2")})


def mirror(vec, side):
    return Vector((vec.x * side, vec.y, vec.z))


# ── 網格基本件 ────────────────────────────────────────────────

def tube(bm, rings, segments=12, cap_start=True, cap_end=True):
    """沿一串環建一條管子。rings: [(中心, 橫向半徑, 前後半徑)]。接縫（環上第 0 點）在背後（+Y）。"""
    made = []
    for at, (centre, rx, ry) in enumerate(rings):
        centre = Vector(centre)
        before, after = Vector(rings[max(at - 1, 0)][0]), Vector(rings[min(at + 1, len(rings) - 1)][0])
        along = (after - before).normalized()
        depth = Vector((0, 1, 0))
        side = along.cross(depth).normalized()
        if side.dot(Vector((1, 0, 0))) < 0:
            side = -side
        ring = []
        for k in range(segments):
            a = 2 * math.pi * k / segments
            ring.append(bm.verts.new(centre + side * (math.sin(a) * rx) + depth * (math.cos(a) * ry)))
        made.append(ring)
    for a, b in zip(made, made[1:]):
        for k in range(segments):
            bm.faces.new((a[k], b[k], b[(k + 1) % segments], a[(k + 1) % segments]))
    for ring, (centre, _, _), on in ((made[0], rings[0], cap_start), (made[-1], rings[-1], cap_end)):
        if on:
            tip = bm.verts.new(Vector(centre))
            for k in range(segments):
                bm.faces.new((ring[(k + 1) % segments], ring[k], tip))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return made


def ellipsoid(bm, centre, radii, u=16, v=8):
    made = bmesh.ops.create_uvsphere(bm, u_segments=u, v_segments=v, radius=1.0)["verts"]
    for vert in made:
        vert.co = Vector(centre) + Vector((vert.co.x * radii[0], vert.co.y * radii[1], vert.co.z * radii[2]))
    return made


def key(co):
    return (round(co.x, 5), round(co.y, 5), round(co.z, 5))


def smoothstep(t):
    t = min(max(t, 0.0), 1.0)
    return t * t * (3 - 2 * t)


def chain_weights(s, bones, joints, half):
    """沿一串骨頭的權重。s 是頂點在這串骨頭上的位置，joints[i] 是 bones[i] 與 bones[i+1] 的交界位置，half 是交界兩側各混多寬。"""
    def after(i):   # 第 i 個交界之後（含）的比例
        return smoothstep((s - (joints[i] - half)) / (2 * half))
    out = {}
    for i, bone in enumerate(bones):
        w = (after(i - 1) if i > 0 else 1.0) * (1.0 - after(i) if i < len(joints) else 1.0)
        if w > 1e-4:
            out[bone] = w
    return out


def along_polyline(p, points):
    """點 p 投影到折線上的弧長位置。"""
    best, best_s, run = 1e9, 0.0, 0.0
    for a, b in zip(points, points[1:]):
        seg = b - a
        length = seg.length
        t = min(max((p - a).dot(seg) / (length * length), 0.0), 1.0) if length > 0 else 0.0
        d = (a + seg * t - p).length
        if d < best:
            best, best_s = d, run + t * length
        run += length
    return best_s


# ── 物件組裝 ──────────────────────────────────────────────────

def _cylinder_uv(bm, zmin, zmax):
    """圓柱展開：u 繞垂直軸、接縫在背後，v 由下到上；位在接縫與軸上的頂點依同一個面的其他頂點決定落在 0 或 1 側。"""
    layer = bm.loops.layers.uv.new("UVMap")
    span = max(zmax - zmin, 1e-6)
    for face in bm.faces:
        raw = []
        for loop in face.loops:
            co = loop.vert.co
            on_axis = co.x * co.x + co.y * co.y < 1e-10
            seam = abs(co.x) < 1e-6 and co.y > 0
            u = None if on_axis else (0.0 if seam else (math.atan2(co.x, -co.y) / (2 * math.pi) + 0.5))
            raw.append((u, seam))
        known = [u for u, seam in raw if u is not None and not seam]
        high = sum(1 for u in known if u > 0.5) > len(known) / 2 if known else False
        mean = (sum(known) / len(known)) if known else 0.5
        for loop, (u, seam) in zip(face.loops, raw):
            if seam:
                u = 1.0 if high else 0.0
            elif u is None:
                u = mean
            loop[layer].uv = (u, (loop.vert.co.z - zmin) / span)


def make(name, bm, collection, weigh, sway=None, trim=None, sash=None):
    """bmesh 變成帶骨架權重、頂點色與 UV 的物件。weigh(co) 回傳 {骨頭: 權重}；sway、trim、sash(co) 回傳 0–1。"""
    zs = [v.co.z for v in bm.verts]
    _cylinder_uv(bm, min(zs), max(zs))
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    for poly in mesh.polygons:
        poly.use_smooth = True
    obj = bpy.data.objects.new(name, mesh)
    collection.objects.link(obj)
    colors = mesh.color_attributes.new("Color", "FLOAT_COLOR", "POINT")
    for vert, item in zip(mesh.vertices, colors.data):
        co = vert.co
        item.color = (sway(co) if sway else 0.0, trim(co) if trim else 0.0, sash(co) if sash else 0.0, 1.0)
        influence = sorted(weigh(co).items(), key=lambda kv: -kv[1])[:4]
        total = sum(w for _, w in influence) or 1.0
        for bone, w in influence:
            group = obj.vertex_groups.get(bone) or obj.vertex_groups.new(name=bone)
            group.add([vert.index], w / total, "REPLACE")
    return obj


# ── 部件 ──────────────────────────────────────────────────────

def _torso_weights(co):
    return chain_weights(co.z, ["waist", "chest", "neck", "head"], [0.60, 0.795, 0.86], 0.03)


def _arm_weights(co, side):
    tag = "_l" if side > 0 else "_r"
    pts = [mirror(v, side) for v in (SHOULDER, ELBOW, WRIST, HAND_END)]
    s = along_polyline(co, pts)
    return chain_weights(s, ["shoulder" + tag, "upper_arm" + tag, "forearm" + tag, "hand" + tag],
                         [0.012, UPPER, UPPER + FORE], 0.03)


def _leg_weights(co, side):
    tag = "_l" if side > 0 else "_r"
    return chain_weights(HIP_Z - co.z, ["thigh" + tag, "shin" + tag, "foot" + tag], [HIP_Z - KNEE_Z, HIP_Z - ANKLE_Z], 0.03)


def _hand(bm, side):
    """手：扁的掌（連指）加拇指根，朝手臂方向。掌心朝向身體，拇指在前內側。"""
    a = ARM_ANGLE
    along = Vector((math.sin(a) * side, 0.0, -math.cos(a)))
    across = Vector((math.cos(a) * side, 0.0, math.sin(a)))
    depth = Vector((0.0, 1.0, 0.0))

    def blob(offset, radii, u, v):
        for vert in bmesh.ops.create_uvsphere(bm, u_segments=u, v_segments=v, radius=1.0)["verts"]:
            c = vert.co
            vert.co = mirror(WRIST, side) + along * (offset[2] + c.z * radii[2]) + across * (offset[0] + c.x * radii[0]) + depth * (offset[1] + c.y * radii[1])
    blob((0.0, 0.0, 0.050), (0.025, 0.013, 0.048), 10, 6)
    blob((-0.020, -0.011, 0.034), (0.010, 0.010, 0.024), 8, 5)


def body(collection):
    """身體：頭、頸、軀幹、雙臂與手、雙腿與腳。衣物蓋住的部分也建全，換裝與動作穿幫時才不會露出空洞。"""
    bm = bmesh.new()
    torso = [((0, 0.003, 0.455), 0.060, 0.055), ((0, 0.003, 0.50), 0.083, 0.062), ((0, 0.0, 0.56), 0.078, 0.056),
             ((0, 0.0, 0.62), 0.088, 0.058), ((0, 0.0, 0.70), 0.098, 0.060), ((0, 0.0, 0.76), 0.099, 0.056),
             ((0, 0.0, 0.795), 0.060, 0.048), ((0, 0.0, 0.84), 0.030, 0.030), ((0, 0.0, 0.88), 0.030, 0.030)]
    tube(bm, torso, 12)
    ellipsoid(bm, HEAD_C, HEAD_R, 16, 8)
    for side in (1, -1):
        points = [(SHOULDER, 0.024), (_along(SHOULDER, UPPER * 0.5), 0.023), (ELBOW, 0.021), (_along(ELBOW, FORE * 0.55), 0.019),
                  (WRIST, 0.017), (_along(WRIST, 0.02), 0.016)]
        tube(bm, [(mirror(p, side), r, r * 0.9) for p, r in points], 8)
        _hand(bm, side)
        legs = [((HIP_X * side, 0.0, HIP_Z), 0.034), ((HIP_X * side, 0.0, 0.39), 0.034), ((HIP_X * side, 0.0, KNEE_Z), 0.033),
                ((HIP_X * side, 0.0, 0.16), 0.027), ((HIP_X * side, 0.0, 0.075), 0.023)]
        tube(bm, [(p, r, r) for p, r in legs], 8, cap_end=False)
        ellipsoid(bm, (HIP_X * side, -0.03, 0.034), (0.030, 0.075, 0.034), 8, 6)

    def weigh(co):
        if co.z < 0.1 and abs(co.x) > 0.02 and co.y < 0.2:
            return _leg_weights(co, 1 if co.x > 0 else -1)
        near_arm = abs(co.x) > 0.115 and co.z > 0.36
        if near_arm:
            return _arm_weights(co, 1 if co.x > 0 else -1)
        if co.z < HIP_Z - 0.04 and abs(co.x) > 0.02:
            return _leg_weights(co, 1 if co.x > 0 else -1)
        return _torso_weights(co)

    return make("body.base", bm, collection, weigh, trim=lambda co: 1.0 if co.z < 0.075 else 0.0)


ROBE_RINGS = [((0, 0.02, 0.05), 0.172, 0.122), ((0, 0.015, 0.09), 0.162, 0.115), ((0, 0.01, 0.16), 0.142, 0.105),
              ((0, 0.005, 0.30), 0.112, 0.083), ((0, 0.003, 0.44), 0.096, 0.075), ((0, 0.0, 0.495), 0.090, 0.070),
              ((0, 0.0, 0.515), 0.094, 0.073), ((0, 0.0, 0.535), 0.090, 0.070), ((0, 0.0, 0.60), 0.092, 0.066),
              ((0, 0.0, 0.70), 0.100, 0.066), ((0, 0.0, 0.765), 0.098, 0.062), ((0, 0.0, 0.800), 0.058, 0.050),
              ((0, 0.0, 0.822), 0.042, 0.040)]


def _robe_surface(x, z, lift=0.004):
    """袍身前面上 (x, z) 處的點（往外抬一點）。"""
    for (ca, rxa, rya), (cb, rxb, ryb) in zip(ROBE_RINGS, ROBE_RINGS[1:]):
        if ca[2] <= z <= cb[2]:
            t = (z - ca[2]) / (cb[2] - ca[2])
            rx, ry, cy = rxa + (rxb - rxa) * t, rya + (ryb - rya) * t, ca[1] + (cb[1] - ca[1]) * t
            inside = max(1.0 - (x / rx) ** 2, 0.0)
            return Vector((x, cy - ry * math.sqrt(inside) - lift, z))
    return Vector((x, -0.06, z))


def robe_male_regular(collection):
    """男袍、常袖：身體往外撐的殼，肩略收、下襬外放，交領右衽（領緣兩條帶）；袖口寬、往下垂，袖與衣擺另接出去，綁在袖骨、衣擺骨上。"""
    bm = bmesh.new()
    tube(bm, ROBE_RINGS, 16)
    sleeve_keys = set()
    for side in (1, -1):
        sleeve = [(SHOULDER + Vector((-0.03, 0, 0.018)), 0.052, 0.050), (SHOULDER + Vector((0.01, 0, 0.002)), 0.056, 0.052),
                  (_along(SHOULDER, UPPER * 0.5), 0.052, 0.050),
                  (ELBOW, 0.056, 0.052), (_along(ELBOW, FORE * 0.5), 0.072, 0.062), (_along(WRIST, -0.02), 0.088, 0.070),
                  (_along(WRIST, 0.032), 0.092, 0.072)]
        for ring in tube(bm, [(mirror(p, side), rx, ry) for p, rx, ry in sleeve], 12, cap_end=False):
            sleeve_keys.update(key(v.co) for v in ring)
    collar_keys = set()
    for points in ([(0.034, 0.824), (0.014, 0.785), (-0.026, 0.71), (-0.058, 0.62), (-0.072, 0.545)],
                   [(-0.034, 0.824), (-0.024, 0.78), (-0.008, 0.745)]):
        before = {key(v.co) for v in bm.verts}
        _strip(bm, points, 0.015, _robe_surface)
        collar_keys.update(key(v.co) for v in bm.verts if key(v.co) not in before)

    def weigh(co):
        if key(co) in sleeve_keys:
            side = 1 if co.x > 0 else -1
            tag = "_l" if side > 0 else "_r"
            pts = [mirror(v, side) for v in (SHOULDER, ELBOW, WRIST, FINGERS)]
            s = along_polyline(co, pts)
            w = chain_weights(s, ["shoulder" + tag, "upper_arm" + tag, "forearm" + tag, "hand" + tag],
                              [0.012, UPPER, UPPER + FORE], 0.04)
            tail = smoothstep((s - UPPER * 0.55) / (UPPER * 0.45)) if s < UPPER else 1.0
            w1 = {"sleeve_1" + tag: 0.0}
            if s < UPPER:
                w1 = {"sleeve_1" + tag: tail * 0.5}
            elif s > UPPER + FORE * 0.5:
                w1 = {"sleeve_2" + tag: smoothstep((s - UPPER - FORE * 0.5) / (FORE * 0.5)) * 0.6}
            scale = 1.0 - sum(w1.values())
            out = {k: v * scale for k, v in w.items()}
            out.update(w1)
            return out
        if co.z < HIP_Z - 0.02:
            # 下襬跟著大腿走（腿彎曲時不從袍裡穿出來），最底下一段再分給衣擺骨。
            below = HIP_Z - 0.02 - co.z
            legs = smoothstep(below / 0.12)
            hem = smoothstep((0.20 - co.z) / 0.14) * 0.6
            left = smoothstep((co.x + 0.035) / 0.07)
            angle = math.atan2(co.x, -co.y)
            dirs = {"hem_f": 0.0, "hem_l": math.pi / 2, "hem_b": math.pi, "hem_r": -math.pi / 2}
            nearest = max(dirs, key=lambda k: math.cos(angle - dirs[k]))
            out = {"waist": 1.0 - legs, "thigh_l": legs * (1 - hem) * left, "thigh_r": legs * (1 - hem) * (1 - left), nearest: legs * hem}
            return {k: v for k, v in out.items() if v > 1e-3}
        return _torso_weights(co)

    def sway(co):
        if key(co) in sleeve_keys:
            s = along_polyline(co, [mirror(v, 1 if co.x > 0 else -1) for v in (SHOULDER, ELBOW, WRIST)])
            return smoothstep((s - UPPER) / FORE) ** 1.5
        return smoothstep((0.34 - co.z) / 0.30) ** 1.2 if co.z < 0.34 else 0.0

    def trim(co):
        if co.z < 0.095 or key(co) in collar_keys or (co.z > 0.795 and key(co) not in sleeve_keys):
            return 1.0
        if key(co) in sleeve_keys and along_polyline(co, [mirror(v, 1 if co.x > 0 else -1) for v in (SHOULDER, ELBOW, WRIST, FINGERS)]) > UPPER + FORE - 0.02:
            return 1.0
        return 0.0

    return make("robe.male_regular", bm, collection, weigh, sway=sway, trim=trim,
                sash=lambda co: 1.0 if 0.485 < co.z < 0.545 and abs(co.x) < 0.12 else 0.0)


def hair_topknot(collection):
    """髮髻：貼著頭頂的髮帽、頭頂一個髻、髻上垂下的髮帶（髮帶綁在 hair_1–3）。"""
    bm = bmesh.new()
    cap = ellipsoid(bm, HEAD_C + Vector((0, 0.004, 0.003)), HEAD_R * 1.06, 16, 8)
    drop = []
    for vert in cap:
        rel = vert.co - HEAD_C
        keep = rel.z > 0.012 or (rel.y > 0.02 and rel.z > -0.045)
        if rel.y < -0.02 and rel.z < 0.036:
            keep = False
        if not keep:
            drop.append(vert)
    bmesh.ops.delete(bm, geom=drop, context="VERTS")
    knot_top = Vector((0, 0.0, 1.035))
    ellipsoid(bm, knot_top, (0.030, 0.030, 0.034), 12, 6)
    ribbon = [((0, 0.032, 1.012), 0.012, 0.004), ((0, 0.075, 0.985), 0.016, 0.004), ((0, 0.115, 0.945), 0.018, 0.004),
              ((0, 0.150, 0.900), 0.017, 0.004), ((0, 0.165, 0.845), 0.012, 0.004)]
    ribbon_keys = {key(v.co) for ring in tube(bm, ribbon, 4) for v in ring}

    def weigh(co):
        if key(co) in ribbon_keys:
            return chain_weights(co.y, ["hair_1", "hair_2", "hair_3"], [0.075, 0.12], 0.03)
        return {"head": 1.0}

    def sway(co):
        return smoothstep((co.y - 0.04) / 0.12) ** 1.5 if key(co) in ribbon_keys else 0.0

    return make("hair.topknot", bm, collection, weigh, sway=sway)


def _on_head(x, z, lift=0.004):
    """頭面上 (x, z) 處的點，往外抬一點，才不被頭面蓋住。"""
    nx = x / HEAD_R.x
    nz = (z - HEAD_C.z) / HEAD_R.z
    inside = max(1.0 - nx * nx - nz * nz, 0.0)
    y = HEAD_C.y - HEAD_R.y * math.sqrt(inside)
    return Vector((x, y - lift, z))


def _strip(bm, points, width, surface=_on_head):
    """表面上沿一串 (x, z) 點鋪的細帶，朝外（−Y）的一面為正面。"""
    rows = []
    for i, (x, z) in enumerate(points):
        a = points[max(i - 1, 0)]
        b = points[min(i + 1, len(points) - 1)]
        tx, tz = b[0] - a[0], b[1] - a[1]
        length = math.hypot(tx, tz) or 1.0
        nx, nz = -tz / length, tx / length
        half = width[i] * 0.5 if isinstance(width, (list, tuple)) else width * 0.5
        rows.append((bm.verts.new(surface(x + nx * half, z + nz * half)), bm.verts.new(surface(x - nx * half, z - nz * half))))
    for (a1, a2), (b1, b2) in zip(rows, rows[1:]):
        face = bm.faces.new((a1, b1, b2, a2))
        face.normal_update()
        if face.normal.y > 0:
            face.normal_flip()


def face_adult(collection):
    """壯年的臉：眉、眼（上眼線加瞳）、鼻、口的墨線小片，貼在頭的前面。眼在頭的中線略下、眉壓低、左右拉開。"""
    bm = bmesh.new()
    cz = HEAD_C.z
    for side in (1, -1):
        _strip(bm, [(side * 0.012, cz + 0.022), (side * 0.030, cz + 0.026), (side * 0.049, cz + 0.020)], [0.009, 0.011, 0.008])
        _strip(bm, [(side * 0.016, cz + 0.006), (side * 0.030, cz + 0.008), (side * 0.045, cz + 0.004)], 0.0085)
        _strip(bm, [(side * 0.031, cz + 0.004), (side * 0.031, cz - 0.006)], 0.010)
    _strip(bm, [(0.002, cz + 0.002), (0.005, cz - 0.014), (-0.002, cz - 0.020)], 0.0055)
    _strip(bm, [(-0.014, cz - 0.037), (0.0, cz - 0.040), (0.014, cz - 0.037)], 0.0075)
    return make("face.adult", bm, collection, lambda co: {"head": 1.0})
