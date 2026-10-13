#!/usr/bin/env python3
"""遊戲手冊原稿 → PDF。

用法（在 repo 任何位置）：
    python tools/release/manual.py [--output <pdf>] [--browser <exe>] [--html <網頁路徑>]
                                   [--source <原稿>] [--images <截圖資料夾>]

原稿 docs/9-發行/9.7-遊戲手冊.md 只取第一個二級標題之後的內容；截圖與標示表在 docs/9-發行/manual/。
以 Chrome 或 Edge 無頭模式列印；需要 markdown-it-py（pip install markdown-it-py）。
"""
import argparse
import base64
import html as htmllib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
MANUAL_SOURCE = "docs/9-發行/9.7-遊戲手冊.md"
MANUAL_IMAGES = "docs/9-發行/manual"
MANUAL_MARKS = "marks.json"
MANUAL_MARK_BASE = (1920, 1080)
MANUAL_MARK_HEADROOM = 60  # 紅框離畫面上緣不到這麼多時，編號改放在框的下方
MANUAL_GAME_NAME = "司命"
MANUAL_TITLE = "遊戲手冊"
MANUAL_RENDER_TIMEOUT = 120  # 秒
PROJECT_FILE = "game/project.godot"
DEFAULT_OUTPUT_DIR = "builds/manual"  # build/ 沒被 .gitignore 忽略，builds/ 有
MANUAL_IMAGE_TYPES = {".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".png": "image/png"}
MANUAL_IMAGE_RE = re.compile(r'<p><img src="manual/([^"/]+)" alt="([^"]*)" /></p>')
BROWSER_CANDIDATES = tuple(
    str(Path(base) / tail)
    for base in (os.environ.get("PROGRAMFILES", r"C:\Program Files"),
                 os.environ.get("PROGRAMFILES(X86)", r"C:\Program Files (x86)"),
                 os.environ.get("LOCALAPPDATA", ""))
    if base
    for tail in (r"Google\Chrome\Application\chrome.exe", r"Microsoft\Edge\Application\msedge.exe"))
INK = "#B23A2E"  # 朱色
MANUAL_CSS = """
@page { size: A4; margin: 20mm 18mm; @bottom-center { content: counter(page); font-size: 9pt; color: #777; } }
@page :first { @bottom-center { content: none; } }
body { font-family: "Noto Serif TC", "PMingLiU", serif; font-size: 11pt; line-height: 1.7; color: #000; background: #fff; }
.cover { height: 240mm; display: flex; flex-direction: column; justify-content: center; align-items: center;
         page-break-after: always; }
.cover h1 { font-size: 44pt; margin: 0; letter-spacing: 0.2em; color: @INK@; }
.cover .title { font-size: 20pt; margin: 8mm 0 0; }
.cover .version { font-size: 12pt; color: #666; margin-top: 30mm; }
h2 { font-size: 17pt; color: @INK@; border-bottom: 1.5pt solid @INK@; padding-bottom: 2mm; margin: 10mm 0 5mm; break-after: avoid; }
h3 { font-size: 13pt; color: @INK@; margin: 6mm 0 2mm; break-after: avoid; }
p, li { margin: 0 0 2.5mm; }
ul, ol { padding-left: 6mm; margin: 0 0 4mm; }
table { border-collapse: collapse; width: 100%; margin: 3mm 0 5mm; }
tr { break-inside: avoid; }
thead { display: table-header-group; }
th, td { border: 0.5pt solid #999; padding: 1.5mm 2.5mm; text-align: left; vertical-align: top; }
th { background: #eee; }
code { font-family: Consolas, monospace; font-size: 10pt; background: #f2f2f2; padding: 0 1mm; }
figure { margin: 3mm 0 4mm; break-inside: avoid; }
figure .shot { position: relative; line-height: 0; }
figure img { display: block; width: 100%; border: 0.5pt solid #999; box-sizing: border-box; }
figcaption { font-size: 9.5pt; line-height: 1.4; color: #555; text-align: center; margin-top: 1.5mm; }
.mark { position: absolute; box-sizing: border-box; border: 1.6pt solid @INK@; }
.mark b { position: absolute; left: -0.6mm; top: -5.2mm; width: 4.8mm; height: 4.8mm; border-radius: 50%;
          background: @INK@; color: #fff; font-size: 8.5pt; line-height: 4.8mm; text-align: center;
          -webkit-print-color-adjust: exact; print-color-adjust: exact; }
.mark.low b { top: auto; bottom: -5.2mm; }
""".replace("@INK@", INK)


class ManualError(Exception):
    """手冊做不出來；訊息是給老闆看的中文原因。"""


def read_manual_marks(images: Path) -> dict:
    path = images / MANUAL_MARKS
    if not path.is_file():
        return {}
    try:
        marks = json.loads(path.read_text(encoding="utf-8"))
    except ValueError as error:
        raise ManualError(f"遊戲手冊的標示表 {path} 不是合法的 JSON：{error}") from error
    if not isinstance(marks, dict):
        raise ManualError(f"遊戲手冊的標示表 {path} 最外層要是「檔名 → 標示清單」的物件")
    width, height = MANUAL_MARK_BASE
    for name, boxes in marks.items():
        if not isinstance(boxes, list) or not boxes:
            raise ManualError(f"遊戲手冊的標示表：{name} 要是非空的標示清單")
        for box in boxes:
            numbers = (isinstance(box, list) and len(box) == 4
                       and all(isinstance(v, (int, float)) and not isinstance(v, bool) for v in box))
            if not numbers or box[0] < 0 or box[1] < 0 or box[2] <= 0 or box[3] <= 0 \
                    or box[0] + box[2] > width or box[1] + box[3] > height:
                raise ManualError(f"遊戲手冊的標示表：{name} 的標示 {box} 不是 {width}×{height} 畫面內的 [x, y, 寬, 高]")
    return marks


def manual_figure(images: Path, name: str, caption: str, boxes: list) -> str:
    path = images / name
    kind = MANUAL_IMAGE_TYPES.get(path.suffix.lower())
    if kind is None:
        raise ManualError(f"遊戲手冊的圖 manual/{name} 不是 {'、'.join(sorted(MANUAL_IMAGE_TYPES))} 檔")
    if not path.is_file():
        raise ManualError(f"遊戲手冊原稿引用的圖不存在：{path}")
    data = base64.b64encode(path.read_bytes()).decode("ascii")
    width, height = MANUAL_MARK_BASE
    spans = "".join(
        f'<span class="mark{" low" if y < MANUAL_MARK_HEADROOM else ""}" style="left:{x / width * 100:.2f}%;'
        f'top:{y / height * 100:.2f}%;width:{w / width * 100:.2f}%;height:{h / height * 100:.2f}%"><b>{number}</b></span>'
        for number, (x, y, w, h) in enumerate(boxes, 1))
    # 說明文字已被 markdown-it 跳脫過，直接沿用
    return (f'<figure><div class="shot"><img src="data:{kind};base64,{data}" alt="{caption}">{spans}</div>'
            f"<figcaption>{caption}</figcaption></figure>")


def manual_html(source: str, version_label: str, images: Path | None = None) -> str:
    """原稿 → 一頁網頁：只取第一個二級標題之後的內容，前面加帶版本的封面。"""
    lines = source.splitlines()
    start = next((i for i, line in enumerate(lines) if line.startswith("## ")), None)
    if start is None:
        raise ManualError(f"遊戲手冊原稿 {MANUAL_SOURCE} 沒有二級標題（## ），不知道手冊從哪裡開始")
    try:
        from markdown_it import MarkdownIt
    except ImportError as error:
        raise ManualError("轉遊戲手冊需要 markdown-it-py：先執行 pip install markdown-it-py") from error
    body = MarkdownIt("commonmark").enable("table").render("\n".join(lines[start:]) + "\n")
    marks = read_manual_marks(images) if images is not None else {}
    used = set()

    def figure(match: re.Match) -> str:
        if images is None:
            raise ManualError(f"遊戲手冊原稿引用了圖 manual/{match[1]}，但沒有給截圖資料夾")
        used.add(match[1])
        return manual_figure(images, match[1], match[2], marks.get(match[1], []))

    body = MANUAL_IMAGE_RE.sub(figure, body)
    stray = re.search(r'<img src="(?!data:)([^"]*)"', body)
    if stray:
        raise ManualError(f"遊戲手冊原稿的圖 {stray[1]} 要寫成獨立成段的 ![說明](manual/<檔名>)")
    unused = sorted(set(marks) - used)
    if unused:
        raise ManualError(f"遊戲手冊的標示表提到原稿沒用到的圖：{'、'.join(unused)}")
    label = htmllib.escape(version_label)
    return ('<!doctype html>\n<html lang="zh-Hant">\n<head>\n<meta charset="utf-8">\n'
            f"<title>{MANUAL_GAME_NAME} {MANUAL_TITLE} {label}</title>\n<style>{MANUAL_CSS}</style>\n</head>\n<body>\n"
            f'<section class="cover"><h1>{MANUAL_GAME_NAME}</h1><p class="title">{MANUAL_TITLE}</p>'
            f'<p class="version">{label}</p></section>\n{body}</body>\n</html>\n')


def read_version(root: Path) -> tuple[str, str]:
    """回 (封面文字, 檔名用版本)。project.godot 有 config/version 用它，否則用 git 短雜湊標開發版。"""
    config = root / PROJECT_FILE
    if config.is_file():
        found = re.search(r'^config/version="([^"]+)"', config.read_text(encoding="utf-8"), re.M)
        if found:
            return f"版本 {found[1]}", found[1]
    try:
        short = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=root, capture_output=True,
                               text=True, timeout=30, check=True).stdout.strip()
    except (OSError, subprocess.SubprocessError) as error:
        raise ManualError(f"{PROJECT_FILE} 沒有 config/version，也取不到 git 短雜湊：{error}") from error
    return f"開發版 {short}", f"dev-{short}"


