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
	assert_eq(shown[0]["seal"], "殺", "印是 7.2 的印字")
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


func test_slip_shows_eight_newest_first_and_list_keeps_fifty() -> void:
	await _boot(FLOW_VIEW)
	for i in 12:
		_post(_event("Killed", 3, 100 + i * 20, {"souls": PackedInt32Array([i])}))
	var shown := _rumor.shown()
	assert_eq(shown.size(), 8, "收合時最多 8 張")
	assert_eq(shown[0]["tick"], 100 + 11 * 20, "新到舊")
	assert_eq(shown[7]["tick"], 100 + 4 * 20)
	_rumor.expanded = true
	assert_eq(_rumor.shown().size(), 12, "展開看到全部")
	for i in 60:
		_post(_event("Killed", 3, 1000 + i * 20, {"souls": PackedInt32Array([100 + i])}))
	assert_eq(_rumor.entry_count(), 50, "清單只留近 50 則")
	assert_eq(_rumor.shown()[0]["tick"], 1000 + 59 * 20, "最舊的被擠掉")


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
	assert_lt(_rumor.size.y, 1080.0, "展開清單不超過畫面")
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


## 收合、命簿開著、展開、兩者並存四種狀態下，傳聞簽不壓到天時儀、題簽、視角鈕、印匣、命簿與引導簽。
func _check_no_overlap(view: Vector2i) -> void:
	await _boot(view)
	for i in 10:
		_post(_event("Killed", 3, 100 + i * 20, {"souls": PackedInt32Array([i]), "pos": Vector2(20.0, 20.0)}))
	await frames(3)
	for state in ["收合", "命簿開著", "展開", "命簿開著且展開"]:
		_rumor.expanded = state.contains("展開")
		if state.contains("命簿"):
			_hud._card.show_province(0)
		else:
			_hud._card.close()
		await wait(0.5)
		var mine: Rect2 = _rumor.occupied_rect()
		var bounds := Rect2(Vector2.ZERO, Vector2(tree.root.get_visible_rect().size))
		assert_true(bounds.encloses(mine), "%s %s：傳聞簽在畫面內 %s" % [view, state, mine])
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
	_rumor.expanded = true
	assert_eq(_rumor.shown().size(), 5, "展開看到全部")
	assert_lt(absf(_rumor.header_rect().size.y - RumorSlip.HEADER_HEIGHT * _hud._unit), 0.01, "尺寸乘縮放倍率")
	assert_lt(absf(_rumor.row_rect(0).size.y - 32.0 * _hud._unit), 0.01, "每張高 32 乘倍率")


func test_desktop_sizes_match_the_layout_table() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Killed", 3, 100))
	_post(_event("Killed", 3, 120, {"souls": PackedInt32Array([1])}))
	await frames(2)
	assert_eq(_rumor.header_rect().size.x, 300.0, "傳聞簽寬 300")
	assert_eq(_rumor.row_rect(0).size.y, 32.0, "每張高 32")
	assert_eq(_rumor.row_rect(1).position.y - _rumor.row_rect(0).end.y, 5.0, "間距 5")
	var bounds := Vector2(tree.root.get_visible_rect().size)
	assert_eq(_rumor.occupied_rect().end.x, bounds.x - 12.0, "右上角內縮 12")
	assert_eq(_rumor.occupied_rect().position.y, 12.0)
	_rumor.expanded = true
	assert_eq(_rumor.header_rect().size.x, 360.0, "展開清單寬 360")


func test_layout_scale_applies_to_the_slip() -> void:
	await _boot(FLOW_VIEW)
	GameSettings.set_value("ui_scale", 1.5)
	_hud._layout()
	await frames(3)
	_rumor = _hud.rumor()
	assert_eq(_rumor.header_rect().size.x, 450.0, "介面縮放 1.5 倍，寬 300 變 450")
