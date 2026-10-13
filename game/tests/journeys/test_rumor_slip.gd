extends "res://tests/harness/suite.gd"
## 傳聞簽（GDD 4.7、5.7）：事件入簽、依重要度篩選與排序、世界大事置頂、點一則飛往、兩種解析度下不壓到其他器物、搖桿操作。

const DECK_VIEW := Vector2i(1280, 800)

var _game: Node
var _sim: SimHost
var _hud: Hud
var _camera: MapCamera
var _rumor: RumorSlip
var _next_id := 1000


func suite_name() -> String:
	return "rumor_slip"


## 縮放測試改過的設定不留給下一個案例（測試用的設定檔在案例之間是同一個）。
func setup() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func teardown() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func _boot(view: Vector2i) -> void:
	_game = await boot(false, view)
	_sim = _game.get_node("SimHost")
	_hud = _game.get_node("Hud")
	_camera = _game.get_node("MapCamera")
	_rumor = _hud.rumor()


## 合成一筆橋接事件。
func _event(kind: String, importance: int, tick: int, extra := {}) -> Dictionary:
	_next_id += 1
	var event := {"id": _next_id, "tick": tick, "kind": kind, "importance": importance, "province": 0, "cell": -1,
		"souls": PackedInt32Array(), "a": "沈青霜", "b": "韓烈", "f": "", "g": "", "p": "青州", "q": "", "x": "", "n": 0, "m": 0, "year": tick / 12 + 1}
	event.merge(extra, true)
	return event


func _post(event: Dictionary) -> void:
	_sim.event_posted.emit(event)


func test_new_events_enter_the_slip_with_their_sentence() -> void:
	await _boot(FLOW_VIEW)
	assert_eq(_rumor.shown().size(), 0, "起始時傳聞簽為空")
	_post(_event("Killed", 3, 100, {"pos": Vector2(40.0, 30.0)}))
	await frames(2)
	var shown := _rumor.shown()
	assert_eq(shown.size(), 1, "重要度 3 的事件入簽")
	assert_eq(shown[0]["line"], "沈青霜殺韓烈於青州", "一句話由 7.2 模板填成")
	assert_eq(shown[0]["seal"], "殺業", "印是 7.2 的印字")
	_post(_event("Born", 1, 101))
	assert_eq(_rumor.entry_count(), 1, "重要度 1 的事件不入簽")


func test_importance_decides_slip_or_list_only() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Married", 2, 100))
	assert_eq(_rumor.shown().size(), 0, "重要度 2 不上簽")
	assert_eq(_rumor.entry_count(), 1, "但留在清單裡")
	_rumor.expanded = true
	assert_eq(_rumor.shown().size(), 1, "展開的清單看得到重要度 2")
	_rumor.expanded = false
	_post(_event("Killed", 3, 101))
	assert_eq(_rumor.shown().size(), 1, "收合時只有重要度 3 以上")


func test_slip_shows_eight_newest_first_and_list_keeps_three_hundred() -> void:
	await _boot(FLOW_VIEW)
	for i in 12:
		_post(_event("Killed", 3, 100 + i * 20, {"souls": PackedInt32Array([i])}))
	var shown := _rumor.shown()
	assert_eq(shown.size(), 8, "收合時最多 8 張")
	assert_eq(shown[0]["tick"], 100 + 11 * 20, "新到舊")
	assert_eq(shown[7]["tick"], 100 + 4 * 20)
	_rumor.expanded = true
	assert_eq(_rumor.shown().size(), 12, "展開看到全部")
	for i in 320:
		_post(_event("Killed", 3, 1000 + i * 20, {"souls": PackedInt32Array([100 + i])}))
	assert_eq(_rumor.entry_count(), 300, "清單留近 300 則")
	assert_eq(_rumor.shown().size(), 300, "全螢幕列出保留的全部，沒有 50 則的上限")
	assert_eq(_rumor.shown()[0]["tick"], 1000 + 319 * 20, "最舊的被擠掉")
	assert_eq(_rumor.shown()[299]["tick"], 1000 + 20 * 20, "留下的最舊一則")