def find_browser(override: str | None) -> str:
    if override:
        if not Path(override).is_file():
            raise ManualError(f"--browser 指的瀏覽器不存在：{override}")
        return str(override)
    for candidate in BROWSER_CANDIDATES:
        if Path(candidate).is_file():
            return candidate
    for name in ("chrome", "msedge", "chromium"):
        found = shutil.which(name)
        if found:
            return found
    raise ManualError("找不到 Chrome 或 Edge，遊戲手冊印不成 PDF；用 --browser 指定瀏覽器執行檔")


def browser_renderer(browser: str, timeout: int = MANUAL_RENDER_TIMEOUT):
    """回 render(html, target)：瀏覽器無頭模式印 PDF，設定檔資料夾用暫存。"""
    def render(html: str, target: Path) -> None:
        with tempfile.TemporaryDirectory(prefix="siming-manual-", ignore_cleanup_errors=True) as scratch:
            page = Path(scratch) / "manual.html"
            page.write_text(html, encoding="utf-8", newline="\n")
            command = [browser, "--headless=new", "--disable-gpu", "--no-pdf-header-footer",
                       f"--user-data-dir={Path(scratch) / 'profile'}", f"--print-to-pdf={target}", page.as_uri()]
            try:
                result = subprocess.run(command, capture_output=True, timeout=timeout)
            except (OSError, subprocess.SubprocessError) as error:
                raise ManualError(f"瀏覽器印遊戲手冊失敗：{error}") from error
            if result.returncode:
                detail = result.stderr.decode("utf-8", errors="replace").strip()[-1000:]
                raise ManualError(f"瀏覽器印遊戲手冊失敗（退出碼 {result.returncode}）：{detail}")
    return render


