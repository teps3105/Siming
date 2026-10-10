class_name Ink
extends RefCounted
## GDD 5.7「司命案頭」的器物繪製：裱框紙簽、按鈕底、圓鈕、實體鍵圖像、墨條、墨劃。所有介面元件以這些函式在 _draw 中畫出，
## 不使用 Godot 預設主題的外觀。色票見 GDD 5.1。
## 邊框語言只有一種——絹本裱框：直邊紙面、外圈綾邊、內緣一道金線、底下一層柔影。

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
const SIZE_TITLE := 20
const SIZE_SECTION := 15
const SIZE_BODY := 16
const SIZE_NOTE := 13
## Steam Deck 版面的最小字級（GDD 5.9）。
const DECK_MIN_SIZE := 16

## 綾邊寬：紙簽 3、命簿 6（GDD 5.8）。
const SLIP_BAND := 3.0
const CARD_BAND := 6.0
## 尚未開放的按鈕，圖示與文字的不透明度。
const DISABLED_GLYPH_ALPHA := 0.75
## 實體鍵圖像的高度。
const KEY_HEIGHT := 20.0
## 實體鍵圖像裡的字級（Steam Deck 版面同樣受 DECK_MIN_SIZE 限制）。
const KEY_TEXT_SIZE := 12
## 搖桿面鍵的顏色（依 Xbox 慣例的綠紅藍黃，取色票內相近的低彩度色，GDD 5.9）。
const PAD_FACE_COLORS := {"A": Color("5E8A5A"), "B": Color("B23A2E"), "X": Color("4F8F9A"), "Y": Color("C9A246")}
## 畫成圖標的搖桿部件（名稱由 InputHints 給）：右搖桿、十字鍵上下、十字鍵左右。
const PAD_RIGHT_STICK := "RS"
const PAD_DPAD_VERTICAL := "DPAD_V"
const PAD_DPAD_HORIZONTAL := "DPAD_H"
const PAD_ICONS: Array[String] = [PAD_RIGHT_STICK, PAD_DPAD_VERTICAL, PAD_DPAD_HORIZONTAL]
const PAD_SHOULDERS: Array[String] = ["LB", "RB", "LT", "RT", "Start", "Select"]

## 紙紋（8.3 `paper`，存的是乘數）；由 hud 在啟動時設定。使用它的 CanvasItem 要開啟 texture_repeat。
static var paper_texture: Texture2D
static var _frame_box: StyleBoxFlat
static var _disc_box: StyleBoxFlat


## 依版面倍率換算字級；放大的版面（Steam Deck）另有下限。
static func font_size(base: int, unit: float) -> int:
	if unit <= 1.0:
		return base
	return maxi(int(round(base * unit)), DECK_MIN_SIZE)


## 裱框紙簽：柔影、綾邊（寬 band）、紙面與紙紋、內緣金線。rect 是紙面的範圍，綾邊畫在它外面。
static func paper_slip(item: CanvasItem, rect: Rect2, band := SLIP_BAND) -> void:
	if rect.size.x < 4.0 or rect.size.y < 4.0:
		return
	if _frame_box == null:
		_frame_box = StyleBoxFlat.new()
		_frame_box.bg_color = SILK
		_frame_box.shadow_color = Color(INK, 0.28)
		_frame_box.shadow_size = 7
		_frame_box.shadow_offset = Vector2(0.0, 2.0)
		_frame_box.anti_aliasing = false
	item.draw_style_box(_frame_box, rect.grow(band))
	_paper(item, rect)
	item.draw_rect(rect.grow(-0.5), Color(GOLD, 0.75), false, 1.0)


## 圓形的裱框紙面（天時儀）：radius 含綾邊。
static func paper_disc(item: CanvasItem, center: Vector2, radius: float, band := SLIP_BAND) -> void:
	if _disc_box == null:
		_disc_box = StyleBoxFlat.new()
		_disc_box.bg_color = SILK
		_disc_box.shadow_color = Color(INK, 0.28)
		_disc_box.shadow_size = 7
		_disc_box.shadow_offset = Vector2(0.0, 2.0)
		_disc_box.corner_detail = 16
	_disc_box.set_corner_radius_all(int(ceil(radius)))
	item.draw_style_box(_disc_box, Rect2(center - Vector2.ONE * radius, Vector2.ONE * radius * 2.0))
	item.draw_circle(center, radius - band, PAPER_LIGHT)
	item.draw_arc(center, radius - band - 0.5, 0.0, TAU, 64, Color(GOLD, 0.75), 1.0, true)