func test_same_importance_orders_by_time_then_id() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Killed", 3, 100, {"souls": PackedInt32Array([1])}))
	_post(_event("Famine", 4, 100, {"souls": PackedInt32Array([2])}))
	var shown := _rumor.shown()
	assert_eq(shown[0]["importance"], 4, "同一時間，重要度高的在前")


func test_world_events_pin_to_the_top_for_a_shi() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Killed", 3, 100, {"souls": PackedInt32Array([1])}))
	_post(_event("WorldChanged", 5, 90, {"change": "ColdBegan", "era": "玄冥", "p": ""}))
	_post(_event("Killed", 3, 200, {"souls": PackedInt32Array([2])}))
	var shown := _rumor.shown()
	assert_true(shown[0]["world"], "世界大事置頂，即使不是最新")
	assert_eq(shown[0]["line"], "天下大寒，改元玄冥", "世界大事用 2.8 的句子")
	assert_eq(shown[1]["tick"], 200)
	_post(_event("Killed", 3, 90 + 360, {"souls": PackedInt32Array([3])}))
	assert_false(_rumor.shown()[0]["world"], "過了一世，世界大事依時間排下去")


func test_world_events_also_reach_the_old_notice_slip() -> void:
	await _boot(FLOW_VIEW)
	_sim.world_changed.emit("ColdBegan", "")
	_post(_event("WorldChanged", 5, 90, {"change": "ColdBegan", "era": ""}))
	assert_true(_hud._notice.showing(), "大事告示照舊")
	assert_eq(_rumor.shown().size(), 1, "世界大事同時進傳聞簽")


func test_one_slip_per_soul_per_year() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Killed", 3, 100, {"souls": PackedInt32Array([5, 6])}))
	_post(_event("Died", 2, 101, {"souls": PackedInt32Array([6]), "x": "Killed", "n": 30}))
	assert_eq(_rumor.entry_count(), 1, "同一魂同一年的兩筆合成一簽")
	assert_eq(_rumor.shown()[0]["importance"], 3, "留下重要度高的")
	_post(_event("Died", 2, 100 + 12, {"souls": PackedInt32Array([6])}))
	assert_eq(_rumor.entry_count(), 2, "隔年不併")


func test_real_world_events_fill_the_slip_with_readable_sentences() -> void:
	await _boot(FLOW_VIEW)
	for shi in 4:
		_sim.advance(360)
	assert_gt(_rumor.entry_count(), 0, "推進四世，傳聞簽有事")
	_rumor.expanded = true
	for entry in _rumor.shown():
		var line: String = entry["line"]
		assert_false(line.is_empty())
		assert_false(line.contains("{") or line.begins_with("event_"), "句子沒有留下未填的模板：" + line)


func test_clicking_a_slip_flies_the_camera_there() -> void:
	await _boot(FLOW_VIEW)
	var target := Vector2(33.0, 21.0)
	_post(_event("Killed", 3, 100, {"pos": target}))
	await frames(2)
	var flown := []
	_rumor.fly_requested.connect(func(p: Vector2) -> void: flown.append(p))
	await click(_rumor.position + _rumor.row_rect(0).get_center())
	assert_eq(flown, [target], "點一則飛往該地")
	assert_lt(_camera._target_focus.distance_to(target), 0.01, "鏡頭的目標焦點是事件地點")


func test_a_slip_without_a_place_does_not_fly() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("WorldChanged", 5, 90, {"change": "ColdBegan", "era": ""}))
	await frames(2)
	var before := _camera._target_focus
	await click(_rumor.position + _rumor.row_rect(0).get_center())
	assert_eq(_camera._target_focus, before, "沒有地點的傳聞點了不動")
	assert_false(_rumor.is_hot(_rumor.row_rect(0).get_center()), "游標不變成可按的手指")


