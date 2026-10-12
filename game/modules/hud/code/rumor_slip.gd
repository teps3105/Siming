class_name RumorSlip
extends Control
## GDD 4.7、5.7 傳聞簽：畫面右上一列紙簽，每張是事件印、一句話與年份，新到舊，世界大事置頂。
## 支援的決定：該追哪一則——點一則鏡頭飛往該地。收合時最多 8 張（Deck 為 1 張），展開為近 50 則的捲動清單。
## 入簽依重要度分級（5.7「通知優先序」）：≥ 3 或世界大事入簽，2 只進展開清單，其餘不顯示；
## 同一魂同一年的多筆事件合併為一簽（取重要度高的）。

## 點一則有地點的傳聞：飛往地圖座標 map_pos。
signal fly_requested(map_pos: Vector2)

const WIDTH := 300.0
const EXPANDED_WIDTH := 360.0
const ITEM_HEIGHT := 32.0
const ITEM_GAP := 5.0
const HEADER_HEIGHT := 26.0
## 收合時最多幾張；展開清單最多留幾則。
const SHOWN_MAX := 8
const KEEP_MAX := 50
## 入簽與只進清單的重要度門檻。
const SLIP_IMPORTANCE := 3
const NOTE_IMPORTANCE := 2
## 世界大事置頂多久（tick）：一世。
const PIN_TICKS := 360
const FADE_SECONDS := 0.35
const SEAL_SIZE := 22.0
const PAD := 8.0
## 控制項四周留給柔影的空間，紙面從這裡起算。
const SHADOW := 8.0
## 展開清單捲動：游標貼近上下緣時每秒捲的像素，與滾輪每格的列數。
const EDGE_ZONE := 26.0
const EDGE_SPEED := 320.0
const SELECT_NONE := -1
const SELECT_HEADER := -2

## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像）。
var gamepad := false:
	set(value):
		gamepad = value
		queue_redraw()
## Deck 版面：收合時只留最新一張（GDD 5.7）。
var compact := false:
	set(value):
		compact = value
		_refit()
## 是否展開為近 50 則的清單。
var expanded := false:
	set(value):
		if expanded == value:
			return
		expanded = value
		_scroll = 0.0
		_refit()
## 紙面右緣與上緣在畫面上的位置。
var right_edge := 0.0:
	set(value):
		right_edge = value
		_place()
var top_edge := 0.0:
	set(value):
		top_edge = value
		_place()
## 展開清單的高度上限（畫面上緣到印匣之間）。
var max_height := 600.0:
	set(value):
		max_height = value
		_refit()

var _unit := 1.0
## 依到達順序，新的在後。
var _entries: Array[Dictionary] = []
var _latest_tick := 0
var _hover := SELECT_NONE
var _pressed := SELECT_NONE
var _scroll := 0.0
var _pointer := Vector2(-1.0, -1.0)


func setup(unit: float) -> void:
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_STOP
	clip_contents = true
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	_refit()


## 新到的事件。重要度不夠的不入簽；回傳是否留下。
func post(event: Dictionary) -> bool:
	var world: bool = event["kind"] == "WorldChanged"
	var importance: int = event["importance"]
	if importance < NOTE_IMPORTANCE and not world:
		return false
	var entry := {
		"id": event["id"],
		"tick": event["tick"],
		"year": event.get("year", 0),
		"importance": importance,
		"world": world,
		"line": EventText.line(event),
		"seal": EventText.seal(event["kind"]),
		"souls": event.get("souls", PackedInt32Array()),
		"age": 0.0,
	}
	if event.has("pos"):
		entry["pos"] = event["pos"]
	_latest_tick = maxi(_latest_tick, int(entry["tick"]))
	var same := _same_soul_year(entry)
	if same >= 0:
		var old := _entries[same]
		if importance >= int(old["importance"]):
			entry["age"] = old["age"]
			_entries[same] = entry
	else:
		_entries.append(entry)
		while _entries.size() > KEEP_MAX:
			_entries.pop_front()
	_refit()
	return true


func clear() -> void:
	_entries.clear()
	_latest_tick = 0
	_scroll = 0.0
	_refit()


## 目前留著的傳聞（重建器物時帶到新的簽上）。
func snapshot() -> Array[Dictionary]:
	return _entries.duplicate()


func restore(entries: Array[Dictionary]) -> void:
	_entries = entries.duplicate()
	for entry in _entries:
		entry["age"] = FADE_SECONDS
		_latest_tick = maxi(_latest_tick, int(entry["tick"]))
	_refit()


