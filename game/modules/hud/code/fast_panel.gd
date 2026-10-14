class_name FastPanel
extends Control
## GDD 2.6 / 5.7 世界快進的兩張紙簽，畫面中央，一次只開一張：
## 終點選單（下一件世界大事、關注者的下一件事、1／10／100／1000 年）與快進停下時的摘要
## （跨過多久、世界大事與改元、各類件數、人口與修士數前後對照）。
## 鍵鼠、搖桿都能選：上下選列、確定、取消；摘要的世界大事列可點開事件卡。

## 選了終點：stop 是 SimHost.Stop，years 只在指定年數用。
signal choice_made(stop: int, years: int)
## 點了摘要裡的一列世界大事。
signal event_requested(id: int)
## 這張簽關掉了（取消選單、或讀完摘要）。
signal dismissed

const MENU_WIDTH := 300.0
const SUMMARY_WIDTH := 480.0
const ROW_HEIGHT := 32.0
const ROW_GAP := 6.0
const PAD := 18.0
## 摘要最多列幾列世界大事與改元，其餘併成一句。
const WORLD_ROWS_MAX := 6
## 指定年數的四個選項。
const YEAR_CHOICES: Array[int] = [1, 10, 100, 1000]
## 摘要的各類件數：標籤鍵 → 算進去的事件種類。
const COUNT_GROUPS := [
	["fast_count_ascended", ["Ascended"]],
	["fast_count_founded", ["PowerFounded"]],
	["fast_count_dynasty", ["DynastyFounded", "DynastyCollapsed", "Usurpation"]],
	["fast_count_death", ["Died", "Killed"]],
	["fast_count_tribulation", ["Tribulation"]],
	["fast_count_famine", ["Famine"]],
]

var gamepad := false:
	set(value):
		gamepad = value
		queue_redraw()

var _sim: SimHost
var _unit := 1.0
var _mode := &""
var _rows: Array[Dictionary] = []
var _lines: Array[Dictionary] = []
var _panel := Rect2()
var _selected := 0
var _hover := -1
var _summary: Dictionary = {}


func setup(sim: SimHost, unit: float) -> void:
	_sim = sim
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	# Hud 是 CanvasLayer，沒有父控制項可撐，簽自己跟著視窗大小。
	position = Vector2.ZERO
	size = get_viewport_rect().size
	get_viewport().size_changed.connect(func() -> void:
		size = get_viewport_rect().size
		if _mode == &"menu":
			_build_menu()
		elif _mode == &"summary":
			_build_summary()
		queue_redraw())
	visible = false


## 開著的是哪一張：&"menu"、&"summary"，都沒開是 &""。
func mode() -> StringName:
	return _mode


func is_open() -> bool:
	return _mode != &""


func summary() -> Dictionary:
	return _summary


## 目前可操作的列（測試用）：每列有 kind（"choice"、"event"、"close"）、rect、enabled，選項另有 stop、years，事件列有 id。
func rows() -> Array[Dictionary]:
	return _rows


func selected_index() -> int:
	return _selected


## 全域座標：第 index 列的中心。
func row_center(index: int) -> Vector2:
	return global_position + (_rows[index]["rect"] as Rect2).get_center()


func open_menu() -> void:
	_mode = &"menu"
	_summary = {}
	_build_menu()
	_show()


func show_summary(summary: Dictionary) -> void:
	_mode = &"summary"
	_summary = summary
	_build_summary()
	_show()


func close() -> void:
	if _mode == &"":
		return
	_mode = &""
	_rows.clear()
	_lines.clear()
	visible = false
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	dismissed.emit()


## 重建器物後把開著的簽帶到新的器物上，不發 dismissed。
func restore(mode: StringName, summary: Dictionary) -> void:
	if mode == &"menu":
		_mode = mode
		_build_menu()
		_show()
	elif mode == &"summary":
		show_summary(summary)


