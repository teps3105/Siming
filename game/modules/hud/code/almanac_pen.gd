class_name AlmanacPen
extends RefCounted
## GDD 5.10 天下冊的圖表畫法：長條、金字塔、圓環、排行、一列大字，全以 CanvasItem 自訂繪製，不用貼圖。
## 每個函式同時負責量與畫：item 為 null 只量高度與可點範圍（hits），不畫；item 不為 null 才畫。
## 座標一律是「內容座標」（原點在欄的左上角），由 LedgerList 的 painter 平移後落到畫面上。
## hits 的每一項有 target（目標字串）與 rect（矩形）或 arc（圓環的一段：center、inner、outer、from、to）。

const ROW_HEIGHT := 28.0
const SECTION_GAP := 18.0
const PYRAMID_MIN_WIDTH := 12.0
const BAR_FRACTION := 0.7
const RING_RADIUS := 40.0
const RING_WIDTH := 12.0

var item: CanvasItem
var hits: Array[Dictionary] = []
var unit := 1.0
## 停留與選取中的目標字串。
var hover := ""
var selected := ""


func _init(canvas: CanvasItem, scale: float) -> void:
	item = canvas
	unit = scale


func body_size() -> int:
	return Ink.font_size(Ink.SIZE_BODY, unit)


func note_size() -> int:
	return Ink.font_size(Ink.SIZE_NOTE, unit)


func row_height() -> float:
	return ROW_HEIGHT * unit


func text_width(text: String, size: int) -> float:
	return Ink.text_size(text, size).x


## 把一句話截到放得進 width；截斷時結尾加「…」。
func fit(text: String, size: int, width: float) -> String:
	if width <= 0.0:
		return ""
	if text_width(text, size) <= width:
		return text
	var count := text.length()
	while count > 0 and text_width(text.substr(0, count) + "…", size) > width:
		count -= 1
	return text.substr(0, count) + "…"


## 在 rect 內垂直置中畫字。
func put(text: String, pos: Vector2, size: int, color: Color, row_h: float) -> void:
	if item != null:
		Ink.text(item, Vector2(pos.x, pos.y + (row_h - Ink.text_size(text, size).y) * 0.5), text, size, color)


func put_right(text: String, right: float, y: float, size: int, color: Color, row_h: float) -> void:
	put(text, Vector2(right - text_width(text, size), y), size, color, row_h)


func _mark(target: String, rect: Rect2) -> void:
	if not target.is_empty():
		hits.append({"target": target, "rect": rect})


func _lit(rect: Rect2, target: String) -> void:
	if item == null or target.is_empty():
		return
	if target == selected:
		item.draw_rect(rect, Color(Ink.SEAL_RED, 0.14), true)
		item.draw_rect(Rect2(rect.position, Vector2(3.0 * unit, rect.size.y)), Ink.SEAL_RED, true)
	elif target == hover:
		item.draw_rect(rect, Color(Ink.INK, 0.08), true)


## 一節的標題：小墨點加節名。回傳高度。
func section(origin: Vector2, title: String) -> float:
	var size := Ink.font_size(Ink.SIZE_SECTION, unit)
	if item != null:
		Ink.section(item, origin, title, size)
	return Ink.text_size(title, size).y + 8.0 * unit


## 一行說明文字（淡墨，放不下就折行）。回傳高度。
func note(origin: Vector2, text: String, width: float, color := Ink.INK_LIGHT) -> float:
	var size := note_size()
	var para := LedgerText.paragraph(text, [] as Array[Dictionary], width, size)
	if item != null:
		LedgerText.draw(item, origin, para, color)
	return para["height"]


## 一列大字：每格上面一行小標、下面一個大數字，放不下就折到下一列。cells: [{label, value}]。回傳高度。
func stats(origin: Vector2, width: float, cells: Array) -> float:
	var small := note_size()
	var big := Ink.font_size(Ink.SIZE_TITLE, unit) + int(round(6.0 * unit))
	var gap := 22.0 * unit
	var x := 0.0
	var y := 0.0
	var line_h := Ink.text_size("字", small).y + Ink.text_size("0", big).y + 8.0 * unit
	for cell: Dictionary in cells:
		var w := maxf(text_width(cell["label"], small), text_width(cell["value"], big))
		if x > 0.0 and x + w > width:
			x = 0.0
			y += line_h + 6.0 * unit
		if item != null:
			Ink.text(item, origin + Vector2(x, y), cell["label"], small, Ink.INK_LIGHT)
			Ink.text(item, origin + Vector2(x, y + Ink.text_size("字", small).y + 2.0 * unit), cell["value"], big, Ink.INK)
		x += w + gap
	return y + line_h


