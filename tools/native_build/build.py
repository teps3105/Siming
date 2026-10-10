#!/usr/bin/env python3
"""建置 Rust 橋接擴充並放進 Godot 專案（GDD 6.6）。

用法：
    python tools/native_build/build.py            # debug
    python tools/native_build/build.py --release  # release

輸入：native/sim/rust（Cargo workspace，套件 siming-gdext）。
輸出：game/modules/sim/generated/<前綴>siming_gdext.<debug|release>.<副檔名>，
      由 game/modules/sim/resources/siming.gdextension 載入。輸出不提交，不手改。
"""
import argparse
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
WORKSPACE = REPO / "native" / "sim" / "rust"
OUT_DIR = REPO / "game" / "modules" / "sim" / "generated"
CRATE = "siming_gdext"


def library_names():
    if sys.platform == "win32":
        return f"{CRATE}.dll", "{crate}.{profile}.dll"
    if sys.platform == "darwin":
        return f"lib{CRATE}.dylib", "lib{crate}.{profile}.dylib"
    return f"lib{CRATE}.so", "lib{crate}.{profile}.so"


def main(argv=None):
    ap = argparse.ArgumentParser(description="建置 siming-gdext 並複製到 Godot 專案")
    ap.add_argument("--release", action="store_true", help="以 release 設定建置")
    args = ap.parse_args(argv)

    profile = "release" if args.release else "debug"
    cmd = ["cargo", "build", "-p", "siming-gdext"]
    if args.release:
        cmd.append("--release")
    subprocess.run(cmd, cwd=WORKSPACE, check=True)

    built_name, out_pattern = library_names()
    built = WORKSPACE / "target" / profile / built_name
    if not built.is_file():
        print(f"找不到建置輸出：{built}", file=sys.stderr)
        return 1
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    dest = OUT_DIR / out_pattern.format(crate=CRATE, profile=profile)
    shutil.copy2(built, dest)
    print(f"{built} -> {dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
