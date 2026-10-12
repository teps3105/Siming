#!/usr/bin/env python3
"""執行 Godot 端的全部測試套件（GDD 6.7）。

用法（在 repo 任何位置）：
    python tools/tests/run.py                      # 全部套件
    python tools/tests/run.py --suite map_journey  # 只跑一個套件
    python tools/tests/run.py --case globe         # 只跑名稱含 globe 的案例
    python tools/tests/run.py --report out.json    # 另存逐案結果

以無視窗模式啟動 Godot 執行 game/tests/harness/run.gd；結束碼 0 表示全過。
超過 --timeout 秒（預設 900）還沒跑完就結束 Godot，結束碼 124。
Godot 的位置取自環境變數 GODOT_PATH（內含 Godot_v*_win64_console.exe），
或以 --godot 指定執行檔。執行前先以 tools/native_build/build.py 建置橋接程式庫。
"""
import argparse
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
GAME = REPO / "game"
RUNNER = "res://tests/harness/run.gd"


def find_godot():
    root = os.environ.get("GODOT_PATH")
    if not root:
        return None
    base = Path(root)
    if base.is_file():
        return base
    patterns = ["Godot_v*_console.exe", "Godot_v*.exe", "godot*"]
    for pattern in patterns:
        found = sorted(base.glob(pattern))
        if found:
            return found[-1]
    return None


def run_bounded(cmd, seconds, **options):
    """執行 cmd，超過 seconds 秒就結束它；回傳 (結束碼, CompletedProcess 或 None)，逾時的結束碼是 124。"""
    try:
        done = subprocess.run(cmd, timeout=seconds, **options)
    except subprocess.TimeoutExpired:
        print(f"逾時：{seconds} 秒內沒有跑完，已結束 Godot", file=sys.stderr)
        return 124, None
    return done.returncode, done


def main(argv=None):
    ap = argparse.ArgumentParser(description="執行 Godot 端的測試套件")
    ap.add_argument("--godot", type=Path, help="Godot 執行檔；預設取自環境變數 GODOT_PATH")
    ap.add_argument("--suite", help="只跑這個套件（suite_name）")
    ap.add_argument("--case", help="只跑名稱含這段文字的案例")
    ap.add_argument("--report", type=Path, help="把逐案結果寫成 JSON")
    ap.add_argument("--timeout", type=float, default=900.0, help="最多跑幾秒")
    args = ap.parse_args(argv)

    godot = args.godot or find_godot()
    if godot is None or not Path(godot).is_file():
        print("找不到 Godot 執行檔：請設定 GODOT_PATH 或以 --godot 指定", file=sys.stderr)
        return 2

    cmd = [str(godot), "--headless", "--audio-driver", "Dummy", "--path", str(GAME), "--script", RUNNER, "--"]
    if args.suite:
        cmd.append(f"--suite={args.suite}")
    if args.case:
        cmd.append(f"--case={args.case}")
    if args.report:
        cmd.append(f"--report={args.report.resolve()}")
    return run_bounded(cmd, args.timeout)[0]


if __name__ == "__main__":
    sys.exit(main())
