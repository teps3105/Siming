class_name TimeDial
extends Control
## GDD 5.7 天時儀：左上圓盤（十二月刻度、朱針、盤心的行止鈕），盤右是寫著日期的題簽，
## 題簽下是時速簽——「運行中／已暫停」加暫停鍵、疾徐四筆加每秒日數（1、2、5、10 日/秒）與調速鍵，最右是世界快進鈕。
## 每個可操作的地方都有圖像、文字與實體鍵圖像（5.7 原則 7）。支援的決定：現在該停還是該快。
## 圓盤外圈兩道弧（GDD 2.8）：左弧寒暖（下寒上暖）、右弧靈潮（下枯上盛），兩端加粗的一段是大寒、大暖、末法、靈氣復甦的範圍；
## 朱點是目前的值，朱點旁的箭頭指向下一世結算後的走向；滑鼠停在弧上，題簽右邊出現一張說明簽。

## 點題簽右端的「冊」印，要求展開天下冊（GDD 5.10）。
signal almanac_requested
## 在快進鈕上按下與放開（GDD 2.6）：點一下開終點選單，按住超過一段時間改為按住才快進；由 Hud 判斷。
signal fast_pressed
signal fast_released

## 圓盤半徑含綾邊（直徑 56）；盤心鈕直徑 30。
const DISC_RADIUS := 28.0
const SEAL_RADIUS := 15.0
## 外圈：圓盤外加這麼寬的一圈紙面放兩道弧；弧線的半徑、弧的半張角、朱點半徑。
const ARC_MARGIN := 13.0
const ARC_RADIUS := 34.5
const ARC_HALF_SPAN := deg_to_rad(58.0)
const ARC_DOT := 4.0
## 下一世的變化量小於這個數就不畫箭頭（寒暖、靈潮）。
const TREND_DEADBAND := {&"warmth": 0.02, &"tide": 0.01}
const STROKE_COUNT := 4
const TITLE_HEIGHT := 30.0
const SPEED_HEIGHT := 24.0
## 疾徐四筆：最短一筆的長度、每筆加長多少、筆距。
const STROKE_BASE := 9.0
const STROKE_STEP := 3.0
const STROKE_GAP := 4.0
const PAD := 8.0
## 題簽的寬度按年幾位數、月幾位數的最寬日期預留；倍速的數字留幾位數的欄寬並靠右對齊。容器因此不隨內容變寬。
const SPEED_DIGITS := 2
const DATE_PAD := 12.0
const PRESSED_PAUSE := 100
const PRESSED_FAST := 101
## 快進鈕的半徑與它和前一個鍵的間距。
const FAST_RADIUS := 9.0
const FAST_GAP := 10.0
## 兩個調速鍵之間的間距。
const KEY_GAP := 10.0
## 題簽右端的「冊」印邊長，與印、鍵右側的留白。
const ALMANAC_SEAL := 20.0
const ALMANAC_PAD := 6.0

## 題簽縮字的下限（字級）。
const MIN_DATE_SCALE := 0.85

## 題簽右端最遠能到哪裡（畫面座標）：右邊的命簿攤開時，題簽不蓋到它；日期放不下就縮小字級。
var max_right := 100000.0:
	set(value):
		max_right = value
		_fit = {}
		queue_redraw()

## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像）。
var gamepad := false:
	set(value):
		gamepad = value
		queue_redraw()

var _sim: SimHost
var _unit := 1.0
var _fit := {}
var _hover_stroke := -1
var _hover_seal := false
var _hover_almanac := false
var _hover_fast := false
## 快進鈕按著（滑鼠移出也要等到放開）。
var _fast_down := false
## 滑鼠停在哪一道弧上：&""、&"warmth"、&"tide"。
var _hover_arc := &""
## 正按住的地方（按下的回饋）：-1 沒有、0–3 疾徐的某一筆、PRESSED_PAUSE 行止鈕。
var _pressed := -1


