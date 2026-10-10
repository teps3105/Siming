class_name Ink
extends RefCounted
## GDD 5.7「司命案頭」的器物繪製：紙簽、印章、圓印、橫條。所有介面元件以這些函式在 _draw 中畫出，
## 不使用 Godot 預設主題的外觀。色票見 GDD 5.1。

const FONT: FontFile = preload("res://modules/ui_kit/assets/fonts/wenkai_tc_subset.ttf")

const PAPER := Color("EDE4CF")
const PAPER_LIGHT := Color("F3EBD8")
const PAPER_DARK := Color("D9CBA8")
const INK := Color("2B2620")
const INK_LIGHT := Color("6B6152")
const SEAL_RED := Color("B23A2E")
const GOLD := Color("C9A246")
const CYAN := Color("4F8F9A")
const SILK := Color("3C4A52")
const KARMA := Color("8C2A1C")

## 字級（GDD 5.7 檢查表第 5 項）。
const SIZE_TITLE := 22
const SIZE_SECTION := 15
const SIZE_BODY := 16
const SIZE_NOTE := 13


## 毛邊紙簽：柔影、紙面、淡墨細邊。邊緣以固定種子的微小起伏做出手撕感。
static func paper_slip(item: CanvasItem, rect: Rect2, lift := 0.0, seed_value := 1) -> void:
	if rect.size.x < 4.0 or rect.size.y < 4.0:
		return
	for i in 4:
		var grow := float(4 - i) * 1.5
		item.draw_rect(rect.grow(grow).grow_individual(0, 0, 0, 2.0 + lift), Color(INK, 0.035), true)
	var points := _torn_outline(rect, seed_value)
	item.draw_colored_polygon(points, PAPER_LIGHT)
	points.append(points[0])
	item.draw_polyline(points, Color(INK_LIGHT, 0.55), 1.0, true)


## 方形朱印：實心朱底反白字（啟用）、淡墨框墨字（未啟用）、淡紙底淡字（不可用）。
static func seal(item: CanvasItem, rect: Rect2, glyph: String, active: bool, enabled := true, hover := false) -> void:
	var inner := rect.grow(-1.0)
	if active:
		item.draw_rect(inner, SEAL_RED, true)
		item.draw_rect(inner.grow(-3.0), Color(PAPER_LIGHT, 0.85), false, 1.0)
		_centered_glyph(item, rect, glyph, PAPER_LIGHT)
	else:
		item.draw_rect(inner, Color(PAPER_LIGHT, 0.92 if enabled else 0.6), true)
		var edge := INK if hover and enabled else INK_LIGHT
		item.draw_rect(inner, Color(edge, 1.0 if enabled else 0.35), false, 1.5 if hover and enabled else 1.0)
		_centered_glyph(item, rect, glyph, Color(INK, 1.0 if enabled else 0.3))


## 圓印（天時儀盤心）。
static func round_seal(item: CanvasItem, center: Vector2, radius: float, glyph: String, red: bool) -> void:
	item.draw_circle(center, radius, SEAL_RED if red else PAPER_LIGHT)
	item.draw_arc(center, radius, 0.0, TAU, 48, SEAL_RED if not red else Color(PAPER_LIGHT, 0.8), 1.5, true)
	var rect := Rect2(center - Vector2.ONE * radius, Vector2.ONE * radius * 2.0)
	_centered_glyph(item, rect, glyph, PAPER_LIGHT if red else SEAL_RED)


## 0..1 的小橫條：淡墨底線加一筆有色墨。
static func bar(item: CanvasItem, rect: Rect2, value: float, color: Color) -> void:
	item.draw_rect(rect, Color(INK_LIGHT, 0.18), true)
	var fill := rect
	fill.size.x = rect.size.x * clampf(value, 0.0, 1.0)
	item.draw_rect(fill, color, true)


## 帶紙色描邊的文字（地圖標籤與紙面外的字）。pos 為文字左上角。
static func outlined_text(item: CanvasItem, pos: Vector2, text: String, size: int, color: Color, outline := 3) -> void:
	var baseline := pos + Vector2(0.0, FONT.get_ascent(size))
	item.draw_string_outline(FONT, baseline, text, HORIZONTAL_ALIGNMENT_LEFT, -1.0, size, outline, Color(PAPER_LIGHT, color.a * 0.9))
	item.draw_string(FONT, baseline, text, HORIZONTAL_ALIGNMENT_LEFT, -1.0, size, color)


## 紙面上的文字。pos 為文字左上角。
static func text(item: CanvasItem, pos: Vector2, content: String, size: int, color := INK) -> void:
	item.draw_string(FONT, pos + Vector2(0.0, FONT.get_ascent(size)), content, HORIZONTAL_ALIGNMENT_LEFT, -1.0, size, color)


static func text_size(content: String, size: int) -> Vector2:
	return FONT.get_string_size(content, HORIZONTAL_ALIGNMENT_LEFT, -1.0, size)


static func _centered_glyph(item: CanvasItem, rect: Rect2, glyph: String, color: Color) -> void:
	var size := int(rect.size.y * 0.62)
	var extent := text_size(glyph, size)
	text(item, rect.position + (rect.size - extent) * 0.5, glyph, size, color)


static func _torn_outline(rect: Rect2, seed_value: int) -> PackedVector2Array:
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value
	var points := PackedVector2Array()
	var corners := [rect.position, Vector2(rect.end.x, rect.position.y), rect.end, Vector2(rect.position.x, rect.end.y)]
	for side in 4:
		var from: Vector2 = corners[side]
		var to: Vector2 = corners[(side + 1) % 4]
		var normal := (to - from).normalized().orthogonal()
		var steps := maxi(int(from.distance_to(to) / 14.0), 1)
		for i in steps:
			var jitter := 0.0 if i == 0 else rng.randf_range(-0.9, 0.9)
			points.append(from.lerp(to, float(i) / steps) + normal * jitter)
	return points
