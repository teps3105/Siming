"""把 .blend 匯出成 figure.glb，並重新匯入核對（8.11）。

    blender -b <.blend> -P tools/figure/model/export.py -- <輸出.glb>

核對項目：骨頭數 ≤ 40、部件名、動作名、每個部件與全部部件的三角形數、檔案大小；任一項不合就以非零結束碼結束。
"""
import os
import sys

import bpy

EXPECT_PARTS = {"body.base", "robe.male_regular", "hair.topknot", "face.adult"}
EXPECT_ACTIONS = {"stand", "brace", "struck"}
MAX_BONES, MAX_FILE, MAX_ALL, MAX_SHOWN = 40, 2 * 1024 * 1024, 20000, 3000


def export(path):
    bpy.ops.export_scene.gltf(
        filepath=path, export_format="GLB", export_yup=True, export_apply=False, export_animations=True,
        export_animation_mode="NLA_TRACKS", export_force_sampling=True, export_skins=True, export_influence_nb=4,
        export_all_influences=False, export_def_bones=False, export_leaf_bone=False, export_extras=False,
        export_vertex_color="ACTIVE", export_image_format="NONE", export_materials="NONE", export_cameras=False,
        export_lights=False, export_optimize_animation_size=False)


def verify(path):
    """重新匯入 path 到空場景，回傳問題清單。"""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=path)
    problems = []
    rigs = [o for o in bpy.data.objects if o.type == "ARMATURE"]
    meshes = {o.name: o for o in bpy.data.objects if o.type == "MESH" and not o.name.startswith("Icosphere")}  # 匯入器替骨頭建的顯示形狀不算部件
    bones = len(rigs[0].data.bones) if rigs else 0
    names = {n.rsplit(".", 1)[0] if n.count(".") > 1 else n for n in meshes}
    actions = {a.name for a in bpy.data.actions}
    tris = {}
    for name, obj in meshes.items():
        mesh = obj.data
        mesh.calc_loop_triangles()
        tris[name] = len(mesh.loop_triangles)
    total = sum(tris.values())
    size = os.path.getsize(path)
    if len(rigs) != 1:
        problems.append(f"骨架數 {len(rigs)}")
    if bones > MAX_BONES:
        problems.append(f"骨頭 {bones} 超過 {MAX_BONES}")
    if names != EXPECT_PARTS:
        problems.append(f"部件名 {sorted(names)}")
    if not EXPECT_ACTIONS <= {a.split("|")[-1] for a in actions}:
        problems.append(f"動作名 {sorted(actions)}")
    if total > MAX_ALL or any(t > MAX_SHOWN for t in tris.values()):
        problems.append(f"三角形 {total}")
    if sum(tris.values()) - tris.get("hair.topknot", 0) > MAX_SHOWN * 2:
        problems.append("一個人顯示的部件超過 3,000 三角形")
    if size > MAX_FILE:
        problems.append(f"檔案 {size} 位元組超過 2 MB")
    print(f"核對：骨頭 {bones}、部件 {sorted(names)}、動作 {sorted(actions)}、三角形 {tris}（合計 {total}）、檔案 {size} 位元組")
    return problems


def main(path):
    path = os.path.abspath(path)
    export(path)
    problems = verify(path)
    for problem in problems:
        print("✗", problem)
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1])
