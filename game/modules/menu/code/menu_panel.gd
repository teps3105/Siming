class_name MenuPanel
extends Control
## GDD 5.7 選單：一列列的字，沒有題名也沒有框——停留的列底下掃一筆淡墨、左邊一方小朱印，其餘的列底下一道細墨線；
## 鍵盤、搖桿與滑鼠都能操作，底下一行實體鍵提示。paper 為真時底下襯一張裱框的紙（遊戲中選單），否則直接寫在畫面上（標題畫面）。
## 列的種類：action（按了就做）、choice（左右換選項）、slider（左右調數值）、text（可打字）、
## bar（進度墨條）、head（節名）、line（一行字，可停留以便捲動）。列以本類別的靜態函式建立。

signal chosen(id: StringName)
## 該列的第二個動作（InputActions.MENU_ALT）；只有 alt_label 不為空時才送。
signal alt_chosen(id: StringName)
signal changed(id: StringName, value: Variant)
signal cancelled

const WIDTH := 440.0
const ROW_HEIGHT := 38.0
const LINE_HEIGHT := 26.0
const ROW_GAP := 6.0
const PAD := 16.0
const FOOT := 30.0
const ICON_RADIUS := 8.0
## 停留的列左邊那方朱印的邊長，與列內文字的左邊距。
const SEAL := 9.0
const INSET := 24.0
## 選項、數值與文字欄佔列寬的右半段。
const VALUE_WIDTH := 200.0
const ARROW := 6.0
const FOCUSABLE: Array[StringName] = [&"action", &"choice", &"slider", &"text", &"line"]

## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像）。
var gamepad := false:
	set(value):
		if gamepad != value:
			gamepad = value
			queue_redraw()
## 返回鍵旁的字；空字串表示這一頁不能返回（不畫提示，也不送 cancelled）。
var back_label := ""
## 第二個動作旁的字；空字串表示這一頁沒有第二個動作。
var alt_label := ""
## 底下要不要襯一張紙。
var paper := true:
	set(value):
		paper = value
		queue_redraw()
## 紙面最高能到多高（超過就捲動）；0 表示不限。
var max_height := 0.0
## 為假時不收鍵盤與搖桿（被別的紙面蓋住時）。
var accepts_input := true

var _unit := 1.0
var _rows: Array[Dictionary] = []
var _focus := -1
## 這一頁有沒有可操作的列。
var _has_controls := false
var _first := 0
## 目前畫出來的列：列索引 → 範圍。
var _rects := {}
var _pressed_row := -1
var _sliding := false
var _editing := -1
var _edit: LineEdit
var _axis_held := {}


static func action(id: StringName, label: String, icon := &"", note := "", enabled := true) -> Dictionary:
	return {"kind": &"action", "id": id, "label": label, "icon": icon, "note": note, "enabled": enabled}


## options 的每一項是 [值, 顯示的字]。
static func choice(id: StringName, label: String, options: Array, value: Variant) -> Dictionary:
	return {"kind": &"choice", "id": id, "label": label, "options": options, "value": value, "enabled": true}


## 顯示的字是 format % (value × display_scale)。
static func slider(id: StringName, label: String, value: float, low: float, high: float, step: float, format := "%.2f", display_scale := 1.0) -> Dictionary:
	return {
		"kind": &"slider", "id": id, "label": label, "value": value, "low": low, "high": high, "step": step,
		"format": format, "display_scale": display_scale, "enabled": true,
	}


static func text_field(id: StringName, label: String, value: String, max_length := 12, digits_only := false) -> Dictionary:
	return {"kind": &"text", "id": id, "label": label, "value": value, "max_length": max_length, "digits_only": digits_only, "enabled": true}


static func bar(id: StringName, label: String, value: float) -> Dictionary:
	return {"kind": &"bar", "id": id, "label": label, "value": value}


static func head(label: String) -> Dictionary:
	return {"kind": &"head", "id": &"", "label": label}


static func line(label: String, dim := false) -> Dictionary:
	return {"kind": &"line", "id": &"", "label": label, "dim": dim, "enabled": true}