func setup(sim: SimHost, unit: float) -> void:
	_sim = sim
	_unit = unit
	_fit = {}
	custom_minimum_size = Vector2(560.0, (DISC_RADIUS + ARC_MARGIN) * 2.0 + 16.0) * unit
	size = custom_minimum_size
	mouse_filter = Control.MOUSE_FILTER_STOP
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	sim.advanced.connect(queue_redraw.unbind(1))
	sim.fast_forward_started.connect(queue_redraw)
	sim.fast_forward_finished.connect(queue_redraw.unbind(1))
	sim.time_state_changed.connect(queue_redraw)


func _disc_radius() -> float:
	return DISC_RADIUS * _unit


## 連外圈兩道弧在內的半徑。
func outer_radius() -> float:
	return (DISC_RADIUS + ARC_MARGIN) * _unit


func _disc_center() -> Vector2:
	return Vector2.ONE * outer_radius()


## 寒暖或靈潮在弧上的位置（0 在下端、1 在上端）：目前的與下一世結算後的。
func _arc_levels(kind: StringName) -> Vector2:
	var c: Dictionary = _sim.world.climate()
	if kind == &"warmth":
		return Vector2(inverse_lerp(-1.0, 1.0, c["warmth"]), inverse_lerp(-1.0, 1.0, c["warmth_next"])).clampf(0.0, 1.0)
	return Vector2(inverse_lerp(c["tide_min"], c["tide_max"], c["tide"]), inverse_lerp(c["tide_min"], c["tide_max"], c["tide_next"])).clampf(0.0, 1.0)


## 下一世的走向：1 往上（轉暖、漸漲）、-1 往下、0 持平。
func trend(kind: StringName) -> int:
	var c: Dictionary = _sim.world.climate()
	var delta: float = c[String(kind) + "_next"] - c[String(kind)]
	return 0 if absf(delta) < TREND_DEADBAND[kind] else signi(int(signf(delta)))


## 弧上 level（0..1）那一點的方向角：左弧繞著正左、右弧繞著正右，下端在下。
func _arc_angle(kind: StringName, level: float) -> float:
	var swing := ARC_HALF_SPAN * (level * 2.0 - 1.0)
	return PI + swing if kind == &"warmth" else -swing


func _arc_point(kind: StringName, level: float) -> Vector2:
	return _disc_center() + Vector2.from_angle(_arc_angle(kind, level)) * ARC_RADIUS * _unit


## 朱點的位置（本器物座標）。
func arc_dot(kind: StringName) -> Vector2:
	return _arc_point(kind, _arc_levels(kind).x)


## 滑鼠停在某一道弧上時說明簽寫的字；沒停在弧上是空字串。
func hover_text() -> String:
	if _hover_arc == &"" or _sim == null:
		return ""
	var c: Dictionary = _sim.world.climate()
	var warmth := _hover_arc == &"warmth"
	var state: String = Locale.t("climate_" + c["climate"]) if warmth else Locale.t("aura_" + c["aura"])
	var way := trend(_hover_arc)
	var trend_key := "trend_flat"
	if way > 0:
		trend_key = "trend_warmer" if warmth else "trend_rising"
	elif way < 0:
		trend_key = "trend_colder" if warmth else "trend_falling"
	return Locale.t("dial_arc_format").format({
		"title": Locale.t("dial_warmth" if warmth else "dial_tide"), "state": state, "trend": Locale.t(trend_key)})


## 說明簽的紙面：題簽右邊。
func _hover_rect() -> Rect2:
	var title := _title_rect()
	var width := Ink.text_size(hover_text(), Ink.font_size(Ink.SIZE_NOTE, _unit)).x + PAD * 2.0 * _unit
	return Rect2(Vector2(title.end.x + Ink.SLIP_BAND * 2.0 + 6.0 * _unit, title.position.y), Vector2(width, title.size.y))


func _arc_at(pos: Vector2) -> StringName:
	var offset := pos - _disc_center()
	if offset.length() <= _disc_radius() or offset.length() > outer_radius():
		return &""
	return &"warmth" if offset.x < 0.0 else &"tide"


