extends "res://tests/harness/suite.gd"
## 流程：標題畫面的主選單——開新世界（GDD 4.6）、繼續與設定（GDD 9.1）；鍵鼠與搖桿各走一遍。
## 存檔與設定寫在使用者資料夾底下另一個目錄，不碰玩家的檔案。

const TITLE := "res://app/scenes/title.tscn"
const ROOT := "user://test_menu"

var _title: Node
var _deck: MenuDeck
var _panel: MenuPanel


func suite_name() -> String:
	return "menu_journey"


func setup() -> void:
	_wipe()
	SaveStore.root = ROOT.path_join("saves")
	GameSettings.file_path = ROOT.path_join("settings.cfg")
	GameSettings.load_file()
	tree.root.size = FLOW_VIEW
	_title = (load(TITLE) as PackedScene).instantiate()
	_title.backdrop_seed = TEST_SEED
	tree.root.add_child(_title)
	await frames(2)
	AudioServer.set_bus_mute(0, true)
	_deck = _title.deck
	_panel = _deck.panel()


func teardown() -> void:
	# 進了遊戲場景之後標題已經換掉；把還在的那一個收掉。
	for child in tree.root.get_children():
		if child == _title or child.name == "Bootstrap":
			tree.root.remove_child(child)
			child.queue_free()
	_wipe()
	SaveStore.root = SaveStore.DEFAULT_ROOT
	GameSettings.file_path = GameSettings.DEFAULT_PATH
	GameSettings.load_file()


func _wipe() -> void:
	for dir: String in [ROOT.path_join("saves"), ROOT]:
		var path := ProjectSettings.globalize_path(dir)
		if DirAccess.dir_exists_absolute(path):
			for name in DirAccess.get_files_at(path):
				DirAccess.remove_absolute(path.path_join(name))
			DirAccess.remove_absolute(path)


## 等到遊戲場景出現（開天或讀檔完成），回傳它。
func _wait_for_game() -> Node:
	for i in 600:
		var game := tree.root.get_node_or_null("Bootstrap")
		if game != null and game.is_node_ready():
			return game
		await frames(1)
	return null


## 打字：每個字送一次帶字元的按鍵。
func _type(text: String) -> void:
	for ch in text:
		var event := InputEventKey.new()
		event.unicode = ch.unicode_at(0)
		event.pressed = true
		Input.parse_input_event(event)
		await frames(2)


## 滑鼠點選單上的某一列。
func _click_row(id: StringName) -> void:
	await click(_panel.position + _panel.row_rect(id).get_center() - Vector2(60.0, 0.0))


func test_main_menu_lists_the_entries_and_locks_what_needs_a_save() -> void:
	assert_eq(_deck.page(), &"root")
	assert_eq(_panel.focused_id(), &"new", "一開始停在新世界")
	await key(KEY_DOWN)
	assert_eq(_panel.focused_id(), &"share", "沒有存檔時跳過繼續與載入")
	await key(KEY_UP)
	await key(KEY_UP)
	assert_eq(_panel.focused_id(), &"quit", "往上繞到最後一列")


func test_keyboard_opens_a_new_world_with_the_chosen_options() -> void:
	await key(KEY_ENTER)
	assert_eq(_deck.page(), &"new")
	assert_eq(_panel.focused_id(), &"new.name")
	var name: String = _panel.value_of(&"new.name")
	assert_false(name.is_empty(), "世界名預設由種子取")
	# 種子：改寫成固定值，世界名跟著換。
	await key(KEY_DOWN)
	await key(KEY_ENTER)
	await _type("42")
	await key(KEY_ENTER)
	assert_eq(_panel.value_of(&"new.seed"), "42")
	assert_eq(_panel.value_of(&"new.name"), SimWorld.random_world_name(42))
	# 尺寸：預設中，往左換成小。
	await key(KEY_DOWN)
	await key(KEY_DOWN)
	assert_eq(_panel.focused_id(), &"new.size")
	assert_eq(_panel.value_of(&"new.size"), "medium")
	await key(KEY_LEFT)
	assert_eq(_panel.value_of(&"new.size"), "small")
	# 陸地比例：往右一格。
	await key(KEY_DOWN)
	await key(KEY_RIGHT)
	assert_lt(absf((_panel.value_of(&"new.land_ratio") as float) - 0.41), 0.001)
	await key(KEY_UP)
	await key(KEY_UP)
	await key(KEY_UP)
	await key(KEY_UP)
	await key(KEY_UP)
	assert_eq(_panel.focused_id(), &"new.start", "往上繞到開天")
	await key(KEY_ENTER)
	var game := await _wait_for_game()
	assert_true(game != null, "開天後進入遊戲場景")
	var sim := game.get_node("SimHost") as SimHost
	sim.set_paused(true)
	var options: Dictionary = sim.world.world_options()
	assert_eq(options["size"], "small")
	assert_eq(options["name"], SimWorld.random_world_name(42))
	assert_lt(absf((options["land_ratio"] as float) - 0.41), 0.001)
	assert_false(is_instance_valid(_title) and _title.is_inside_tree(), "標題畫面已經換掉")