func setup(unit: float) -> void:
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_STOP
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	if _edit == null:
		_edit = LineEdit.new()
		_edit.visible = false
		_edit.context_menu_enabled = false
		_edit.caret_blink = true
		_edit.select_all_on_focus = true
		for style_name: String in ["normal", "focus", "read_only"]:
			_edit.add_theme_stylebox_override(style_name, StyleBoxEmpty.new())
		_edit.add_theme_font_override("font", Ink.FONT)
		_edit.add_theme_color_override("font_color", Ink.INK)
		_edit.add_theme_color_override("caret_color", Ink.INK)
		_edit.add_theme_color_override("selection_color", Color(Ink.GOLD, 0.45))
		_edit.text_submitted.connect(func(_text: String) -> void: _end_edit(true))
		_edit.focus_exited.connect(func() -> void: _end_edit(true))
		add_child(_edit)
	_edit.add_theme_font_size_override("font_size", Ink.font_size(Ink.SIZE_BODY, _unit))
	_resize()


## 換一頁。focus_id 指定一開始停在哪一列；沒指定或找不到時停在第一個可選的列。
func open(rows: Array[Dictionary], focus_id := &"") -> void:
	_end_edit(false)
	_rows = rows
	_first = 0
	_pressed_row = -1
	_sliding = false
	_focus = -1
	_has_controls = _rows.any(func(row: Dictionary) -> bool:
		return row["kind"] != &"line" and row["kind"] in FOCUSABLE and row.get("enabled", true))
	for i in _rows.size():
		if _can_focus(i) and (_focus < 0 or (focus_id != &"" and _rows[i]["id"] == focus_id)):
			_focus = i
			if focus_id == &"" or _rows[i]["id"] == focus_id:
				break
	_resize()


## 目前停在哪一列（沒有可選的列時是空名稱）。
func focused_id() -> StringName:
	return _rows[_focus]["id"] if _focus >= 0 else &""


func row_count() -> int:
	return _rows.size()


## 改某一列的值（進度條、選項、數值、文字）；不送 changed。
func set_value(id: StringName, value: Variant) -> void:
	for row in _rows:
		if row["id"] == id:
			row["value"] = value
	queue_redraw()


func value_of(id: StringName) -> Variant:
	for row in _rows:
		if row["id"] == id:
			return row.get("value")
	return null


## 改某一列右側的小字。
func set_note(id: StringName, note: String) -> void:
	for row in _rows:
		if row["id"] == id:
			row["note"] = note
	queue_redraw()


## 游標停在這裡時是否可以按（GDD 5.9 游標圖示）。
func is_hot(point: Vector2) -> bool:
	var index := _row_at(point)
	return index >= 0 and _can_focus(index) and _rows[index]["kind"] != &"line"


## 某一列在紙面上的範圍（沒畫出來時是空範圍）；測試與引導用。
func row_rect(id: StringName) -> Rect2:
	for index: int in _rects:
		if _rows[index]["id"] == id:
			return _rects[index]
	return Rect2()


## 說明文字只有在整頁沒有可操作的列時才停得上去（讓長文能捲動）。
func _can_focus(index: int) -> bool:
	var row := _rows[index]
	if row["kind"] == &"line" and _has_controls:
		return false
	return row["kind"] in FOCUSABLE and row.get("enabled", true)


func _row_height(index: int) -> float:
	var kind: StringName = _rows[index]["kind"]
	return (LINE_HEIGHT if kind == &"line" or kind == &"head" or kind == &"bar" else ROW_HEIGHT) * _unit


func _content_left() -> float:
	return PAD * _unit


