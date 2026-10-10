#!/usr/bin/env python3
"""追溯檢查（GDD 7.10 SDD 流程、7.3 T13）。

檢查三件事：
  (a) crates/sim/src/params.rs 與 docs/8-附錄/8.1-參數總表.md 的對應
      - 每個 `pub const` 的文件註解須引用 8.1 的代號（以反引號標示，例如 `WG_SIZE_S`），
        且該代號確實出現在 8.1。
      - 8.1 中屬於 M1 範圍的代號（「世界生成」「州郡與人口」「靈氣與氣場」三節，見
        M1_SECTIONS）若沒有任何常數引用，視為問題。其餘章節的代號尚未抄寫屬預期，只列數量。
  (b) docs/ 內所有相對 Markdown 連結都能解析到既有檔案（忽略外部網址與純錨點）。
  (c) docs/ 內每個 Markdown 檔（索引.md 除外）都被 docs/索引.md 連結到。

用法：
    python3 tools/check_trace.py [--root <repo 根目錄>] [--list-info]

有問題時以 1 結束，否則以 0 結束。--list-info 會同時列出 M1 範圍外尚未抄寫的 8.1 代號。
"""

import argparse
import re
import sys
from pathlib import Path
from urllib.parse import unquote

PARAMS_REL = "crates/sim/src/params.rs"
PARAM_DOC_REL = "docs/8-附錄/8.1-參數總表.md"
INDEX_REL = "docs/索引.md"

# 8.1 中 M1 範圍的章節標題關鍵字（對應 7.3 M1 範圍：2.2、2.3、2.4）。
M1_SECTIONS = ("世界生成", "州郡與人口", "靈氣與氣場")

CODE_IN_TICKS = re.compile(r"`([A-Z][A-Z0-9_]+)`")
CODE_TOKEN = re.compile(r"`([A-Z0-9_]+)`")
CONST_RE = re.compile(r"^\s*pub const ([A-Z0-9_]+)\s*:")
LINK_RE = re.compile(r"\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")


def line_names(text):
    """回傳一段文字可對應到的 8.1 代號名稱集合。

    8.1 常以簡寫列出同一組參數，例如「`WG_SIZE_S` / `M` / `L`」「`WG_SWAMP_Q` / `H`」
    「`WG_NODE_RATIO` / `MIN_DIST`」，其中後續代號是接在前一個代號的前綴之後。
    這裡把第一個代號當作基底，將其後的每個代號同時以「原文」與「基底前綴＋原文」兩種形式收錄。
    """
    raw = CODE_TOKEN.findall(text)
    names = {t for t in raw if len(t) >= 2}
    if raw:
        segs = raw[0].split("_")
        for t in raw[1:]:
            # 簡寫的前綴長度不固定（例如 WG_PROVINCE_MIN_RATIO / MIN_CELLS），
            # 因此各個前綴長度都收錄；較寬鬆，但只用於判斷代號是否存在。
            for k in range(1, len(segs)):
                names.add("_".join(segs[:k]) + "_" + t)
    return names


def parse_param_doc(path):
    """回傳 (全部代號 → 章節集合, 表格首欄定義的列 [(章節, 名稱集合, 首代號)])。"""
    codes = {}
    rows = []
    section = ""
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("## "):
            section = line[3:].strip()
            continue
        for name in line_names(line):
            codes.setdefault(name, set()).add(section)
        if line.startswith("|"):
            cells = line.split("|")
            if len(cells) > 2:
                first = CODE_TOKEN.findall(cells[1])
                if first:
                    rows.append((section, line_names(cells[1]), first[0]))
    return codes, rows


def parse_params(path):
    """回傳 [(常數名, 行號, 文件註解全文)]。"""
    lines = path.read_text(encoding="utf-8").splitlines()
    consts = []
    for i, line in enumerate(lines):
        m = CONST_RE.match(line)
        if not m:
            continue
        doc = []
        j = i - 1
        while j >= 0 and lines[j].lstrip().startswith("///"):
            doc.insert(0, lines[j].lstrip()[3:])
            j -= 1
        consts.append((m.group(1), i + 1, " ".join(doc)))
    return consts


