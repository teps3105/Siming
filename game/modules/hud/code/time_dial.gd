class_name TimeDial
extends Control
## GDD 5.7 天時儀與題簽：左上圓盤（十二月刻度、朱針、盤心「行」／「止」圓印），
## 盤右是寫著日期的題簽，題簽下是畫著「疾徐」四筆（速度）的疾徐簽。支援的決定：現在該停還是該快。

## 圓盤半徑含綾邊（直徑 56）；盤心印直徑 30。
const DISC_RADIUS := 28.0
const SEAL_RADIUS := 15.0
const STROKE_COUNT := 4
const TITLE_HEIGHT := 30.0
const SPEED_HEIGHT := 14.0
## 疾徐四筆：最短一筆的長度、每筆加長多少、筆距。
const STROKE_BASE := 9.0
const STROKE_STEP := 3.0
const STROKE_GAP := 4.0
const SPEED_PAD := 6.0

var _sim: SimHost
var _unit := 1.0
var _hover_stroke := -1
var _hover_seal := false


func setup(sim: SimHost, unit: float) -> void:
	_sim = sim
	_unit = unit
	custom_minimum_size = Vector2(300.0, DISC_RADIUS * 2.0 + 8.0) * unit
	size = custom_minimum_size
	mouse_filter = Control.MOUSE_FILTER_STOP
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	sim.advanced.connect(queue_redraw.unbind(1))
	sim.time_state_changed.connect(queue_redraw)


func _disc_radius() -> float:
	return DISC_RADIUS * _unit


func _disc_center() -> Vector2:
	return Vector2.ONE * _disc_radius()


## 題簽的紙面（不含綾邊）。
func _title_rect() -> Rect2:
	var date_size := Ink.font_size(Ink.SIZE_BODY, _unit)
	var width := Ink.text_size(_sim.world.date_short(), date_size).x + 24.0 * _unit
	return Rect2(Vector2(_disc_radius() * 2.0 + 8.0 * _unit + Ink.SLIP_BAND, Ink.SLIP_BAND), Vector2(width, TITLE_HEIGHT * _unit))


## 疾徐簽的紙面：題簽正下方。
func _speed_rect() -> Rect2:
	var strokes := 0.0
	for i in STROKE_COUNT:
		strokes += STROKE_BASE + STROKE_STEP * i
	var width := (strokes + STROKE_GAP * (STROKE_COUNT - 1) + SPEED_PAD * 2.0) * _unit
	var title := _title_rect()
	return Rect2(Vector2(title.position.x, title.end.y + Ink.SLIP_BAND * 2.0 + 4.0 * _unit), Vector2(width, SPEED_HEIGHT * _unit))


func _stroke_rect(index: int) -> Rect2:
	var slip := _speed_rect()
	var offset := SPEED_PAD
	for i in index:
		offset += STROKE_BASE + STROKE_STEP * i + STROKE_GAP
	return Rect2(slip.position + Vector2(offset * _unit, 0.0), Vector2((STROKE_BASE + STROKE_STEP * index) * _unit, slip.size.y))


func _draw() -> void:
	if _sim == null:
		return
	var center := _disc_center()
	var radius := _disc_radius()
	var face := radius - Ink.SLIP_BAND
	Ink.paper_disc(self, center, radius)
	# 十二月刻度：正月在正上方，順時針。
	for month in 12:
		var dir := Vector2.UP.rotated(TAU * month / 12.0)
		var long := month % 3 == 0
		draw_line(center + dir * (face - (6.0 if long else 4.0) * _unit), center + dir * (face - 1.5 * _unit), Ink.INK if long else Ink.INK_LIGHT, 1.5 if long else 1.0, true)
	# 盤心圓印：行／止。
	var stopped := _sim.paused
	Ink.round_seal(self, center, SEAL_RADIUS * _unit, Locale.t("dial_stop") if stopped else Locale.t("dial_run"), stopped, _hover_seal)
	# 朱針指向本月。
	var month_index := int((maxi(_sim.world.tick(), 1) - 1) % 12)
	var needle := Vector2.UP.rotated(TAU * month_index / 12.0)
	draw_line(center + needle * (SEAL_RADIUS + 1.0) * _unit, center + needle * (face - 1.0 * _unit), Ink.SEAL_RED, 2.5, true)

	# 題簽：日期。
	var date := _sim.world.date_short()
	var date_size := Ink.font_size(Ink.SIZE_BODY, _unit)
	var slip := _title_rect()
	Ink.paper_slip(self, slip)
	Ink.text(self, slip.position + Vector2(12.0 * _unit, (slip.size.y - Ink.text_size(date, date_size).y) * 0.5), date, date_size)

	# 疾徐簽：四筆由短到長，已達速度者濃墨；暫停時四筆都是淡墨。
	Ink.paper_slip(self, _speed_rect())
	for i in STROKE_COUNT:
		var rect := _stroke_rect(i)
		var on := i <= _sim.speed_index and not stopped
		var color := Ink.INK if on else Color(Ink.INK_LIGHT, 0.5)
		if i == _hover_stroke:
			color = Ink.SEAL_RED
		var y := rect.get_center().y
		Ink.stroke(self, Vector2(rect.position.x, y + 1.5 * _unit), Vector2(rect.end.x, y - 1.5 * _unit), 3.5 * _unit, color)


func _gui_input(event: InputEvent) -> void:
	if _sim == null:
		return
	if event is InputEventMouseMotion:
		var on_seal: bool = (event as InputEventMouseMotion).position.distance_to(_disc_center()) <= _disc_radius()
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
		elif (event as InputEventMouseButton).position.distance_to(_disc_center()) <= _disc_radius():
			_sim.toggle_paused()
			accept_event()


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and (_hover_seal or _hover_stroke >= 0):
		_hover_seal = false
		_hover_stroke = -1
		queue_redraw()


## 滑鼠只在圓盤與疾徐簽上才攔截，其餘讓地圖收到事件。
func _has_point(point: Vector2) -> bool:
	return point.distance_to(_disc_center()) <= _disc_radius() or _speed_rect().grow(Ink.SLIP_BAND).has_point(point)


## 疾徐簽上的位置屬於哪一筆：每筆連同右邊的筆距，整張簽都點得到。
func _stroke_at(pos: Vector2) -> int:
	if not _speed_rect().grow(Ink.SLIP_BAND).has_point(pos):
		return -1
	for i in STROKE_COUNT:
		if pos.x <= _stroke_rect(i).end.x + STROKE_GAP * 0.5 * _unit:
			return i
	return STROKE_COUNT - 1
