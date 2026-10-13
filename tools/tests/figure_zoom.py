"""修士放大圖：呼叫 figure_zoom.gd 把指定身分的修士放大畫成 PNG。

用法：python tools/tests/figure_zoom.py --out <檔案> [--given <JSON>] [--vary <欄位>:<值,值>] [--pose N]
       [--height N] [--count N] [--seed N] [--focus head|body] [--position X,Y] [--timeout 秒]

開一個 Godot 視窗畫（無視窗模式取不到畫面），預設放在第二個螢幕；超過 --timeout 秒（預設 120）就結束 Godot，結束碼 124。
"""
import argparse
import sys
from pathlib import Path

from run import find_godot, run_bounded

GAME = Path(__file__).resolve().parents[2] / "game"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    for name in ("given", "vary", "pose", "height", "count", "seed", "focus"):
        parser.add_argument("--" + name)
    parser.add_argument("--position", default="2600,40")
    parser.add_argument("--timeout", type=float, default=120.0)
    args = parser.parse_args()
    command = [str(find_godot()), "--audio-driver", "Dummy", "--path", str(GAME), "--position", args.position, "--resolution", "480x270",
               "-s", "res://tests/harness/figure_zoom.gd", "--"]
    for name in ("out", "given", "vary", "pose", "height", "count", "seed", "focus"):
        value = getattr(args, name)
        if value is not None:
            command.append(f"--{name}={Path(value).resolve().as_posix() if name == 'out' else value}")
    code, result = run_bounded(command, args.timeout, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if result is None:
        return code
    problems = [line for line in (result.stdout + result.stderr).splitlines() if "ERROR" in line or "SCRIPT" in line or "Parse" in line]
    for line in problems[:20]:
        print(line)
    return 1 if problems or result.returncode != 0 else 0


if __name__ == "__main__":
    sys.exit(main())