static func _paper(item: CanvasItem, rect: Rect2) -> void:
	if paper_texture == null:
		item.draw_rect(rect, PAPER_LIGHT, true)
	else:
		item.draw_texture_rect_region(paper_texture, rect, Rect2(rect.position, rect.size), PAPER_LIGHT)


## 按鈕的底：實心朱底（啟用中）、紙底綾框（可選）、紙底虛線框（尚未開放）。
## 回傳畫在上面的圖示與文字該用的顏色。
## pressed：按住時整塊往內縮一圈並壓上一層淡墨，像把印按下去。
static func plate(item: CanvasItem, rect: Rect2, active: bool, enabled := true, hover := false, pressed := false) -> Color:
	var inner := rect.grow(-2.5 if pressed and enabled else -1.0)
	if pressed and enabled:
		item.draw_rect(rect.grow(-1.0), Color(INK, 0.22), true)
	if active:
		item.draw_rect(inner, SEAL_RED, true)
		item.draw_rect(inner.grow(-3.0), Color(PAPER_LIGHT, 0.85), false, 1.0)
		return PAPER_LIGHT
	if enabled:
		item.draw_rect(inner, PAPER_LIGHT, true)
		item.draw_rect(inner, INK if hover else SILK, false, 2.0)
		return INK
	item.draw_rect(inner, Color(PAPER_LIGHT, 0.82), true)
	var corners := [inner.position, Vector2(inner.end.x, inner.position.y), inner.end, Vector2(inner.position.x, inner.end.y)]
	for side in 4:
		item.draw_dashed_line(corners[side], corners[(side + 1) % 4], Color(INK_LIGHT, 0.8), 1.0, 4.0)
	return Color(INK_LIGHT, DISABLED_GLYPH_ALPHA)


## 實體鍵的圖像（GDD 5.9）：A／B／X／Y 是帶色圓鈕，LB／RB／LT／RT／Start 是深色膠囊，其餘是鍵盤鍵帽。
## 右搖桿與十字鍵畫成圖標，不寫字。
## pos 為左上角，高度為 KEY_HEIGHT × unit；回傳寬度。label 為空時不畫、回傳 0。
static func key_glyph(item: CanvasItem, pos: Vector2, label: String, unit: float) -> float:
	if label.is_empty():
		return 0.0
	var height := KEY_HEIGHT * unit
	var width := key_glyph_width(label, unit)
	var rect := Rect2(pos, Vector2(width, height))
	var size := font_size(KEY_TEXT_SIZE, unit)
	var extent := text_size(label, size)
	var at := pos + (rect.size - extent) * 0.5
	if label == PAD_RIGHT_STICK:
		# 搖桿：底座圓、搖桿帽，帽上寫 R。
		var center := rect.get_center()
		item.draw_circle(center, height * 0.5, SILK)
		item.draw_circle(center, height * 0.36, PAPER_LIGHT)
		item.draw_arc(center, height * 0.36, 0.0, TAU, 32, INK, 1.0, true)
		var letter := text_size("R", size)
		text(item, center - letter * 0.5, "R", size, INK)
	elif label == PAD_DPAD_VERTICAL or label == PAD_DPAD_HORIZONTAL:
		# 十字鍵：十字形；要按的那兩臂是亮的。
		var center := rect.get_center()
		var arm := height * 0.5
		var thick := height * 0.34
		item.draw_rect(Rect2(center - Vector2(thick * 0.5, arm), Vector2(thick, arm * 2.0)), SILK, true)
		item.draw_rect(Rect2(center - Vector2(arm, thick * 0.5), Vector2(arm * 2.0, thick)), SILK, true)
		var lit := Vector2(0.0, 1.0) if label == PAD_DPAD_VERTICAL else Vector2(1.0, 0.0)
		for side: float in [-1.0, 1.0]:
			var tip := center + lit * side * (arm - thick * 0.5)
			item.draw_rect(Rect2(tip - Vector2.ONE * thick * 0.32, Vector2.ONE * thick * 0.64), PAPER_LIGHT, true)
	elif PAD_FACE_COLORS.has(label):
		item.draw_circle(rect.get_center(), height * 0.5, PAD_FACE_COLORS[label])
		item.draw_arc(rect.get_center(), height * 0.5, 0.0, TAU, 32, INK, 1.0, true)
		text(item, at, label, size, PAPER_LIGHT)
	elif label in PAD_SHOULDERS:
		var box := StyleBoxFlat.new()
		box.bg_color = SILK
		box.set_corner_radius_all(int(height * 0.5))
		item.draw_style_box(box, rect)
		text(item, at, label, size, PAPER_LIGHT)
	else:
		# 鍵帽：紙底墨框，下緣加厚像是有高度的鍵。
		var cap := StyleBoxFlat.new()
		cap.bg_color = PAPER_LIGHT
		cap.border_color = INK
		cap.set_border_width_all(1)
		cap.border_width_bottom = 3
		cap.set_corner_radius_all(3)
		item.draw_style_box(cap, rect)
		text(item, at - Vector2(0.0, 1.0), label, size, INK)
	return width


