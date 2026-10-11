class_name ProvinceCard
extends Control
## GDD 5.8 州郡卡（命簿）：卷軸式卡片——上下兩根軸頭，軸間是綾邊裱著的紙面；左側一條直書的題簽寫州郡名。顯示治所、
## 地形組成、民生、氣場墨條與三教教化；頁尾的置中鈕把鏡頭移到治所，右上的關閉鈕收卷——兩個鈕都是圖示加文字加實體鍵圖像。
## 值為零的墨條與沒有教化時的教化環不畫，卡片高度跟著內容走。

signal center_requested(map_pos: Vector2)
signal closed

const WIDTH := 380.0
## 軸頭的高度。
const ROD := 12.0
const UNROLL_SECONDS := 0.25
const ROLL_UP_SECONDS := 0.18
## 左側直書題簽的寬度。
const STRIP := 40.0
const CLOSE_RADIUS := 13.0
const ACTION_RADIUS := 17.0
## 低於這個值的氣場視為零，不畫墨條。
const BAR_EPSILON := 0.005
const BARS := [
	["card_aura", "aura", Ink.CYAN],
	["card_karma_qi", "karma_qi", Ink.KARMA],
	["card_merit", "merit", Ink.GOLD],
	["card_righteous", "righteous", Ink.INK],
]
const FAITHS := [["faith_dao", Ink.CYAN], ["faith_shi", Ink.GOLD], ["faith_ru", Ink.INK]]

var province := -1
## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像）。
var gamepad := false:
	set(value):
		gamepad = value
		queue_redraw()

var _world: SimWorld
var _unit := 1.0
var _width := WIDTH
var _info: Dictionary = {}
var _open := 0.0
var _target_open := 0.0
var _hover := ""
## 正按住的鈕（按下的回饋）。
var _pressed := ""


## width 為卡片寬度（像素）；不指定時為 380 乘版面倍率。
func setup(world: SimWorld, unit: float, width := -1.0) -> void:
	_world = world
	_unit = unit
	_width = width if width > 0.0 else WIDTH * unit
	custom_minimum_size = Vector2(_width, ROD * 2.0 * unit)
	size = custom_minimum_size
	mouse_filter = Control.MOUSE_FILTER_STOP
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	visible = false


func show_province(id: int) -> void:
	if id < 0:
		close()
		return
	province = id
	refresh()
	visible = true
	_target_open = 1.0


func close() -> void:
	if province < 0:
		return
	province = -1
	_target_open = 0.0
	closed.emit()


## 重新讀取州郡數值（每月推進後呼叫）。
func refresh() -> void:
	if province >= 0:
		_info = _world.province_info(province)
		custom_minimum_size.y = _content_height() + ROD * 2.0 * _unit
		size = custom_minimum_size
		queue_redraw()


func is_open() -> bool:
	return province >= 0


## 頁尾動作「置中」：把鏡頭移到治所（滑鼠點鈕、鍵盤 C、搖桿 X 共用）。
func request_center() -> void:
	if is_open() and not _info.is_empty():
		center_requested.emit(_info["seat"])


func _process(delta: float) -> void:
	if _open == _target_open:
		return
	_open = move_toward(_open, _target_open, delta / (UNROLL_SECONDS if _target_open > _open else ROLL_UP_SECONDS))
	if _open <= 0.0:
		visible = false
	queue_redraw()


## 要畫的墨條（字串鍵）：靈氣一律顯示，其餘只在值不為零時顯示。
func _bar_keys() -> Array[String]:
	var keys: Array[String] = []
	for entry: Array in BARS:
		if entry[0] == "card_aura" or float(_info[entry[1]]) >= BAR_EPSILON:
			keys.append(entry[0])
	return keys


func _shows_faith_ring() -> bool:
	var faith: Vector3 = _info["faith"]
	return faith.x + faith.y + faith.z >= BAR_EPSILON


## 兩根軸頭之間的高度。
func _content_height() -> float:
	var rows := 14.0 + 34.0 + 12.0 + 28.0 + 26.0 + 32.0 + 26.0 * _bar_keys().size() + 8.0 + 26.0
	if _shows_faith_ring():
		rows += 34.0
	return (rows + 12.0 + ACTION_RADIUS * 2.0 + 14.0) * _unit + Ink.CARD_BAND * 2.0


func _paper_rect(height: float) -> Rect2:
	var inset := Ink.CARD_BAND + 4.0 * _unit
	return Rect2(Vector2(inset, ROD * _unit + Ink.CARD_BAND), Vector2(size.x - inset * 2.0, height - Ink.CARD_BAND * 2.0))


func _close_key() -> String:
	return InputHints.label(InputActions.CANCEL, gamepad)


func _center_key() -> String:
	return InputHints.label(InputActions.CARD_CENTER, gamepad)


## 直書題簽：紙面左側的一條，州郡名由上往下寫（GDD 5.8）。
func _title_strip_rect() -> Rect2:
	var paper := _paper_rect(_content_height())
	return Rect2(paper.position, Vector2(STRIP * _unit, paper.size.y))


