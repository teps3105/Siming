# 《司業命緣》Siming: Karma & Fate

《司業命緣》是一款因果輪迴沙盒遊戲：玩家扮演司命星君，俯瞰一個自行運轉的修仙世界，只能以牽線、點化、移山等權能介入，再看每一個因在某一世結果。專案以 Godot 4.7 呈現、Rust 撰寫模擬核心，目標是 Steam Demo；目前模擬核心可無頭執行，Godot 端只有一個顯示地形預覽的入口場景。

## 內容

| 位置 | 內容 |
|---|---|
| `docs/` | 遊戲設計文件，從 [索引](docs/索引.md) 讀起 |
| `game/` | Godot 專案 |
| `native/sim/rust/` | Rust workspace：模擬核心 `siming-sim`、無頭工具 `siming-cli`、Godot 橋接 `siming-gdext` |
| `tools/` | 建置與檢查工具 |

## 開始

需要 Godot 4.7.1、Rust stable（1.94 以上）與 Python 3.10 以上。

```
python tools/native_build/build.py
```

建置完成後以 Godot 開啟 `game/` 並執行，畫面會出現一張由種子生成的世界地形與一行狀態。

只跑模擬、不開 Godot：

```
cd native/sim/rust
cargo run -p siming-cli --release -- sim --seed 1 --yun 1
```

架構見 [6.1 架構總覽](docs/6-開發/6.1-架構總覽.md)，建置與匯出見 [6.6 平台與建置](docs/6-開發/6.6-平台與建置.md)，測試見 [6.7 測試與工具](docs/6-開發/6.7-測試與工具.md)。
