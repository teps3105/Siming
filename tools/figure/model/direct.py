"""請影片模型（本機 ComfyUI）當動作導演：由立姿的算圖演出一段動作，逐格存下並排成對照表（8.11「管線」第 2 步）。

    python tools/figure/model/direct.py <動作> [--seed N] [--first]

- 第一格：Blender 5.2 以 build.py 的模型算立姿（30° 俯角、三分面朝左前、純色底），存 .shiftblame/tmp/figure-direct/first.png；
  已有則沿用，--first 強制重算。也可手動換成別的第一格（同檔名）。
- 影片：送 ComfyUI 的 MiniMax H3 工作流，第一格與最後一格都是立姿圖（動作做完回到立姿）。
- 逐格圖與影片放 .shiftblame/tmp/figure-direct/<動作>/（不入庫）；對照表、direct.json、工作流 JSON 放 sources/figure/direct/<動作>/。
每個種子輸出 sheet_s<種子>.jpg 與 direct_s<種子>.json；挑中的那一份複製成 sheet.jpg、direct.json。動作描述在本檔 PROMPTS。影片只是參照，不進成品。
"""
import glob
import json
import math
import os
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
TMP = os.path.join(ROOT, ".shiftblame", "tmp", "figure-direct")
SRC = os.path.join(ROOT, "sources", "figure", "direct")
BLENDER = "C:/Program Files/Blender Foundation/Blender 5.2/blender.exe"
WORKFLOW_BASE = "h3-motion"
COMFY_OUTPUT = os.environ.get("COMFY_OUTPUT", os.path.expanduser("~/AppData/Local/Comfy-Desktop/ComfyUI-Shared/output")).replace("\\", "/").rstrip("/") + "/"

SIDE = 640      # 影片的邊長（hikari 在 16 GB 卡上跑得動的設定）
LENGTH = 56     # 影片格數
FPS = 24        # 影片模型的輸出每秒格數（以 direct.json 記錄，依官方文件）
EVERY = 2       # 對照表每幾格取一格
DEFAULT_SEED = 20261013
WAIT = 1500

LEAD = ("A full-body ancient Chinese cultivator in a long robe with a topknot, seen from a fixed high-angle camera "
        "(about 30 degrees looking down), three-quarter view facing front-left, the camera does not move, zoom or pan, "
        "the viewing angle stays the same, plain flat solid background, a single character only, no other people or objects. ")
END = " At the end of the clip the character returns to the same relaxed standing pose as the first frame."
PROMPTS = {
    "walk": "The character walks forward in place with a natural steady gait, two full steps, arms swinging gently, robe hem swaying.",
    "brace": "The character tilts his head far back and looks up at the sky, one arm raised high toward the heavens as if bracing against a coming storm, the other hand hanging at his side, knees slightly bent, feet planted wide.",
    "struck": "The character, one arm raised to the sky, is suddenly struck by lightning: his body snaps violently backward and his arms fling open, knees buckle, then he recoils forward, steadies himself and stands up straight again.",
    "fall": "The character loses strength and collapses: knees buckle, the body falls backward and lies flat on the ground and stays still.",
    "rise": "The character's feet clearly leave the ground and he rises into the air, floating up well above the floor with the body fully off the ground and legs hanging, arms slightly spread, sleeves and robe hem billowing upward, then settles back down to stand.",
    "strike": "The character draws a sword from his waist with one hand and swings it in one decisive diagonal slash, steps forward with the stroke, holds the finish, then sheathes it and returns to standing.",
    "seal": "The character forms a hand seal by pinching two fingers together and thrusts the hand forward pointing ahead to cast a spell, the other hand held at the chest, holds the pose, then lowers the hands.",
    "bow": "The character brings both hands together before the chest in a respectful salute and bows deeply from the waist, holds the bow briefly, then straightens up.",
}


# ── Blender 內：算第一格 ─────────────────────────────────────

def render_first(blend, out):
    import bpy
    bpy.ops.wm.open_mainfile(filepath=os.path.abspath(blend))
    scene = bpy.context.scene
    rig = bpy.data.objects["rig"]
    rig.animation_data.nla_tracks.foreach_set("mute", [True] * len(rig.animation_data.nla_tracks))
    rig.animation_data.action = bpy.data.actions["stand"]
    scene.frame_set(0)
    colors = {"body": (0.80, 0.62, 0.50, 1), "robe": (0.82, 0.80, 0.72, 1), "hair": (0.08, 0.07, 0.07, 1), "face": (0.05, 0.04, 0.04, 1)}
    for obj in bpy.data.objects:
        if obj.type != "MESH":
            continue
        mat = bpy.data.materials.new(obj.name)
        mat.diffuse_color = colors.get(obj.name.split(".")[0], (0.5, 0.5, 0.5, 1))
        obj.data.materials.clear()
        obj.data.materials.append(mat)
    pivot = bpy.data.objects.new("pivot", None)
    scene.collection.objects.link(pivot)
    pivot.rotation_euler = (0, 0, math.radians(-45))   # 正面（−Y）轉向畫面左前
    for obj in list(scene.objects):
        if obj.parent is None and obj is not pivot:
            obj.parent = pivot
    tilt = math.radians(30)
    camera = bpy.data.objects.new("camera", bpy.data.cameras.new("camera"))
    scene.collection.objects.link(camera)
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 1.2
    centre, away = (0, 0, 0.5), 6.0
    camera.location = (centre[0], centre[1] - away * math.cos(tilt), centre[2] + away * math.sin(tilt))
    camera.rotation_euler = (math.pi / 2 - tilt, 0, 0)
    scene.camera = camera
    scene.render.engine = "BLENDER_WORKBENCH"
    shading = scene.display.shading
    shading.light, shading.color_type = "STUDIO", "MATERIAL"
    scene.render.resolution_x = scene.render.resolution_y = SIDE
    scene.render.film_transparent = False
    scene.world = bpy.data.worlds.new("w")
    scene.world.color = (0.80, 0.82, 0.85)
    scene.view_settings.view_transform = "Standard"
    scene.render.filepath = os.path.abspath(out)
    bpy.ops.render.render(write_still=True)


