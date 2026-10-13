extends "res://tests/harness/suite.gd"
## 流程：導演模式（GDD 5.13）——鏡頭自己動、玩家一動就讓出、設定頁的各項、減少鏡頭運動、字卡內容與版面。
## 時間不等：事件由 SimHost.advance 一次推出來，導演的計時靠真實的幾個畫面。

static var ROOT: String = TEST_STORAGE.path_join("director")

const VIEWS: Array[Vector2i] = [Vector2i(1280, 800), Vector2i(1920, 1080)]

var _game: Node
var _sim: SimHost
var _camera: MapCamera
var _hud: Hud
var _director: Director
var _events: Array[Dictionary] = []


func suite_name() -> String:
	return "director_journey"


func setup() -> void:
	_wipe()
	GameSettings.file_path = ROOT.path_join("settings.cfg")
	GameSettings.load_file()
	GameSettings.set_value("director", true)


func teardown() -> void:
	_wipe()
	GameSettings.file_path = ROOT.path_join("none.cfg")
	GameSettings.load_file()


func _wipe() -> void:
	var path := ProjectSettings.globalize_path(ROOT)
	if DirAccess.dir_exists_absolute(path):
		for name in DirAccess.get_files_at(path):
			DirAccess.remove_absolute(path.path_join(name))
		DirAccess.remove_absolute(path)


## 開遊戲、放行時間，並一次推出一批事件讓導演有地方去。
func _start(view := FLOW_VIEW) -> void:
	_game = await boot(false, view)
	_sim = _game.get_node("SimHost")
	_camera = _game.get_node("MapCamera")
	_hud = _game.get_node("Hud")
	_director = _hud._director.director
	_sim.set_speed(0)
	_sim.set_paused(false)
	_sim.event_posted.connect(func(event: Dictionary) -> void:
		if int(event["importance"]) >= 2 and int(event.get("province", -1)) >= 0 and event["kind"] != "WorldChanged":
			_events.append(event))
	_sim.advance(240)
	await frames(2)


## 忘掉剛推出來的事件，導演回到沒事可做的樣子（大事與暫停的案例自己再送事件）。
func _quiet() -> void:
	_director.reset()
	_camera.cancel_directed()
	await frames(2)


## 等導演到位（STAY），最多 seconds 秒。
func _until_stay(seconds := 6.0) -> bool:
	var left := seconds
	while left > 0.0 and _director.state != Director.State.STAY:
		await frames(1)
		left -= 1.0 / 30.0
	return _director.state == Director.State.STAY


func test_director_moves_the_camera_and_a_drag_takes_it_back() -> void:
	await _start()
	assert_gt(_events.size(), 0, "推出來的事件裡有可去的州郡")
	var start := _camera.focus
	assert_true(await _until_stay(), "導演把鏡頭帶到某處")
	var seat: Vector2 = _sim.world.province_info(_director.current)["seat"]
	assert_gt(_camera.focus.distance_to(start), 0.5, "鏡頭離開了原位")
	assert_lt(_camera.focus.distance_to(seat), 1.0, "鏡頭停在那個州郡的治所")
	await drag(Vector2(600, 300), Vector2(700, 380))
	assert_eq(_director.state, Director.State.YIELDED, "玩家拖曳後導演讓出")
	await wait(0.6)
	var after := _camera.focus
	await wait(1.0)
	assert_lt(_camera.focus.distance_to(after), 0.1, "讓出後鏡頭不再自己動")
	assert_false(_hud._director_card.showing(), "字卡收起")


func test_a_card_opening_or_zoom_also_takes_over() -> void:
	await _start()
	assert_true(await _until_stay())
	await wheel(Vector2(600, 300), true)
	assert_eq(_director.state, Director.State.YIELDED, "縮放算玩家操作")
	_director.enabled = true
	_hud.open_province(0)
	await frames(3)
	assert_true(_director.busy, "命簿開著，導演視為玩家在用畫面")