def render_manual(html: str, target: Path, renderer) -> None:
    """印到 .part，確認開頭是 %PDF- 才換名；失敗不留檔。"""
    partial = target.with_name(target.name + ".part")
    try:
        partial.unlink(missing_ok=True)
        renderer(html, partial)
        if not partial.is_file() or partial.read_bytes()[:5] != b"%PDF-":
            raise ManualError(f"遊戲手冊印出來的不是 PDF：{target.name}")
        os.replace(partial, target)
    finally:
        partial.unlink(missing_ok=True)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="遊戲手冊原稿轉 PDF")
    parser.add_argument("--output", type=Path, help="PDF 輸出路徑；預設 builds/manual/siming-<版本>-manual.pdf")
    parser.add_argument("--browser", help="Chrome 或 Edge 執行檔")
    parser.add_argument("--html", type=Path, help="只輸出網頁到這個路徑，不印 PDF")
    parser.add_argument("--source", type=Path, help=f"覆寫原稿（預設 {MANUAL_SOURCE}）")
    parser.add_argument("--images", type=Path, help=f"覆寫截圖資料夾（預設 {MANUAL_IMAGES}）")
    parser.add_argument("--root", type=Path, default=REPO, help="repo 根（預設為本檔所在 repo）")
    args = parser.parse_args(argv)
    root = args.root
    try:
        source = args.source or root / MANUAL_SOURCE
        if not source.is_file():
            raise ManualError(f"找不到遊戲手冊原稿 {source}")
        label, tag = read_version(root)
        page = manual_html(source.read_text(encoding="utf-8"), label, args.images or root / MANUAL_IMAGES)
        if args.html:
            args.html.parent.mkdir(parents=True, exist_ok=True)
            args.html.write_text(page, encoding="utf-8", newline="\n")
            print(f"已輸出網頁：{args.html}")
            return 0
        target = args.output or root / DEFAULT_OUTPUT_DIR / f"siming-{tag}-manual.pdf"
        renderer = browser_renderer(find_browser(args.browser))
        target.parent.mkdir(parents=True, exist_ok=True)
        render_manual(page, target, renderer)
    except ManualError as error:
        print(f"錯誤：{error}", file=sys.stderr)
        return 1
    print(f"已輸出手冊：{target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