def check_params(root, info):
    """(a) 回傳 (問題清單, 額外資訊 dict)；info 收集摘要文字。"""
    problems = []
    param_path = root / PARAMS_REL
    doc_path = root / PARAM_DOC_REL
    if not param_path.is_file() or not doc_path.is_file():
        return [f"缺少檔案：{PARAMS_REL} 或 {PARAM_DOC_REL}"], {}

    doc_codes, rows = parse_param_doc(doc_path)
    consts = parse_params(param_path)
    const_names = {name for name, _line, _doc in consts}

    # 1. 常數的文件註解須引用 8.1 內的代號。
    referenced = set(const_names)
    for name, line, doc in consts:
        ticked = set(CODE_IN_TICKS.findall(doc))
        referenced |= ticked
        if not doc:
            problems.append(f"{PARAMS_REL}:{line}: `{name}` 沒有文件註解")
        elif not ticked:
            problems.append(f"{PARAMS_REL}:{line}: `{name}` 文件註解未引用 8.1 代號（反引號）")
        elif not (ticked & set(doc_codes)):
            problems.append(
                f"{PARAMS_REL}:{line}: `{name}` 引用的代號 "
                + ", ".join(f"`{c}`" for c in sorted(ticked)) + " 不在 8.1")

    # 2. M1 範圍（表格首欄定義的代號列）中，每一列至少要有一個代號被 params.rs 採用。
    m1_rows = [(names, first) for sec, names, first in rows
               if any(k in sec for k in M1_SECTIONS)]
    missing_m1 = [first for names, first in m1_rows if not (names & referenced)]
    for first in missing_m1:
        problems.append(f"8.1 M1 代號 `{first}`（列）未在 {PARAMS_REL} 中出現")

    m1_firsts = {first for _n, first in m1_rows}
    all_firsts = {first for _sec, _n, first in rows}
    missing_other = sorted(f for f in all_firsts - m1_firsts
                           if not (dict_names_for(rows, f) & referenced))
    info.append(
        f"  常數 {len(consts)} 個；8.1 代號（含簡寫展開）{len(doc_codes)} 個，表列 {len(all_firsts)} 列；"
        f"M1 範圍列 {len(m1_rows)} 個，未抄寫 {len(missing_m1)} 個；"
        f"M1 範圍外尚未抄寫 {len(missing_other)} 個（預期）")
    return problems, {"missing_other": missing_other}


def dict_names_for(rows, first):
    names = set()
    for _sec, n, f in rows:
        if f == first:
            names |= n
    return names


def iter_links(md_path):
    """逐行產出 (行號, 連結目標)，略過程式碼區塊。"""
    in_fence = False
    for ln, line in enumerate(md_path.read_text(encoding="utf-8").splitlines(), start=1):
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        for m in LINK_RE.finditer(line):
            yield ln, m.group(1)


def resolve(md_path, target):
    """回傳相對於 repo 的連結目標檔案路徑（Path），外部或純錨點則回傳 None。"""
    if re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*:", target) or target.startswith("//"):
        return None
    target = target.split("#", 1)[0]
    if not target:
        return None
    return (md_path.parent / unquote(target)).resolve()


def docs_md_files(root):
    return sorted(p for p in (root / "docs").rglob("*.md"))


def check_links(root):
    problems, count = [], 0
    for md in docs_md_files(root):
        for ln, target in iter_links(md):
            dest = resolve(md, target)
            if dest is None:
                continue
            count += 1
            if not dest.is_file():
                rel = md.relative_to(root).as_posix()
                problems.append(f"{rel}:{ln}: 連結無法解析：{target}")
    return problems, count


def check_index(root):
    index = root / INDEX_REL
    if not index.is_file():
        return [f"缺少 {INDEX_REL}"], 0, 0
    linked = set()
    for _ln, target in iter_links(index):
        dest = resolve(index, target)
        if dest is not None:
            linked.add(dest)
    problems = []
    all_docs = [p for p in docs_md_files(root) if p.resolve() != index.resolve()]
    for p in all_docs:
        if p.resolve() not in linked:
            problems.append(f"{p.relative_to(root).as_posix()} 未被 {INDEX_REL} 連結")
    return problems, len(all_docs), len(linked)


def main(argv=None):
    ap = argparse.ArgumentParser(description="追溯檢查：params.rs↔8.1、文件連結、索引")
    ap.add_argument("--root", default=None, help="repo 根目錄（預設為本腳本的上一層）")
    ap.add_argument("--list-info", action="store_true",
                    help="列出 M1 範圍外尚未抄寫的 8.1 代號（資訊，不影響結果）")
    args = ap.parse_args(argv)
    root = Path(args.root).resolve() if args.root else Path(__file__).resolve().parent.parent

    total_problems = []

    print("[a] params.rs ↔ 8.1 參數總表")
    info = []
    pa, extra = check_params(root, info)
    for line in info:
        print(line)
    if args.list_info and extra.get("missing_other"):
        print("  M1 範圍外尚未抄寫的代號：" + ", ".join(extra["missing_other"]))
    for p in pa:
        print("  問題：" + p)
    total_problems += pa

    print("[b] 文件相對連結")
    pb, n_links = check_links(root)
    print(f"  檢查 {n_links} 個相對連結；無法解析 {len(pb)} 個")
    for p in pb:
        print("  問題：" + p)
    total_problems += pb

    print("[c] docs 檔案是否皆由 索引.md 連結")
    pc, n_docs, n_linked = check_index(root)
    print(f"  docs 檔案（不含索引）{n_docs} 個；索引連結目標 {n_linked} 個；未連結 {len(pc)} 個")
    for p in pc:
        print("  問題：" + p)
    total_problems += pc

    print(f"摘要：問題 {len(total_problems)} 項")
    if total_problems:
        print("FAIL")
        return 1
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