func test_turning_the_setting_off_stops_all_movement() -> void:
	GameSettings.set_value("director", false)
	await _start()
	var start := _camera.focus
	await wait(1.5)
	assert_eq(_director.state, Director.State.OFF)
	assert_lt(_camera.focus.distance_to(start), 0.001, "設定關閉後鏡頭完全不自動移動")
	await key(KEY_F7)
	assert_true(GameSettings.flag("director"), "F7 切換成開")
	assert_true(_hud._notice.showing(), "切換時畫面上提示")
	assert_eq(_hud._notice.text(), "導演　開")
	await key(KEY_F7)
	assert_false(GameSettings.flag("director"), "再按一次切成關")


func test_big_event_slows_to_one_x_and_restores_when_the_player_takes_over() -> void:
	await _start()
	await _quiet()
	_sim.set_speed(3)
	_sim.set_paused(false)
	var big := _events[0].duplicate()
	big["importance"] = Director.BIG_LEVEL
	big["province"] = 3
	_sim.event_posted.emit(big)
	await frames(4)
	assert_eq(_sim.speed_index, 0, "對準大事降到一倍速")
	await drag(Vector2(600, 300), Vector2(700, 380))
	assert_eq(_sim.speed_index, 3, "離開那個鏡頭時恢復降速前的速度")


func test_player_speed_change_wins_over_the_restore() -> void:
	await _start()
	await _quiet()
	_sim.set_speed(3)
	var big := _events[0].duplicate()
	big["importance"] = Director.BIG_LEVEL
	big["province"] = 3
	_sim.event_posted.emit(big)
	await frames(4)
	assert_eq(_sim.speed_index, 0)
	_sim.set_speed(1)
	await drag(Vector2(600, 300), Vector2(700, 380))
	assert_eq(_sim.speed_index, 1, "玩家自己改了速度，以玩家的為準")


func test_slowdown_can_be_turned_off() -> void:
	GameSettings.set_value("director_slow", false)
	await _start()
	await _quiet()
	_sim.set_speed(3)
	var big := _events[0].duplicate()
	big["importance"] = Director.BIG_LEVEL
	big["province"] = 3
	_sim.event_posted.emit(big)
	await frames(4)
	assert_eq(_sim.speed_index, 3, "設定關閉時不變速")


func test_paused_game_keeps_the_camera_still() -> void:
	await _start()
	await _quiet()
	_sim.set_paused(true)
	for event in _events.duplicate():
		_sim.event_posted.emit(event)
	var start := _camera.focus
	await wait(1.0)
	assert_lt(_camera.focus.distance_to(start), 0.001, "暫停時導演不動")
	assert_true(_sim.paused, "也不解除暫停")


func test_reduced_motion_fades_instead_of_travelling() -> void:
	GameSettings.set_value("director_reduce", true)
	await _start()
	var start := _camera.focus
	var start_z := _camera.z
	var fade := 0.0
	var frames_left := 240
	while frames_left > 0 and _director.state != Director.State.STAY:
		await frames(1)
		frames_left -= 1
		fade = maxf(fade, _camera.directed_fade)
	assert_eq(_director.state, Director.State.STAY)
	assert_gt(fade, 0.6, "畫面淡出得很深")
	assert_eq(_camera.directed_fade, 0.0, "到位後淡入完畢")
	var seat: Vector2 = _sim.world.province_info(_director.current)["seat"]
	assert_lt(_camera.focus.distance_to(seat), 1.0)
	assert_lt(absf(_camera.z - start_z), 0.001, "沒有拉高")