## 關閉鈕的外接方框。
func _close_rect() -> Rect2:
	var paper := _paper_rect(_content_height())
	var diameter := CLOSE_RADIUS * 2.0 * _unit
	return Rect2(Vector2(paper.end.x - 12.0 * _unit - diameter, paper.position.y + 12.0 * _unit), Vector2(diameter, diameter))


## 置中鈕的外接方框。
func _center_rect() -> Rect2:
	var paper := _paper_rect(_content_height())
	var diameter := ACTION_RADIUS * 2.0 * _unit
	return Rect2(Vector2(paper.position.x + (STRIP + 14.0) * _unit, paper.end.y - 14.0 * _unit - diameter), Vector2(diameter, diameter))


func _draw() -> void:
	if _info.is_empty():
		return
	var eased := smoothstep(0.0, 1.0, _open)
	var full := _content_height()
	var body_height := full * eased
	var rod := ROD * _unit
	# 紙面由上軸頭往下拉開；下軸頭跟著紙的下緣走。
	if body_height > Ink.CARD_BAND * 2.0 + 4.0:
		Ink.paper_slip(self, _paper_rect(body_height), Ink.CARD_BAND)
	_draw_rod(0.0)
	_draw_rod(rod + body_height)
	if eased < 0.98:
		return

	var paper := _paper_rect(full)
	var x := paper.position.x + (STRIP + 14.0) * _unit
	var right := paper.end.x - 16.0 * _unit
	var y := paper.position.y + 14.0 * _unit
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	var section := Ink.font_size(Ink.SIZE_SECTION, _unit)
	var title := Ink.font_size(Ink.SIZE_TITLE, _unit)

	# 直書題簽：紙面左側較深的一條，州郡名由上往下寫。
	var strip := _title_strip_rect()
	draw_rect(strip, Color(Ink.PAPER_DARK, 0.55), true)
	draw_line(Vector2(strip.end.x, strip.position.y), strip.end, Color(Ink.INK_LIGHT, 0.6), 1.0)
	Ink.vertical_text(self, Vector2(strip.get_center().x, strip.position.y + 14.0 * _unit), _info["name"], title)
	# 首列：治所與治所名；下方一道墨劃。
	var seat_label := Locale.t("card_seat")
	Ink.text(self, Vector2(x, y + (body - note) * 0.6), seat_label, note, Ink.INK_LIGHT)
	Ink.text(self, Vector2(x + Ink.text_size(seat_label, note).x + 8.0 * _unit, y), _info["seat_name"], body)
	var close := _close_rect()
	Ink.round_button(self, close.get_center(), close.size.x * 0.5, &"close", true, _hover == "close", _pressed == "close")
	# 關閉鈕左邊：文字與實體鍵圖像。
	var close_label := Locale.t("card_close")
	var close_x := close.position.x - 6.0 * _unit - Ink.text_size(close_label, note).x
	Ink.text(self, Vector2(close_x, close.position.y + (close.size.y - Ink.text_size(close_label, note).y) * 0.5), close_label, note, Ink.INK_LIGHT)
	var close_key := _close_key()
	Ink.key_glyph(self, Vector2(close_x - 6.0 * _unit - Ink.key_glyph_width(close_key, _unit), close.position.y + (close.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), close_key, _unit)
	y += 34.0 * _unit
	Ink.stroke(self, Vector2(x, y), Vector2(right, y), 2.0 * _unit, Color(Ink.INK, 0.7))
	y += 12.0 * _unit

	# 地形組成：佔比最高的三種陸地地形，各一個色塊加百分比。
	var mix: PackedInt32Array = _info["terrain_mix"]
	var land := 0
	for code in range(2, mix.size()):
		land += mix[code]
	var order := range(2, mix.size())
	order.sort_custom(func(a: int, b: int) -> bool: return mix[a] > mix[b])
	var cursor := x
	for i in mini(3, order.size()):
		var code: int = order[i]
		if land == 0 or mix[code] == 0:
			break
		var key := TerrainSwatches.KEYS[code]
		TerrainSwatches.chip(self, Rect2(Vector2(cursor, y + (note - 12.0 * _unit) * 0.5 + 2.0), Vector2(12.0, 12.0) * _unit), key)
		var label := "%s %d%%" % [Locale.t(key), int(round(100.0 * mix[code] / land))]
		Ink.text(self, Vector2(cursor + 17.0 * _unit, y), label, note)
		cursor += 17.0 * _unit + Ink.text_size(label, note).x + 14.0 * _unit
	y += 28.0 * _unit

	Ink.section(self, Vector2(x, y), Locale.t("card_people"), section)
	y += 26.0 * _unit
	Ink.text(self, Vector2(x, y), "%s %s" % [Locale.t("card_population"), Locale.count(_info["population"])], body)
	Ink.text(self, Vector2(x + (right - x) * 0.5, y), "%s %s" % [Locale.t("card_capacity"), Locale.count(_info["capacity"])], body)
	y += 32.0 * _unit

	# 氣場墨條：數字寫在條的右端。
	var shown := _bar_keys()
	var label_width := 56.0 * _unit
	var number_width := Ink.text_size("100%", note).x + 8.0 * _unit
	for entry: Array in BARS:
		if not shown.has(entry[0]):
			continue
		var value := float(_info[entry[1]])
		Ink.text(self, Vector2(x, y), Locale.t(entry[0]), body)
		var bar := Rect2(Vector2(x + label_width, y + body * 0.5 - 1.0), Vector2(right - x - label_width - number_width, 6.0 * _unit))
		Ink.ink_bar(self, bar, value, entry[2])
		var number := "%d%%" % int(round(clampf(value, 0.0, 1.0) * 100.0))
		Ink.text(self, Vector2(right - Ink.text_size(number, note).x, y + (body - note) * 0.6), number, note, Ink.INK_LIGHT)
		y += 26.0 * _unit
	y += 8.0 * _unit

	# 教化：道、釋、儒三段弧；字用墨色，顏色只在色塊與弧上。
	Ink.section(self, Vector2(x, y), Locale.t("card_faith"), section)
	if _shows_faith_ring():
		var faith: Vector3 = _info["faith"]
		var total := faith.x + faith.y + faith.z
		var ring_radius := 18.0 * _unit
		var ring_center := Vector2(right - ring_radius - 4.0 * _unit, y + 30.0 * _unit)
		var start := -PI * 0.5
		draw_arc(ring_center, ring_radius, 0.0, TAU, 48, Color(Ink.INK_LIGHT, 0.2), 6.0 * _unit, true)
		cursor = x
		y += 28.0 * _unit
		for i in 3:
			var sweep: float = TAU * faith[i] / total
			if sweep > 0.01:
				draw_arc(ring_center, ring_radius, start, start + sweep, 32, FAITHS[i][1], 6.0 * _unit, true)
			start += sweep
			draw_rect(Rect2(Vector2(cursor, y + (note - 10.0 * _unit) * 0.5 + 2.0), Vector2(10.0, 10.0) * _unit), FAITHS[i][1], true)
			var label := "%s %d%%" % [Locale.t(FAITHS[i][0]), int(round(100.0 * faith[i] / total))]
			Ink.text(self, Vector2(cursor + 15.0 * _unit, y), label, note)
			cursor += 15.0 * _unit + Ink.text_size(label, note).x + 14.0 * _unit
	else:
		var heading := Ink.text_size(Locale.t("card_faith"), section).x + section * 1.1
		Ink.text(self, Vector2(x + heading + 10.0 * _unit, y + (section - note) * 0.6), Locale.t("card_faith_none"), note, Ink.INK_LIGHT)

	# 頁尾動作印。
	var action := _center_rect()
	Ink.round_button(self, action.get_center(), action.size.x * 0.5, &"center", false, _hover == "center", _pressed == "center")
	var action_label := Locale.t("act_center")
	var action_x := action.end.x + 8.0 * _unit
	Ink.text(self, Vector2(action_x, action.position.y + (action.size.y - Ink.text_size(action_label, body).y) * 0.5), action_label, body)
	Ink.key_glyph(self, Vector2(action_x + Ink.text_size(action_label, body).x + 8.0 * _unit, action.position.y + (action.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), _center_key(), _unit)


## 軸頭：墨色圓柱，兩端略粗。
func _draw_rod(y: float) -> void:
	var rod := ROD * _unit
	var cap := 10.0 * _unit
	draw_rect(Rect2(Vector2(cap * 0.5, y + rod * 0.2), Vector2(size.x - cap, rod * 0.6)), Ink.INK, true)
	draw_rect(Rect2(Vector2(cap * 0.5, y + rod * 0.3), Vector2(size.x - cap, 1.0)), Color(Ink.INK_LIGHT, 0.9), true)
	for cap_x: float in [0.0, size.x - cap]:
		draw_rect(Rect2(Vector2(cap_x, y), Vector2(cap, rod)), Ink.INK, true)
		draw_rect(Rect2(Vector2(cap_x, y + rod * 0.5 - 1.0), Vector2(cap, 2.0)), Color(Ink.GOLD, 0.8), true)


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var hover: String = _target_at((event as InputEventMouseMotion).position)
		if hover != _hover:
			_hover = hover
			queue_redraw()
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		_pressed = _target_at((event as InputEventMouseButton).position)
		queue_redraw()
		match _pressed:
			"close":
				close()
			"center":
				request_center()
		accept_event()
	elif event is InputEventMouseButton and not event.pressed and not _pressed.is_empty():
		_pressed = ""
		queue_redraw()


func _target_at(pos: Vector2) -> String:
	var close := _close_rect()
	if pos.distance_to(close.get_center()) <= close.size.x * 0.5:
		return "close"
	var action := _center_rect()
	if pos.distance_to(action.get_center()) <= action.size.x * 0.5:
		return "center"
	return ""
