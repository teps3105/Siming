"""送畫：把一份指示與輸入圖交給本機 ComfyUI 的 qwen 改圖，等輸出、複製到工作目錄、寫紀錄。已有輸出的不重送。

    python tools/parts/paint.py <部件代號> [--try N] [--seed S] [--base m|f] [--steps 25]

部件代號見 portrait.py 的 PARTS（如 hair-bun、eyes-a、brows-a、beard-3、collar-dao-mid）。
輸入：底版（性別由 PARTS 指定或 --base）為第一張，畫風基準圖為第二張。
輸出：.shiftblame/tmp/parts/portrait/paint/<部件>-t<N>.png（畫家原圖，不入庫），
      sources/parts/portrait/<部件>.json 的 "paint" 清單追加一筆（輸入雜湊、指示、模型、參數、種子、輸出雜湊、耗時）。
本機只有一個佇列：一次只送一件、等它畫完才返回。
"""
import argparse
import glob
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402

ROOT = P.ROOT
WORKFLOW = ROOT + "/sources/parts/workflows/qwen-edit.api.json"
OUTPUT = os.environ.get("COMFY_OUTPUT", os.path.expanduser("~/AppData/Local/Comfy-Desktop/ComfyUI-Shared/output")).replace("\\", "/").rstrip("/") + "/"
MODEL = "qwen_image_2.1_int8_convrot"
WAIT = 1800   # 一件最多等這麼多秒


def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def run(*args):
    done = subprocess.run(["comfy", "--where", "local", *args], capture_output=True, text=True, encoding="utf-8", shell=os.name == "nt")
    lines = [line for line in done.stdout.splitlines() if line.startswith("{")]
    if not lines:
        raise SystemExit(f"{' '.join(args[:2])} 沒有回應：{done.stdout[-300:]}{done.stderr[-300:]}")
    reply = json.loads(lines[-1])
    if not reply.get("ok"):
        raise SystemExit(f"{' '.join(args[:2])} 失敗：{reply.get('error')}")
    return reply["data"]


def paint(tag, image1, image2, prompt, seed=20261013, steps=25, record=None, negative=""):
    """image1 是要保住的圖（底版或送畫圖），image2 是畫風基準；回傳畫家原圖路徑。"""
    out_dir = P.TMP + "paint/"
    os.makedirs(out_dir, exist_ok=True)
    result = out_dir + tag + ".png"
    if os.path.exists(result):
        print("已有輸出，不重送：", result)
        return result
    names = []
    for key, path in (("1", image1), ("2", image2)):
        name = f"parts-{tag}-{key}.png"
        shutil.copy(path, out_dir + name)
        run("upload", out_dir + name)
        names.append(name)
    flow = json.load(open(WORKFLOW, encoding="utf-8"))
    flow["5"]["inputs"]["image"] = names[0]
    flow["11"] = {"class_type": "LoadImage", "inputs": {"image": names[1]}}
    flow["6"]["inputs"]["images.image_2"] = ["11", 0]
    flow["6"]["inputs"]["prompt"] = prompt
    flow["6"]["inputs"]["negative_prompt"] = negative
    flow["7"]["inputs"]["seed"] = seed
    flow["7"]["inputs"]["steps"] = steps
    prefix = "pp_" + tag.replace("-", "_")
    flow["9"]["inputs"]["filename_prefix"] = prefix
    wf = out_dir + tag + ".api.json"
    json.dump(flow, open(wf, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
    started = time.time()
    run("run", "--no-watch", "--workflow", wf)
    while time.time() - started < WAIT:
        found = sorted(glob.glob(OUTPUT + prefix + "_*.png"))
        if found:
            time.sleep(1.5)
            shutil.copy(found[-1], result)
            break
        time.sleep(4)
    else:
        raise SystemExit(f"{tag}：等了 {WAIT} 秒沒有輸出")
    took = round(time.time() - started, 1)
    if record is not None:
        P.log(record, "paint", {"tag": tag, "model": MODEL, "workflow": "sources/parts/workflows/qwen-edit.api.json", "seed": seed, "steps": steps, "cfg": 1.0,
                                "sampler": "euler/simple", "inputs": {"image_1": [image1, sha(image1)], "image_2": [image2, sha(image2)]},
                                "prompt": prompt, "output": result, "output_sha256": sha(result), "seconds": took})
    print(tag, "完成", took, "秒")
    return result


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("part")
    ap.add_argument("--try", dest="n", type=int, default=1)
    ap.add_argument("--seed", type=int, default=None)
    ap.add_argument("--steps", type=int, default=25)
    a = ap.parse_args()
    spec = P.PARTS[a.part]
    prompt_text = P.prompt(a.part)
    seed = a.seed if a.seed is not None else 20261013 + a.n
    paint(f"{a.part}-t{a.n}", P.input_path(a.part), P.STYLE, prompt_text, seed, a.steps, record=a.part)
