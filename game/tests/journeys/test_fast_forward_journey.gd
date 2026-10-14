extends "res://tests/harness/suite.gd"
## GDD 2.6 / 5.7 / 5.9 世界快進的操作：R 或搖桿左搖桿鍵或天時儀的快進鈕開終點選單，
## 四個終點、按住快進、任何一鍵中止，停下出摘要，鍵鼠與搖桿都走得通。

var _game: Node
var _sim: SimHost
var _hud: Hud
var _panel: FastPanel


func suite_name() -> String:
	return "fast_forward_journey"


func _boot(view := FLOW_VIEW) -> void:
	_game = await boot(false, view)
	_sim = _game.get_node("SimHost")
	_hud = _game.get_node("Hud")
	_panel = _hud.fast_panel()


## 等到快進結束，最多 seconds 秒。
func _await_fast_done(seconds := 20.0) -> void:
	var waited := 0.0
	while _sim.fast_active() and waited < seconds:
		await frames(5)
		waited += 5.0 / 60.0


func _press(keycode: Key, pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = keycode
	event.physical_keycode = keycode
	event.pressed = pressed
	Input.parse_input_event(event)
	await frames(2)


func test_key_opens_the_menu_with_six_rows_and_followed_greyed() -> void:
	await _boot()
	assert_false(_panel.is_open())
	_sim.set_paused(false)
	await key(KEY_R)
	assert_eq(_panel.mode(), &"menu", "點一下 R 開終點選單")
	assert_true(_sim.paused, "選單開著時間暫停")
	var rows := _panel.rows()
	assert_eq(rows.size(), 6, "下一件世界大事、關注者、1／10／100／1000 年")
	assert_true(rows[0]["enabled"])
	assert_false(rows[1]["enabled"], "關注者的下一件事灰掉，只留介面")
	for i in range(2, 6):
		assert_true(rows[i]["enabled"])
	assert_eq(rows[5]["years"], 1000)
	await key(KEY_ESCAPE)
	assert_false(_panel.is_open(), "Esc 取消")
	assert_false(_sim.paused, "取消後行止回到開選單前")


func test_dial_button_opens_the_menu() -> void:
	await _boot()
	var at: Vector2 = _hud._dial.global_position + _hud._dial.fast_button_center()
	await click(at)
	assert_eq(_panel.mode(), &"menu", "點天時儀的快進鈕開選單")


func test_year_choice_runs_and_any_key_aborts() -> void:
	await _boot()
	var start := _sim.world.tick()
	await key(KEY_R)
	await key(KEY_DOWN)
	assert_eq(_panel.selected_index(), 2, "往下跳過灰掉的關注者")
	await key(KEY_DOWN)
	await key(KEY_DOWN)
	await key(KEY_DOWN)
	assert_eq(_panel.selected_index(), 5, "選到一千年")
	await key(KEY_ENTER)
	assert_true(_sim.fast_active(), "確定後開始快進")
	assert_false(_panel.is_open())
	await wait(0.3)
	assert_gt(_sim.world.tick(), start, "快進中時間在走")
	await key(KEY_X)
	assert_false(_sim.fast_active(), "任何一鍵中止")
	assert_eq(_panel.mode(), &"summary", "中止後出摘要")
	assert_lt(_sim.world.tick(), start + 1000 * 360, "沒有跑完一千年")


func test_one_year_finishes_with_summary_and_closes() -> void:
	await _boot()
	var start := _sim.world.tick()
	_sim.set_paused(false)
	await key(KEY_R)
	await click(_panel.row_center(2))
	assert_true(_sim.fast_active() or _panel.mode() == &"summary")
	await _await_fast_done()
	assert_eq(_sim.world.tick(), start + 360, "整一年後的同一天")
	assert_eq(_panel.mode(), &"summary")
	assert_true(_sim.paused, "摘要開著時間暫停")
	var summary := _panel.summary()
	assert_eq(summary["years"], 1)
	var rows := _panel.rows()
	assert_eq(rows[rows.size() - 1]["kind"], "close")
	await click(_panel.row_center(rows.size() - 1))
	assert_false(_panel.is_open())
	assert_false(_sim.paused, "讀完摘要，行止回到快進前")


func test_world_event_choice_stops_on_the_event_and_summary_rows_open_it() -> void:
	await _boot()
	await key(KEY_R)
	await key(KEY_ENTER)
	await _await_fast_done(60.0)
	assert_eq(_panel.mode(), &"summary")
	var ids: Array[int] = []
	var rows := _panel.rows()
	var first_event := -1
	for i in rows.size():
		if rows[i]["kind"] == "event":
			first_event = i
			ids.append(int(rows[i]["id"]))
	assert_gt(ids.size(), 0, "摘要列出使它停下的世界大事")
	var asked := [-1]
	_panel.event_requested.connect(func(id: int) -> void: asked[0] = id)
	await click(_panel.row_center(first_event))
	assert_eq(asked[0], ids[0], "點世界大事列開那件事的事件卡")
	assert_false(_panel.is_open())


func test_hold_runs_until_release() -> void:
	await _boot()
	var start := _sim.world.tick()
	await _press(KEY_R, true)
	await wait(0.7)
	assert_true(_sim.fast_active(), "按住超過門檻開始快進")
	assert_false(_panel.is_open(), "按住不開選單")
	await _press(KEY_R, false)
	assert_false(_sim.fast_active(), "放開就停")
	assert_gt(_sim.world.tick(), start)
	assert_eq(_panel.mode(), &"summary")


func test_no_rumor_or_spectacle_during_fast_forward() -> void:
	await _boot()
	var posted := [0]
	_sim.event_posted.connect(func(_e: Dictionary) -> void: posted[0] += 1)
	await key(KEY_R)
	await key(KEY_DOWN)
	await key(KEY_ENTER)
	await _await_fast_done()
	assert_eq(posted[0], 0, "快進期間不入傳聞簽也不排演出")


func test_gamepad_opens_chooses_and_aborts() -> void:
	await _boot()
	_hud._apply_device()
	await joy_button(JOY_BUTTON_LEFT_STICK)
	assert_eq(_panel.mode(), &"menu", "左搖桿鍵開選單")
	await joy_button(JOY_BUTTON_B)
	assert_false(_panel.is_open(), "B 取消")
	await joy_button(JOY_BUTTON_LEFT_STICK)
	for i in 4:
		await joy_button(JOY_BUTTON_DPAD_DOWN)
	assert_eq(_panel.selected_index(), 5, "搖桿十字鍵下移到一千年")
	await joy_button(JOY_BUTTON_A)
	assert_true(_sim.fast_active(), "A 確定後快進")
	await wait(0.2)
	await joy_button(JOY_BUTTON_X)
	assert_false(_sim.fast_active(), "搖桿任何一鍵中止")
	assert_eq(_panel.mode(), &"summary")
	await joy_button(JOY_BUTTON_A)
	assert_false(_panel.is_open(), "A 關掉摘要")


func test_deck_size_menu_and_summary_fit_the_screen() -> void:
	await _boot(Vector2i(1280, 800))
	await key(KEY_R)
	var bounds := Rect2(Vector2.ZERO, Vector2(tree.root.get_visible_rect().size))
	for row in _panel.rows():
		assert_true(bounds.encloses(row["rect"]), "選單每列都在畫面內")
	await key(KEY_ENTER)
	await _await_fast_done(60.0)
	for row in _panel.rows():
		assert_true(bounds.encloses(row["rect"]), "摘要每列都在畫面內")