func test_header_key_toggles_the_list_and_escape_closes_it() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Killed", 3, 100))
	assert_eq(_rumor._list_key(), "T", "鍵鼠的簽頭掛展開鍵")
	await key(KEY_T)
	assert_true(_rumor.expanded, "T 展開清單")
	await key(KEY_ESCAPE)
	assert_false(_rumor.expanded, "Esc 先收回清單")
	assert_false(_hud.menu_open(), "收清單的 Esc 不開選單")
	await click(_rumor.position + _rumor.header_rect().get_center())
	assert_true(_rumor.expanded, "點簽頭展開")
	await click(_rumor.position + _rumor.header_rect().get_center())
	assert_false(_rumor.expanded, "再點收合")


func test_expanded_list_scrolls_with_the_wheel() -> void:
	await _boot(FLOW_VIEW)
	for i in 30:
		_post(_event("Killed", 3, 100 + i * 20, {"souls": PackedInt32Array([i])}))
	_rumor.expanded = true
	await frames(2)
	assert_eq(_rumor.size, Vector2(tree.root.get_visible_rect().size), "全螢幕：控制項蓋滿畫面")
	var rows := _rumor.visible_row_count()
	assert_lt(rows, 30, "放不下的列要捲動")
	var first_before: float = _rumor.row_rect(0).position.y
	await wheel(_rumor.position + _rumor.row_rect(2).get_center(), false)
	assert_lt(_rumor.row_rect(0).position.y, first_before, "滾輪往下捲")


func test_gamepad_clicks_through_the_cursor_and_shows_pad_keys() -> void:
	await _boot(FLOW_VIEW)
	var target := Vector2(50.0, 40.0)
	_post(_event("Killed", 3, 100, {"pos": target}))
	await joy_button(JOY_BUTTON_LEFT_STICK)
	await frames(2)
	assert_true(_rumor.gamepad, "搖桿模式切過去")
	assert_eq(_rumor._list_key(), "LT", "搖桿的簽頭畫 LT（游標的左鍵）")
	assert_eq(_rumor._fly_key(), "LT", "飛往的鍵是 LT")
	_camera.set_cursor_position(_rumor.position + _rumor.header_rect().get_center())
	await frames(2)
	await joy_axis(JOY_AXIS_TRIGGER_LEFT, 1.0, 0.05)
	assert_true(_rumor.expanded, "游標移到簽頭按 LT 展開")
	_camera.set_cursor_position(_rumor.position + _rumor.row_rect(0).get_center())
	await frames(2)
	await joy_axis(JOY_AXIS_TRIGGER_LEFT, 1.0, 0.05)
	assert_lt(_camera._target_focus.distance_to(target), 0.01, "游標移到一則按 LT 飛往")
	assert_false(_rumor.expanded, "從清單飛往後收回")
	await joy_button(JOY_BUTTON_B)
	assert_false(_hud.menu_open(), "搖桿的 B 不開選單")


func test_gamepad_edge_hover_scrolls_the_list() -> void:
	await _boot(FLOW_VIEW)
	for i in 30:
		_post(_event("Killed", 3, 100 + i * 20, {"souls": PackedInt32Array([i])}))
	await joy_button(JOY_BUTTON_LEFT_STICK)
	_rumor.expanded = true
	await frames(2)
	var view: Rect2 = _rumor._view_rect()
	_camera.set_cursor_position(_rumor.position + Vector2(view.get_center().x, view.end.y - 6.0))
	await wait(0.6)
	assert_gt(_rumor._scroll, 0.0, "游標停在清單下緣，清單自己往下捲")


func test_slip_stays_off_the_other_instruments_on_desktop() -> void:
	await _check_no_overlap(FLOW_VIEW)


