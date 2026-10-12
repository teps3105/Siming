class_name MenuDeck
extends Control
## GDD 9.1 選單：主選單與遊戲中選單共用的一疊頁面——開新世界（4.6）、存讀檔與分享碼（4.8）、設定、製作群。
## 只管頁面與存檔、設定的讀寫；進入世界、回到標題與關閉選單由外面接訊號處理。

## 新開的或讀出來的世界已經備妥。
signal world_ready(world: SimWorld, loaded: bool)
## 遊戲中選單關掉了。
signal resumed
signal title_requested
## 介面縮放改了，外面的器物要跟著重排。
signal layout_changed
## 玩家要重看引導。
signal guide_reset

## 讀檔每幀最多花的時間（毫秒）與每次重演的月數。
const LOAD_BUDGET_MSEC := 12
const LOAD_SLICE_MONTHS := 60
const EDGE_GAP := 28.0
const DISPLAY_KEYS: Array[String] = ["fullscreen", "window_size", "vsync"]
const SIZES: Array[String] = ["small", "medium", "large"]
const ERAS: Array[String] = ["ancient", "middle", "decline"]
const ROOT_ICONS := {
	&"new": &"sun", &"continue": &"play", &"load": &"scroll", &"save": &"brush", &"share": &"knot",
	&"settings": &"dial", &"book": &"book", &"credits": &"seal", &"quit": &"door", &"resume": &"play", &"to_title": &"back",
}

## 是否為遊戲中選單（否則是主選單）。
var in_game := false
## 遊戲中選單所屬的世界（存檔與匯出分享碼用）。
var world: SimWorld
## 選單左上角的位置（畫面座標）與放大倍數；x 為負時置中並襯一張紙（遊戲中選單），否則直接寫在畫面上（標題畫面）。
var panel_origin := Vector2(-1.0, -1.0)
var panel_zoom := 1.0
## 排版前呼叫（畫面大小、版面倍率），讓外面依畫面決定上面兩個值。
var arrange := Callable()
## 剪貼簿的讀與寫；測試會換成自己的。
var clipboard_get := func() -> String: return DisplayServer.clipboard_get()
var clipboard_set := func(text: String) -> void: DisplayServer.clipboard_set(text)

var _panel: MenuPanel
var _unit := 1.0
## 目前的頁面與一路走來的頁面；每一項是 [頁名, 離開那一頁時停在哪一列]。
var _page := &""
var _trail: Array = []
var _options := {}
var _name_custom := false
var _loading: SimWorld
var _confirm := Callable()
var _notice := ""


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_STOP
	_panel = MenuPanel.new()
	add_child(_panel)
	_panel.chosen.connect(_on_chosen)
	_panel.alt_chosen.connect(_on_alt)
	_panel.changed.connect(_on_changed)
	_panel.cancelled.connect(_back)
	_panel.resized.connect(_place)
	get_viewport().size_changed.connect(_relayout)
	_relayout()
	open_root()


## 回到第一頁（主選單或遊戲中選單）。
func open_root() -> void:
	_trail.clear()
	_loading = null
	_show(&"root")


## 目前在哪一頁（測試用）。
func page() -> StringName:
	return _page


func panel() -> MenuPanel:
	return _panel


func _draw() -> void:
	if in_game:
		draw_rect(Rect2(Vector2.ZERO, size), Color(Ink.INK, 0.35), true)


func _relayout() -> void:
	var bounds := get_viewport().get_visible_rect().size
	_unit = Ink.layout_unit(bounds, GameSettings.ui_scale())
	if arrange.is_valid():
		arrange.call(bounds, _unit)
	position = Vector2.ZERO
	size = bounds
	var pinned := panel_origin.x >= 0.0
	_panel.paper = not pinned
	_panel.max_height = bounds.y - EDGE_GAP - (panel_origin.y if pinned else EDGE_GAP)
	_panel.setup(_unit * panel_zoom)
	_place()
	queue_redraw()


func _place() -> void:
	var bounds := get_viewport().get_visible_rect().size
	var at := panel_origin if panel_origin.x >= 0.0 else (bounds - _panel.size) * 0.5
	_panel.position = at.clamp(Vector2(EDGE_GAP, EDGE_GAP), (bounds - _panel.size - Vector2(EDGE_GAP, EDGE_GAP)).max(Vector2(EDGE_GAP, EDGE_GAP))).round()


func _process(_delta: float) -> void:
	if _loading == null:
		return
	var deadline := Time.get_ticks_msec() + LOAD_BUDGET_MSEC
	while Time.get_ticks_msec() < deadline:
		var state: Dictionary = _loading.load_step(LOAD_SLICE_MONTHS)
		_panel.set_value(&"progress", state["progress"])
		if state["done"]:
			var loaded := _loading
			_loading = null
			if (state["error"] as String).is_empty():
				world_ready.emit(loaded, true)
			else:
				_tell(state["error"])
			return