## 排出看得到的列，並把紙面調到剛好的大小。停留的列一定在看得到的範圍內。
func _resize() -> void:
	var limit := max_height if max_height > 0.0 else INF
	var chrome := (PAD * 2.0 + FOOT) * _unit
	if _focus >= 0 and _focus < _first:
		_first = _focus
	while true:
		_rects.clear()
		var y := PAD * _unit
		for i in range(_first, _rows.size()):
			var height := _row_height(i)
			if y + height + chrome - PAD * _unit > limit and not _rects.is_empty():
				break
			_rects[i] = Rect2(_content_left(), y, (WIDTH - PAD * 2.0) * _unit, height)
			y += height + ROW_GAP * _unit
		if _focus < 0 or _rects.has(_focus) or _first >= _rows.size() - 1:
			break
		_first += 1
	var bottom := PAD * _unit
	for index: int in _rects:
		bottom = maxf(bottom, (_rects[index] as Rect2).end.y)
	custom_minimum_size = Vector2(WIDTH * _unit, bottom + chrome - PAD * _unit)
	size = custom_minimum_size
	queue_redraw()


func _value_rect(rect: Rect2) -> Rect2:
	var width := VALUE_WIDTH * _unit
	return Rect2(rect.end.x - width - 8.0 * _unit, rect.position.y, width, rect.size.y)


func _draw() -> void:
	if paper:
		Ink.paper_slip(self, Rect2(Vector2.ZERO, size), Ink.CARD_BAND)
	for index: int in _rects:
		_draw_row(index, _rects[index])
	# 上下還有沒畫出來的列：提示那一行的右端畫小箭頭。
	var last := -1
	for index: int in _rects:
		last = maxi(last, index)
	var mark_x := size.x - (PAD + ARROW) * _unit
	var mark_y := size.y - (PAD + FOOT * 0.5) * _unit + 4.0 * _unit
	if _first > 0:
		_triangle(Vector2(mark_x - ARROW * 3.0 * _unit, mark_y), Vector2.UP, Ink.INK_LIGHT)
	if last >= 0 and last < _rows.size() - 1:
		_triangle(Vector2(mark_x, mark_y), Vector2.DOWN, Ink.INK_LIGHT)
	_draw_hints()


## 可操作的列的底：停留時一筆淡墨由左掃到右（右端收尖）加一方小朱印，按住時墨更重；其餘只有底下一道細墨線。
## 回傳列上文字該用的顏色。
func _draw_ground(rect: Rect2, enabled: bool, focused: bool, pressed: bool) -> Color:
	if focused:
		var mid := rect.get_center().y
		var lift := rect.size.y * 0.5 - 3.0 * _unit
		draw_colored_polygon(PackedVector2Array([
			Vector2(rect.position.x - 4.0 * _unit, mid - lift + 2.0 * _unit), Vector2(rect.end.x - 14.0 * _unit, mid - lift),
			Vector2(rect.end.x + 6.0 * _unit, mid + 1.0 * _unit), Vector2(rect.end.x - 14.0 * _unit, mid + lift - 2.0 * _unit),
			Vector2(rect.position.x - 4.0 * _unit, mid + lift),
		]), Color(Ink.INK, 0.22 if pressed and enabled else 0.11))
		var seal := SEAL * _unit
		draw_rect(Rect2(rect.position.x + 5.0 * _unit, mid - seal * 0.5, seal, seal), Ink.SEAL_RED if enabled else Color(Ink.INK_LIGHT, 0.6), true)
	else:
		draw_line(Vector2(rect.position.x, rect.end.y), Vector2(rect.end.x, rect.end.y), Color(Ink.INK_LIGHT, 0.22), 1.0)
	return Ink.INK if enabled else Color(Ink.INK_LIGHT, Ink.DISABLED_GLYPH_ALPHA)