func test_slip_stays_off_the_other_instruments_on_deck() -> void:
	await _check_no_overlap(DECK_VIEW)


## 收合、命簿開著兩種狀態下，傳聞簽不壓到天時儀、題簽、視角鈕、印匣、命簿與引導簽；
## 展開（全螢幕）時紙面四邊各留 24 × 介面縮放，其餘器物都在壓暗的底下。
func _check_no_overlap(view: Vector2i) -> void:
	await _boot(view)
	for i in 10:
		_post(_event("Killed", 3, 100 + i * 20, {"souls": PackedInt32Array([i]), "pos": Vector2(20.0, 20.0)}))
	await frames(3)
	var bounds := Rect2(Vector2.ZERO, Vector2(tree.root.get_visible_rect().size))
	for state in ["收合", "命簿開著", "展開"]:
		_rumor.expanded = state == "展開"
		if state.contains("命簿"):
			_hud._card.show_province(0)
		else:
			_hud._card.close()
		await wait(0.5)
		var mine: Rect2 = _rumor.occupied_rect()
		assert_true(bounds.encloses(mine), "%s %s：傳聞簽在畫面內 %s" % [view, state, mine])
		if state == "展開":
			var edge: float = 24.0 * _hud._unit
			assert_true(_rumor.paper_rect().is_equal_approx(bounds.grow(-edge)), "%s：紙面四邊各留 24 × 縮放 %s" % [view, _rumor.paper_rect()])
			continue
		var others := {
			"天時儀": Rect2(_hud._dial.position, Vector2.ONE * _hud._dial.outer_radius() * 2.0),
			"題簽": _hud._dial._title_rect().grow(Ink.SLIP_BAND).merge(_hud._dial._speed_rect().grow(Ink.SLIP_BAND)),
			"視角鈕": Rect2(_hud._view.position, _hud._view.size),
			"印匣": _hud._seals._seal_rect(0).merge(_hud._seals._seal_rect(SealBox.SEALS.size() - 1)),
		}
		others["題簽"].position += _hud._dial.position
		others["印匣"].position += _hud._seals.position
		if _hud._card.visible:
			others["命簿"] = Rect2(_hud._card.position, _hud._card.size)
		if _hud._guide.visible:
			others["引導簽"] = Rect2(_hud._guide.position, _hud._guide.size)
		for name: String in others:
			assert_false(mine.intersects(others[name]), "%s %s：傳聞簽不壓到%s（%s 對 %s）" % [view, state, name, mine, others[name]])


func test_deck_layout_collapses_to_one_slip_and_keeps_text_readable() -> void:
	await _boot(DECK_VIEW)
	for i in 5:
		_post(_event("Killed", 3, 100 + i * 20, {"souls": PackedInt32Array([i])}))
	assert_eq(_rumor.shown().size(), 1, "Deck 收合為最新一張")
	assert_eq(_rumor.shown()[0]["tick"], 180)
	assert_lt(absf(_rumor.header_rect().size.y - RumorSlip.HEADER_HEIGHT * _hud._unit), 0.01, "尺寸乘縮放倍率")
	assert_lt(absf(_rumor.row_rect(0).size.y - 32.0 * _hud._unit), 0.01, "每張高 32 乘倍率")
	_rumor.expanded = true
	assert_eq(_rumor.shown().size(), 5, "展開看到全部")
	assert_lt(absf(_rumor.row_rect(0).size.y - RumorSlip.FULL_ROW_HEIGHT * _hud._unit), 0.01, "全螢幕每列高乘縮放倍率")