## 同一魂同一年已有的傳聞（世界大事不併）；沒有則 -1。
func _same_soul_year(entry: Dictionary) -> int:
	var souls: PackedInt32Array = entry["souls"]
	if entry["world"] or souls.is_empty():
		return -1
	for i in _entries.size():
		var other := _entries[i]
		if other["world"] or _year_of(other) != _year_of(entry):
			continue
		for soul in souls:
			if (other["souls"] as PackedInt32Array).has(soul):
				return i
	return -1


func _year_of(entry: Dictionary) -> int:
	return floori(float(entry["tick"]) / 12.0)


## 世界大事是否仍置頂：發生在最近一世之內。
func _pinned(entry: Dictionary) -> bool:
	return entry["world"] and _latest_tick - int(entry["tick"]) < PIN_TICKS


func _before(a: Dictionary, b: Dictionary) -> bool:
	var pa := _pinned(a)
	var pb := _pinned(b)
	if pa != pb:
		return pa
	if a["tick"] != b["tick"]:
		return a["tick"] > b["tick"]
	if a["importance"] != b["importance"]:
		return a["importance"] > b["importance"]
	return a["id"] > b["id"]


## 目前畫出來的傳聞，由上到下：置頂的世界大事先，再依新到舊、重要度排。
## 收合時只列入簽的（重要度 ≥ 3 或世界大事）並受張數上限，展開時全部。
func shown() -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	for entry in _entries:
		if expanded or entry["world"] or int(entry["importance"]) >= SLIP_IMPORTANCE:
			out.append(entry)
	out.sort_custom(_before)
	if not expanded:
		out.resize(mini(out.size(), 1 if compact else SHOWN_MAX))
	return out


func entry_count() -> int:
	return _entries.size()


func _width() -> float:
	return (EXPANDED_WIDTH if expanded else WIDTH) * _unit


func _row_height() -> float:
	return ITEM_HEIGHT * _unit


func _gap() -> float:
	return ITEM_GAP * _unit


## 紙面左上角在控制項內的位置。
func _origin() -> Vector2:
	return Vector2.ONE * SHADOW


func header_rect() -> Rect2:
	return Rect2(_origin(), Vector2(_width(), HEADER_HEIGHT * _unit))


## 清單的可視範圍（簽頭下方到底）。
func _view_rect() -> Rect2:
	var head := header_rect()
	var top_y := head.end.y + _gap()
	return Rect2(Vector2(head.position.x, top_y), Vector2(head.size.x, size.y - SHADOW - top_y))


func _content_height() -> float:
	var rows := maxi(shown().size(), 1)
	return rows * _row_height() + (rows - 1) * _gap()


func _max_scroll() -> float:
	return maxf(0.0, _content_height() - _view_rect().size.y)


## 第 index 張的紙面（控制項座標）；捲動後可能落在可視範圍外。
func row_rect(index: int) -> Rect2:
	var view := _view_rect()
	return Rect2(view.position + Vector2(0.0, index * (_row_height() + _gap()) - _scroll), Vector2(view.size.x, _row_height()))


## 目前看得到（落在可視範圍內）的傳聞張數。
func visible_row_count() -> int:
	var count := 0
	for i in shown().size():
		if _view_rect().intersects(row_rect(i)):
			count += 1
	return count


## 簽佔住的畫面範圍（地圖標籤要避開）。
func occupied_rect() -> Rect2:
	var count := maxi(shown().size(), 1)
	var rect := header_rect().merge(row_rect(count - 1).intersection(_view_rect()) if expanded else row_rect(count - 1))
	rect.position += position
	return rect


func _refit() -> void:
	if _unit <= 0.0:
		return
	var natural := header_rect().size.y + _gap() + _content_height()
	var height := minf(natural, max_height - SHADOW * 2.0) if expanded else natural
	custom_minimum_size = Vector2(_width(), height) + Vector2.ONE * SHADOW * 2.0
	size = custom_minimum_size
	_scroll = clampf(_scroll, 0.0, _max_scroll())
	_place()
	queue_redraw()


func _place() -> void:
	position = Vector2(right_edge - size.x + SHADOW, top_edge - SHADOW).round()


## 簽頭右端的鍵：鍵鼠是展開清單的鍵，搖桿沒有專屬鍵，以游標的左鍵（LT）點。
func _list_key() -> String:
	if gamepad:
		return InputHints.label(InputActions.PAD_LEFT, true)
	return InputHints.label(InputActions.RUMOR_LIST, false)


## 飛往的實體鍵：鍵鼠用滑鼠左鍵，沒有鍵可畫；搖桿是游標的左鍵（LT）。
func _fly_key() -> String:
	return InputHints.label(InputActions.PAD_LEFT, true) if gamepad else ""


