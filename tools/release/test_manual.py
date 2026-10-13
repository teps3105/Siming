"""manual.py 的測試：python -m unittest discover -s tools/release -p "test_manual.py" """
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import manual  # noqa: E402

PNG = bytes.fromhex("89504e470d0a1a0a")  # 內容不解碼，只要是位元組
SAMPLE = ("# 9.7 遊戲手冊\n\n給開發者看的說明。\n\n## 開始遊戲\n\n點擊新遊戲。\n\n![主畫面](manual/a.png)\n\n"
          "| 甲 | 乙 |\n|---|---|\n| 1 | 2 |\n")


class HtmlTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.images = Path(self.tmp.name)
        (self.images / "a.png").write_bytes(PNG)

    def tearDown(self):
        self.tmp.cleanup()

    def test_only_after_first_h2_and_cover_has_version(self):
        html = manual.manual_html(SAMPLE, "開發版 abc1234", self.images)
        self.assertNotIn("給開發者看的說明", html)
        self.assertIn("<h2>開始遊戲</h2>", html)
        self.assertIn("開發版 abc1234", html)
        self.assertIn("<table>", html)

    def test_image_embedded(self):
        html = manual.manual_html(SAMPLE, "v", self.images)
        self.assertIn("data:image/png;base64,", html)
        self.assertIn("<figcaption>主畫面</figcaption>", html)

    def test_marks_become_percent_boxes(self):
        (self.images / "marks.json").write_text('{"a.png": [[192, 108, 960, 540], [0, 10, 100, 100]]}', encoding="utf-8")
        html = manual.manual_html(SAMPLE, "v", self.images)
        self.assertIn("left:10.00%;top:10.00%;width:50.00%;height:50.00%", html)
        self.assertIn("<b>2</b>", html)
        self.assertIn('class="mark low"', html)

    def test_error_missing_image(self):
        (self.images / "a.png").unlink()
        with self.assertRaisesRegex(manual.ManualError, "不存在"):
            manual.manual_html(SAMPLE, "v", self.images)

    def test_error_image_not_own_paragraph(self):
        with self.assertRaisesRegex(manual.ManualError, "獨立成段"):
            manual.manual_html("## 章\n\n文字 ![x](manual/a.png) 文字\n", "v", self.images)

    def test_error_marks_unused_image(self):
        (self.images / "marks.json").write_text('{"b.png": [[0, 0, 10, 10]]}', encoding="utf-8")
        with self.assertRaisesRegex(manual.ManualError, "沒用到的圖：b.png"):
            manual.manual_html(SAMPLE, "v", self.images)

    def test_error_no_h2(self):
        with self.assertRaisesRegex(manual.ManualError, "二級標題"):
            manual.manual_html("# 只有說明\n", "v", self.images)


class RenderTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.target = Path(self.tmp.name) / "m.pdf"

    def tearDown(self):
        self.tmp.cleanup()

    def test_pdf_renamed_from_part(self):
        def renderer(html, target):
            self.assertTrue(target.name.endswith(".part"))
            target.write_bytes(b"%PDF-1.7\nx")
        manual.render_manual("<html>", self.target, renderer)
        self.assertTrue(self.target.is_file())
        self.assertFalse(self.target.with_name("m.pdf.part").exists())

    def test_not_pdf_fails_and_leaves_nothing(self):
        with self.assertRaisesRegex(manual.ManualError, "不是 PDF"):
            manual.render_manual("<html>", self.target, lambda h, t: t.write_bytes(b"<html>"))
        self.assertEqual(list(Path(self.tmp.name).iterdir()), [])


if __name__ == "__main__":
    unittest.main()