## 收合的簽：句子與年月同一行放不下時，年月移到第二行（列加高），句子取全寬；放得下仍是單行。
func test_collapsed_slip_moves_the_date_to_a_second_line_only_when_it_must() -> void:
	await _boot(DECK_VIEW)
	_post(_event("Killed", 3, 100, {"a": "沈", "b": "韓", "p": "青"}))
	await frames(2)
	var short_row := _rumor.row_rect(0)
	assert_lt(absf(short_row.size.y - 32.0 * _hud._unit), 0.01, "短句單行，列高不變")
	var long_a := "沈青霜青霜青霜青霜青霜青霜青霜"
	_post(_event("Killed", 3, 400, {"a": long_a, "b": "韓烈韓烈韓烈韓烈韓烈", "p": "青州青州青州青州"}))
	await frames(2)
	var row := _rumor.row_rect(0)
	assert_gt(row.size.y, 32.0 * _hud._unit + 1.0, "長句的列加高一行")
	var line := _rumor.line_rect(0)
	assert_gt(line.size.x, short_row.size.x * 0.5, "句子取全寬")
	assert_lt(line.size.y, row.size.y, "句子在第一行")
	assert_true(_rumor.get_rect().size.y >= row.size.y, "簽高容得下加高的列")


func test_desktop_sizes_match_the_layout_table() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Killed", 3, 100))
	_post(_event("Killed", 3, 120, {"souls": PackedInt32Array([1])}))
	await frames(2)
	assert_eq(_rumor.header_rect().size.x, 520.0, "傳聞簽寬 520")
	assert_eq(_rumor.row_rect(0).size.y, 32.0, "每張高 32")
	assert_eq(_rumor.row_rect(1).position.y - _rumor.row_rect(0).end.y, 5.0, "間距 5")
	var bounds := Vector2(tree.root.get_visible_rect().size)
	assert_eq(_rumor.occupied_rect().end.x, bounds.x - 12.0, "右上角內縮 12")
	assert_eq(_rumor.occupied_rect().position.y, 12.0)
	_rumor.expanded = true
	assert_eq(_rumor.paper_rect(), Rect2(24.0, 24.0, 1920.0 - 48.0, 1080.0 - 48.0), "展開是蓋滿畫面的一張紙，四邊各留 24")
	assert_eq(_rumor.row_rect(0).size.y, RumorSlip.FULL_ROW_HEIGHT, "全螢幕每列高")


func test_layout_scale_applies_to_the_slip() -> void:
	await _boot(FLOW_VIEW)
	GameSettings.set_value("ui_scale", 1.5)
	_hud._layout()
	await frames(3)
	_rumor = _hud.rumor()
	assert_eq(_rumor.header_rect().size.x, 780.0, "介面縮放 1.5 倍，寬 520 變 780")


## 世界裡實際存在的前幾位人物編號（傳聞簽「看人物」要開得起來）。
func _valid_people(count: int) -> Array[int]:
	var out: Array[int] = []
	var id := 0
	while out.size() < count and id < 5000:
		if not _sim.world.person(id).is_empty():
			out.append(id)
		id += 1
	return out


func test_fullscreen_covers_the_screen_pauses_time_and_restores_it() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Killed", 3, 100, {"souls": PackedInt32Array([1])}))
	_sim.set_paused(false)
	_sim.set_speed(1)
	await key(KEY_T)
	assert_true(_rumor.expanded, "T 展開")
	assert_eq(_rumor.position, Vector2.ZERO, "控制項從畫面左上角起")
	assert_eq(_rumor.size, Vector2(1920.0, 1080.0), "蓋滿畫面")
	assert_true(_rumor._has_point(Vector2(2.0, 2.0)), "紙外的暗處也攔下輸入，地圖收不到")
	assert_true(_sim.paused, "展開期間模擬暫停")
	# 展開中換了介面縮放（器物重建）也不能把「展開前在跑」記成暫停。
	GameSettings.set_value("ui_scale", 1.5)
	_hud._layout()
	await frames(3)
	_rumor = _hud.rumor()
	assert_true(_rumor.expanded, "重建後仍展開")
	assert_true(_sim.paused)
	await key(KEY_T)
	assert_false(_rumor.expanded, "T 收合")
	assert_false(_sim.paused, "展開前在跑，收合後回到在跑")
	assert_eq(_sim.speed_index, 1, "速度不變")
	# 展開前本來就暫停的，收合後仍暫停。
	_sim.set_paused(true)
	await key(KEY_T)
	await key(KEY_T)
	assert_true(_sim.paused, "展開前暫停，收合後仍暫停")