func _process(delta: float) -> void:
	var dirty := false
	for entry in _entries:
		if float(entry["age"]) < FADE_SECONDS:
			entry["age"] = float(entry["age"]) + delta
			dirty = true
	if expanded and _hover != SELECT_NONE and _max_scroll() > 0.0:
		var view := _view_rect()
		var step := 0.0
		if _pointer.y < view.position.y + EDGE_ZONE * _unit and _pointer.y >= view.position.y:
			step = -EDGE_SPEED
		elif _pointer.y > view.end.y - EDGE_ZONE * _unit and _pointer.y <= view.end.y:
			step = EDGE_SPEED
		if step != 0.0:
			_scroll = clampf(_scroll + step * _unit * delta, 0.0, _max_scroll())
			_update_hover(_pointer)
			dirty = true
	if dirty:
		queue_redraw()


func _draw() -> void:
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	_draw_header(note)
	var entries := shown()
	var view := _view_rect()
	if entries.is_empty():
		var rect := row_rect(0)
		Ink.paper_slip(self, rect)
		var text := Locale.t("rumor_empty")
		Ink.text(self, rect.position + Vector2(PAD * _unit * 1.5, (rect.size.y - Ink.text_size(text, note).y) * 0.5), text, note, Ink.INK_LIGHT)
		return
	for i in entries.size():
		var rect := row_rect(i)
		if not view.intersects(rect):
			continue
		_draw_row(entries[i], rect, i, note)