func test_far_travel_lifts_and_takes_the_short_way_over_the_seam() -> void:
	await _start()
	_director.enabled = false
	GameSettings.set_value("director", false)
	await frames(2)
	var width := _camera.map_width()
	_camera.look_at_map(Vector2(width * 0.1, _camera.focus.y), width * 0.1, true)
	await frames(2)
	var start_z := _camera.z
	_camera.direct_to(Vector2(width * 0.75, _camera.focus.y), start_z)
	var peak := start_z
	var crossed_middle := false
	var frames_left := 200
	while _camera.is_directed() and frames_left > 0:
		await frames(1)
		frames_left -= 1
		peak = maxf(peak, _camera.z)
		crossed_middle = crossed_middle or (_camera.focus.x > width * 0.3 and _camera.focus.x < width * 0.7)
	assert_false(_camera.is_directed(), "過場結束")
	assert_false(crossed_middle, "走東西較短的那一邊，不橫掃整張地圖")
	assert_gt(peak, start_z * 1.2, "遠處先拉高")
	assert_lt(absf(_camera.z - start_z), 0.01, "落回原本的高度")
	assert_lt(_camera.focus.distance_to(Vector2(width * 0.75, _camera.focus.y)), 0.5)


func test_globe_view_works_too() -> void:
	await _start()
	await _quiet()
	_camera.set_globe(true)
	await frames(3)
	for event in _events.duplicate():
		_sim.event_posted.emit(event)
	var ok := await _until_stay(20.0)
	assert_true(ok, "星球視角下導演照常到位 狀態 %s 忙碌 %s 啟用 %s 目標 %d 飛行 %s" % [Director.State.keys()[_director.state], _director.busy, _director.enabled, _director.target, _camera.is_directed()])
	assert_true(_camera.globe, "導演不改視角")


func test_arrival_height_is_one_where_figures_are_visible() -> void:
	await _start()
	await _quiet()
	var width := _camera.map_width()
	_camera.look_at_map(_camera.focus, width * 0.7, true)
	await frames(3)
	_director.reset()
	for event in _events.duplicate():
		_sim.event_posted.emit(event)
	assert_true(await _until_stay(8.0))
	await frames(10)
	assert_lt(absf(_camera.z - width * 0.25), 0.5, "太高（超過 0.3 倍寬）時降到 0.25 倍寬")
	await _quiet()
	_camera.look_at_map(_camera.focus, width * 0.2, true)
	await frames(3)
	_director.reset()
	for event in _events.duplicate():
		_sim.event_posted.emit(event)
	assert_true(await _until_stay(8.0))
	await frames(10)
	assert_lt(absf(_camera.z - width * 0.2), 0.5, "玩家原本就在範圍內，維持玩家的高度")


func test_camera_lands_on_the_event_cell_not_the_province_seat() -> void:
	await _start()
	await _quiet()
	var event: Dictionary = _events[0].duplicate()
	event["importance"] = 4
	var seat: Vector2 = _sim.world.province_info(int(event["province"]))["seat"]
	var cell := seat + Vector2(3.0, -2.0)
	event["pos"] = cell
	_sim.event_posted.emit(event)
	assert_true(await _until_stay(20.0))
	await frames(10)
	assert_lt(_camera.focus.distance_to(cell), 0.5, "鏡頭焦點在事件格，不在州郡治所")
	assert_lt(_camera.focus.distance_to(cell), 0.5, "焦點就是畫面中心，事件格在中央三分之一內")


func test_near_travel_keeps_the_height() -> void:
	await _start()
	GameSettings.set_value("director", false)
	await frames(2)
	var z0 := _camera.z
	_camera.direct_to(_camera.focus + Vector2(2.0, 0.0), z0)
	var peak := z0
	while _camera.is_directed():
		await frames(1)
		peak = maxf(peak, _camera.z)
	assert_lt(peak, z0 * 1.01, "近處只平移")


func test_caption_shows_the_local_event_and_names_open_cards() -> void:
	await _start()
	assert_true(await _until_stay())
	await frames(15)
	var card: DirectorCard = _hud._director_card
	assert_true(card.showing(), "鏡頭到位就出字卡")
	var event := card.event()
	assert_eq(int(event["province"]), _director.current, "字卡講的是當地的事件")
	assert_true(card.text().contains(EventText.line(event).right(3)), "句子是傳聞的那一句")
	var targets := card.targets()
	assert_gt(targets.size(), 0, "名字可點")
	var target := targets[0]
	await click(card.target_center(target))
	var kind := target.split(":")[0]
	match kind:
		"person":
			assert_true(_hud.person_card().is_open(), "點人名開人物卡")
		"province":
			assert_true(_hud.province_card().is_open(), "點州名開州郡卡")
		"sect":
			assert_true(_hud.sect_card().is_open(), "點宗門名開宗門卡")
	assert_eq(_director.state, Director.State.YIELDED, "點字卡算玩家操作")