func _push(next: StringName) -> void:
	_trail.append([_page, _panel.focused_id()])
	_show(next)


func _back() -> void:
	if _trail.is_empty():
		if in_game:
			resumed.emit()
		return
	var last: Array = _trail.pop_back()
	_show(last[0], last[1])


## 畫出某一頁；focus_id 指定停在哪一列。
func _show(next: StringName, focus_id := &"") -> void:
	_page = next
	_panel.alt_label = ""
	_panel.back_label = Locale.t("menu_back")
	var rows: Array[Dictionary] = []
	match next:
		&"root":
			rows = _root_rows()
			_panel.back_label = Locale.t("pause_resume") if in_game else ""
		&"new":
			rows = _new_rows()
		&"load":
			rows = _load_rows()
			_panel.alt_label = Locale.t("load_delete")
		&"save":
			rows = _save_rows()
		&"settings":
			rows = _settings_rows()
		&"share":
			rows = _share_rows()
		&"credits":
			rows = _credits_rows()
		&"notice":
			rows = [MenuPanel.line(_notice), MenuPanel.action(&"back", Locale.t("menu_back"), &"back")]
			focus_id = &"back"
	_panel.open(rows, focus_id)


func _root_rows() -> Array[Dictionary]:
	var has_save := not SaveStore.latest().is_empty()
	var rows: Array[Dictionary] = []
	if in_game:
		rows.append(_root(&"resume", "pause_resume"))
		rows.append(_root(&"save", "pause_save"))
		rows.append(_root(&"load", "main_load", has_save))
	else:
		rows.append(_root(&"new", "main_new"))
		rows.append(_root(&"continue", "main_continue", has_save, _latest_note()))
		rows.append(_root(&"load", "main_load", has_save))
	rows.append(_root(&"share", "main_share"))
	rows.append(_root(&"settings", "main_settings"))
	if not in_game:
		rows.append(_root(&"book", "main_book", false, Locale.t("menu_locked")))
		rows.append(_root(&"credits", "main_credits"))
	else:
		rows.append(_root(&"to_title", "pause_to_title"))
	rows.append(_root(&"quit", "main_quit"))
	return rows


func _root(id: StringName, key: String, enabled := true, note := "") -> Dictionary:
	return MenuPanel.action(id, Locale.t(key), ROOT_ICONS[id], note, enabled)


func _latest_note() -> String:
	var latest := SaveStore.latest()
	return "" if latest.is_empty() else "%s　%s" % [latest["name"], latest["date"]]


func _new_rows() -> Array[Dictionary]:
	return [
		MenuPanel.text_field(&"new.name", Locale.t("new_name"), _options["name"], 8),
		MenuPanel.text_field(&"new.seed", Locale.t("new_seed"), str(_options["seed"]), 12, true),
		MenuPanel.action(&"new.random", Locale.t("new_random"), &"dice"),
		MenuPanel.choice(&"new.size", Locale.t("new_size"), _named(SIZES, "size_"), _options["size"]),
		MenuPanel.slider(&"new.land_ratio", Locale.t("new_land"), _options["land_ratio"], 0.25, 0.55, 0.01, "%.0f%%", 100.0),
		MenuPanel.slider(&"new.relief", Locale.t("new_relief"), _options["relief"], 0.0, 1.0, 0.05),
		MenuPanel.slider(&"new.humidity", Locale.t("new_humidity"), _options["humidity"], 0.6, 1.4, 0.05),
		MenuPanel.choice(&"new.era", Locale.t("new_era"), _named(ERAS, "era_"), _options["era"]),
		MenuPanel.action(&"new.start", Locale.t("new_start"), &"sun"),
	]


## 一串值配上各自的顯示名稱（語系鍵為 prefix + 值）。
static func _named(values: Array[String], prefix: String) -> Array:
	var out := []
	for value in values:
		out.append([value, Locale.t(prefix + value)])
	return out


## 開局選項回到 GDD 4.6 的預設值，種子與世界名隨機。
func _reset_options() -> void:
	_options = {"size": "medium", "land_ratio": 0.4, "relief": 0.5, "humidity": 1.0, "era": "middle"}
	_name_custom = false
	_roll_seed()


func _roll_seed() -> void:
	_set_seed((randi() % 1000000) * 1000000 + randi() % 1000000)


## 換種子；世界名沒被玩家改過時跟著換成這個種子取的名字。
func _set_seed(value: int) -> void:
	_options["seed"] = value
	if not _name_custom:
		_options["name"] = SimWorld.random_world_name(value)


func _slot_label(slot: int) -> String:
	return Locale.t("slot_label") % Locale.t("slot_numbers")[slot]


