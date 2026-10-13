"""細線收件的測試：給一張已知的細線稿，斷言線條留下、膚色底被去掉、碎屑被濾掉而短線不被誤殺。

    python tools/parts/test_lines.py
"""
import os
import sys

import cv2
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import collect as C  # noqa: E402
import portrait as P  # noqa: E402

SKIN = (222, 177, 130)
GREEN = (0, 255, 0)
LINE = (96, 62, 44)   # 畫家的細棕線：比墨色淺，帶一點色相


def make():
    base = np.zeros((300, 300, 3), np.uint8)
    base[:] = GREEN
    cv2.ellipse(base, (150, 150), (110, 130), 0, 0, 360, SKIN, -1)
    cv2.ellipse(base, (150, 150), (110, 130), 0, 0, 360, P.INK, 2)    # 臉輪廓
    cv2.line(base, (140, 200), (160, 200), P.INK, 2)                  # 底版自己的暗線（口）
    painted = base.copy()
    truth = np.zeros((300, 300), bool)
    for y in (80, 92, 104):   # 額紋：三道 2 像素寬的長弧線
        t = np.zeros((300, 300), np.uint8)
        cv2.ellipse(t, (150, y + 40), (60, 40), 0, 200, 340, 255, 2)
        painted[t > 0] = LINE
        truth |= t > 0
    t = np.zeros((300, 300), np.uint8)
    cv2.line(t, (200, 150), (222, 164), 255, 2)   # 一道只有 26 像素的短線，不能被誤殺
    painted[t > 0] = LINE
    short = t > 0
    truth |= short
    speck = np.zeros((300, 300), np.uint8)
    cv2.circle(speck, (70, 210), 2, 255, -1)       # 一粒碎屑
    painted[speck > 0] = LINE
    painted = np.roll(painted, 1, axis=1)          # 對位殘留：整張偏一像素，底版的暗線與輪廓會出現殘邊
    base_w = np.roll(base, 1, axis=1)
    truth = np.roll(truth, 1, axis=1)
    short = np.roll(short, 1, axis=1)
    speck = np.roll(speck > 0, 1, axis=1)
    return base, painted, base_w, truth, short, speck


def main():
    base, painted, _, truth, short, speck = make()
    zone = np.zeros(base.shape[:2], bool)
    zone[40:260, 40:260] = True
    face = np.abs(base.astype(int) - np.array(GREEN)[None, None]).max(2) > 60
    out, kept = C.extract_lines(painted.astype(np.uint8), base, zone, face)
    alpha = out[..., 3].astype(float) / 255
    on = alpha > 0.3
    recall = (on & truth).sum() / truth.sum()
    assert recall > 0.9, f"線條留下的比例 {recall:.2f}"
    assert (on & short).sum() / short.sum() > 0.85, "26 像素的短線被誤殺"
    assert not (on & speck).any(), "碎屑沒濾掉"
    assert kept == 4, f"連通塊數 {kept}（應為 3 道弧加 1 道短線）"
    solid = out[..., 3] > 0
    assert (out[solid, :3] == np.array(P.INK, np.uint8)).all(), "成品顏色不是純墨色（帶了膚色）"
    skin_area = (np.abs(painted.astype(int) - np.array(SKIN)[None, None]).max(2) < 6) & zone
    assert not (on & skin_area).any(), "膚色底留在成品裡"
    bad = on & ~cv2.dilate(truth.astype(np.uint8), np.ones((5, 5), np.uint8)).astype(bool)
    assert bad.sum() == 0, f"線條以外有 {bad.sum()} 個像素（底版暗線的殘邊被收進來）"
    print("細線收件測試通過：線條留下比例 %.2f，連通塊 %d" % (recall, kept))


if __name__ == "__main__":
    main()
