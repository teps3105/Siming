class_name TimeDial
extends Control
## GDD 5.7 天時儀與題簽：左上圓盤（十二月刻度、朱針、盤心「行」／「止」圓印），
## 盤右是寫著日期的題簽與「疾徐」四筆（速度）。支援的決定：現在該停還是該快。

const DISC_RADIUS := 44.0
const SEAL_RADIUS := 15.0
const STROKE_COUNT := 4

var _sim: SimHost
var _unit := 1.0
var _hover_stroke := -1
var _hover_seal := false


func setup(sim: SimHost, unit: float) -> void:
	_sim = sim
	_unit = unit
	custom_minimum_size = Vector2(330.0, 104.0) * unit
	size = custom_minimum_size
	mouse_filter = Control.MOUSE_FILTER_STOP
	sim.advanced.connect(queue_redraw.unbind(1))
	sim.time_state_changed.connect(queue_redraw)


func _disc_center() -> Vector2:
	return Vector2(52.0, 52.0) * _unit


func _stroke_rect(index: int) -> Rect2:
	var origin := Vector2(112.0, 62.0) * _unit
	var width := (22.0 + 9.0 * index) * _unit
	var offset := 0.0
	for i in index:
		offset += (22.0 + 9.0 * i + 8.0) * _unit
	return Rect2(origin + Vector2(offset, 0.0), Vector2(width, 22.0 * _unit))


func _draw() -> void:
	if _sim == null:
		return
	var center := _disc_center()
	var radius := DISC_RADIUS * _unit
	# 盤：柔影、紙面、墨圈。
	for i in 4:
		draw_circle(center + Vector2(0.0, 2.0), radius + float(4 - i) * 1.5, Color(Ink.INK, 0.04))
	draw_circle(center, radius, Ink.PAPER_LIGHT)
	draw_arc(center, radius, 0.0, TAU, 64, Ink.INK_LIGHT, 1.5, true)
	draw_arc(center, radius - 5.0 * _unit, 0.0, TAU, 64, Color(Ink.INK_LIGHT, 0.4), 1.0, true)
	# 十二月刻度：正月在正上方，順時針。
	for month in 12:
		var dir := Vector2.UP.rotated(TAU * month / 12.0)
		var long := month % 3 == 0
		draw_line(center + dir * (radius - (12.0 if long else 8.0) * _unit), center + dir * (radius - 5.0 * _unit), Ink.INK if long else Ink.INK_LIGHT, 1.5 if long else 1.0, true)
	# 朱針指向本月。
	var month_index := int((maxi(_sim.world.tick(), 1) - 1) % 12)
	var needle := Vector2.UP.rotated(TAU * month_index / 12.0)
	draw_line(center + needle * (SEAL_RADIUS + 3.0) * _unit, center + needle * (radius - 9.0 * _unit), Ink.SEAL_RED, 2.5, true)
	# 盤心圓印：行／止。
	var stopped := _sim.paused
	Ink.round_seal(self, center, (SEAL_RADIUS + (1.5 if _hover_seal else 0.0)) * _unit, Locale.t("dial_stop") if stopped else Locale.t("dial_run"), stopped)

	# 題簽：日期。
	var date := _sim.world.date_short()
	var date_size := int(Ink.SIZE_BODY * _unit)
	var slip := Rect2(Vector2(104.0, 18.0) * _unit, Vector2(Ink.text_size(date, date_size).x + 24.0 * _unit, 34.0 * _unit))
	Ink.paper_slip(self, slip, 0.0, 7)
	Ink.text(self, slip.position + Vector2(12.0 * _unit, (slip.size.y - Ink.text_size(date, date_size).y) * 0.5), date, date_size)

	# 疾徐四筆：由短到長，已達速度者濃墨。
	for i in STROKE_COUNT:
		var rect := _stroke_rect(i)
		var on := i <= _sim.speed_index and not stopped
		var color := Ink.INK if on else Color(Ink.INK_LIGHT, 0.45)
		if i == _hover_stroke:
			color = Ink.SEAL_RED
		var y := rect.position.y + rect.size.y * 0.5
		var from := Vector2(rect.position.x, y + 2.0 * _unit)
		var to := Vector2(rect.end.x, y - 2.0 * _unit)
		draw_line(from, to, color, 4.0 * _unit, true)
		draw_circle(from, 2.5 * _unit, color)


func _gui_input(event: InputEvent) -> void:
	if _sim == null:
		return
	if event is InputEventMouseMotion:
		var on_seal: bool = (event as InputEventMouseMotion).position.distance_to(_disc_center()) <= DISC_RADIUS * _unit
		var on_stroke: int = _stroke_at((event as InputEventMouseMotion).position)
		if on_seal != _hover_seal or on_stroke != _hover_stroke:
			_hover_seal = on_seal
			_hover_stroke = on_stroke
			queue_redraw()
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var stroke: int = _stroke_at((event as InputEventMouseButton).position)
		if stroke >= 0:
			_sim.set_speed(stroke)
			_sim.set_paused(false)
			accept_event()
		elif (event as InputEventMouseButton).position.distance_to(_disc_center()) <= DISC_RADIUS * _unit:
			_sim.toggle_paused()
			accept_event()


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and (_hover_seal or _hover_stroke >= 0):
		_hover_seal = false
		_hover_stroke = -1
		queue_redraw()


## 滑鼠只在圓盤與四筆上才攔截，其餘讓地圖收到事件。
func _has_point(point: Vector2) -> bool:
	return point.distance_to(_disc_center()) <= DISC_RADIUS * _unit or _stroke_at(point) >= 0


func _stroke_at(pos: Vector2) -> int:
	for i in STROKE_COUNT:
		if _stroke_rect(i).grow(3.0).has_point(pos):
			return i
	return -1
