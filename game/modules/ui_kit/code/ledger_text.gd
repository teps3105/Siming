class_name LedgerText
extends RefCounted
## GDD 5.8 命簿裡的折行文字：一段話依寬度逐字折行（不截斷），段中的人名、地名可標成可點的連結。
## 段落（paragraph）是一個字典：lines（各行文字）、height（總高）、size（字級）、line_height、hits（可點範圍）。
## 範圍的座標以段落左上角為原點，hits 的每一項有 rect 與 target（呼叫端自訂的目標字串）。

## 不該出現在行首的標點。
const NO_LINE_START := "，。、；：！？）」』】》…"
const LINE_FACTOR := 1.35
const DIGITS := "0123456789"

static var _widths := {}


static func glyph_width(glyph: String, size: int) -> float:
	var key := "%s|%d" % [glyph, size]
	if not _widths.has(key):
		_widths[key] = Ink.text_size(glyph, size).x
	return _widths[key]


static func line_height(size: int) -> float:
	return float(size) * LINE_FACTOR


## 折行並找出連結的範圍。links 每項有 text（要標成連結的字）與 target；各取第一次出現的位置。
static func paragraph(text: String, links: Array[Dictionary], width: float, size: int) -> Dictionary:
	var lines: Array[String] = []
	var starts: Array[int] = []
	var current := ""
	var start := 0
	var run := 0.0
	for i in text.length():
		var glyph := text[i]
		var advance := glyph_width(glyph, size)
		# 數字不從中間折開（「148年」不折成「14」「8年」）。
		var inside_number := i > 0 and DIGITS.contains(glyph) and DIGITS.contains(text[i - 1])
		if run + advance > width and not current.is_empty() and not NO_LINE_START.contains(glyph) and not inside_number:
			lines.append(current)
			starts.append(start)
			current = ""
			start = i
			run = 0.0
		current += glyph
		run += advance
	if not current.is_empty() or lines.is_empty():
		lines.append(current)
		starts.append(start)
	var height := line_height(size)
	var hits: Array[Dictionary] = []
	var cursor := 0
	for link in links:
		var word: String = link["text"]
		if word.is_empty():
			continue
		# 依序往後找：同名的兩個連結（例如所在與故里同一州）各落在各自的位置。
		var at := text.find(word, cursor)
		if at < 0:
			at = text.find(word)
		if at < 0:
			continue
		cursor = at + word.length()
		for row in lines.size():
			var from := maxi(at, starts[row])
			var to := mini(at + word.length(), starts[row] + lines[row].length())
			if to <= from:
				continue
			var left := Ink.text_size(lines[row].substr(0, from - starts[row]), size).x
			var right := Ink.text_size(lines[row].substr(0, to - starts[row]), size).x
			hits.append({"rect": Rect2(left, row * height, right - left, height), "target": link["target"]})
	return {"lines": lines, "height": lines.size() * height, "size": size, "line_height": height, "hits": hits}


## 畫出段落。origin 為左上角；hover 為停留中的連結目標，pressed 為按住的。
static func draw(item: CanvasItem, origin: Vector2, para: Dictionary, color: Color, hover := "", pressed := "") -> void:
	var size: int = para["size"]
	var step: float = para["line_height"]
	var lines: Array = para["lines"]
	for hit: Dictionary in para["hits"]:
		var rect: Rect2 = hit["rect"]
		rect.position += origin
		var lit: bool = hit["target"] == hover or hit["target"] == pressed
		if lit:
			item.draw_rect(rect.grow_individual(2.0, -1.0, 2.0, -2.0), Color(Ink.INK, 0.2 if hit["target"] == pressed else 0.1), true)
		var base := rect.position.y + rect.size.y - 3.0
		item.draw_line(Vector2(rect.position.x, base), Vector2(rect.end.x, base), Color(Ink.SEAL_RED, 0.75 if lit else 0.45), 1.0)
	for row in lines.size():
		var y := origin.y + row * step + (step - Ink.text_size("字", size).y) * 0.5
		Ink.text(item, Vector2(origin.x, y), lines[row], size, color)


## 段落裡點在 point（控制項座標）的連結目標；沒有為空字串。
static func target_at(origin: Vector2, para: Dictionary, point: Vector2) -> String:
	for hit: Dictionary in para["hits"]:
		var rect: Rect2 = hit["rect"]
		rect.position += origin
		if rect.has_point(point):
			return hit["target"]
	return ""