func test_time_hotkeys_do_nothing_while_fullscreen() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Killed", 3, 100, {"souls": PackedInt32Array([1])}))
	_rumor.expanded = true
	await key(KEY_SPACE)
	assert_true(_sim.paused, "全螢幕時空白鍵不解除暫停")
	await key(KEY_TAB)
	assert_true(_sim.paused, "Tab 不換速也不解除暫停")


func test_clicking_the_dim_area_or_a_row_collapses_and_flies() -> void:
	await _boot(FLOW_VIEW)
	var target := Vector2(33.0, 21.0)
	_post(_event("Killed", 3, 100, {"pos": target, "souls": PackedInt32Array([1])}))
	_sim.set_paused(false)
	_rumor.expanded = true
	await frames(2)
	await click(Vector2(6.0, 6.0))
	assert_false(_rumor.expanded, "點紙外的暗處收合")
	assert_false(_sim.paused, "收合回到展開前的行止")
	_rumor.expanded = true
	await frames(2)
	await click(_rumor.position + _rumor.row_rect(0).get_center())
	assert_false(_rumor.expanded, "點一則：收合")
	assert_lt(_camera._target_focus.distance_to(target), 0.01, "並飛往該地")


func test_arrow_keys_and_pages_scroll_the_list_not_the_map() -> void:
	await _boot(FLOW_VIEW)
	for i in 40:
		_post(_event("Killed", 3, 100 + i * 20, {"souls": PackedInt32Array([i])}))
	_rumor.expanded = true
	await frames(3)
	assert_gt(_rumor._max_scroll(), 0.0, "放不下的列要捲動")
	var focus := _camera._target_focus
	await key(KEY_DOWN)
	assert_gt(_rumor._scroll, 0.0, "方向鍵下捲")
	assert_eq(_camera._target_focus, focus, "方向鍵不平移地圖")
	var before: float = _rumor._scroll
	await key(KEY_PAGEDOWN)
	assert_gt(_rumor._scroll, before + _rumor._view_rect().size.y * 0.5, "PageDown 捲近一頁")
	before = _rumor._scroll
	await key(KEY_PAGEUP)
	assert_lt(_rumor._scroll, before, "PageUp 往回捲")
	for i in 20:
		await key(KEY_PAGEUP)
	assert_eq(_rumor._scroll, 0.0, "捲到頂為止")
	# 滑鼠停在清單下緣不會自己捲（邊緣自捲只給搖桿游標）。
	_rumor._pointer = Vector2(960.0, _rumor._view_rect().end.y - 4.0)
	await wait(0.4)
	assert_eq(_rumor._scroll, 0.0, "鍵鼠不邊緣自捲")


