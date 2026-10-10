#!/usr/bin/env bash
# GDD 6.6 WASM（瀏覽器）建置流程：產生 dist/web/（Siming.js + Siming_bg.wasm + index.html + assets/）。
#
# 需求：
#   - rustup target add wasm32-unknown-unknown
#   - wasm-bindgen-cli，版本須與 Cargo.lock 內 wasm-bindgen 完全一致
#       cargo install wasm-bindgen-cli --version <Cargo.lock 中的版本>
#   - wasm-opt（binaryen，選用；有則再縮小體積，無則略過）
# 預覽：  (cd dist/web && python3 -m http.server 8080)  然後以支援 WebGL2 的瀏覽器開啟。
# 瀏覽器版只是另一種顯示形態，原生版為主；共用磁碟請先 export CARGO_TARGET_DIR。
set -euo pipefail
cd "$(dirname "$0")/.."
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
OUT=dist/web
PROFILE=release-web   # 見根 Cargo.toml：opt-level=s、fat LTO、panic=abort

command -v wasm-bindgen >/dev/null || { echo "缺少 wasm-bindgen-cli，請見本檔開頭說明" >&2; exit 1; }
want=$(awk '/^name = "wasm-bindgen"$/{getline; gsub(/[^0-9.]/,"",$0); print; exit}' Cargo.lock)
have=$(wasm-bindgen --version | awk '{print $2}')
[ "$want" = "$have" ] || { echo "wasm-bindgen 版本不符：Cargo.lock=$want，CLI=$have" >&2; exit 1; }

cargo build -p siming --profile "$PROFILE" --target wasm32-unknown-unknown
rm -rf "$OUT"; mkdir -p "$OUT"
wasm-bindgen --target web --no-typescript --remove-name-section --remove-producers-section \
  --out-dir "$OUT" --out-name Siming \
  "$TARGET_DIR/wasm32-unknown-unknown/$PROFILE/Siming.wasm"
if command -v wasm-opt >/dev/null; then
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals \
    "$OUT/Siming_bg.wasm" -o "$OUT/Siming_bg.opt.wasm" && mv "$OUT/Siming_bg.opt.wasm" "$OUT/Siming_bg.wasm"
else
  echo "wasm-opt 未安裝，略過（體積較大）" >&2
fi
cp crates/app/index.html "$OUT/"
cp -r crates/app/assets "$OUT/"
ls -l "$OUT"
echo "wasm raw:  $(stat -c %s "$OUT/Siming_bg.wasm") bytes"
echo "wasm gzip: $(gzip -9 -c "$OUT/Siming_bg.wasm" | wc -c) bytes"