## 長條：橫向墨條，左端名稱、右端數值，最長的一條佔欄寬的 70%。items: [{key, label, value, text, color}]（key 為點選的目標）。
func bars(origin: Vector2, width: float, items: Array) -> float:
	var size := body_size()
	var row_h := row_height()
	var label_w := 0.0
	var value_w := 0.0
	var peak := 0.0
	for it: Dictionary in items:
		label_w = maxf(label_w, text_width(it["label"], size))
		value_w = maxf(value_w, text_width(it["text"], size))
		peak = maxf(peak, float(it["value"]))
	label_w = minf(label_w, width * 0.34)
	var gap := 10.0 * unit
	var room := maxf(minf(width * BAR_FRACTION, width - label_w - value_w - gap * 3.0), 10.0 * unit)
	for i in items.size():
		var it: Dictionary = items[i]
		var y := i * row_h
		var rect := Rect2(origin + Vector2(0.0, y), Vector2(width, row_h))
		_mark(it.get("key", ""), rect)
		_lit(rect, it.get("key", ""))
		put(fit(it["label"], size, label_w), origin + Vector2(6.0 * unit, y), size, Ink.INK, row_h)
		var length := room * (float(it["value"]) / peak if peak > 0.0 else 0.0)
		var left := origin.x + label_w + gap + 6.0 * unit
		if item != null:
			var color: Color = it.get("color", Ink.SILK)
			var mid := origin.y + y + row_h * 0.5
			item.draw_line(Vector2(left, mid), Vector2(left + room, mid), Color(Ink.INK_LIGHT, 0.25), 1.0)
			if length >= 2.0:
				Ink.stroke(item, Vector2(left, mid), Vector2(left + length, mid), row_h * 0.46, color)
		# 數值固定排在軌道右端之外，各列對齊，不壓在軌道線上。
		put(it["text"], Vector2(left + room + gap * 0.6, origin.y + y), size, Ink.INK, row_h)
	return items.size() * row_h


## 金字塔：由上往下列出各層（呼叫端把最高的層放第一個），逐層置中的橫條，層高相同，寬度取對數、最少 12 px。
## items: [{key, label, value, text, color}]。
func pyramid(origin: Vector2, width: float, items: Array) -> float:
	var size := body_size()
	var row_h := row_height()
	var label_w := 0.0
	var value_w := 0.0
	var peak := 0.0
	for it: Dictionary in items:
		label_w = maxf(label_w, text_width(it["label"], size))
		value_w = maxf(value_w, text_width(it["text"], size))
		peak = maxf(peak, float(it["value"]))
	var gap := 10.0 * unit
	var area_left := origin.x + label_w + gap + 6.0 * unit
	var area_w := maxf(width - label_w - value_w - gap * 3.0 - 6.0 * unit, 40.0 * unit)
	var center := area_left + area_w * 0.5
	for i in items.size():
		var it: Dictionary = items[i]
		var y := i * row_h
		var rect := Rect2(origin + Vector2(0.0, y), Vector2(width, row_h))
		_mark(it.get("key", ""), rect)
		_lit(rect, it.get("key", ""))
		put(it["label"], origin + Vector2(6.0 * unit, y), size, Ink.INK, row_h)
		var v := float(it["value"])
		# 沒有人的一層不畫塊，只留一道細線。
		var span := 0.0
		if peak > 0.0 and v > 0.0:
			span = maxf(PYRAMID_MIN_WIDTH * unit, area_w * log(1.0 + v) / log(1.0 + peak))
		if item != null and span <= 0.0:
			item.draw_line(Vector2(center - 6.0 * unit, origin.y + y + row_h * 0.5), Vector2(center + 6.0 * unit, origin.y + y + row_h * 0.5), Color(Ink.INK_LIGHT, 0.5), 1.0)
		elif item != null:
			var layer := Rect2(Vector2(center - span * 0.5, origin.y + y + 3.0 * unit), Vector2(span, row_h - 6.0 * unit))
			item.draw_rect(layer, it.get("color", Ink.SILK), true)
			item.draw_rect(layer, Color(Ink.INK, 0.5), false, 1.0)
		put_right(it["text"], origin.x + width - 6.0 * unit, origin.y + y, size, Ink.INK, row_h)
	return items.size() * row_h