func _draw_row(index: int, rect: Rect2) -> void:
	var row := _rows[index]
	var kind: StringName = row["kind"]
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var focused := index == _focus
	if kind == &"head":
		Ink.section(self, rect.position + Vector2(0.0, (rect.size.y - Ink.text_size("永", note).y) * 0.5), row["label"], Ink.font_size(Ink.SIZE_SECTION, _unit))
		return
	if kind == &"line":
		if focused:
			draw_rect(Rect2(rect.position.x - 8.0 * _unit, rect.position.y + 4.0 * _unit, 2.0, rect.size.y - 8.0 * _unit), Ink.GOLD, true)
		_text_middle(rect.position.x, rect, row["label"], body, Ink.INK_LIGHT if row["dim"] else Ink.INK)
		return
	if kind == &"bar":
		var label_width := Ink.text_size(row["label"], note).x
		_text_middle(rect.position.x, rect, row["label"], note, Ink.INK_LIGHT)
		var left := rect.position.x + label_width + 10.0 * _unit
		Ink.ink_bar(self, Rect2(left, rect.position.y + rect.size.y * 0.5 - 4.0 * _unit, rect.end.x - left, 8.0 * _unit), row["value"], Ink.INK)
		return

	var enabled: bool = row.get("enabled", true)
	var ink := _draw_ground(rect, enabled, focused, index == _pressed_row and not _sliding)
	var x := rect.position.x + INSET * _unit
	var icon: StringName = row.get("icon", &"")
	if icon != &"":
		InkIcons.draw(self, icon, Vector2(x + ICON_RADIUS * _unit, rect.get_center().y), ICON_RADIUS * _unit, ink)
		x += (ICON_RADIUS * 2.0 + 8.0) * _unit
	_text_middle(x, rect, row["label"], body, ink)
	var value := _value_rect(rect)
	match kind:
		&"action":
			var note_text: String = row.get("note", "")
			if not note_text.is_empty():
				_text_middle(rect.end.x - 12.0 * _unit - Ink.text_size(note_text, note).x, rect, note_text, note, Color(Ink.INK_LIGHT, ink.a))
		&"choice":
			var shown := _choice_label(row)
			_triangle(Vector2(value.position.x + ARROW * _unit, value.get_center().y), Vector2.LEFT, ink)
			_triangle(Vector2(value.end.x - ARROW * _unit, value.get_center().y), Vector2.RIGHT, ink)
			_text_middle(value.get_center().x - Ink.text_size(shown, body).x * 0.5, rect, shown, body, ink)
		&"slider":
			var shown: String = row["format"] % ((row["value"] as float) * (row["display_scale"] as float))
			var text_width := 54.0 * _unit
			var track := Rect2(value.position.x, value.get_center().y - 4.0 * _unit, value.size.x - text_width - 8.0 * _unit, 8.0 * _unit)
			var t := inverse_lerp(row["low"], row["high"], row["value"])
			Ink.ink_bar(self, track, t, Ink.INK)
			draw_circle(Vector2(track.position.x + track.size.x * t, track.get_center().y), 4.0 * _unit, Ink.SEAL_RED)
			_text_middle(value.end.x - Ink.text_size(shown, note).x, rect, shown, note, ink)
		&"text":
			# 文字欄：底下一道墨線；編輯時由輸入欄蓋在上面。
			draw_line(Vector2(value.position.x, rect.end.y - 8.0 * _unit), Vector2(value.end.x, rect.end.y - 8.0 * _unit), Color(Ink.INK_LIGHT, 0.6), 1.0)
			if index != _editing:
				_text_middle(value.position.x + 2.0, rect, row["value"], body, ink)


func _choice_label(row: Dictionary) -> String:
	for option: Array in row["options"]:
		if option[0] == row["value"]:
			return option[1]
	return ""


