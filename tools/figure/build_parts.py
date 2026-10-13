"""修士部件：把 sources/figure/*.svg 的貝茲路徑轉成遊戲讀的曲線資料（game/modules/ui_kit/resources/figure/*.json）。

SVG 是部件輪廓的來源，用 VectorCraft（或任何向量工具）拉控制點；遊戲只讀轉出來的 JSON。

SVG 的寫法：
- 參照方框 <g id="ref-frame" data-reference="1"> 裡放一條正方形路徑：方框中心是頭心，半邊長是頭半徑。
  所有座標都以它換算（頭半徑 = 1），向量工具存檔時平移或縮放整張圖不影響結果。x 負的是面朝的那一邊，y 往下。
- 最外層每個 <g id="部件名"> 是一個部件；裡面每個 <g data-layer="層次" data-part="部位"> 是一層
  （data-open="1" 表示不自動勾輪廓）。
- 一層裡的 <path>：data-role="fill" 封閉的一塊（沒寫就是它）；data-role="line" 墨線；data-role="band" 色帶
  （另寫 data-part 與 data-width，寬度以頭半徑為 1）。只用路徑，不用其他圖形。角色寫在 data-role，
  不寫在 class：VectorCraft 存檔會丟掉 class，data-* 會留著。
- 會隨長度伸縮的部件（鬍鬚、垂帶）在部件的 <g> 上寫 data-hang-y="Y" 與 data-hang="H"：
  Y 與 H 都以頭半徑為 1；y 超過 Y 的部分算「垂下來的」，H = 垂長 1（例如鬚長）；沒寫就全部是固定的。
- id 以 ref 開頭、或帶 data-reference 的群組是對位用的參照（頭的輪廓等），不轉出。

用法：python tools/figure/build_parts.py [--check]
--check 只比對，來源與已轉出的 JSON 不一致時回 1。
"""
import io
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCES = ROOT / "sources" / "figure"
OUT = ROOT / "game" / "modules" / "ui_kit" / "resources" / "figure"
SVG = "{http://www.w3.org/2000/svg}"
TOKEN = re.compile(r"[MmLlHhVvCcSsQqZz]|-?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?")
COUNTS = {"M": 2, "L": 2, "H": 1, "V": 1, "C": 6, "S": 4, "Q": 4}


def cubics(d):
    """路徑資料 → [(是否封閉, [節點…])]；節點是 [進來的控制點, 位置, 出去的控制點]，都是絕對座標。"""
    tokens = TOKEN.findall(d)
    subpaths = []
    nodes = None
    here = (0.0, 0.0)
    start = (0.0, 0.0)
    last_control = None
    command = None
    i = 0

    def close(closed):
        nonlocal nodes
        if nodes:
            if closed and len(nodes) > 1 and abs(nodes[0][1][0] - nodes[-1][1][0]) < 1e-6 and abs(nodes[0][1][1] - nodes[-1][1][1]) < 1e-6:
                nodes[0][0] = nodes[-1][0]
                nodes.pop()
            subpaths.append((closed, nodes))
        nodes = None

    while i < len(tokens):
        if tokens[i].isalpha():
            command = tokens[i]
            i += 1
            if command in "Zz":
                close(True)
                here = start
                last_control = None
                continue
        upper = command.upper()
        relative = command.islower()
        values = [float(v) for v in tokens[i:i + COUNTS[upper]]]
        i += COUNTS[upper]
        base = here if relative else (0.0, 0.0)
        if upper == "M":
            close(False)
            here = (base[0] + values[0], base[1] + values[1])
            start = here
            nodes = [[list(here), list(here), list(here)]]
            command = "l" if relative else "L"
            last_control = None
            continue
        if nodes is None:
            nodes = [[list(here), list(here), list(here)]]
        if upper in "LHV":
            if upper == "L":
                to = (base[0] + values[0], base[1] + values[1])
            elif upper == "H":
                to = ((here[0] if relative else 0.0) + values[0], here[1])
            else:
                to = (here[0], (here[1] if relative else 0.0) + values[0])
            first, second = here, to
            last_control = None
        elif upper == "C":
            first = (base[0] + values[0], base[1] + values[1])
            second = (base[0] + values[2], base[1] + values[3])
            to = (base[0] + values[4], base[1] + values[5])
            last_control = second
        elif upper == "S":
            first = (2 * here[0] - last_control[0], 2 * here[1] - last_control[1]) if last_control else here
            second = (base[0] + values[0], base[1] + values[1])
            to = (base[0] + values[2], base[1] + values[3])
            last_control = second
        else:  # Q：二次轉三次
            control = (base[0] + values[0], base[1] + values[1])
            to = (base[0] + values[2], base[1] + values[3])
            first = (here[0] + (control[0] - here[0]) * 2 / 3, here[1] + (control[1] - here[1]) * 2 / 3)
            second = (to[0] + (control[0] - to[0]) * 2 / 3, to[1] + (control[1] - to[1]) * 2 / 3)
            last_control = None
        nodes[-1][2] = list(first)
        nodes.append([list(second), list(to), list(to)])
        here = to
    close(False)
    return subpaths