## 鍵盤與搖桿：上下選列、確定、取消。有處理回傳 true。
func handle_action(event: InputEvent) -> bool:
	if not is_open():
		return false
	if event.is_action_pressed(InputActions.MENU_UP):
		_move(-1)
	elif event.is_action_pressed(InputActions.MENU_DOWN):
		_move(1)
	elif event.is_action_pressed(InputActions.MENU_ACCEPT):
		activate(_selected)
	elif event.is_action_pressed(InputActions.CANCEL) or event.is_action_pressed(InputActions.MENU):
		close()
	else:
		# 開著時別的快捷鍵都不動，免得選到一半時間被改掉。
		return event.is_pressed() and not event.is_echo()
	return true


## 選了第 index 列。
func activate(index: int) -> void:
	if index < 0 or index >= _rows.size() or not bool(_rows[index]["enabled"]):
		return
	var row := _rows[index]
	match row["kind"]:
		"choice":
			var stop: int = row["stop"]
			var years: int = row["years"]
			_close_silently()
			choice_made.emit(stop, years)
		"event":
			var id: int = row["id"]
			close()
			event_requested.emit(id)
		"close":
			close()


func _close_silently() -> void:
	_mode = &""
	_rows.clear()
	_lines.clear()
	visible = false
	mouse_filter = Control.MOUSE_FILTER_IGNORE


func _show() -> void:
	_selected = _first_enabled()
	_hover = -1
	visible = true
	mouse_filter = Control.MOUSE_FILTER_STOP
	queue_redraw()


func _first_enabled() -> int:
	for i in _rows.size():
		if bool(_rows[i]["enabled"]):
			return i
	return 0


func _move(step: int) -> void:
	if _rows.is_empty():
		return
	var at := _selected
	for i in _rows.size():
		at = posmod(at + step, _rows.size())
		if bool(_rows[at]["enabled"]):
			_selected = at
			break
	queue_redraw()


# ── 版面 ──────────────────────────────────────────────────────

func _body() -> int:
	return Ink.font_size(Ink.SIZE_BODY, _unit)


func _note() -> int:
	return Ink.font_size(Ink.SIZE_NOTE, _unit)


func _center_rect(width: float, height: float) -> Rect2:
	var view := get_viewport_rect().size if is_inside_tree() else Vector2(1920, 1080)
	var size_px := Vector2(width * _unit, height)
	return Rect2(((view - size_px) * 0.5).round(), size_px)


func _build_menu() -> void:
	_rows.clear()
	_lines.clear()
	var width := MENU_WIDTH * _unit
	var row_h := ROW_HEIGHT * _unit
	var gap := ROW_GAP * _unit
	var pad := PAD * _unit
	var title_h := _body() + 14.0 * _unit
	var choices: Array[Dictionary] = [
		{"label": Locale.t("fast_world_event"), "stop": SimHost.Stop.WORLD_EVENT, "years": 0},
		{"label": Locale.t("fast_followed"), "stop": SimHost.Stop.FOLLOWED, "years": 0},
	]
	for years in YEAR_CHOICES:
		choices.append({"label": Locale.t("fast_years").format({"n": years}), "stop": SimHost.Stop.YEARS, "years": years})
	var foot_h := Ink.KEY_HEIGHT * _unit + 10.0 * _unit
	var height := pad + title_h + choices.size() * (row_h + gap) + foot_h + pad * 0.5
	_panel = _center_rect(MENU_WIDTH, height)
	var y := _panel.position.y + pad + title_h
	for choice in choices:
		var enabled := _sim != null and _sim.can_fast_to(choice["stop"])
		var rect := Rect2(Vector2(_panel.position.x + pad, y), Vector2(width - pad * 2.0, row_h))
		_rows.append({"kind": "choice", "rect": rect, "enabled": enabled, "stop": choice["stop"], "years": choice["years"], "label": choice["label"]})
		y += row_h + gap