func _entry_note(entry: Dictionary) -> String:
	var when := Time.get_datetime_dict_from_unix_time(int(entry["saved_at"]) + Time.get_time_zone_from_system()["bias"] * 60)
	return "%s　%s　%02d/%02d %02d:%02d" % [entry["name"], entry["date"], when["month"], when["day"], when["hour"], when["minute"]]


func _load_rows() -> Array[Dictionary]:
	var rows: Array[Dictionary] = []
	var autos := SaveStore.auto_saves()
	if not autos.is_empty():
		rows.append(MenuPanel.head(Locale.t("load_auto")))
		for entry in autos:
			rows.append(MenuPanel.action(StringName("load:" + (entry["path"] as String)), Locale.t("load_auto"), &"scroll", _entry_note(entry)))
	var manual: Array[Dictionary] = []
	var slots := SaveStore.manual_slots()
	for slot in slots.size():
		if not slots[slot].is_empty():
			manual.append(MenuPanel.action(StringName("load:" + (slots[slot]["path"] as String)), _slot_label(slot), &"scroll", _entry_note(slots[slot])))
	if not manual.is_empty():
		rows.append(MenuPanel.head(Locale.t("load_manual")))
		rows.append_array(manual)
	if rows.is_empty():
		rows.append(MenuPanel.line(Locale.t("load_empty"), true))
	return rows


func _save_rows() -> Array[Dictionary]:
	var rows: Array[Dictionary] = []
	var slots := SaveStore.manual_slots()
	for slot in slots.size():
		var note := Locale.t("slot_empty") if slots[slot].is_empty() else _entry_note(slots[slot])
		rows.append(MenuPanel.action(StringName("save:%d" % slot), _slot_label(slot), &"brush", note))
	return rows


func _settings_rows() -> Array[Dictionary]:
	var switch := [[true, Locale.t("switch_on")], [false, Locale.t("switch_off")]]
	var windows := []
	for i in GameSettings.WINDOW_SIZES.size():
		windows.append([i, "%d x %d" % [GameSettings.WINDOW_SIZES[i].x, GameSettings.WINDOW_SIZES[i].y]])
	var scales := []
	var percent := int(round(GameSettings.UI_SCALE_MIN * 100.0))
	while percent <= int(round(GameSettings.UI_SCALE_MAX * 100.0)):
		scales.append([percent, "%d%%" % percent])
		percent += 10
	return [
		MenuPanel.choice(&"set.fullscreen", Locale.t("set_fullscreen"), switch, GameSettings.flag("fullscreen")),
		MenuPanel.choice(&"set.window_size", Locale.t("set_window"), windows, GameSettings.value("window_size")),
		MenuPanel.choice(&"set.vsync", Locale.t("set_vsync"), switch, GameSettings.flag("vsync")),
		MenuPanel.choice(&"set.ui_scale", Locale.t("set_ui_scale"), scales, int(round(GameSettings.ui_scale() * 10.0)) * 10),
		MenuPanel.choice(&"set.edge_scroll", Locale.t("set_edge"), switch, GameSettings.flag("edge_scroll")),
		MenuPanel.choice(&"set.language", Locale.t("set_language"), [["zh-Hant", Locale.t("language_zh_hant")]], "zh-Hant"),
		MenuPanel.action(&"set.tutorial", Locale.t("set_tutorial"), &"book"),
	]


func _share_rows() -> Array[Dictionary]:
	var rows: Array[Dictionary] = [MenuPanel.line(Locale.t("share_about"), true)]
	if in_game:
		rows.append(MenuPanel.action(&"share.copy", Locale.t("share_copy"), &"knot"))
	rows.append(MenuPanel.action(&"share.paste", Locale.t("share_paste"), &"scroll"))
	return rows


func _credits_rows() -> Array[Dictionary]:
	var rows: Array[Dictionary] = [MenuPanel.head(Locale.t("credits_made")), MenuPanel.line(Locale.t("credits_author"))]
	rows.append(MenuPanel.head(Locale.t("credits_third")))
	for key: String in ["credits_font", "credits_logo_font", "credits_chars", "credits_engine", "credits_bridge"]:
		rows.append(MenuPanel.line(Locale.t(key)))
	return rows


