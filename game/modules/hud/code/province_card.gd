class_name ProvinceCard
extends Control
## GDD 5.8 州郡卡（命簿）：右側卷軸式卡片，顯示選取州郡的名稱、民生、四條氣場橫條與三教教化；
## 頁尾動作印「中」把鏡頭移到治所，右上「閉」印收卷。開關以展卷動畫呈現。

signal center_requested(map_pos: Vector2)
signal closed

const WIDTH := 300.0
const HEIGHT := 348.0
const ROD := 12.0
const UNROLL_SECONDS := 0.18

var province := -1

var _world: SimWorld
var _unit := 1.0
var _info: Dictionary = {}
var _open := 0.0
var _target_open := 0.0
var _hover := ""


func setup(world: SimWorld, unit: float) -> void:
	_world = world
	_unit = unit
	custom_minimum_size = Vector2(WIDTH, HEIGHT) * unit
	size = custom_minimum_size
	mouse_filter = Control.MOUSE_FILTER_STOP
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
		queue_redraw()


func is_open() -> bool:
	return province >= 0


func _process(delta: float) -> void:
	if _open == _target_open:
		return
	_open = move_toward(_open, _target_open, delta / UNROLL_SECONDS)
	if _open <= 0.0:
		visible = false
	queue_redraw()


func _close_rect() -> Rect2:
	return Rect2(Vector2(size.x - 44.0 * _unit, (ROD + 10.0) * _unit), Vector2(28.0, 28.0) * _unit)


func _center_rect() -> Rect2:
	return Rect2(Vector2(20.0 * _unit, size.y - (ROD + 46.0) * _unit), Vector2(32.0, 32.0) * _unit)


func _draw() -> void:
	if _info.is_empty():
		return
	var eased := smoothstep(0.0, 1.0, _open)
	var body_height := (size.y - ROD * 2.0 * _unit) * eased
	var rod := ROD * _unit
	# 上下軸頭（裱綾色）與紙面；展卷時紙面由上往下拉開。
	var paper := Rect2(Vector2(6.0 * _unit, rod), Vector2(size.x - 12.0 * _unit, body_height))
	Ink.paper_slip(self, paper, 2.0, 23)
	for y in [0.0, rod + body_height]:
		var bar := Rect2(Vector2(0.0, y), Vector2(size.x, rod))
		draw_rect(bar, Ink.SILK, true)
		draw_rect(Rect2(bar.position + Vector2(0.0, rod * 0.5 - 1.0), Vector2(size.x, 2.0)), Color(Ink.GOLD, 0.6), true)
	if eased < 0.98:
		return

	var x := 22.0 * _unit
	var y := rod + 14.0 * _unit
	var note := int(Ink.SIZE_NOTE * _unit)
	var body := int(Ink.SIZE_BODY * _unit)
	Ink.text(self, Vector2(x, y), _info["name"], int(Ink.SIZE_TITLE * _unit))
	var city := Locale.t("city_level_%d" % int(_info["city_level"]))
	var title_width := Ink.text_size(_info["name"], int(Ink.SIZE_TITLE * _unit)).x
	Ink.text(self, Vector2(x + title_width + 10.0 * _unit, y + 8.0 * _unit), city, note, Ink.INK_LIGHT)
	Ink.seal(self, _close_rect(), Locale.t("card_close_seal"), false, true, _hover == "close")
	y += 40.0 * _unit
	draw_line(Vector2(x, y), Vector2(size.x - x, y), Color(Ink.INK_LIGHT, 0.5), 1.0)
	y += 10.0 * _unit

	Ink.text(self, Vector2(x, y), Locale.t("card_people"), int(Ink.SIZE_SECTION * _unit), Ink.INK_LIGHT)
	y += 24.0 * _unit
	Ink.text(self, Vector2(x, y), "%s %s" % [Locale.t("card_population"), Locale.count(_info["population"])], body)
	Ink.text(self, Vector2(x + 140.0 * _unit, y), "%s %s" % [Locale.t("card_capacity"), Locale.count(_info["capacity"])], body)
	y += 32.0 * _unit

	var bars := [
		["card_aura", _info["aura"], Ink.CYAN],
		["card_karma_qi", _info["karma_qi"], Ink.KARMA],
		["card_merit", _info["merit"], Ink.GOLD],
		["card_righteous", _info["righteous"], Ink.INK],
	]
	for entry: Array in bars:
		Ink.text(self, Vector2(x, y), Locale.t(entry[0]), body)
		Ink.bar(self, Rect2(Vector2(x + 52.0 * _unit, y + 8.0 * _unit), Vector2(size.x - x * 2.0 - 52.0 * _unit, 6.0 * _unit)), entry[1], entry[2])
		y += 26.0 * _unit
	y += 6.0 * _unit

	# 教化：道、釋、儒三段弧。
	Ink.text(self, Vector2(x, y), Locale.t("card_faith"), int(Ink.SIZE_SECTION * _unit), Ink.INK_LIGHT)
	var faith: Vector3 = _info["faith"]
	var total := maxf(faith.x + faith.y + faith.z, 1e-4)
	var ring_center := Vector2(size.x - 60.0 * _unit, y + 26.0 * _unit)
	var start := -PI * 0.5
	var colors := [Ink.CYAN, Ink.GOLD, Ink.INK]
	var names := ["faith_dao", "faith_shi", "faith_ru"]
	draw_arc(ring_center, 20.0 * _unit, 0.0, TAU, 48, Color(Ink.INK_LIGHT, 0.2), 7.0 * _unit, true)
	for i in 3:
		var sweep: float = TAU * faith[i] / total
		if sweep > 0.01:
			draw_arc(ring_center, 20.0 * _unit, start, start + sweep, 32, colors[i], 7.0 * _unit, true)
		start += sweep
		Ink.text(self, Vector2(x + i * 56.0 * _unit, y + 22.0 * _unit), "%s %d%%" % [Locale.t(names[i]), int(round(100.0 * faith[i] / total))], note, colors[i])

	# 頁尾動作印。
	Ink.seal(self, _center_rect(), Locale.t("act_center_seal"), false, true, _hover == "center")
	Ink.text(self, _center_rect().position + Vector2(40.0 * _unit, 8.0 * _unit), Locale.t("act_center"), note, Ink.INK_LIGHT)


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var hover: String = _target_at((event as InputEventMouseMotion).position)
		if hover != _hover:
			_hover = hover
			queue_redraw()
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		match _target_at((event as InputEventMouseButton).position):
			"close":
				close()
			"center":
				center_requested.emit(_info["seat"])
		accept_event()


func _target_at(pos: Vector2) -> String:
	if _close_rect().has_point(pos):
		return "close"
	if _center_rect().has_point(pos):
		return "center"
	return ""