## 圓環（與州郡卡的三教圓環同一個畫法）：左邊一個環，右邊每段一行「色塊 名稱 百分比」。
## items: [{key, label, value, color}]；每段旁寫名稱與百分比。回傳高度。
func ring(origin: Vector2, width: float, items: Array) -> float:
	var size := body_size()
	var radius := RING_RADIUS * unit
	var stroke := RING_WIDTH * unit
	var center := origin + Vector2(radius + stroke * 0.5 + 6.0 * unit, radius + stroke * 0.5 + 4.0 * unit)
	var total := 0.0
	for it: Dictionary in items:
		total += float(it["value"])
	var row_h := row_height()
	var height := maxf(radius * 2.0 + stroke + 8.0 * unit, items.size() * row_h)
	if item != null:
		item.draw_arc(center, radius, 0.0, TAU, 48, Color(Ink.INK_LIGHT, 0.2), stroke, true)
		# 淺色的一段（雪原）在紙上看不清，環的內外緣各描一圈細線。
		item.draw_arc(center, radius - stroke * 0.5, 0.0, TAU, 48, Color(Ink.INK_LIGHT, 0.55), 1.0, true)
		item.draw_arc(center, radius + stroke * 0.5, 0.0, TAU, 48, Color(Ink.INK_LIGHT, 0.55), 1.0, true)
	var start := -PI * 0.5
	var legend_x := origin.x + radius * 2.0 + stroke + 26.0 * unit
	var legend_top := origin.y + (height - items.size() * row_h) * 0.5
	for i in items.size():
		var it: Dictionary = items[i]
		var share := float(it["value"]) / total if total > 0.0 else 0.0
		var sweep := TAU * share
		var key: String = it.get("key", "")
		var color: Color = it.get("color", Ink.SILK)
		if sweep > 0.002:
			if item != null:
				var lit := key == selected or key == hover
				item.draw_arc(center, radius, start, start + sweep, maxi(int(sweep * 12.0), 4), color, stroke * (1.25 if lit else 1.0), true)
			if not key.is_empty():
				hits.append({"target": key, "arc": {"center": center, "inner": radius - stroke * 0.7, "outer": radius + stroke * 0.7, "from": start, "to": start + sweep}})
		start += sweep
		var rect := Rect2(Vector2(legend_x - 6.0 * unit, legend_top + i * row_h), Vector2(maxf(origin.x + width - legend_x + 6.0 * unit, 10.0), row_h))
		_mark(key, rect)
		_lit(rect, key)
		if item != null:
			item.draw_rect(Rect2(Vector2(legend_x, rect.position.y + (row_h - 12.0 * unit) * 0.5), Vector2.ONE * 12.0 * unit), color, true)
			item.draw_rect(Rect2(Vector2(legend_x, rect.position.y + (row_h - 12.0 * unit) * 0.5), Vector2.ONE * 12.0 * unit), Color(Ink.INK_LIGHT, 0.6), false, 1.0)
		var text := "%s %d%%" % [it["label"], int(round(share * 100.0))]
		put(fit(text, size, origin.x + width - legend_x - 18.0 * unit), Vector2(legend_x + 18.0 * unit, rect.position.y), size, Ink.INK, row_h)
	return height