func _on_chosen(id: StringName) -> void:
	var text := String(id)
	if text.begins_with("load:"):
		_load_file(text.trim_prefix("load:"))
		return
	if text.begins_with("save:"):
		var slot := int(text.trim_prefix("save:"))
		if SaveStore.manual_slots()[slot].is_empty():
			_save_to(slot)
		else:
			_ask("confirm_overwrite", func() -> void: _save_to(slot))
		return
	match id:
		&"new":
			_reset_options()
			_push(&"new")
		&"continue":
			_load_file(SaveStore.latest()["path"])
		&"load", &"save", &"share", &"settings", &"credits":
			_push(id)
		&"resume":
			resumed.emit()
		&"to_title":
			_ask("confirm_leave", func() -> void: title_requested.emit())
		&"quit":
			if in_game:
				_ask("confirm_leave", func() -> void: get_tree().quit())
			else:
				get_tree().quit()
		&"back", &"confirm.no":
			_back()
		&"confirm.yes":
			var todo := _confirm
			_confirm = Callable()
			_back()
			todo.call()
		&"new.random":
			_roll_seed()
			_show(&"new", &"new.random")
		&"new.start":
			_start_world()
		&"set.tutorial":
			GameSettings.set_value("tutorial_done", false)
			GameSettings.set_value("tutorial_step", 0)
			guide_reset.emit()
			_show(&"settings", &"set.tutorial")
			_panel.set_note(&"set.tutorial", Locale.t("set_tutorial_note"))
		&"share.copy":
			clipboard_set.call(SaveStore.share_code(world.save_text()))
			_panel.set_note(&"share.copy", Locale.t("share_copied"))
		&"share.paste":
			var parsed := SaveStore.parse_share_code(clipboard_get.call())
			if (parsed["error"] as String).is_empty():
				_load_text(parsed["text"])
			else:
				_tell(parsed["error"])


## 第二個動作：在載入頁刪除停留的那份存檔。
func _on_alt(id: StringName) -> void:
	var text := String(id)
	if _page == &"load" and text.begins_with("load:"):
		_ask("confirm_delete", func() -> void:
			SaveStore.delete(text.trim_prefix("load:"))
			_show(&"load"))


func _on_changed(id: StringName, value: Variant) -> void:
	var text := String(id)
	if text.begins_with("set."):
		var key := text.trim_prefix("set.")
		if key == "language":
			return
		GameSettings.set_value(key, float(value) / 100.0 if key == "ui_scale" else value)
		if key in DISPLAY_KEYS:
			GameSettings.apply_display(get_window())
		if key == "ui_scale":
			_relayout()
			layout_changed.emit()
	elif id == &"new.name":
		_options["name"] = value
		_name_custom = true
	elif id == &"new.seed":
		_set_seed(int(value))
		_show(&"new", &"new.seed")
	elif text.begins_with("new."):
		_options[text.trim_prefix("new.")] = value


## 問一句再做：確定就執行 todo，取消回到原來那一頁。
func _ask(text_key: String, todo: Callable) -> void:
	_confirm = todo
	_trail.append([_page, _panel.focused_id()])
	_page = &"confirm"
	_panel.alt_label = ""
	_panel.back_label = Locale.t("confirm_no")
	_panel.open([
		MenuPanel.line(Locale.t(text_key)),
		MenuPanel.action(&"confirm.yes", Locale.t("confirm_yes"), &"seal"),
		MenuPanel.action(&"confirm.no", Locale.t("confirm_no"), &"back"),
	], &"confirm.no")


## 告知一件事（讀檔失敗、分享碼不對）；返回後回到原來那一頁。
func _tell(text: String) -> void:
	_notice = text
	if _page != &"busy":
		_trail.append([_page, _panel.focused_id()])
	_show(&"notice")


## 等候頁：不能返回；with_bar 為真時帶一條進度墨條。
func _busy(text_key: String, with_bar: bool) -> void:
	if _page != &"busy":
		_trail.append([_page, _panel.focused_id()])
	_page = &"busy"
	_panel.alt_label = ""
	_panel.back_label = ""
	var rows: Array[Dictionary] = [MenuPanel.line(Locale.t(text_key))]
	if with_bar:
		rows.append(MenuPanel.bar(&"progress", "", 0.0))
	_panel.open(rows)


func _save_to(slot: int) -> void:
	var error := SaveStore.save_manual(slot, world)
	if error.is_empty():
		_show(&"save", StringName("save:%d" % slot))
	else:
		_tell(error)


## GDD 4.6 開天：先把等候頁畫出來，再生成世界（生成期間畫面不動）。
func _start_world() -> void:
	_busy("busy_genesis", false)
	await get_tree().process_frame
	await get_tree().process_frame
	var created := SimWorld.new()
	var error := created.create_with(_options)
	if error.is_empty():
		world_ready.emit(created, false)
	else:
		_tell(error)


func _load_file(path: String) -> void:
	_load_text(SaveStore.read_text(path))


## GDD 4.8 讀檔：重新開天後分段重演，期間顯示進度。
func _load_text(text: String) -> void:
	_busy("busy_load", true)
	await get_tree().process_frame
	await get_tree().process_frame
	var target := SimWorld.new()
	var error := target.load_begin(text)
	if error.is_empty():
		_loading = target
	else:
		_tell(error)
