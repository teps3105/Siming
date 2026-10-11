class_name TimeDial
extends Control
## GDD 5.7 天時儀：左上圓盤（十二月刻度、朱針、盤心的行止鈕），盤右是寫著日期的題簽，
## 題簽下是時速簽——「運行中／已暫停」加暫停鍵、疾徐四筆加「幾倍速」與調速鍵。
## 每個可操作的地方都有圖像、文字與實體鍵圖像（5.7 原則 7）。支援的決定：現在該停還是該快。

## 圓盤半徑含綾邊（直徑 56）；盤心鈕直徑 30。
const DISC_RADIUS := 28.0
const SEAL_RADIUS := 15.0
const STROKE_COUNT := 4
const TITLE_HEIGHT := 30.0
const SPEED_HEIGHT := 24.0
## 疾徐四筆：最短一筆的長度、每筆加長多少、筆距。
const STROKE_BASE := 9.0
const STROKE_STEP := 3.0
const STROKE_GAP := 4.0
const PAD := 8.0
const PRESSED_PAUSE := 100
## 兩個調速鍵之間的間距。
const KEY_GAP := 10.0

## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像）。
var gamepad := false:
	set(value):
		gamepad = value
		queue_redraw()

var _sim: SimHost
var _unit := 1.0
var _hover_stroke := -1
var _hover_seal := false
## 正按住的地方（按下的回饋）：-1 沒有、0–3 疾徐的某一筆、PRESSED_PAUSE 行止鈕。
var _pressed := -1


func setup(sim: SimHost, unit: float) -> void:
	_sim = sim
	_unit = unit
	custom_minimum_size = Vector2(560.0, DISC_RADIUS * 2.0 + 16.0) * unit
	size = custom_minimum_size
	mouse_filter = Control.MOUSE_FILTER_STOP
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	sim.advanced.connect(queue_redraw.unbind(1))
	sim.time_state_changed.connect(queue_redraw)


func _disc_radius() -> float:
	return DISC_RADIUS * _unit


func _disc_center() -> Vector2:
	return Vector2.ONE * _disc_radius()


func _state_label() -> String:
	return Locale.t("dial_paused") if _sim.paused else Locale.t("dial_running")


func _speed_label() -> String:
	return "%d %s" % [_sim.speed_multiplier(), Locale.t("dial_speed_unit")]


func _pause_key() -> String:
	return InputHints.label(InputActions.PAUSE, gamepad)


## 調速的鍵：鍵盤是 Tab、搖桿是 Y，每按一次切到下一檔。
func _speed_keys() -> Array[String]:
	return [InputHints.label(InputActions.SPEED_CYCLE, gamepad)]


## 題簽的紙面（不含綾邊）。
func _title_rect() -> Rect2:
	var date_size := Ink.font_size(Ink.SIZE_BODY, _unit)
	var width := Ink.text_size(_sim.world.date_short(), date_size).x + 24.0 * _unit
	return Rect2(Vector2(_disc_radius() * 2.0 + 8.0 * _unit + Ink.SLIP_BAND, Ink.SLIP_BAND), Vector2(width, TITLE_HEIGHT * _unit))


func _strokes_width() -> float:
	var strokes := 0.0
	for i in STROKE_COUNT:
		strokes += STROKE_BASE + STROKE_STEP * i
	return (strokes + STROKE_GAP * (STROKE_COUNT - 1)) * _unit


## 時速簽各段的 x 座標：狀態字、暫停鍵、四筆、倍速字、調速鍵、右緣。
func _columns() -> Dictionary:
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var x := _title_rect().position.x + PAD * _unit
	var out := {"state": x}
	x += maxf(Ink.text_size(Locale.t("dial_paused"), note).x, Ink.text_size(Locale.t("dial_running"), note).x) + 6.0 * _unit
	out["pause_key"] = x
	x += Ink.key_glyph_width(_pause_key(), _unit) + 14.0 * _unit
	out["strokes"] = x
	x += _strokes_width() + 8.0 * _unit
	out["speed"] = x
	x += Ink.text_size("%d %s" % [SimHost.SPEED_MULTIPLIERS[-1], Locale.t("dial_speed_unit")], note).x + 6.0 * _unit
	out["speed_keys"] = x
	var keys := _speed_keys()
	x += Ink.key_glyph_width(keys[0], _unit) + PAD * _unit
	out["end"] = x
	return out


## 時速簽的紙面：題簽正下方。
func _speed_rect() -> Rect2:
	var title := _title_rect()
	return Rect2(Vector2(title.position.x, title.end.y + Ink.SLIP_BAND * 2.0 + 4.0 * _unit), Vector2(_columns()["end"] - title.position.x, SPEED_HEIGHT * _unit))