## 排行：文字表，名次、名稱、數值欄。點欄名依該欄排序（欄有 sort 才可點）。
## columns: [{label, sort（排序目標，空為不可排）, align: "left"|"right"|"center"}]；欄 0 是可縮短的欄。
## rows: [{cells: [{text, target, color}], target（整列的目標）}]。回傳高度。
func table(origin: Vector2, width: float, columns: Array, rows: Array, sorted_by := "") -> float:
	var size := body_size()
	var pad := 16.0 * unit
	var measure := func(font: int) -> PackedFloat32Array:
		var out := PackedFloat32Array()
		for c in columns.size():
			var w := text_width(columns[c]["label"], font)
			for row: Dictionary in rows:
				w = maxf(w, text_width(row["cells"][c]["text"], font))
			out.append(w + pad)
		return out
	var widths: PackedFloat32Array = measure.call(size)
	var total := 0.0
	for w in widths:
		total += w
	if total > width:
		size = note_size()
		widths = measure.call(size)
		total = 0.0
		for w in widths:
			total += w
	if total > width and columns.size() > 0:
		widths[0] = maxf(widths[0] - (total - width), 36.0 * unit)
	elif total < width:
		# 有餘裕就平均攤給各欄，數值不擠在一起。
		var extra := (width - total) / columns.size()
		for c in columns.size():
			widths[c] += extra
	var row_h := row_height()
	var head := Rect2(origin, Vector2(width, row_h))
	if item != null:
		item.draw_line(Vector2(origin.x, origin.y + row_h), Vector2(origin.x + width, origin.y + row_h), Color(Ink.INK_LIGHT, 0.6), 1.0)
	var x := origin.x
	for c in columns.size():
		var col: Dictionary = columns[c]
		var sort_target: String = col.get("sort", "")
		var cell_rect := Rect2(Vector2(x, origin.y), Vector2(widths[c], row_h))
		_mark(sort_target, cell_rect)
		_lit(cell_rect, sort_target)
		var label: String = col["label"] + (" ▾" if sort_target == sorted_by and not sort_target.is_empty() else "")
		_cell(fit(label, size, widths[c] - pad), cell_rect, size, Ink.SEAL_RED if not sort_target.is_empty() else Ink.INK_LIGHT, col.get("align", "left"), pad)
		x += widths[c]
	for r in rows.size():
		var row: Dictionary = rows[r]
		var y := origin.y + (r + 1) * row_h
		var row_rect := Rect2(Vector2(origin.x, y), Vector2(width, row_h))
		var row_target: String = row.get("target", "")
		_mark(row_target, row_rect)
		_lit(row_rect, row_target)
		if item != null and r % 2 == 1:
			item.draw_rect(row_rect, Color(Ink.PAPER_DARK, 0.18), true)
		x = origin.x
		for c in columns.size():
			var cell: Dictionary = row["cells"][c]
			var cell_rect := Rect2(Vector2(x, y), Vector2(widths[c], row_h))
			var target: String = cell.get("target", "")
			if not target.is_empty():
				# 格內的連結優先於整列的目標。
				_mark_front(target, cell_rect)
				_lit(cell_rect, target)
			var color: Color = cell.get("color", Ink.INK)
			_cell(fit(cell["text"], size, widths[c] - pad), cell_rect, size, color, columns[c].get("align", "left"), pad, not target.is_empty())
			x += widths[c]
	return (rows.size() + 1) * row_h


func _mark_front(target: String, rect: Rect2) -> void:
	hits.push_front({"target": target, "rect": rect})


func _cell(text: String, rect: Rect2, size: int, color: Color, align: String, pad: float, link := false) -> void:
	if item == null or text.is_empty():
		return
	var extent := Ink.text_size(text, size)
	var x := rect.position.x + pad * 0.5
	if align == "right":
		x = rect.end.x - pad * 0.5 - extent.x
	elif align == "center":
		x = rect.position.x + (rect.size.x - extent.x) * 0.5
	var y := rect.position.y + (rect.size.y - extent.y) * 0.5
	Ink.text(item, Vector2(x, y), text, size, color)
	if link:
		item.draw_line(Vector2(x, y + extent.y - 2.0), Vector2(x + extent.x, y + extent.y - 2.0), Color(Ink.SEAL_RED, 0.45), 1.0)


## 目標字串：point 落在哪一個可點的項目（內容座標）；沒有為空字串。
static func target_at(hit_list: Array[Dictionary], point: Vector2) -> String:
	for hit in hit_list:
		if hit.has("rect"):
			if (hit["rect"] as Rect2).has_point(point):
				return hit["target"]
		else:
			var arc: Dictionary = hit["arc"]
			var offset: Vector2 = point - (arc["center"] as Vector2)
			var dist := offset.length()
			if dist >= arc["inner"] and dist <= arc["outer"]:
				var angle := offset.angle()
				var from: float = arc["from"]
				var to: float = arc["to"]
				# 角度換到 [from, from + TAU) 比較。
				var rel := fposmod(angle - from, TAU)
				if rel <= to - from:
					return hit["target"]
	return ""