func _draw_arc_gauge(kind: StringName) -> void:
	var c: Dictionary = _sim.world.climate()
	var center := _disc_center()
	var radius := ARC_RADIUS * _unit
	var low: float = inverse_lerp(-1.0, 1.0, -c["warmth_enter"]) if kind == &"warmth" else inverse_lerp(c["tide_min"], c["tide_max"], c["decline_enter"])
	var high: float = inverse_lerp(-1.0, 1.0, c["warmth_enter"]) if kind == &"warmth" else inverse_lerp(c["tide_min"], c["tide_max"], c["revival_enter"])
	draw_arc(center, radius, _arc_angle(kind, 0.0), _arc_angle(kind, 1.0), 32, Ink.INK_LIGHT, 1.5 * _unit, true)
	draw_arc(center, radius, _arc_angle(kind, 0.0), _arc_angle(kind, low), 12, Ink.INK, 3.5 * _unit, true)
	draw_arc(center, radius, _arc_angle(kind, high), _arc_angle(kind, 1.0), 12, Ink.INK, 3.5 * _unit, true)
	var levels := _arc_levels(kind)
	var dot := _arc_point(kind, levels.x)
	draw_circle(dot, (ARC_DOT + 1.2) * _unit, Ink.PAPER_LIGHT)
	draw_circle(dot, ARC_DOT * _unit, Ink.SEAL_RED)
	var way := trend(kind)
	if way == 0:
		return
	# 箭頭貼著朱點、沿弧指向走向；朱點已在弧的那一端時畫在弧端外。
	var step := (ARC_DOT + 2.5) / ARC_RADIUS / (ARC_HALF_SPAN * 2.0)
	var base_level := levels.x + way * step
	var tip_level := base_level + way * step * 1.2
	var base := _arc_point(kind, base_level)
	var tip := _arc_point(kind, tip_level)
	var side := (tip - base).orthogonal().normalized() * 4.2 * _unit
	draw_colored_polygon(PackedVector2Array([tip, base + side, base - side]), Ink.SEAL_RED)


func _state_label() -> String:
	if _sim.fast_active():
		return Locale.t("dial_fast")
	return Locale.t("dial_paused") if _sim.paused else Locale.t("dial_running")


func _speed_label() -> String:
	return "%d%s" % [_sim.days_per_second(), Locale.t("dial_speed_unit")]


func _fast_key() -> String:
	return InputHints.label(InputActions.FAST, gamepad)


func _pause_key() -> String:
	return InputHints.label(InputActions.PAUSE, gamepad)


## 調速的鍵：鍵盤是 Tab、搖桿是 Y，每按一次切到下一檔。
func _speed_keys() -> Array[String]:
	return [InputHints.label(InputActions.SPEED_CYCLE, gamepad)]


## 題簽的紙面（不含綾邊）。
func _title_rect() -> Rect2:
	var fit := _date_fit()
	var height := TITLE_HEIGHT * _unit
	if fit["lines"] == 2:
		height += LedgerText.line_height(fit["size"])
	return Rect2(Vector2(_slip_left(), Ink.SLIP_BAND + ARC_MARGIN * 0.5 * _unit), Vector2(_slip_width(), height))


## 兩張簽共用的左緣。
func _slip_left() -> float:
	return outer_radius() * 2.0 + 8.0 * _unit + Ink.SLIP_BAND


## 兩張簽共用的寬度：取日期與時速內容中較寬者，與目前的日期、倍速、輸入裝置無關。
func _slip_width() -> float:
	return maxf(_date_max_width() + DATE_PAD * 2.0 * _unit + _almanac_strip() * 2.0, _columns()["end"] - _slip_left())


## 題簽右端留給「冊」印與它的鍵圖像的寬度；日期仍在題簽正中，所以兩側各留一份。
func _almanac_strip() -> float:
	return (ALMANAC_SEAL + ALMANAC_PAD * 2.0) * _unit + _widest_key(InputActions.ALMANAC) + 6.0 * _unit


## 「冊」印的範圍（題簽右端，垂直置中）。
func _almanac_rect() -> Rect2:
	var title := _title_rect()
	var side := ALMANAC_SEAL * _unit
	return Rect2(Vector2(title.end.x - ALMANAC_PAD * _unit - side, title.position.y + (title.size.y - side) * 0.5), Vector2.ONE * side)


## 一個數字佔的寬度（取 0–9 中最寬者）。
static func _digit_width(size: int) -> float:
	var widest := 0.0
	for digit in 10:
		widest = maxf(widest, Ink.text_size(str(digit), size).x)
	return widest