func _build_summary() -> void:
	_rows.clear()
	_lines.clear()
	var s := _summary
	var width := SUMMARY_WIDTH * _unit
	var pad := PAD * _unit
	var body := _body()
	var note := _note()
	var line_h := body + 10.0 * _unit
	var inner := width - pad * 2.0
	# 先把要畫的東西排成一串：text（文字）、event（可點的世界大事列）、section（分節）、pair（左標籤右數字）。
	var items: Array[Dictionary] = []
	items.append({"t": "title", "text": Locale.t("fast_summary_title")})
	items.append({"t": "text", "text": Locale.t("fast_span").format({"span": span_text(s)}), "size": body})
	for line in range_lines(s):
		items.append({"t": "text", "text": line, "size": note, "color": Ink.INK_LIGHT})
	items.append({"t": "section", "text": Locale.t("fast_section_world")})
	var world_rows := world_row_texts(s)
	if world_rows.is_empty():
		items.append({"t": "text", "text": Locale.t("fast_none"), "size": note, "color": Ink.INK_LIGHT})
	for i in mini(world_rows.size(), WORLD_ROWS_MAX):
		items.append({"t": "event", "text": world_rows[i]["text"], "id": world_rows[i]["id"]})
	if world_rows.size() > WORLD_ROWS_MAX:
		items.append({"t": "text", "text": Locale.t("fast_more_world").format({"n": world_rows.size() - WORLD_ROWS_MAX}), "size": note, "color": Ink.INK_LIGHT})
	items.append({"t": "section", "text": Locale.t("fast_section_counts")})
	var counts := count_pairs(s)
	if counts.is_empty():
		items.append({"t": "text", "text": Locale.t("fast_none"), "size": note, "color": Ink.INK_LIGHT})
	for pair in counts:
		items.append({"t": "pair", "left": pair[0], "right": Locale.t("fast_count_unit").format({"n": pair[1]})})
	items.append({"t": "section", "text": Locale.t("fast_section_compare")})
	items.append({"t": "pair", "left": Locale.t("fast_people"), "right": Locale.t("fast_compare").format({"a": group(int(s.get("people0", 0))), "b": group(int(s.get("people1", 0)))})})
	items.append({"t": "pair", "left": Locale.t("fast_population"), "right": Locale.t("fast_compare").format({"a": big_number(float(s.get("population0", 0.0))), "b": big_number(float(s.get("population1", 0.0)))})})
	var height := pad * 2.0 + Ink.KEY_HEIGHT * _unit + 24.0 * _unit
	for item in items:
		height += line_h + (8.0 * _unit if item["t"] == "section" else 0.0) + (6.0 * _unit if item["t"] == "title" else 0.0)
	_panel = _center_rect(SUMMARY_WIDTH, height)
	var y := _panel.position.y + pad
	for item in items:
		if item["t"] == "section":
			y += 8.0 * _unit
		var rect := Rect2(Vector2(_panel.position.x + pad, y), Vector2(inner, line_h))
		item["rect"] = rect
		_lines.append(item)
		if item["t"] == "event":
			_rows.append({"kind": "event", "rect": rect, "enabled": true, "id": item["id"]})
		y += line_h + (6.0 * _unit if item["t"] == "title" else 0.0)
	var close_w := 120.0 * _unit
	var close_rect := Rect2(Vector2(_panel.get_center().x - close_w * 0.5, _panel.end.y - pad - Ink.KEY_HEIGHT * _unit - 8.0 * _unit - 4.0 * _unit), Vector2(close_w, Ink.KEY_HEIGHT * _unit + 8.0 * _unit))
	_rows.append({"kind": "close", "rect": close_rect, "enabled": true})


# ── 摘要的文字（純函式，測試也用）────────────────────────────

## 跨過多久：「12 年 3 月 5 日」，零的部分略去；不到一日寫「0 日」。
static func span_text(s: Dictionary) -> String:
	var parts: Array[String] = []
	if int(s.get("years", 0)) > 0:
		parts.append(Locale.t("fast_span_year").format({"n": group(int(s["years"]))}))
	if int(s.get("months", 0)) > 0:
		parts.append(Locale.t("fast_span_month").format({"n": s["months"]}))
	if int(s.get("rest_days", 0)) > 0 or parts.is_empty():
		parts.append(Locale.t("fast_span_day").format({"n": s.get("rest_days", 0)}))
	return " ".join(parts)


