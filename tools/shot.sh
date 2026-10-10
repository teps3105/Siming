#!/usr/bin/env bash
# 開發與驗收用截圖（GDD 7.9 #30）：在 Xvfb 中執行程式，等待後擷取整個 X 畫面（含 egui 介面）。
# 用法：tools/shot.sh 輸出.png 等待秒數 [寬x高] -- 指令...
# 例：SIMING_DEV_WORLD=1 tools/shot.sh /tmp/a.png 30 1600x900 -- target/debug/Siming --seed 1
set -euo pipefail
out="$1"; wait_s="$2"; shift 2
size="1600x900"
if [[ "${1:-}" != "--" ]]; then size="$1"; shift; fi
[[ "${1:-}" == "--" ]] && shift
display=":$((90 + RANDOM % 9))"
Xvfb "$display" -screen 0 "${size}x24" >/dev/null 2>&1 &
xvfb_pid=$!
trap 'kill "$xvfb_pid" 2>/dev/null || true' EXIT
sleep 1
DISPLAY="$display" "$@" >/tmp/shot_app.log 2>&1 &
app_pid=$!
sleep "$wait_s"
DISPLAY="$display" import -window root "$out"
kill "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
echo "截圖：$out"