def frame_of(root):
    """參照方框 → (頭心 x, 頭心 y, 頭半徑)。"""
    group = next((g for g in root.findall(SVG + "g") if g.get("id") == "ref-frame"), None)
    if group is None:
        raise SystemExit("來源缺少參照方框 <g id=\"ref-frame\">")
    points = [node[1] for path in group.iter(SVG + "path") for _, nodes in cubics(path.get("d", "")) for node in nodes]
    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    return (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2, (max(xs) - min(xs)) / 2


def convert(nodes, frame, hang_y, hang):
    """絕對座標的節點 → 資料檔的節點：[偏移, [x, y, 垂x, 垂y], 偏移]。"""
    centre_x, centre_y, unit = frame

    def split(point):
        x = (point[0] - centre_x) / unit
        y = (point[1] - centre_y) / unit
        if hang_y is not None and y > hang_y:
            return [x, hang_y, 0.0, (y - hang_y) / hang]
        return [x, y, 0.0, 0.0]

    converted = []
    for into, at, out in nodes:
        place = split(at)
        entry = []
        for control in (into, None, out):
            if control is None:
                entry.append([round(v, 4) + 0.0 for v in place])
            else:
                entry.append([round(a - b, 4) + 0.0 for a, b in zip(split(control), place)])
        converted.append(entry)
    return converted


def build(svg_path):
    tree = ET.parse(svg_path)
    frame = frame_of(tree.getroot())
    parts = {}
    for group in tree.getroot().findall(SVG + "g"):
        name = group.get("id", "")
        if not name or name.startswith("ref") or group.get("data-reference") is not None:
            continue
        hang_y = float(group.get("data-hang-y")) if group.get("data-hang-y") else None
        hang = float(group.get("data-hang", "1"))
        pieces = []
        for layer in group.findall(SVG + "g"):
            piece = {"layer": layer.get("data-layer"), "part": layer.get("data-part")}
            if layer.get("data-open") == "1":
                piece["open"] = True
            fills, lines, bands = [], [], []
            for path in layer.iter(SVG + "path"):
                kind = path.get("data-role") or path.get("class", "fill")
                for closed, nodes in cubics(path.get("d", "")):
                    curve = convert(nodes, frame, hang_y, hang)
                    if kind == "fill":
                        fills.append(curve)
                    elif kind == "line":
                        lines.append(curve)
                    elif kind == "band":
                        bands.append({"part": path.get("data-part", "SHADE"), "width": float(path.get("data-width", "0.04")), "curve": curve})
            for key, value in (("fills", fills), ("lines", lines), ("bands", bands)):
                if value:
                    piece[key] = value
            pieces.append(piece)
        parts[name] = {"pieces": pieces}
    return parts


def dump(parts):
    """一個節點一行，方便看差異。"""
    text = json.dumps(parts, ensure_ascii=False, indent="\t")
    text = re.sub(r"\[\s+(-?[\d.e-]+),\s+(-?[\d.e-]+),\s+(-?[\d.e-]+),\s+(-?[\d.e-]+)\s+\]", r"[\1, \2, \3, \4]", text)
    text = re.sub(r"\[\s+(\[[^\[\]]+\]),\s+(\[[^\[\]]+\]),\s+(\[[^\[\]]+\])\s+\]", r"[\1, \2, \3]", text)
    return text + "\n"


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    check = "--check" in sys.argv
    stale = []
    for svg_path in sorted(SOURCES.glob("*.svg")):
        text = dump(build(svg_path))
        target = OUT / (svg_path.stem + ".json")
        current = target.read_text(encoding="utf-8") if target.exists() else ""
        if current != text:
            stale.append(target.name)
            if not check:
                OUT.mkdir(parents=True, exist_ok=True)
                with io.open(target, "w", encoding="utf-8", newline="\n") as handle:
                    handle.write(text)
    if check and stale:
        print("曲線資料與來源不一致，請重跑 tools/figure/build_parts.py：" + "、".join(stale))
        return 1
    print("部件曲線：" + ("已是最新" if not stale else "已更新 " + "、".join(stale)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