## 最寬的日期佔多寬：紀元名兩個字、紀元年四位、世界年六位、十二月。題簽的寬度以它為準，不隨當下的日期變。
func _date_max_width() -> float:
	return _date_fit()["width"]


func _date_font_size() -> int:
	return _date_fit()["size"]


## 題簽日期怎麼排：先是單行、內文字級；右邊要留給命簿，放不下就縮字級（最多縮到 85%），
## 還放不下折成兩行（「竭黃紀元 一五年」「(11145年) 十二月」），兩行也先用內文字級、再逐級縮到 85%。
## 回傳 lines（1 或 2）、size（字級）、width（最寬的日期佔多寬）。
func _date_fit() -> Dictionary:
	if not _fit.is_empty():
		return _fit
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	var floor_size := maxi(int(floor(body * MIN_DATE_SCALE)), 1)
	var room := max_right - position.x - _slip_left() - DATE_PAD * 2.0 * _unit - _almanac_strip() * 2.0
	for size in range(body, floor_size - 1, -1):
		var width := EventText.widest_label(size)
		if width <= room:
			_fit = {"lines": 1, "size": size, "width": width}
			return _fit
	for size in range(body, floor_size - 1, -1):
		var width := EventText.widest_label_lines(size)
		if width <= room or size == floor_size:
			_fit = {"lines": 2, "size": size, "width": width}
			return _fit
	return _fit


## 日期文字畫在哪裡：整串在題簽裡置中。
func _date_rect() -> Rect2:
	var title := _title_rect()
	var fit := _date_fit()
	var extent := Vector2.ZERO
	for line in _date_lines():
		extent = Vector2(maxf(extent.x, Ink.text_size(line, fit["size"]).x), extent.y + LedgerText.line_height(fit["size"]) if fit["lines"] == 2 else Ink.text_size(line, fit["size"]).y)
	return Rect2(title.position + (title.size - extent) * 0.5, extent)


## 現在的日期分行：單行一個、兩行兩個。
func _date_lines() -> PackedStringArray:
	var label := EventText.now_label(_sim.world)
	return EventText.label_lines(label) if _date_fit()["lines"] == 2 else PackedStringArray([label])


func _strokes_width() -> float:
	var strokes := 0.0
	for i in STROKE_COUNT:
		strokes += STROKE_BASE + STROKE_STEP * i
	return (strokes + STROKE_GAP * (STROKE_COUNT - 1)) * _unit


## 時速簽各段的 x 座標：狀態字、暫停鍵、四筆、每秒日數、調速鍵、快進鈕與它的鍵、右緣。
func _columns() -> Dictionary:
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var x := _slip_left() + PAD * _unit
	var out := {"state": x}
	x += maxf(Ink.text_size(Locale.t("dial_paused"), note).x, maxf(Ink.text_size(Locale.t("dial_running"), note).x, Ink.text_size(Locale.t("dial_fast"), note).x)) + 6.0 * _unit
	out["pause_key"] = x
	x += _widest_key(InputActions.PAUSE) + 14.0 * _unit
	out["strokes"] = x
	x += _strokes_width() + 8.0 * _unit
	out["speed"] = x
	x += _digit_width(note) * SPEED_DIGITS
	out["speed_end"] = x
	x += Ink.text_size(Locale.t("dial_speed_unit"), note).x + 8.0 * _unit
	out["speed_keys"] = x
	x += _widest_key(InputActions.SPEED_CYCLE) + FAST_GAP * _unit
	out["fast"] = x + FAST_RADIUS * _unit
	x += FAST_RADIUS * 2.0 * _unit + 4.0 * _unit
	out["fast_key"] = x
	x += _widest_key(InputActions.FAST) + PAD * _unit
	out["end"] = x
	return out


## 動作的鍵圖像在鍵鼠與搖桿兩種裝置下較寬的那個：預留這個寬度，換裝置時簽不變寬。
func _widest_key(action: StringName) -> float:
	return maxf(Ink.key_glyph_width(InputHints.label(action, false), _unit), Ink.key_glyph_width(InputHints.label(action, true), _unit))


