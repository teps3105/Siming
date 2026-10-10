#!/usr/bin/env python3
"""Subset LXGW WenKai for Siming.

Spec: docs/8-素材/8.2-字型.md (子集化), docs/8-素材/8.10-授權清單.md (OFL 保留名稱),
docs/6-開發/6.6-平台與建置.md (字型子集化).

Usage:
    python3 tools/subset_font.py --font <original.ttf> --out <subset.ttf> --lang tc|sc \
        [--sha256 <hex>]

Character set (union):
  - every non-control character in native/sim/rust/crates/**/*.rs, its sim/data/*.json and
    game/modules/locale/resources/*.json (Rust \\u{XXXX} escapes are decoded);
  - tools/font_subset/data/common_4808.txt;
  - ASCII printable, assigned U+3000–U+303F and U+FF00–U+FFEF, Chinese numerals.
Characters that the font does not contain are reported and skipped.

The output is deterministic for a given input font and source set (head timestamps are
not recalculated), so CI can re-run the script and compare the result byte-for-byte.
"""

import argparse
import hashlib
import json
import re
import sys
import unicodedata
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

REPO = Path(__file__).resolve().parents[2]
COMMON_LIST = REPO / "tools" / "font_subset" / "data" / "common_4808.txt"

FAMILY = {"tc": "CflWenKaiSubset", "sc": "CflWenKaiScSubset"}

# Layout features kept in GSUB/GPOS; everything else (vertical forms, fractions,
# ligatures, ...) is dropped. egui does not apply GSUB/GPOS, so this only keeps
# the subset self-consistent for other shapers.
LAYOUT_FEATURES = ["ccmp", "locl", "calt", "mark", "mkmk"]

ASCII_PRINTABLE = range(0x20, 0x7F)
FULLWIDTH_PUNCT_RANGES = [(0x3000, 0x303F), (0xFF00, 0xFFEF)]
CHINESE_NUMERALS = "〇零一二三四五六七八九十百千萬億兩壹貳參肆伍陸柒捌玖拾佰仟"

UNICODE_ESCAPE = re.compile(r"\\u\{([0-9A-Fa-f]{1,6})\}")


def is_control(ch):
    # Cc (controls), Cf (format), Cs, Co (private use), Cn (unassigned) are not text glyphs.
    return unicodedata.category(ch)[0] == "C"


def scan_text(text, found):
    for ch in text:
        if not is_control(ch):
            found.add(ord(ch))


def rust_sources():
    return sorted(REPO.glob("native/sim/rust/crates/**/*.rs"))


def json_sources():
    paths = sorted((REPO / "native" / "sim" / "rust" / "crates" / "sim" / "data").glob("*.json"))
    text_dir = REPO / "game" / "modules" / "locale" / "resources"
    if text_dir.is_dir():
        paths += sorted(text_dir.glob("*.json"))
    return paths


def collect():
    """Return (wanted code points, per-source counts, common-list code points)."""
    from_sources = set()
    counts = {}

    rs_files = rust_sources()
    for path in rs_files:
        text = path.read_text(encoding="utf-8")
        scan_text(text, from_sources)
        # Rust may spell characters as \u{XXXX} escapes; decode and scan them too.
        for m in UNICODE_ESCAPE.finditer(text):
            scan_text(chr(int(m.group(1), 16)), from_sources)
    counts["native/sim/rust/crates/**/*.rs"] = len(rs_files)

    json_files = json_sources()
    for path in json_files:
        # Decode first so \uXXXX escapes and keys are both scanned.
        obj = json.loads(path.read_text(encoding="utf-8"))
        scan_text(json.dumps(obj, ensure_ascii=False), from_sources)
    counts["sim/data + locale/resources *.json"] = len(json_files)

    common = set()
    for line in COMMON_LIST.read_text(encoding="utf-8").splitlines():
        for ch in line.strip():
            common.add(ord(ch))
    counts["tools/font_subset/data/common_4808.txt"] = len(common)

    fixed = set(ASCII_PRINTABLE)
    for lo, hi in FULLWIDTH_PUNCT_RANGES:
        fixed.update(cp for cp in range(lo, hi + 1) if unicodedata.category(chr(cp)) != "Cn")
    fixed.update(ord(ch) for ch in CHINESE_NUMERALS)
    counts["ASCII + full-width punct + Chinese numerals"] = len(fixed)

    wanted = from_sources | common | fixed
    return wanted, counts, common, from_sources


def sha256_of(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def rename_for_osl(font, family):
    """Give the subset its own family name so it does not use the original name."""
    new_names = {
        1: family,
        3: f"{family}-Regular;subset of LXGW WenKai",
        4: f"{family} Regular",
        6: f"{family}-Regular",
    }
    name = font["name"]
    for rec in list(name.names):
        if rec.nameID in new_names:
            name.setName(new_names[rec.nameID], rec.nameID, rec.platformID, rec.platEncID, rec.langID)


def show_chars(cps):
    return "".join(chr(cp) for cp in sorted(cps))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--font", required=True, type=Path, help="original LXGW WenKai .ttf")
    parser.add_argument("--out", required=True, type=Path, help="output subset .ttf")
    parser.add_argument("--lang", required=True, choices=["tc", "sc"])
    parser.add_argument("--sha256", help="expected SHA-256 of --font (hex)")
    args = parser.parse_args(argv)

    if not args.font.is_file():
        parser.error(f"font not found: {args.font}")

    digest = sha256_of(args.font)
    print(f"input sha256: {digest}  ({args.font.stat().st_size} bytes)")
    if args.sha256 and digest != args.sha256.strip().lower():
        print(f"error: SHA-256 mismatch: expected {args.sha256.lower()}", file=sys.stderr)
        return 1

    wanted, counts, common, from_sources = collect()
    print("character sources (inputs):")
    for label, n_inputs in counts.items():
        print(f"  {label}: {n_inputs}")
    print(f"distinct code points wanted: {len(wanted)}")

    font = TTFont(str(args.font), recalcTimestamp=False)
    cmap = font.getBestCmap()

    missing_common = {cp for cp in common if cp not in cmap}
    if missing_common:
        print(f"warning: {len(missing_common)} common-list characters are not in the font: {show_chars(missing_common)}")

    missing_sources = {cp for cp in from_sources if cp not in cmap}
    if missing_sources:
        print(f"warning: {len(missing_sources)} characters from source files are not in the font and are skipped: {show_chars(missing_sources)}")

    missing_total = {cp for cp in wanted if cp not in cmap}
    print(f"wanted code points not in the font (skipped): {len(missing_total)}")

    keep = sorted(cp for cp in wanted if cp in cmap)

    options = subset.Options()
    options.layout_features = LAYOUT_FEATURES
    options.hinting = False
    options.glyph_names = False
    options.name_IDs = ["*"]
    options.name_languages = ["*"]
    options.drop_tables += ["BASE", "meta", "vhea", "vmtx"]

    subsetter = subset.Subsetter(options=options)
    subsetter.populate(unicodes=keep)
    subsetter.subset(font)

    family = FAMILY[args.lang]
    rename_for_osl(font, family)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    font.save(str(args.out))

    glyphs = len(font.getGlyphOrder())
    size = args.out.stat().st_size
    out_cmap = TTFont(str(args.out)).getBestCmap()
    print(f"family name: {family}")
    print(f"characters in subset: {len(out_cmap)} (requested {len(keep)})")
    print(f"glyph count: {glyphs}")
    print(f"output: {args.out} ({size} bytes, {size / (1024 * 1024):.2f} MiB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