static func key_glyph_width(label: String, unit: float) -> float:
	if label.is_empty():
		return 0.0
	var height := KEY_HEIGHT * unit
	if PAD_FACE_COLORS.has(label) or label in PAD_ICONS:
		return height
	return maxf(height, text_size(label, font_size(KEY_TEXT_SIZE, unit)).x + 10.0 * unit)


## 圓鈕：朱底（red）或紙底朱圈，中間一個圖示。回傳圖示該用的顏色。
static func round_button(item: CanvasItem, center: Vector2, radius: float, icon: StringName, red: bool, hover := false, pressed := false) -> void:
	if hover or pressed:
		item.draw_circle(center, radius + 3.0, Color(INK, 0.3 if pressed else 0.16))
	if pressed:
		radius *= 0.9
	item.draw_circle(center, radius, SEAL_RED if red else PAPER_LIGHT)
	item.draw_arc(center, radius, 0.0, TAU, 48, SEAL_RED if not red else Color(PAPER_LIGHT, 0.8), 1.5, true)
	InkIcons.draw(item, icon, center, radius * 0.55, PAPER_LIGHT if red else SEAL_RED)


## 墨劃：兩端收細的一筆。
static func stroke(item: CanvasItem, from: Vector2, to: Vector2, width: float, color: Color) -> void:
	var side := (to - from).normalized().orthogonal() * width * 0.5
	var a := from.lerp(to, 0.18)
	var b := from.lerp(to, 0.82)
	item.draw_colored_polygon(PackedVector2Array([from, a + side, b + side, to, b - side, a - side]), color)
	item.draw_polyline(PackedVector2Array([from, a + side, b + side, to, b - side, a - side, from]), color, 1.0, true)


## 墨條（GDD 5.8）：淡墨細線為底，0..1 的值畫成一筆墨劃。
static func ink_bar(item: CanvasItem, rect: Rect2, value: float, color: Color) -> void:
	var y := rect.get_center().y
	item.draw_line(Vector2(rect.position.x, y), Vector2(rect.end.x, y), Color(INK_LIGHT, 0.35), 1.0)
	var length := rect.size.x * clampf(value, 0.0, 1.0)
	if length >= 2.0:
		stroke(item, Vector2(rect.position.x, y), Vector2(rect.position.x + length, y), rect.size.y, color)


## 分節：小墨點加節名（GDD 5.8）。pos 為左上角。
static func section(item: CanvasItem, pos: Vector2, title: String, size: int) -> void:
	var half := size * 0.24
	var center := pos + Vector2(half, size * 0.62)
	item.draw_colored_polygon(
		PackedVector2Array([center + Vector2(0.0, -half), center + Vector2(half, 0.0), center + Vector2(0.0, half), center + Vector2(-half, 0.0)]),
		INK_LIGHT
	)
	text(item, pos + Vector2(half * 2.0 + size * 0.4, 0.0), title, size, INK_LIGHT)


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