func test_filter_row_narrows_the_fullscreen_list_only() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Killed", 3, 100, {"souls": PackedInt32Array([1])}))
	_post(_event("WorldChanged", 5, 90, {"change": "ColdBegan", "era": ""}))
	_post(_event("PowerFounded", 3, 110, {"souls": PackedInt32Array([2])}))
	_post(_event("DynastyFounded", 4, 120, {"souls": PackedInt32Array([3])}))
	_post(_event("ThreadResolved", 4, 130, {"souls": PackedInt32Array([4]), "n": 1}))
	assert_eq(RumorSlip.FILTERS, ["all", "world", "person", "sect", "dynasty", "karma"], "全部、大事、人物、宗門、王朝、宿緣")
	_rumor.expanded = true
	await frames(2)
	assert_eq(_rumor.shown().size(), 5, "一開始是全部")
	var labels: Array[String] = []
	for i in RumorSlip.FILTERS.size():
		labels.append(_rumor.filter_label(i))
	assert_eq(labels, ["全部", "大事", "人物", "宗門", "王朝", "宿緣"])
	await click(_rumor.position + _rumor.filter_rect(5).get_center())
	assert_eq(_rumor.filter_index(), 5, "點宿緣")
	assert_eq(_rumor.shown().size(), 1)
	assert_eq(_rumor.shown()[0]["kind"], "ThreadResolved")
	await key(KEY_LEFT)
	assert_eq(_rumor.filter_index(), 4, "左鍵退一格")
	assert_eq(_rumor.shown()[0]["kind"], "DynastyFounded")
	await key(KEY_LEFT)
	assert_eq(_rumor.shown()[0]["kind"], "PowerFounded", "宗門")
	await key(KEY_LEFT)
	assert_eq(_rumor.shown()[0]["kind"], "Killed", "人物")
	await key(KEY_LEFT)
	assert_true(_rumor.shown()[0]["world"], "大事")
	await key(KEY_LEFT)
	await key(KEY_LEFT)
	assert_eq(_rumor.filter_index(), 0, "到頭為止，不繞回")
	await key(KEY_RIGHT)
	assert_eq(_rumor.filter_index(), 1, "右鍵進一格")
	assert_eq(_rumor.shown().size(), 1)
	# 篩選只影響全螢幕：收合的簽照舊，再展開從全部開始。
	_rumor.expanded = false
	assert_eq(_rumor.shown().size(), 5, "收合不受篩選影響")
	_rumor.expanded = true
	assert_eq(_rumor.filter_index(), 0, "再展開從全部開始")
	assert_eq(_rumor.shown().size(), 5)


func test_opening_a_card_collapses_the_fullscreen_list() -> void:
	await _boot(FLOW_VIEW)
	var people := _valid_people(1)
	assert_eq(people.size(), 1, "世界裡有人")
	_sim.set_paused(false)
	_rumor.expanded = true
	_hud.open_event(5)
	assert_false(_rumor.expanded, "開事件卡即收合")
	assert_false(_sim.paused, "收合回到展開前的行止")
	_hud._close_cards()
	_rumor.expanded = true
	_hud.open_person(people[0])
	assert_false(_rumor.expanded, "開人物卡即收合")
	assert_true(_hud.person_card().is_open())
	_hud._close_cards()
	# 從「看人物」小印進去也一樣。
	_post(_event("Killed", 3, 100, {"persons": PackedInt32Array([people[0]]), "a_person": people[0], "souls": PackedInt32Array([7])}))
	_rumor.expanded = true
	await frames(2)
	await click(_rumor.position + _rumor.action_slots(0)["person"].get_center())
	assert_true(_hud.person_card().is_open(), "點看人物開卡")
	assert_false(_rumor.expanded, "開卡的同時全螢幕收合")


func test_menu_opens_from_fullscreen_with_the_pause_state_before_the_list() -> void:
	await _boot(FLOW_VIEW)
	_sim.set_paused(false)
	_rumor.expanded = true
	_hud.open_menu()
	assert_false(_rumor.expanded, "開選單先收合")
	_hud.close_menu()
	assert_false(_sim.paused, "關選單後回到展開前在跑")


func test_fullscreen_fits_1920_at_scale_1() -> void:
	await _check_fullscreen_fits(Vector2i(1920, 1080), 1.0)


func test_fullscreen_fits_1920_at_scale_1_5() -> void:
	await _check_fullscreen_fits(Vector2i(1920, 1080), 1.5)


func test_fullscreen_fits_1280_at_scale_1() -> void:
	await _check_fullscreen_fits(Vector2i(1280, 800), 1.0)


func test_fullscreen_fits_1280_at_scale_1_5() -> void:
	await _check_fullscreen_fits(Vector2i(1280, 800), 1.5)