func _stroke_rect(index: int) -> Rect2:
	var slip := _speed_rect()
	var offset := 0.0
	for i in index:
		offset += STROKE_BASE + STROKE_STEP * i + STROKE_GAP
	return Rect2(Vector2(_columns()["strokes"] + offset * _unit, slip.position.y), Vector2((STROKE_BASE + STROKE_STEP * index) * _unit, slip.size.y))


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
	# 盤心行止鈕：暫停中是朱底的暫停圖示，運行中是紙底的播放圖示。
	var stopped := _sim.paused
	Ink.round_button(self, center, SEAL_RADIUS * _unit, &"pause" if stopped else &"play", stopped, _hover_seal, _pressed == PRESSED_PAUSE)
	# 朱針指向本月。
	var month_index := int((maxi(_sim.world.tick(), 1) - 1) % 12)
	var needle := Vector2.UP.rotated(TAU * month_index / 12.0)
	draw_line(center + needle * (SEAL_RADIUS + 1.0) * _unit, center + needle * (face - 1.0 * _unit), Ink.SEAL_RED, 2.5, true)

	# 題簽：日期。
	var date := _sim.world.date_short()
	var date_size := Ink.font_size(Ink.SIZE_BODY, _unit)
	var title := _title_rect()
	Ink.paper_slip(self, title)
	Ink.text(self, title.position + Vector2(12.0 * _unit, (title.size.y - Ink.text_size(date, date_size).y) * 0.5), date, date_size)

	# 時速簽：狀態與暫停鍵；四筆由短到長，已達速度者濃墨；倍速與調速鍵。
	var slip := _speed_rect()
	var columns := _columns()
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var text_y := slip.position.y + (slip.size.y - Ink.text_size(_state_label(), note).y) * 0.5
	var key_y := slip.position.y + (slip.size.y - Ink.KEY_HEIGHT * _unit) * 0.5
	Ink.paper_slip(self, slip)
	Ink.text(self, Vector2(columns["state"], text_y), _state_label(), note, Ink.SEAL_RED if stopped else Ink.INK)
	Ink.key_glyph(self, Vector2(columns["pause_key"], key_y), _pause_key(), _unit)
	var divider: float = columns["strokes"] - 7.0 * _unit
	draw_line(Vector2(divider, slip.position.y + 4.0), Vector2(divider, slip.end.y - 4.0), Color(Ink.INK_LIGHT, 0.4), 1.0)
	for i in STROKE_COUNT:
		var rect := _stroke_rect(i)
		var on := i <= _sim.speed_index and not stopped
		var color := Ink.INK if on else Color(Ink.INK_LIGHT, 0.5)
		if i == _hover_stroke:
			color = Ink.SEAL_RED
		if i == _pressed:
			draw_rect(rect.grow(2.0), Color(Ink.INK, 0.18), true)
		var y := rect.get_center().y
		Ink.stroke(self, Vector2(rect.position.x, y + 1.5 * _unit), Vector2(rect.end.x, y - 1.5 * _unit), 3.5 * _unit, color)
	Ink.text(self, Vector2(columns["speed"], text_y), _speed_label(), note)
	var keys := _speed_keys()
	Ink.key_glyph(self, Vector2(columns["speed_keys"], key_y), keys[0], _unit)


func _gui_input(event: InputEvent) -> void:
	if _sim == null:
		return
	if event is InputEventMouseMotion:
		var on_seal: bool = _on_pause((event as InputEventMouseMotion).position)
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
			_pressed = stroke
			accept_event()
		elif _on_pause((event as InputEventMouseButton).position):
			_sim.toggle_paused()
			_pressed = PRESSED_PAUSE
			accept_event()
		queue_redraw()
	elif event is InputEventMouseButton and not event.pressed and _pressed >= 0:
		_pressed = -1
		queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and (_hover_seal or _hover_stroke >= 0 or _pressed >= 0):
		_pressed = -1
		_hover_seal = false
		_hover_stroke = -1
		queue_redraw()


## 滑鼠只在圓盤與時速簽上才攔截，其餘讓地圖收到事件。
func _has_point(point: Vector2) -> bool:
	return point.distance_to(_disc_center()) <= _disc_radius() or _speed_rect().grow(Ink.SLIP_BAND).has_point(point)


## 圓盤，或時速簽上狀態字與暫停鍵那一段：點了切換行止。
func _on_pause(pos: Vector2) -> bool:
	if pos.distance_to(_disc_center()) <= _disc_radius():
		return true
	return _speed_rect().grow(Ink.SLIP_BAND).has_point(pos) and pos.x < _columns()["strokes"] - 7.0 * _unit


## 時速簽上四筆與倍速字那一段屬於哪一筆：每筆連同右邊的筆距都點得到。
func _stroke_at(pos: Vector2) -> int:
	if not _speed_rect().grow(Ink.SLIP_BAND).has_point(pos) or _on_pause(pos):
		return -1
	if pos.x > _columns()["strokes"] + _strokes_width() + 4.0 * _unit:
		return -1
	for i in STROKE_COUNT:
		if pos.x <= _stroke_rect(i).end.x + STROKE_GAP * 0.5 * _unit:
			return i
	return STROKE_COUNT - 1