## 起訖日期各一行：摘要帶 from_parts、to_parts（SimWorld.date_parts）時才有。
static func range_lines(s: Dictionary) -> Array[String]:
	var out: Array[String] = []
	if s.has("from_parts") and s.has("to_parts"):
		out.append(Locale.t("fast_from").format({"date": _parts_label(s["from_parts"])}))
		out.append(Locale.t("fast_to").format({"date": _parts_label(s["to_parts"])}))
	return out


static func _parts_label(parts: Dictionary) -> String:
	return EventText.date_inline(parts["era"], parts["era_year"], parts["month"])


## 期間的世界大事與改元，依時間：每件世界大事一列；開啟紀元的那件另有一列「改元」。回傳 [{text, id}]。
static func world_row_texts(s: Dictionary) -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	for event: Dictionary in s.get("world_events", []):
		var id := int(event["id"])
		out.append({"id": id, "text": "%s　%s" % [EventText.event_inline(event), EventText.world_line(str(event["change"]), "")]})
		var era := str(event.get("era", ""))
		if not era.is_empty():
			out.append({"id": id, "text": Locale.t("fast_era_row").format({"era": era})})
	return out


## 各類件數 [[標籤, 件數]]，件數為零的略去。
static func count_pairs(s: Dictionary) -> Array:
	var out := []
	var kinds: Dictionary = s.get("kinds", {})
	for group_def: Array in COUNT_GROUPS:
		var total := 0
		for kind: String in group_def[1]:
			total += int(kinds.get(kind, 0))
		if total > 0:
			out.append([Locale.t(group_def[0]), total])
	return out


## 千分位：1234567 → 1,234,567。
static func group(n: int) -> String:
	var digits := str(absi(n))
	var out := ""
	for i in digits.length():
		if i > 0 and (digits.length() - i) % 3 == 0:
			out += ","
		out += digits[i]
	return ("-" if n < 0 else "") + out


## 人口：萬、億為單位。
static func big_number(n: float) -> String:
	if n >= 1.0e8:
		return "%.2f 億" % (n / 1.0e8)
	if n >= 1.0e4:
		return "%s 萬" % group(int(n / 1.0e4))
	return group(int(n))


# ── 繪製與滑鼠 ────────────────────────────────────────────────

func _draw() -> void:
	if _mode == &"":
		return
	draw_rect(Rect2(Vector2.ZERO, size), Color(Ink.INK, 0.28), true)
	Ink.paper_slip(self, _panel)
	if _mode == &"menu":
		_draw_menu()
	else:
		_draw_summary()


func _draw_menu() -> void:
	var body := _body()
	var title := Locale.t("fast_menu_title")
	var pad := PAD * _unit
	Ink.text(self, Vector2(_panel.get_center().x - Ink.text_size(title, body).x * 0.5, _panel.position.y + pad * 0.7), title, body, Ink.SEAL_RED)
	for i in _rows.size():
		var row := _rows[i]
		var rect: Rect2 = row["rect"]
		var enabled: bool = row["enabled"]
		var color := Ink.plate(self, rect, i == _selected and enabled, enabled, i == _hover, false)
		var text: String = row["label"]
		Ink.text(self, Vector2(rect.position.x + 12.0 * _unit, rect.get_center().y - Ink.text_size(text, body).y * 0.5), text, body, color)
		if not enabled and int(row["stop"]) == SimHost.Stop.FOLLOWED:
			var note := Locale.t("fast_followed_off")
			Ink.text(self, Vector2(rect.end.x - 10.0 * _unit - Ink.text_size(note, _note()).x, rect.get_center().y - Ink.text_size(note, _note()).y * 0.5), note, _note(), color)
	var foot_y := _panel.end.y - pad * 0.5 - Ink.KEY_HEIGHT * _unit
	var x := _panel.position.x + pad
	x += Ink.key_glyph(self, Vector2(x, foot_y), InputHints.label(InputActions.MENU_UP, gamepad) if gamepad else Ink.KEY_ARROWS_VERTICAL, _unit, not gamepad) + 4.0 * _unit
	Ink.text(self, Vector2(x, foot_y + 2.0 * _unit), Locale.t("fast_hint_pick"), _note(), Ink.INK_LIGHT)
	x += Ink.text_size(Locale.t("fast_hint_pick"), _note()).x + 14.0 * _unit
	var cancel := InputHints.label(InputActions.CANCEL, gamepad)
	x += Ink.key_glyph(self, Vector2(x, foot_y), cancel, _unit, not gamepad) + 4.0 * _unit
	Ink.text(self, Vector2(x, foot_y + 2.0 * _unit), Locale.t("prompt_back"), _note(), Ink.INK_LIGHT)