func test_escape_goes_back_one_page() -> void:
	await key(KEY_ENTER)
	assert_eq(_deck.page(), &"new")
	await key(KEY_ESCAPE)
	assert_eq(_deck.page(), &"root")
	assert_eq(_panel.focused_id(), &"new", "回到原來停的那一列")
	await key(KEY_ESCAPE)
	assert_eq(_deck.page(), &"root", "主選單沒有上一頁")


func test_gamepad_walks_the_menu_and_changes_a_setting() -> void:
	for i in 2:
		await joy_button(JOY_BUTTON_DPAD_DOWN)
	assert_eq(_panel.focused_id(), &"settings")
	assert_true(_panel.gamepad, "按了搖桿就換成搖桿的鍵圖像")
	await joy_button(JOY_BUTTON_A)
	assert_eq(_deck.page(), &"settings")
	await joy_axis(JOY_AXIS_LEFT_Y, 1.0, 0.1)
	await joy_axis(JOY_AXIS_LEFT_Y, 1.0, 0.1)
	await joy_axis(JOY_AXIS_LEFT_Y, 1.0, 0.1)
	assert_eq(_panel.focused_id(), &"set.ui_scale", "搖桿推一次只移一列")
	await joy_button(JOY_BUTTON_DPAD_RIGHT)
	assert_eq(GameSettings.ui_scale(), 1.1)
	GameSettings.load_file()
	assert_eq(GameSettings.ui_scale(), 1.1, "設定立刻寫進設定檔")
	await joy_button(JOY_BUTTON_B)
	assert_eq(_deck.page(), &"root")
	assert_eq(_panel.focused_id(), &"settings")


func test_mouse_picks_rows_and_the_continue_entry_loads_the_latest_save() -> void:
	var world := SimWorld.new()
	assert_eq(world.create(TEST_SEED, TEST_SIZE), "")
	world.step(5)
	assert_eq(SaveStore.save_manual(0, world), "")
	_deck.open_root()
	await frames(1)
	await _click_row(&"continue")
	var game := await _wait_for_game()
	assert_true(game != null, "讀檔後進入遊戲場景")
	var sim := game.get_node("SimHost") as SimHost
	assert_true(sim.paused, "讀檔進來先暫停")
	assert_eq(sim.world.hash_hex(), world.hash_hex(), "讀出來的世界與存檔時相同")


func test_mouse_changes_a_choice_by_clicking_its_arrows() -> void:
	await _click_row(&"settings")
	assert_eq(_deck.page(), &"settings")
	var rect := _panel.row_rect(&"set.edge_scroll")
	assert_false(GameSettings.flag("edge_scroll"))
	await click(_panel.position + Vector2(rect.end.x - 20.0, rect.get_center().y))
	assert_true(GameSettings.flag("edge_scroll"), "點右邊的箭頭換下一個選項")


func test_every_menu_string_has_a_glyph() -> void:
	var missing := ""
	var table: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(Locale.UI_PATH))
	for key: String in table:
		for ch: String in table[key]:
			if ch != " " and ch != "\n" and not Ink.FONT.has_char(ch.unicode_at(0)) and not missing.contains(ch):
				missing += ch
	assert_eq(missing, "", "介面字串裡字型沒有的字")