## 時速簽的紙面：題簽正下方。
func _speed_rect() -> Rect2:
	var title := _title_rect()
	return Rect2(Vector2(title.position.x, title.end.y + Ink.SLIP_BAND * 2.0 + 4.0 * _unit), Vector2(title.size.x, SPEED_HEIGHT * _unit))


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
	# 外圈的紙面與兩道弧，圓盤疊在上面。
	Ink.paper_disc(self, center, outer_radius())
	_draw_arc_gauge(&"warmth")
	_draw_arc_gauge(&"tide")
	Ink.paper_disc(self, center, radius)
	# 十二月刻度：正月在正上方，順時針。
	for month in 12:
		var dir := Vector2.UP.rotated(TAU * month / 12.0)
		var long := month % 3 == 0
		draw_line(center + dir * (face - (6.0 if long else 4.0) * _unit), center + dir * (face - 1.5 * _unit), Ink.INK if long else Ink.INK_LIGHT, 1.5 if long else 1.0, true)
	# 盤心行止鈕：暫停中是朱底的暫停圖示，運行中是紙底的播放圖示。
	var stopped := _sim.paused
	Ink.round_button(self, center, SEAL_RADIUS * _unit, &"pause" if stopped else &"play", stopped, _hover_seal, _pressed == PRESSED_PAUSE)
	# 朱針逐日緩緩前行：月內第幾日決定它在兩個刻度之間的位置。
	var parts: Dictionary = _sim.world.date_parts()
	var month_pos := float(int(parts["month"]) - 1) + float(int(parts["day"]) - 1) / float(SimHost.DAYS_PER_MONTH)
	var needle := Vector2.UP.rotated(TAU * month_pos / 12.0)
	draw_line(center + needle * (SEAL_RADIUS + 1.0) * _unit, center + needle * (face - 1.0 * _unit), Ink.SEAL_RED, 2.5, true)

	# 題簽：日期。
	var date_size := _date_font_size()
	var title := _title_rect()
	Ink.paper_slip(self, title)
	# 整串日期在題簽裡置中。
	var date_rect := _date_rect()
	var step := LedgerText.line_height(date_size)
	var lines := _date_lines()
	for i in lines.size():
		var line_width := Ink.text_size(lines[i], date_size).x
		var line_y := date_rect.position.y + (step * i if lines.size() == 2 else 0.0)
		var line_h := step if lines.size() == 2 else date_rect.size.y
		var offset_y := (line_h - Ink.text_size(lines[i], date_size).y) * 0.5 if lines.size() == 2 else 0.0
		Ink.text(self, Vector2(date_rect.get_center().x - line_width * 0.5, line_y + offset_y), lines[i], date_size)
	_draw_almanac_seal()

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
	# 倍速的數字在固定寬的欄位裡靠右對齊。
	var multiplier := str(_sim.days_per_second())
	Ink.text(self, Vector2(columns["speed_end"] - Ink.text_size(multiplier, note).x, text_y), multiplier, note)
	Ink.text(self, Vector2(columns["speed_end"], text_y), Locale.t("dial_speed_unit"), note)
	var keys := _speed_keys()
	Ink.key_glyph(self, Vector2(columns["speed_keys"], key_y), keys[0], _unit)
	var fast_center := Vector2(columns["fast"], slip.get_center().y)
	Ink.round_button(self, fast_center, FAST_RADIUS * _unit, &"fast", _sim.fast_active(), _hover_fast, _pressed == PRESSED_FAST)
	Ink.key_glyph(self, Vector2(columns["fast_key"], key_y), _fast_key(), _unit)
	if _hover_arc != &"":
		var hint := _hover_rect()
		Ink.paper_slip(self, hint)
		Ink.text(self, hint.position + Vector2(PAD * _unit, (hint.size.y - Ink.text_size(hover_text(), note).y) * 0.5), hover_text(), note)