## 底下一行實體鍵提示：選列、調整（停在選項或數值上時）、確定、第二個動作、返回。
func _draw_hints() -> void:
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var hints: Array = []
	var kind: StringName = _rows[_focus]["kind"] if _focus >= 0 else &""
	if _focus >= 0:
		hints.append([InputActions.MENU_UP, Locale.t("menu_hint_move")])
	if kind == &"choice" or kind == &"slider":
		hints.append([InputActions.MENU_LEFT, Locale.t("menu_hint_adjust")])
	elif kind == &"action" or kind == &"text":
		hints.append([InputActions.MENU_ACCEPT, Locale.t("menu_hint_edit" if kind == &"text" else "menu_hint_accept")])
	if not alt_label.is_empty() and kind == &"action":
		hints.append([InputActions.MENU_ALT, alt_label])
	if not back_label.is_empty():
		hints.append([InputActions.CANCEL, back_label])
	var x := _content_left()
	var top := size.y - (FOOT + PAD) * _unit + (FOOT * _unit - Ink.KEY_HEIGHT * _unit) * 0.5 + 6.0 * _unit
	draw_line(Vector2(x, size.y - (FOOT + PAD) * _unit + 3.0 * _unit), Vector2(size.x - PAD * _unit, size.y - (FOOT + PAD) * _unit + 3.0 * _unit), Color(Ink.INK_LIGHT, 0.35), 1.0)
	for hint: Array in hints:
		var label: String = hint[1]
		x += Ink.key_glyph(self, Vector2(x, top), InputHints.label(hint[0], gamepad), _unit) + 5.0 * _unit
		Ink.text(self, Vector2(x, top + (Ink.KEY_HEIGHT * _unit - Ink.text_size(label, note).y) * 0.5), label, note, Ink.INK_LIGHT)
		x += Ink.text_size(label, note).x + 14.0 * _unit


func _text_middle(x: float, rect: Rect2, content: String, size_px: int, color: Color) -> void:
	Ink.text(self, Vector2(x, rect.position.y + (rect.size.y - Ink.text_size(content, size_px).y) * 0.5), content, size_px, color)


func _triangle(center: Vector2, dir: Vector2, color: Color) -> void:
	var across := dir.orthogonal() * ARROW * _unit
	draw_colored_polygon(PackedVector2Array([center + dir * ARROW * _unit, center - dir * ARROW * 0.4 * _unit + across, center - dir * ARROW * 0.4 * _unit - across]), color)


func _input(event: InputEvent) -> void:
	if not accepts_input or not is_visible_in_tree():
		return
	if _editing >= 0:
		# 打字中：只攔返回鍵，其餘交給輸入欄。
		if event.is_action_pressed(InputActions.CANCEL):
			_end_edit(false)
			get_viewport().set_input_as_handled()
		return
	var from_pad := event is InputEventJoypadButton or event is InputEventJoypadMotion
	var from_key := event is InputEventKey
	if not from_pad and not from_key:
		return
	if from_key and (event as InputEventKey).keycode == KEY_F12:
		return
	if from_pad and event is InputEventJoypadButton or from_key:
		gamepad = from_pad
	get_viewport().set_input_as_handled()
	if _pressed(event, InputActions.MENU_UP):
		_move(-1)
	elif _pressed(event, InputActions.MENU_DOWN):
		_move(1)
	elif _pressed(event, InputActions.MENU_LEFT):
		_step(_focus, -1)
	elif _pressed(event, InputActions.MENU_RIGHT):
		_step(_focus, 1)
	elif event.is_action_pressed(InputActions.MENU_ACCEPT):
		_activate(_focus, 1)
	elif event.is_action_pressed(InputActions.MENU_ALT):
		if _focus >= 0 and not alt_label.is_empty() and _rows[_focus]["kind"] == &"action":
			alt_chosen.emit(_rows[_focus]["id"])
	elif event.is_action_pressed(InputActions.CANCEL):
		if not back_label.is_empty():
			cancelled.emit()


## 方向動作是否剛按下：鍵盤允許長按連發；搖桿軸只在推過門檻的那一刻算一次。
func _pressed(event: InputEvent, action: StringName) -> bool:
	if event is InputEventJoypadMotion:
		if not event.is_action(action):
			return false
		var now := event.get_action_strength(action) > 0.5
		var before: bool = _axis_held.get(action, false)
		_axis_held[action] = now
		if now:
			gamepad = true
		return now and not before
	return event.is_action_pressed(action, true)


func _move(direction: int) -> void:
	if _focus < 0:
		return
	var index := _focus
	for i in _rows.size():
		index = posmod(index + direction, _rows.size())
		if _can_focus(index):
			break
	_set_focus(index)


func _set_focus(index: int) -> void:
	if index == _focus or index < 0 or not _can_focus(index):
		return
	_focus = index
	_resize()


