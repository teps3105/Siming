extends "res://tests/harness/suite.gd"
## 流程：遊戲中選單（GDD 9.1）——開關與時間的行止、存檔與讀檔、介面縮放、自動存檔（GDD 4.8）與邊緣捲動（GDD 5.9）。
## 存檔與設定寫在使用者資料夾底下另一個目錄，不碰玩家的檔案。

static var ROOT: String = TEST_STORAGE.path_join("pause_menu")

var _game: Node
var _sim: SimHost
var _camera: MapCamera
var _hud: Hud


func suite_name() -> String:
	return "pause_menu_journey"


func setup() -> void:
	_wipe()
	SaveStore.root = ROOT.path_join("saves")
	GameSettings.file_path = ROOT.path_join("settings.cfg")
	GameSettings.load_file()
	_game = await boot()
	_sim = _game.get_node("SimHost")
	_camera = _game.get_node("MapCamera")
	_hud = _game.get_node("Hud")


func teardown() -> void:
	# 讀檔會換一個遊戲場景；把換上來的那一個也收掉。
	for child in tree.root.get_children():
		if child.name == "Bootstrap" and child != _game:
			tree.root.remove_child(child)
			child.queue_free()
	_wipe()


func _wipe() -> void:
	for dir: String in [ROOT.path_join("saves"), ROOT]:
		var path := ProjectSettings.globalize_path(dir)
		if DirAccess.dir_exists_absolute(path):
			for name in DirAccess.get_files_at(path):
				DirAccess.remove_absolute(path.path_join(name))
			DirAccess.remove_absolute(path)