## 題簽右端：鍵鼠時先畫 B 鍵圖像，再畫朱底白字的「冊」印。
func _draw_almanac_seal() -> void:
	var rect := _almanac_rect()
	var key := InputHints.label(InputActions.ALMANAC, gamepad)
	if not key.is_empty():
		var key_width := Ink.key_glyph_width(key, _unit)
		Ink.key_glyph(self, Vector2(rect.position.x - 6.0 * _unit - key_width, rect.get_center().y - Ink.KEY_HEIGHT * _unit * 0.5), key, _unit, not gamepad)
	draw_rect(rect, Ink.SEAL_RED.darkened(0.15) if _hover_almanac else Ink.SEAL_RED, true)
	draw_rect(rect.grow(-2.0 * _unit), Ink.PAPER_LIGHT, false, 1.0)
	var size := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var text := Locale.t("almanac_seal")
	Ink.text(self, rect.get_center() - Ink.text_size(text, size) * 0.5, text, size, Ink.PAPER_LIGHT)


func _gui_input(event: InputEvent) -> void:
	if _sim == null:
		return
	if event is InputEventMouseMotion:
		var on_almanac := _almanac_rect().grow(Ink.SLIP_BAND).has_point((event as InputEventMouseMotion).position)
		if on_almanac != _hover_almanac:
			_hover_almanac = on_almanac
			queue_redraw()
		var on_fast := _on_fast((event as InputEventMouseMotion).position)
		if on_fast != _hover_fast:
			_hover_fast = on_fast
			queue_redraw()
		var on_seal: bool = _on_pause((event as InputEventMouseMotion).position)
		var on_stroke: int = _stroke_at((event as InputEventMouseMotion).position)
		var on_arc := _arc_at((event as InputEventMouseMotion).position)
		if on_seal != _hover_seal or on_stroke != _hover_stroke or on_arc != _hover_arc:
			_hover_seal = on_seal
			_hover_stroke = on_stroke
			_hover_arc = on_arc
			queue_redraw()
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var stroke: int = _stroke_at((event as InputEventMouseButton).position)
		if _on_fast((event as InputEventMouseButton).position):
			_pressed = PRESSED_FAST
			_fast_down = true
			fast_pressed.emit()
			accept_event()
		elif _almanac_rect().grow(Ink.SLIP_BAND).has_point((event as InputEventMouseButton).position):
			almanac_requested.emit()
			accept_event()
		elif stroke >= 0:
			_sim.set_speed(stroke)
			_sim.set_paused(false)
			_pressed = stroke
			accept_event()
		elif _on_pause((event as InputEventMouseButton).position):
			_sim.toggle_paused()
			_pressed = PRESSED_PAUSE
			accept_event()
		queue_redraw()
	elif event is InputEventMouseButton and not event.pressed and event.button_index == MOUSE_BUTTON_LEFT and _fast_down:
		_fast_down = false
		_pressed = -1
		fast_released.emit()
		queue_redraw()
	elif event is InputEventMouseButton and not event.pressed and _pressed >= 0:
		_pressed = -1
		queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and (_hover_almanac or _hover_seal or _hover_fast or _hover_stroke >= 0 or _pressed >= 0 or _hover_arc != &""):
		if not _fast_down:
			_pressed = -1
		_hover_fast = false
		_hover_arc = &""
		_hover_seal = false
		_hover_almanac = false
		_hover_stroke = -1
		queue_redraw()


## 滑鼠只在圓盤（連外圈）與時速簽上才攔截，其餘讓地圖收到事件。
func _has_point(point: Vector2) -> bool:
	return point.distance_to(_disc_center()) <= outer_radius() or _speed_rect().grow(Ink.SLIP_BAND).has_point(point) or _almanac_rect().grow(Ink.SLIP_BAND).has_point(point)


## 游標停在這裡時是否可以按（GDD 5.9 游標圖示）。
func is_hot(point: Vector2) -> bool:
	return _on_pause(point) or _on_fast(point) or _stroke_at(point) >= 0 or _almanac_rect().grow(Ink.SLIP_BAND).has_point(point)


## 快進鈕（時速簽最右的圓鈕）。
func _on_fast(pos: Vector2) -> bool:
	var slip := _speed_rect()
	return pos.distance_to(Vector2(_columns()["fast"], slip.get_center().y)) <= (FAST_RADIUS + 3.0) * _unit


## 快進鈕的中心（本器物座標）。
func fast_button_center() -> Vector2:
	return Vector2(_columns()["fast"], _speed_rect().get_center().y)


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