## 左右調整：選項換前一個或後一個（到頭繞回），數值加減一格。
func _step(index: int, direction: int) -> void:
	if index < 0:
		return
	var row := _rows[index]
	if row["kind"] == &"choice":
		var options: Array = row["options"]
		var at := 0
		for i in options.size():
			if options[i][0] == row["value"]:
				at = i
		row["value"] = options[posmod(at + direction, options.size())][0]
	elif row["kind"] == &"slider":
		var next := clampf(snappedf((row["value"] as float) + (row["step"] as float) * direction, row["step"]), row["low"], row["high"])
		if is_equal_approx(next, row["value"]):
			return
		row["value"] = next
	else:
		return
	changed.emit(row["id"], row["value"])
	queue_redraw()


func _activate(index: int, direction: int) -> void:
	if index < 0 or not _can_focus(index):
		return
	match _rows[index]["kind"]:
		&"action":
			chosen.emit(_rows[index]["id"])
		&"choice":
			_step(index, direction)
		&"text":
			_begin_edit(index)


func _begin_edit(index: int) -> void:
	var row := _rows[index]
	var value := _value_rect(_rects[index])
	_editing = index
	_edit.max_length = row["max_length"]
	_edit.text = row["value"]
	_edit.position = value.position
	_edit.size = value.size
	_edit.visible = true
	_edit.grab_focus()
	_edit.select_all()
	queue_redraw()


## 結束打字；keep 為真時收下內容（空白或不合規則時維持原值）。
func _end_edit(keep: bool) -> void:
	if _editing < 0:
		return
	var row := _rows[_editing]
	_editing = -1
	_edit.visible = false
	var typed := _edit.text.strip_edges()
	if row["digits_only"]:
		var digits := ""
		for ch in typed:
			if ch >= "0" and ch <= "9":
				digits += ch
		typed = digits
	if keep and not typed.is_empty() and typed != row["value"]:
		row["value"] = typed
		changed.emit(row["id"], typed)
	queue_redraw()


func _row_at(point: Vector2) -> int:
	for index: int in _rects:
		if (_rects[index] as Rect2).has_point(point):
			return index
	return -1


func _slide_to(index: int, x: float) -> void:
	var row := _rows[index]
	var value := _value_rect(_rects[index])
	var track_width := value.size.x - 62.0 * _unit
	var t := clampf((x - value.position.x) / track_width, 0.0, 1.0)
	var next := clampf(snappedf(lerpf(row["low"], row["high"], t), row["step"]), row["low"], row["high"])
	if not is_equal_approx(next, row["value"]):
		row["value"] = next
		changed.emit(row["id"], next)
		queue_redraw()


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		if event.device != MapCamera.PAD_DEVICE:
			gamepad = false
		if _sliding and _pressed_row >= 0:
			_slide_to(_pressed_row, event.position.x)
		elif _editing < 0:
			_set_focus(_row_at(event.position))
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		accept_event()
		var index := _row_at(event.position)
		if event.pressed:
			if _editing >= 0:
				_end_edit(true)
			if index < 0 or not _can_focus(index):
				return
			_set_focus(index)
			_pressed_row = index
			_sliding = _rows[index]["kind"] == &"slider" and _value_rect(_rects[index]).has_point(event.position)
			if _sliding:
				_slide_to(index, event.position.x)
			queue_redraw()
		else:
			var was := _pressed_row
			var slid := _sliding
			_pressed_row = -1
			_sliding = false
			queue_redraw()
			if was >= 0 and was == index and not slid:
				# 選項：點左半換前一個，點右半換後一個。
				var left_half: bool = _rows[was]["kind"] == &"choice" and event.position.x < _value_rect(_rects[was]).get_center().x
				_activate(was, -1 if left_half else 1)
	elif event is InputEventMouseButton and event.pressed and event.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_DOWN]:
		accept_event()
		_move(-1 if event.button_index == MOUSE_BUTTON_WHEEL_UP else 1)


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and _pressed_row >= 0 and not _sliding:
		_pressed_row = -1
		queue_redraw()