## 全螢幕時：紙面在畫面內、頁首篩選列與每一列都在紙面內、一句話放得進列裡，捲到頂、中段與底都一樣。
func _check_fullscreen_fits(view: Vector2i, ui_scale: float) -> void:
	GameSettings.set_value("ui_scale", ui_scale)
	await _boot(view)
	var people := _valid_people(3)
	for i in 30:
		_post(_event("Killed", 3, 100 + i * 20, {"souls": PackedInt32Array([i]), "pos": Vector2(20.0, 20.0), "a": "鶴微宗長老霍靜沉的弟子", "persons": PackedInt32Array([people[0]]), "a_person": people[0]}))
	for i in 6:
		_post(_event("Tribulation", 3, 900, {"souls": PackedInt32Array([50 + i]), "a": "霍靜沉%d" % i, "x": "Passed", "n": 2, "a_role": "Rogue", "a_realm": 1}))
	_post(_event("WorldChanged", 5, 90, {"change": "ColdBegan", "era": "玄冥"}))
	_rumor.expanded = true
	await frames(3)
	var unit: float = _hud._unit
	var bounds := Rect2(Vector2.ZERO, Vector2(tree.root.get_visible_rect().size))
	var tag := "%s 縮放 %s" % [view, ui_scale]
	assert_eq(_rumor.size, bounds.size, "%s：蓋滿畫面" % tag)
	var paper: Rect2 = _rumor.paper_rect()
	assert_true(paper.is_equal_approx(bounds.grow(-24.0 * unit)), "%s：紙面四邊各留 24 × 縮放 %s" % [tag, paper])
	assert_true(paper.encloses(_rumor.header_rect()), "%s：頁首在紙面內" % tag)
	var last_end := -1.0e9
	for i in RumorSlip.FILTERS.size():
		var chip: Rect2 = _rumor.filter_rect(i)
		assert_true(paper.encloses(chip), "%s：篩選 %d 在紙面內" % [tag, i])
		assert_gt(chip.position.x, last_end - 0.01, "%s：篩選不互相壓住" % tag)
		last_end = chip.end.x
	var view_rect: Rect2 = _rumor._view_rect()
	assert_true(paper.encloses(view_rect), "%s：清單在紙面內" % tag)
	var max_scroll: float = _rumor._max_scroll()
	assert_gt(max_scroll, 0.0, "%s：這些傳聞要捲動" % tag)
	for scroll in [0.0, max_scroll * 0.5, max_scroll]:
		_rumor._scroll = scroll
		var seen := 0
		for i in _rumor.shown().size():
			var row: Rect2 = _rumor.row_rect(i)
			if not view_rect.intersects(row):
				continue
			seen += 1
			assert_gt(row.position.x, view_rect.position.x - 0.01, "%s：列不出清單左緣" % tag)
			assert_lt(row.end.x, view_rect.end.x + 0.01, "%s：列不出清單右緣" % tag)
			var room: Rect2 = _rumor.line_rect(i)
			assert_true(row.encloses(room), "%s：一句話的範圍在列內" % tag)
			assert_gt(room.size.x, 240.0 * unit, "%s：一句話有足夠的寬度" % tag)
			assert_lt(Ink.text_size(_rumor.row_line(i), _rumor._font()).x, room.size.x + 0.5, "%s：一句話放得進列" % tag)
			if _rumor.has_trace_slot(i):
				assert_true(row.encloses(_rumor.action_slots(i)["trace"]), "%s：溯因印在列內" % tag)
			if _rumor.has_person_slot(i):
				assert_true(row.encloses(_rumor.action_slots(i)["person"]), "%s：看人物印在列內" % tag)
		assert_gt(seen, 0, "%s：捲到 %s 看得到列" % [tag, scroll])
		assert_true(bounds.encloses(_rumor.occupied_rect()))
