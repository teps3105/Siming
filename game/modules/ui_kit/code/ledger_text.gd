class_name LedgerText
extends RefCounted
## GDD 5.8 命簿裡的折行文字：一段話依寬度逐字折行（不截斷），段中的人名、地名可標成可點的連結。
## 段落（paragraph）是一個字典：lines（各行文字）、height（總高）、size（字級）、line_height、hits（可點範圍）。
## 範圍的座標以段落左上角為原點，hits 的每一項有 rect 與 target（呼叫端自訂的目標字串）。

## 不該出現在行首的標點。
const NO_LINE_START := "，。、；：！？）」』】》…"
const LINE_FACTOR := 1.35
const DIGITS := "0123456789"
## 緊跟數字的量詞，與數字不拆開；連接號不落行首。
const QUANTIFIERS := "年月日歲人位世則章回州郡"
const DASHES := "–—－~"

static var _widths := {}


static func glyph_width(glyph: String, size: int) -> float:
	var key := "%s|%d" % [glyph, size]
	if not _widths.has(key):
		_widths[key] = Ink.text_size(glyph, size).x
	return _widths[key]


static func line_height(size: int) -> float:
	return float(size) * LINE_FACTOR


## 折行並找出連結的範圍。links 每項有 text（要標成連結的字）與 target；各取第一次出現的位置。
## 折行的最小單位（不拆開）：連續數字連同緊跟的量詞（「148年」）、行首禁用的標點連同前一個字、可點的連結名、
## keep 裡指定的片段（呼叫端要整段不拆的，例如「姓名（起年–迄年）」）。單位放不下一行時依序放寬：先允許
## keep 與連結名內部折開但括號內仍不拆，再允許括號內折開（數字與量詞仍不拆），最後才逐字折。
static func paragraph(text: String, links: Array[Dictionary], width: float, size: int, keep: Array[String] = []) -> Dictionary:
	var count := text.length()
	# 連結的位置：依序往後找，同名的兩個連結（例如所在與故里同一州）各落在各自的位置。
	var found: Array[Vector2i] = []
	var cursor := 0
	for link in links:
		var word: String = link["text"]
		var at := -1 if word.is_empty() else text.find(word, cursor)
		if at < 0 and not word.is_empty():
			at = text.find(word)
		if at >= 0:
			cursor = at + word.length()
			found.append(Vector2i(at, at + word.length()))
		else:
			found.append(Vector2i(-1, -1))
	var glue := _glue(text, found, keep)
	var lines: Array[String] = []
	var starts: Array[int] = []
	var current := ""
	var start := 0
	var run := 0.0
	for unit: Vector2i in _units(text, glue, 0, 0, count, width, size):
		var piece := text.substr(unit.x, unit.y - unit.x)
		var unit_width := Ink.text_size(piece, size).x
		if run + unit_width > width and not current.is_empty():
			lines.append(current)
			starts.append(start)
			current = ""
			run = 0.0
		if current.is_empty():
			start = unit.x
		if unit_width <= width:
			current += piece
			run += unit_width
			continue
		# 連最小單位都比一行寬：逐字折。
		for i in range(unit.x, unit.y):
			var advance := glyph_width(text[i], size)
			if run + advance > width and not current.is_empty():
				lines.append(current)
				starts.append(start)
				current = ""
				start = i
				run = 0.0
			current += text[i]
			run += advance
	if not current.is_empty() or lines.is_empty():
		lines.append(current)
		starts.append(start)
	var height := line_height(size)
	var hits: Array[Dictionary] = []
	for index in links.size():
		var span: Vector2i = found[index]
		if span.x < 0:
			continue
		for row in lines.size():
			var from := maxi(span.x, starts[row])
			var to := mini(span.y, starts[row] + lines[row].length())
			if to <= from:
				continue
			var left := Ink.text_size(lines[row].substr(0, from - starts[row]), size).x
			var right := Ink.text_size(lines[row].substr(0, to - starts[row]), size).x
			hits.append({"rect": Rect2(left, row * height, right - left, height), "target": links[index]["target"]})
	return {"lines": lines, "height": lines.size() * height, "size": size, "line_height": height, "hits": hits}


## 三層「這個字黏在前一個字上」的標記：0 層最緊（含連結名與 keep 片段），1 層放開它們但括號內仍黏，2 層只留數字與標點。
static func _glue(text: String, found: Array[Vector2i], keep: Array[String]) -> Array[PackedByteArray]:
	var count := text.length()
	var basic := PackedByteArray()
	basic.resize(count)
	for i in range(1, count):
		var glyph := text[i]
		var before := text[i - 1]
		if NO_LINE_START.contains(glyph) or DASHES.contains(glyph):
			basic[i] = 1
		elif DIGITS.contains(glyph) and DIGITS.contains(before):
			basic[i] = 1
		elif QUANTIFIERS.contains(glyph) and DIGITS.contains(before):
			basic[i] = 1
	var inner := basic.duplicate()
	var open := -1
	for i in count:
		if text[i] == "（":
			open = i
		elif text[i] == "）" and open >= 0:
			for j in range(open + 1, i + 1):
				inner[j] = 1
			open = -1
	var tight := inner.duplicate()
	var spans: Array[Vector2i] = []
	for span in found:
		if span.x >= 0:
			spans.append(span)
	var cursor := 0
	for piece in keep:
		var at := text.find(piece, cursor)
		if at >= 0:
			spans.append(Vector2i(at, at + piece.length()))
			cursor = at + piece.length()
	for span in spans:
		for j in range(span.x + 1, span.y):
			tight[j] = 1
	return [tight, inner, basic]


## 把 [from, to) 切成折行單位：依 level 層的標記切，切出來的單位仍比一行寬就換下一層再切。
static func _units(text: String, glue: Array[PackedByteArray], level: int, from: int, to: int, width: float, size: int) -> Array[Vector2i]:
	var out: Array[Vector2i] = []
	var s := from
	for i in range(from + 1, to + 1):
		if i < to and glue[level][i] == 1:
			continue
		if level < glue.size() - 1 and Ink.text_size(text.substr(s, i - s), size).x > width:
			out.append_array(_units(text, glue, level + 1, s, i, width, size))
		else:
			out.append(Vector2i(s, i))
		s = i
	return out


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