func _move_mouse(pos: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = pos
	motion.global_position = pos
	Input.parse_input_event(motion)
	await frames(2)


func test_escape_opens_the_menu_pauses_time_and_restores_it_on_close() -> void:
	_sim.set_paused(false)
	await key(KEY_ESCAPE)
	assert_true(_hud.menu_open(), "沒有卡片開著時 Esc 開選單")
	assert_true(_sim.paused, "選單開著時時間暫停")
	assert_false(_camera.input_enabled, "選單開著時地圖不收輸入")
	var tick: int = _sim.world.tick()
	await wait(0.6)
	assert_eq(_sim.world.tick(), tick, "時間沒有往前")
	await key(KEY_ESCAPE)
	assert_false(_hud.menu_open(), "再按一次關掉")
	assert_false(_sim.paused, "原本在跑的，關掉後繼續跑")
	assert_true(_camera.input_enabled)


func test_closing_the_menu_keeps_time_paused_if_it_was_paused() -> void:
	assert_true(_sim.paused)
	await key(KEY_ESCAPE)
	await key(KEY_ENTER)
	assert_false(_hud.menu_open(), "第一列是回到世界")
	assert_true(_sim.paused, "原本暫停的，關掉後還是暫停")


func test_escape_closes_an_open_card_before_opening_the_menu() -> void:
	_hud._card.show_province(0)
	await frames(1)
	assert_true(_hud._card.visible)
	await key(KEY_ESCAPE)
	assert_false(_hud._card.visible, "先關卡片")
	assert_false(_hud.menu_open())
	await key(KEY_ESCAPE)
	assert_true(_hud.menu_open(), "再按才開選單")


func test_map_ignores_input_while_the_menu_is_open() -> void:
	await key(KEY_ESCAPE)
	var focus := _camera.focus
	var mode := (_game.get_node("TerrainMap") as TerrainMap).mode
	await joy_axis(JOY_AXIS_LEFT_X, 1.0, 0.3)
	await key(KEY_F2)
	await wait(0.3)
	assert_lt(_camera.focus.distance_to(focus), 0.01, "鏡頭沒有被推走")
	assert_eq((_game.get_node("TerrainMap") as TerrainMap).mode, mode, "地圖模式沒有被換掉")


func test_gamepad_saves_into_a_slot_and_overwriting_asks_first() -> void:
	await joy_button(JOY_BUTTON_B)
	assert_false(_hud.menu_open(), "B 只取消與關卡片，不開選單")
	await joy_button(JOY_BUTTON_START)
	assert_true(_hud.menu_open(), "Start 開選單")
	var deck := _hud.menu()
	await joy_button(JOY_BUTTON_DPAD_DOWN)
	assert_eq(deck.panel().focused_id(), &"save")
	await joy_button(JOY_BUTTON_A)
	assert_eq(deck.page(), &"save")
	assert_eq(deck.panel().row_count(), SaveStore.MANUAL_SLOTS)
	await joy_button(JOY_BUTTON_A)
	var slots := SaveStore.manual_slots()
	assert_false(slots[0].is_empty(), "存進第一欄")
	assert_eq(slots[0]["tick"], _sim.world.tick())
	# 再存同一欄：先問，取消就不動。
	_sim.advance(3)
	await joy_button(JOY_BUTTON_A)
	assert_eq(deck.page(), &"confirm")
	await joy_button(JOY_BUTTON_B)
	assert_eq(deck.page(), &"save")
	assert_eq(SaveStore.manual_slots()[0]["tick"], slots[0]["tick"], "取消後存檔沒變")
	await joy_button(JOY_BUTTON_A)
	await joy_button(JOY_BUTTON_DPAD_UP)
	await joy_button(JOY_BUTTON_A)
	assert_eq(SaveStore.manual_slots()[0]["tick"], _sim.world.tick(), "確定後蓋過去")
	await joy_button(JOY_BUTTON_B)
	await joy_button(JOY_BUTTON_B)
	assert_false(_hud.menu_open(), "B 一路退到關掉選單")


func test_loading_from_the_menu_replaces_the_world() -> void:
	assert_eq(SaveStore.save_manual(2, _sim.world), "")
	var saved_hash: String = _sim.world.hash_hex()
	_sim.advance(6)
	assert_ne(_sim.world.hash_hex(), saved_hash)
	await key(KEY_ESCAPE)
	var deck := _hud.menu()
	await key(KEY_DOWN)
	await key(KEY_DOWN)
	assert_eq(deck.panel().focused_id(), &"load")
	await key(KEY_ENTER)
	assert_eq(deck.page(), &"load")
	await key(KEY_ENTER)
	var loaded: Node = null
	for i in 600:
		for child in tree.root.get_children():
			if child.name == "Bootstrap" and child != _game and child.is_node_ready():
				loaded = child
		if loaded != null:
			break
		await frames(1)
	assert_true(loaded != null, "讀檔後換成新的遊戲場景")
	assert_eq((loaded.get_node("SimHost") as SimHost).world.hash_hex(), saved_hash, "世界回到存檔的時刻")


func test_delete_key_removes_a_save_after_asking() -> void:
	assert_eq(SaveStore.save_manual(4, _sim.world), "")
	await key(KEY_ESCAPE)
	var deck := _hud.menu()
	await key(KEY_DOWN)
	await key(KEY_DOWN)
	await key(KEY_ENTER)
	assert_eq(deck.page(), &"load")
	await key(KEY_DELETE)
	assert_eq(deck.page(), &"confirm")
	assert_eq(deck.panel().focused_id(), &"confirm.no", "預設停在取消")
	await key(KEY_UP)
	await key(KEY_ENTER)
	assert_true(SaveStore.manual_slots()[4].is_empty(), "確定後刪掉")
	assert_eq(deck.page(), &"load")


func test_ui_scale_setting_resizes_the_desk_objects() -> void:
	var dial_before: Vector2 = _hud._dial.size
	await key(KEY_ESCAPE)
	var deck := _hud.menu()
	# 沒有存檔時「載入」不可選，往下會跳過它。
	for i in 3:
		await key(KEY_DOWN)
	assert_eq(deck.panel().focused_id(), &"settings")
	await key(KEY_ENTER)
	for i in 3:
		await key(KEY_DOWN)
	assert_eq(deck.panel().focused_id(), &"set.ui_scale")
	await key(KEY_RIGHT)
	await key(KEY_RIGHT)
	await frames(2)
	assert_eq(GameSettings.ui_scale(), 1.2)
	assert_true(_hud.menu_open(), "重排器物後選單還開著")
	assert_gt(_hud._dial.size.x, dial_before.x * 1.1, "天時儀跟著放大")


func test_share_code_copies_and_pastes_through_the_clipboard() -> void:
	_sim.advance(4)
	var hash: String = _sim.world.hash_hex()
	await key(KEY_ESCAPE)
	var deck := _hud.menu()
	var board := [""]
	deck.clipboard_set = func(text: String) -> void: board[0] = text
	deck.clipboard_get = func() -> String: return board[0]
	for i in 2:
		await key(KEY_DOWN)
	assert_eq(deck.panel().focused_id(), &"share")
	await key(KEY_ENTER)
	assert_eq(deck.panel().focused_id(), &"share.copy", "一進來就停在第一個能按的列，不停在說明文字上")
	await key(KEY_ENTER)
	assert_true(board[0].begins_with(SaveStore.SHARE_PREFIX), "分享碼進了剪貼簿")
	# 剪貼簿裡不是分享碼：告知，不換世界。
	var code: String = board[0]
	board[0] = "hello"
	await key(KEY_DOWN)
	await key(KEY_ENTER)
	assert_eq(deck.page(), &"notice")
	await key(KEY_ENTER)
	assert_eq(deck.page(), &"share")
	board[0] = code
	await key(KEY_ENTER)
	var loaded: Node = null
	for i in 600:
		for child in tree.root.get_children():
			if child.name == "Bootstrap" and child != _game and child.is_node_ready():
				loaded = child
		if loaded != null:
			break
		await frames(1)
	assert_true(loaded != null, "匯入分享碼後換成那個世界")
	assert_eq((loaded.get_node("SimHost") as SimHost).world.hash_hex(), hash)


func test_auto_save_once_per_shi_keeps_the_newest_five() -> void:
	_game.auto_save = true
	_sim.advance(200)
	assert_eq(SaveStore.auto_saves().size(), 0, "還沒滿一世")
	_sim.advance(200)
	assert_eq(SaveStore.auto_saves().size(), 1, "跨過一世存一份")
	for i in 6:
		_sim.advance(SaveStore.AUTO_INTERVAL_MONTHS)
	assert_eq(SaveStore.auto_saves().size(), SaveStore.AUTO_KEEP)


func test_edge_scroll_pans_only_when_the_setting_is_on() -> void:
	var middle := Vector2(FLOW_VIEW) * 0.5
	await _move_mouse(Vector2(2.0, middle.y))
	var focus := _camera.focus
	await wait(0.3)
	assert_lt(_camera.focus.distance_to(focus), 0.01, "預設關：游標在邊緣不動")
	GameSettings.set_value("edge_scroll", true)
	await wait(0.3)
	assert_gt(focus.x - _camera.focus.x, 0.5, "開了之後往西平移")
	await _move_mouse(middle)
	await wait(0.6)
	focus = _camera.focus
	await wait(0.3)
	assert_lt(_camera.focus.distance_to(focus), 0.5, "游標離開邊緣就停")