func test_caption_can_be_turned_off() -> void:
	GameSettings.set_value("director_card", false)
	await _start()
	assert_true(await _until_stay())
	await frames(10)
	assert_false(_hud._director_card.showing())


func test_settings_page_lists_and_saves_every_director_item() -> void:
	await _start()
	_hud.open_menu()
	var deck := _hud.menu()
	deck._on_chosen(&"settings")
	await frames(2)
	var panel := deck.panel()
	for id: StringName in [&"set.director", &"set.director_pace", &"set.director_idle", &"set.director_slow", &"set.director_card", &"set.director_reduce"]:
		assert_true(panel.row_rect(id).size.x > 0.0, "設定頁有 %s" % id)
	var expected := {"director": false, "director_pace": 2, "director_idle": 40, "director_slow": false, "director_card": false, "director_reduce": true}
	deck._on_changed(&"set.director", false)
	deck._on_changed(&"set.director_pace", 2)
	deck._on_changed(&"set.director_idle", 40)
	deck._on_changed(&"set.director_slow", false)
	deck._on_changed(&"set.director_card", false)
	deck._on_changed(&"set.director_reduce", true)
	GameSettings.load_file()
	for key: String in expected:
		assert_eq(GameSettings.value(key), expected[key], "存檔重讀後保留 %s" % key)
	var rows := deck._settings_rows()
	var shown := {}
	for row: Dictionary in rows:
		shown[row["id"]] = row.get("value")
	assert_eq(shown[&"set.director_pace"], 2, "設定頁帶出存下的值")
	assert_eq(shown[&"set.director_idle"], 40)
	assert_eq(shown[&"set.director_reduce"], true)


func test_defaults_match_the_spec() -> void:
	var defaults := GameSettings.DEFAULTS
	assert_eq(defaults["director"], true)
	assert_eq(defaults["director_pace"], 1, "節奏預設標準")
	assert_eq(defaults["director_idle"], 20)
	assert_eq(defaults["director_slow"], true)
	assert_eq(defaults["director_card"], true)
	assert_eq(defaults["director_reduce"], false)


func test_caption_stays_inside_the_screen_and_clear_of_other_slips_in_all_four_views() -> void:
	for view: Vector2i in VIEWS:
		for ui_scale: float in [1.0, 1.5]:
			GameSettings.set_value("ui_scale", ui_scale)
			_events.clear()
			await _start(view)
			for spare in tree.root.get_children():
				if spare.name == "Bootstrap" and spare != _game:
					spare.queue_free()
			var longest: Dictionary = _events[0]
			for event in _events:
				if EventText.line(event).length() > EventText.line(longest).length():
					longest = event
			var card: DirectorCard = _hud._director_card
			card.show_event(longest)
			await frames(4)
			var rect := Rect2(card.position, card.size).grow(Ink.SLIP_BAND)
			var label := "%dx%d %d%%" % [view.x, view.y, int(ui_scale * 100.0)]
			assert_lt(absf(_hud._unit - ui_scale * (Ink.DECK_UNIT if view.y <= Ink.DECK_HEIGHT else 1.0)), 0.001, "%s：介面縮放有套上" % label)
			assert_true(Rect2(Vector2.ZERO, Vector2(view)).encloses(rect), "%s：字卡不出界 %s" % [label, rect])
			assert_lt((card._para["lines"] as Array).size(), 3, "%s：最多兩行" % label)
			assert_lt(absf(rect.get_center().x - view.x * 0.5), 2.0, "%s：水平置中於整個畫面" % label)
			for other: Rect2 in _hud._occupied():
				assert_false(rect.intersects(other), "%s：字卡 %s 不與器物 %s 重疊" % [label, rect, other])
			_game.queue_free()
			await frames(2)
