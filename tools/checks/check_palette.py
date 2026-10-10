"""量測 GDD 5.1 色票在一般視覺與三種色覺缺陷模擬下的色差。

量法：sRGB → 線性 RGB → Machado 2009 模擬矩陣（嚴重度 1.0）→ CIELAB（D65）→ CIE76 ΔE。
矩陣來源：https://www.inf.ufrgs.br/~oliveira/pubs_files/CVD_Simulation/CVD_Simulation.html 表 1。

用法：python tools/checks/check_palette.py
色值必須出現在 docs/5-表現/5.1-美術風格.md，否則視為腳本與文件不同步而失敗。
"""

from __future__ import annotations

import itertools
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
GDD = ROOT / "docs" / "5-表現" / "5.1-美術風格.md"

VISIONS = {
    "一般": ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)),
    "紅色盲": (
        (0.152286, 1.052583, -0.204868),
        (0.114503, 0.786281, 0.099216),
        (-0.003882, -0.048116, 1.051998),
    ),
    "綠色盲": (
        (0.367322, 0.860646, -0.227968),
        (0.280085, 0.672501, 0.047413),
        (-0.011820, 0.042940, 0.968881),
    ),
    "藍黃色盲": (
        (1.255528, -0.076749, -0.178779),
        (-0.078411, 0.930809, 0.147602),
        (0.004733, 0.691367, 0.303900),
    ),
}

VEGETATION = {
    "沙漠": "D9C59A", "乾草原": "D0CA9C", "草地": "BAC690", "森林": "7AA06C", "雨林": "618D60",
    "寒林": "8DA68A", "凍原": "C3C2AD", "雪": "F1F0EA", "沼澤": "8FA287",
}
ROCK = {"丘陵土": "C9B58C", "山地岩": "B09C80", "高山灰": "A49D97", "山雪": "F2F1EC"}
SEA = {"海一": "C9DDD8", "海二": "A3C3C4", "海三": "7BA3B0", "海四": "567F97", "海五": "3A5F7A"}
COAST = {"海岸線": "3B5566"}
LINES = {"朱": "B23A2E", "金": "C9A246", "粉": "D48A9A", "青": "4F8F9A", "紫": "5B3A6B"}
PAPER = {"紙": "EDE4CF"}
INK = {"墨": "2B2620"}


def linear(hex_color: str) -> tuple[float, float, float]:
    out = []
    for i in (0, 2, 4):
        v = int(hex_color[i:i + 2], 16) / 255.0
        out.append(v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4)
    return tuple(out)


def lab(rgb: tuple[float, float, float]) -> tuple[float, float, float]:
    r, g, b = (min(max(c, 0.0), 1.0) for c in rgb)
    x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047
    y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b
    z = (0.0193339 * r + 0.1191920 * g + 0.9503041 * b) / 1.08883

    def f(t: float) -> float:
        return t ** (1.0 / 3.0) if t > 216.0 / 24389.0 else (24389.0 / 27.0 * t + 16.0) / 116.0

    fx, fy, fz = f(x), f(y), f(z)
    return 116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)


def seen(hex_color: str, matrix) -> tuple[float, float, float]:
    rgb = linear(hex_color)
    return lab(tuple(sum(m * c for m, c in zip(row, rgb)) for row in matrix))


def delta(a: str, b: str, matrix) -> float:
    la, lb = seen(a, matrix), seen(b, matrix)
    return sum((p - q) ** 2 for p, q in zip(la, lb)) ** 0.5


def lightness_gap(a: str, b: str, matrix) -> float:
    return abs(seen(a, matrix)[0] - seen(b, matrix)[0])


def worst(pairs, matrix, measure=delta):
    """回傳 (最小值, 名稱甲, 名稱乙)。"""
    return min((measure(ca, cb, matrix), na, nb) for (na, ca), (nb, cb) in pairs)


def within(group: dict[str, str]):
    return list(itertools.combinations(group.items(), 2))


def between(a: dict[str, str], b: dict[str, str]):
    return list(itertools.product(a.items(), b.items()))


def main() -> int:
    text = GDD.read_text(encoding="utf-8").upper()
    missing = [
        f"{name} #{value}"
        for group in (VEGETATION, ROCK, SEA, COAST, LINES, PAPER, INK)
        for name, value in group.items()
        if f"#{value}" not in text
    ]
    if missing:
        print("色值不在 GDD 5.1 內，腳本與文件不同步：" + "、".join(missing))
        return 1

    land = {**VEGETATION, **ROCK}
    # 海岸線只貼著淺海：取海深前兩階。
    shallow = dict(list(SEA.items())[:2])
    rows = [
        ("植被底色兩兩", within(VEGETATION), delta),
        ("海深相鄰兩階", list(zip(list(SEA.items())[:-1], list(SEA.items())[1:])), delta),
        ("陸色對海色", between(land, SEA), delta),
        ("海岸線對陸色與淺海（明度差）", between(COAST, {**land, **shallow}), lightness_gap),
        ("因果線色兩兩", within(LINES), delta),
        ("因果線色對紙", between(LINES, PAPER), delta),
        ("墨對紙（明度差）", between(INK, PAPER), lightness_gap),
    ]
    for title, pairs, measure in rows:
        print(title)
        for vision, matrix in VISIONS.items():
            value, a, b = worst(pairs, matrix, measure)
            print(f"  {vision}：最小 {value:5.1f}（{a}／{b}）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
