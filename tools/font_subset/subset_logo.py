#!/usr/bin/env python3
"""把源流明體 Heavy 子集成標題畫面的標誌字（GDD 8.2「標誌字」）。

用法：
    python tools/font_subset/subset_logo.py --font <GenRyuMin2TW-H.otf>

只收標誌用到的字；子集是 OFL 所說的修改版本，所以改用自己的字族名稱。
來源檔在素材庫 packs/github/genryu-font/GenRyuMin2TW-otf.zip 裡。
"""
import argparse
import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

REPO = Path(__file__).resolve().parents[2]
OUT = REPO / "game" / "modules" / "ui_kit" / "assets" / "fonts" / "genryu_logo_subset.otf"
CHARS = "司業命緣"
FAMILY = "CflGenRyuLogoSubset"


def main(argv=None):
    ap = argparse.ArgumentParser(description="子集化標誌字型")
    ap.add_argument("--font", type=Path, required=True, help="GenRyuMin2TW-H.otf")
    ap.add_argument("--out", type=Path, default=OUT)
    args = ap.parse_args(argv)

    options = subset.Options()
    options.layout_features = []
    options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14]
    options.notdef_outline = True
    options.recalc_timestamp = False
    font = TTFont(args.font)
    missing = [ch for ch in CHARS if ord(ch) not in font.getBestCmap()]
    if missing:
        print(f"字型缺字：{''.join(missing)}", file=sys.stderr)
        return 1
    subsetter = subset.Subsetter(options)
    subsetter.populate(text=CHARS)
    subsetter.subset(font)
    names = {1: FAMILY, 3: f"{FAMILY}-Heavy", 4: f"{FAMILY} Heavy", 6: f"{FAMILY}-Heavy"}
    for record in list(font["name"].names):
        if record.nameID in names:
            font["name"].setName(names[record.nameID], record.nameID, record.platformID, record.platEncID, record.langID)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    font.save(args.out)
    print(f"標誌字型：{args.out}（{args.out.stat().st_size} 位元組）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
