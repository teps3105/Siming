"""建出人物的粗模、骨架、部件與動作，存成 .blend（8.11）。

    blender -b --factory-startup -P tools/figure/model/build.py -- <輸出.blend>

這一次只建：body、robe.male_regular、hair.topknot、face.adult 四個部件；骨架照 8.11 的骨頭表建全；動作 stand、brace、struck。
只用 Blender 內建功能，不開視窗。
"""
import os
import sys

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import actions  # noqa: E402
import parts  # noqa: E402

PARTS = [parts.body, parts.robe_male_regular, parts.hair_topknot, parts.face_adult]


def build_rig(scene):
    data = bpy.data.armatures.new("rig")
    rig = bpy.data.objects.new("rig", data)
    scene.collection.objects.link(rig)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    for name, (head, _) in parts.BONES.items():
        bone = data.edit_bones.new(name)
        bone.head = head
        bone.tail = (head[0], head[1], head[2] + 0.05)
        bone.roll = 0.0
    for name, (_, parent) in parts.BONES.items():
        if parent:
            data.edit_bones[name].parent = data.edit_bones[parent]
    bpy.ops.object.mode_set(mode="OBJECT")
    for bone in rig.pose.bones:
        bone.rotation_mode = "XYZ"
    return rig


def main(path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.fps = actions.FPS
    rig = build_rig(scene)
    collection = bpy.data.collections.new("parts")
    scene.collection.children.link(collection)
    for builder in PARTS:
        obj = builder(collection)
        obj.parent = rig
        obj.modifiers.new("rig", "ARMATURE").object = rig
    actions.build(rig)
    scene.frame_start, scene.frame_end = 0, 48
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(path))


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1])