# ── 一般 Python：跑 ComfyUI ───────────────────────────────────

def comfy(*args):
    done = subprocess.run(["comfy", "--where", "local", *args], capture_output=True, text=True, encoding="utf-8", shell=os.name == "nt")
    lines = [line for line in done.stdout.splitlines() if line.startswith("{")]
    if not lines:
        raise SystemExit(f"{' '.join(args[:2])} 沒有回應：{done.stdout[-300:]}{done.stderr[-300:]}")
    reply = json.loads(lines[-1])
    if not reply["ok"]:
        raise SystemExit(f"{' '.join(args[:2])} 失敗：{reply['error']}")
    return reply["data"]


def first_frame(force):
    path = os.path.join(TMP, "first.png")
    if force or not os.path.exists(path):
        blend = os.path.join(TMP, "rig.blend")
        if not os.path.exists(blend):
            subprocess.run([BLENDER, "-b", "--factory-startup", "-P", os.path.join(HERE, "build.py"), "--", blend], check=True)
        subprocess.run([BLENDER, "-b", "--factory-startup", "-P", os.path.abspath(__file__), "--", "render", blend, path], check=True)
    return path


def sheet(frames, out, fps):
    from PIL import Image, ImageDraw
    picked = list(range(0, len(frames), EVERY))
    columns = 7
    side = 224
    pad = 14
    table = Image.new("RGB", (columns * side, -(-len(picked) // columns) * (side + pad)), "white")
    for spot, at in enumerate(picked):
        small = Image.open(frames[at]).convert("RGB").resize((side, side), Image.LANCZOS)
        x, y = spot % columns * side, spot // columns * (side + pad)
        table.paste(small, (x, y + pad))
        ImageDraw.Draw(table).text((x + 4, y + 1), "#%d  %.2fs" % (at, at / fps), fill=(180, 0, 0))
    table.save(out, quality=88)


def main(action, seed, force_first):
    from PIL import Image
    seed = seed if seed is not None else DEFAULT_SEED
    tag = "figdir_%s_s%d" % (action, seed)
    work, dest = os.path.join(TMP, action), os.path.join(SRC, action)
    os.makedirs(work, exist_ok=True)
    os.makedirs(dest, exist_ok=True)
    first = first_frame(force_first)
    still = os.path.join(work, tag + ".png")
    Image.open(first).convert("RGB").resize((SIDE, SIDE), Image.LANCZOS).save(still)
    prompt = LEAD + PROMPTS[action] + END
    flow = json.load(open(os.path.join(HERE, "..", "..", "..", "sources", "figure", "direct", WORKFLOW_BASE + ".api.json"), encoding="utf-8"))
    flow["5"]["inputs"]["image"] = tag + ".png"
    flow["6"]["inputs"].update(prompt=prompt, width=SIDE, height=SIDE, length=LENGTH)
    flow["13"]["inputs"]["filename_prefix"] = tag
    flow["10"]["inputs"]["noise_seed"] = seed
    api = os.path.join(work, tag + ".api.json")
    json.dump(flow, open(api, "w", encoding="utf-8", newline="\n"), ensure_ascii=False, indent=1)
    started = time.time()
    if not glob.glob(COMFY_OUTPUT + tag + "_*.png"):
        comfy("upload", still)
        comfy("run", "--no-watch", "--workflow", api)
        for _ in range(WAIT // 5):
            if len(glob.glob(COMFY_OUTPUT + tag + "_*.png")) >= LENGTH - 3:
                break
            time.sleep(5)
        else:
            raise SystemExit("等了 %d 秒沒有輸出" % WAIT)
        time.sleep(3)
    seconds = round(time.time() - started)
    frames = sorted(glob.glob(COMFY_OUTPUT + tag + "_*.png"))
    out = os.path.join(work, "s%d" % seed)
    os.makedirs(out, exist_ok=True)
    for at, path in enumerate(frames):
        shutil.copyfile(path, os.path.join(out, "%03d.png" % at))
    table = os.path.join(dest, "sheet_s%d.jpg" % seed)
    sheet(frames, table, FPS)
    json.dump({"action": action, "seed": seed, "prompt": prompt, "model": flow["1"]["inputs"]["unet_name"],
               "lora": flow["2"]["inputs"]["lora_name"], "text_encoder": flow["3"]["inputs"]["clip_name"], "vae": flow["4"]["inputs"]["vae_name"],
               "steps": flow["8"]["inputs"]["steps"], "sampler": flow["9"]["inputs"]["sampler_name"], "scheduler": flow["8"]["inputs"]["scheduler"],
               "width": SIDE, "height": SIDE, "length": len(frames), "fps": FPS, "table_every": EVERY, "generate_seconds": seconds,
               "first_frame": "Blender 5.2 build.py 的立姿，30° 俯角，三分面朝左前"},
              open(os.path.join(dest, "direct_s%d.json" % seed), "w", encoding="utf-8", newline="\n"), ensure_ascii=False, indent=1)
    print(len(frames), "格", seconds, "秒 →", table)


if __name__ == "__main__":
    if "--" in sys.argv and "render" in sys.argv:
        tail = sys.argv[sys.argv.index("--") + 1:]
        render_first(tail[1], tail[2])
    else:
        args = sys.argv[1:]
        seed = int(args[args.index("--seed") + 1]) if "--seed" in args else None
        main(args[0], seed, "--first" in args)
