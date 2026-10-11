#!/usr/bin/env python3
"""存下介面品質檢查表（GDD 5.7）要對照的截圖。

用法（在 repo 任何位置）：
    python tools/tests/shots.py                         # 存到 .shiftblame/tmp/shots/
    python tools/tests/shots.py --out 資料夾 --position 2600,40

開一個 Godot 視窗執行 game/tests/harness/shots.gd，在 1920×1080 鍵鼠、1920×1080 搖桿、
1280×800 搖桿三種畫面下，把標題、選單各頁與遊玩畫面各存一張 PNG。
--position 是視窗左上角的實體像素座標，預設放在主螢幕右邊的第二個螢幕。
Godot 的位置取自環境變數 GODOT_PATH，或以 --godot 指定執行檔。
"""
import argparse
import subprocess
import sys
from pathlib import Path

from run import GAME, REPO, find_godot

RUNNER = "res://tests/harness/shots.gd"


def main(argv=None):
    ap = argparse.ArgumentParser(description="存下介面品質檢查表要對照的截圖")
    ap.add_argument("--godot", type=Path, help="Godot 執行檔；預設取自環境變數 GODOT_PATH")
    ap.add_argument("--out", type=Path, default=REPO / ".shiftblame" / "tmp" / "shots", help="截圖存放的資料夾")
    ap.add_argument("--position", default="2600,40", help="視窗左上角的實體像素座標")
    args = ap.parse_args(argv)

    godot = args.godot or find_godot()
    if godot is None or not Path(godot).is_file():
        print("找不到 Godot 執行檔：請設定 GODOT_PATH 或以 --godot 指定", file=sys.stderr)
        return 2

    args.out.mkdir(parents=True, exist_ok=True)
    cmd = [str(godot), "--audio-driver", "Dummy", "--path", str(GAME), "--position", args.position,
           "--resolution", "1920x1080", "--script", RUNNER, "--", f"--out={args.out.resolve().as_posix()}"]
    return subprocess.run(cmd).returncode


if __name__ == "__main__":
    sys.exit(main())