func _draw_summary() -> void:
	var body := _body()
	for item in _lines:
		var rect: Rect2 = item["rect"]
		match item["t"]:
			"title":
				var text: String = item["text"]
				Ink.text(self, Vector2(rect.get_center().x - Ink.text_size(text, body).x * 0.5, rect.position.y), text, body, Ink.SEAL_RED)
			"section":
				Ink.section(self, rect.position, item["text"], _note())
			"text":
				var size_px: int = item["size"]
				var color: Color = item.get("color", Ink.INK)
				Ink.text(self, rect.position, _clip(item["text"], size_px, rect.size.x), size_px, color)
			"event":
				var index := _row_of(item)
				var hot := index == _hover or index == _selected
				var text := _clip(item["text"], body, rect.size.x - 12.0 * _unit)
				if hot:
					draw_rect(rect, Color(Ink.SEAL_RED, 0.12), true)
				Ink.text(self, Vector2(rect.position.x + 6.0 * _unit, rect.position.y), text, body, Ink.SEAL_RED if hot else Ink.INK)
			"pair":
				Ink.text(self, rect.position + Vector2(10.0 * _unit, 0.0), item["left"], body, Ink.INK_LIGHT)
				var right: String = item["right"]
				Ink.text(self, Vector2(rect.end.x - Ink.text_size(right, body).x, rect.position.y), right, body)
	var close: Dictionary = _rows[_rows.size() - 1]
	var rect: Rect2 = close["rect"]
	var index := _rows.size() - 1
	var color := Ink.plate(self, rect, index == _selected, true, index == _hover, false)
	var label := Locale.t("fast_close")
	Ink.text(self, rect.get_center() - Ink.text_size(label, body) * 0.5, label, body, color)


func _row_of(item: Dictionary) -> int:
	for i in _rows.size():
		if (_rows[i]["rect"] as Rect2) == (item["rect"] as Rect2) and _rows[i]["kind"] == "event":
			return i
	return -1


## 字太長就截斷加刪節號。
func _clip(text: String, size_px: int, max_width: float) -> String:
	if Ink.text_size(text, size_px).x <= max_width:
		return text
	var out := text
	while out.length() > 1 and Ink.text_size(out + "…", size_px).x > max_width:
		out = out.substr(0, out.length() - 1)
	return out + "…"


func _row_at(point: Vector2) -> int:
	for i in _rows.size():
		if (_rows[i]["rect"] as Rect2).grow(2.0).has_point(point):
			return i
	return -1


func _gui_input(event: InputEvent) -> void:
	if _mode == &"":
		return
	if event is InputEventMouseMotion:
		var at := _row_at((event as InputEventMouseMotion).position)
		if at != _hover:
			_hover = at
			if at >= 0 and bool(_rows[at]["enabled"]):
				_selected = at
			queue_redraw()
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var at := _row_at((event as InputEventMouseButton).position)
		if at >= 0:
			activate(at)
		elif _mode == &"menu" and not _panel.grow(Ink.SLIP_BAND).has_point((event as InputEventMouseButton).position):
			close()
		accept_event()
	elif event is InputEventMouseButton:
		accept_event()


## 游標停在這裡時是否可以按（GDD 5.9 游標圖示）。
func is_hot(point: Vector2) -> bool:
	if _mode == &"":
		return false
	var at := _row_at(point)
	return at >= 0 and bool(_rows[at]["enabled"])
