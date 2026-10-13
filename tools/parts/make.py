"""一件部件從畫家原圖到明度鍵的收件流程：收件（臉走去背）→ 一致化。

    python tools/parts/make.py <部件> [--try N] [--thr 38] [--thick 6] [--keep-skin]

先把這件現有的 collect/、key/ 成品備份到 old/（<部件>.* 與 <部件>-t<前一版>），再重收；之後 check.py 量測。
"""
import argparse
import os
import shutil
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import portrait as P  # noqa: E402
import collect as C  # noqa: E402
import key as K  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("part")
    ap.add_argument("--try", dest="n", type=int, default=1)
    ap.add_argument("--thr", type=int, default=38)
    ap.add_argument("--thick", type=float, default=6.0)
    ap.add_argument("--keep-skin", action="store_true")
    a = ap.parse_args()
    os.makedirs(P.TMP + "old", exist_ok=True)
    for sub in ("collect", "key"):
        d = P.TMP + sub + "/"
        for f in os.listdir(d) if os.path.isdir(d) else []:
            if f.startswith(a.part + "."):
                shutil.copy(d + f, P.TMP + "old/" + sub + "-" + f)
    if P.PARTS[a.part].get("zone") is None:
        C.collect_face(a.part, a.n)
    else:
        C.collect(a.part, a.n, a.thr, thick=a.thick, keep_skin=a.keep_skin)
    K.key(a.part)


if __name__ == "__main__":
    main()