func _draw_header(note: int) -> void:
	var rect := header_rect()
	Ink.paper_slip(self, rect)
	if _hover == SELECT_HEADER:
		draw_rect(rect.grow(-1.0), Color(Ink.INK, 0.1 if _pressed != SELECT_HEADER else 0.2), true)
	var section := Ink.font_size(Ink.SIZE_SECTION, _unit)
	var y := rect.position.y + (rect.size.y - Ink.text_size(Locale.t("rumor_title"), section).y) * 0.5
	Ink.section(self, Vector2(rect.position.x + PAD * _unit, y), Locale.t("rumor_title"), section)
	# 右端：展開或收合的字與實體鍵。
	var label := Locale.t("rumor_collapse" if expanded else "rumor_expand")
	var key := _list_key()
	var key_width := Ink.key_glyph_width(key, _unit)
	var right := rect.end.x - PAD * _unit
	var key_x := right - key_width
	if not key.is_empty():
		Ink.key_glyph(self, Vector2(key_x, rect.position.y + (rect.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), key, _unit, not gamepad)
	var label_x := (key_x - 6.0 * _unit if not key.is_empty() else right) - Ink.text_size(label, note).x
	Ink.text(self, Vector2(label_x, rect.position.y + (rect.size.y - Ink.text_size(label, note).y) * 0.5), label, note, Ink.INK)
	# 中段：展開時寫留著幾則（收合時簽頭右端的字已寫了近 50 則）。
	var count := "%s %d %s" % [Locale.t("rumor_recent"), _entries.size(), Locale.t("rumor_unit")]
	var count_x := rect.position.x + PAD * _unit + Ink.text_size(Locale.t("rumor_title"), section).x + 34.0 * _unit
	if count_x + Ink.text_size(count, note).x < label_x - 8.0 * _unit and expanded and not _entries.is_empty():
		Ink.text(self, Vector2(count_x, rect.position.y + (rect.size.y - Ink.text_size(count, note).y) * 0.5), count, note, Ink.INK_LIGHT)


func _draw_row(entry: Dictionary, rect: Rect2, index: int, note: int) -> void:
	var fade := clampf(float(entry["age"]) / FADE_SECONDS, 0.0, 1.0)
	# 新到的簽從上方滑入。
	var slid := rect
	slid.position.y -= (1.0 - fade) * 10.0 * _unit
	var hot := _hover == index
	var pressed := hot and _pressed == index
	var has_pos: bool = entry.has("pos")
	var inked := Ink.INK if int(entry["importance"]) >= SLIP_IMPORTANCE or entry["world"] else Ink.INK_LIGHT
	Ink.paper_slip(self, slid.grow(-1.0) if pressed else slid)
	if hot:
		draw_rect(slid.grow(-1.0), Color(Ink.INK, 0.2 if pressed else 0.07), true)
		draw_rect(slid.grow(-2.0), Ink.INK if has_pos else Ink.INK_LIGHT, false, 1.0)
	# 事件印：方形朱印，世界大事用靛。
	var seal_size := SEAL_SIZE * _unit
	var seal_rect := Rect2(slid.position + Vector2(PAD * _unit, (slid.size.y - seal_size) * 0.5), Vector2.ONE * seal_size)
	draw_rect(seal_rect, Color(Ink.SILK if entry["world"] else Ink.SEAL_RED, fade), true)
	draw_rect(seal_rect.grow(-2.0), Color(Ink.PAPER_LIGHT, 0.7 * fade), false, 1.0)
	var glyph_size := Ink.font_size(Ink.SIZE_SECTION, _unit)
	var glyph: String = entry["seal"]
	Ink.text(self, seal_rect.get_center() - Ink.text_size(glyph, glyph_size) * 0.5, glyph, glyph_size, Color(Ink.PAPER_LIGHT, fade))
	# 右端：停留時是「飛往」加實體鍵，否則是年份。
	var right := slid.end.x - PAD * _unit
	var tail := ""
	var key := ""
	if hot and has_pos:
		tail = Locale.t("rumor_fly")
		key = _fly_key()
	else:
		tail = Locale.t("rumor_year").format({"n": entry.get("year", 0)})
	var key_width := Ink.key_glyph_width(key, _unit)
	if not key.is_empty():
		right -= key_width
		Ink.key_glyph(self, Vector2(right, slid.position.y + (slid.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), key, _unit)
		right -= 4.0 * _unit
	var tail_size := Ink.text_size(tail, note)
	right -= tail_size.x
	Ink.text(self, Vector2(right, slid.position.y + (slid.size.y - tail_size.y) * 0.5), tail, note, Color(Ink.SEAL_RED if hot and has_pos else Ink.INK_LIGHT, fade))
	# 一句話：放不下以「…」截斷。
	var text_x := seal_rect.end.x + 8.0 * _unit
	var line := _fit(entry["line"], right - 8.0 * _unit - text_x, note)
	Ink.text(self, Vector2(text_x, slid.position.y + (slid.size.y - Ink.text_size(line, note).y) * 0.5), line, note, Color(inked, fade))


## 把一句話截到放得進 width；截斷時結尾加「…」。
func _fit(line: String, width: float, font: int) -> String:
	if Ink.text_size(line, font).x <= width:
		return line
	var count := line.length()
	while count > 0 and Ink.text_size(line.substr(0, count) + "…", font).x > width:
		count -= 1
	return line.substr(0, count) + "…"


## 控制項座標的 point 落在哪：簽頭、第幾張，或沒有。
func _select_at(point: Vector2) -> int:
	if header_rect().has_point(point):
		return SELECT_HEADER
	if not _view_rect().has_point(point):
		return SELECT_NONE
	var entries := shown()
	for i in entries.size():
		if row_rect(i).has_point(point):
			return i
	return SELECT_NONE


## 游標只在簽頭與簽上才攔截，其餘讓地圖收到事件。
func _has_point(point: Vector2) -> bool:
	return _select_at(point) != SELECT_NONE


## 游標停在這裡時是否可以按（GDD 5.9 游標圖示）：簽頭，或有地點的簽。
func is_hot(point: Vector2) -> bool:
	var at := _select_at(point)
	if at == SELECT_HEADER:
		return true
	return at >= 0 and shown()[at].has("pos")


func _update_hover(point: Vector2) -> void:
	var at := _select_at(point)
	if at != _hover:
		_hover = at
		queue_redraw()


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_pointer = (event as InputEventMouseMotion).position
		_update_hover(_pointer)
	elif event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		var at := _select_at(button.position)
		if button.button_index == MOUSE_BUTTON_LEFT:
			if button.pressed:
				_pressed = at
				queue_redraw()
				accept_event()
			else:
				var released := _pressed
				_pressed = SELECT_NONE
				queue_redraw()
				if released != SELECT_NONE and released == at:
					activate(at)
				accept_event()
		elif expanded and button.pressed and (button.button_index == MOUSE_BUTTON_WHEEL_UP or button.button_index == MOUSE_BUTTON_WHEEL_DOWN):
			var dir := -1.0 if button.button_index == MOUSE_BUTTON_WHEEL_UP else 1.0
			_scroll = clampf(_scroll + dir * (_row_height() + _gap()), 0.0, _max_scroll())
			_update_hover(button.position)
			queue_redraw()
			accept_event()


## 簽頭：展開或收合；一張簽：有地點就飛往，展開的清單隨後收回。
func activate(select: int) -> void:
	if select == SELECT_HEADER:
		expanded = not expanded
	elif select >= 0:
		var entry := shown()[select]
		if entry.has("pos"):
			fly_requested.emit(entry["pos"])
			if expanded:
				expanded = false


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and (_hover != SELECT_NONE or _pressed != SELECT_NONE):
		_hover = SELECT_NONE
		_pressed = SELECT_NONE
		queue_redraw()
