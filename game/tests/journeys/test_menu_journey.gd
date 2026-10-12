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
	_title.framing_roll = 7
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


func test_gdd_9_1_credits_list_only_what_ships_by_name_and_licence() -> void:
	var lines: Array[String] = []
	var third := false
	for row: Dictionary in _title.deck._credits_rows():
		if row["kind"] == &"head":
			third = row["label"] == Locale.t("credits_third")
		elif third:
			lines.append(row["label"])
	assert_eq(lines.size(), 4, "第三方只列隨遊戲發佈的四項：兩套字型、引擎、橋接")
	for text in lines:
		var parts := text.split("／")
		assert_true(parts.size() == 2 or parts.size() == 3, "每列是名稱、（作者）、授權：%s" % text)
		assert_true(parts[parts.size() - 1] in ["SIL OFL 1.1", "MIT", "MPL-2.0"], "每列以授權條款收尾：%s" % text)


func test_gdd_9_1_title_sits_above_the_menu_on_the_right_and_the_gathering_on_the_left() -> void:
	var logo: Rect2 = _title.logo.logo_rect()
	var at := _panel.global_position
	assert_gt(logo.position.x, FLOW_VIEW.x * 0.5, "標題在右半")
	assert_lt(logo.end.x, float(FLOW_VIEW.x), "標題在畫面內")
	assert_gt(at.y, logo.end.y, "選單在標題下面")
	assert_gt(at.x, FLOW_VIEW.x * 0.5, "選單在右半")
	assert_false(_panel.paper, "標題畫面的選單直接寫在畫面上，不襯紙")
	assert_lt(at.x + _panel.size.x, float(FLOW_VIEW.x), "整個選單在畫面內")
	assert_lt(at.y + _panel.size.y, float(FLOW_VIEW.y), "整個選單在畫面內")
	for who: Array in TitleGathering.CAST:
		assert_lt(_title.gathering.place(who[0]).x, logo.position.x, "修士都在標題與選單的左邊")


func test_gdd_9_1_three_sit_on_the_terrace_and_the_rest_live_across_the_landscape() -> void:
	assert_eq(TitleGathering.CAST.size(), 3, "石台上只有對弈兩人與一名觀棋者")
	assert_gt(TitleGathering.WANDERERS.size(), 3, "其餘的人散在山水裡")
	var gathering: TitleGathering = _title.gathering
	var menu := Rect2(_panel.global_position, _panel.size).merge(_title.logo.logo_rect())
	var doing := {}
	for i in TitleGathering.WANDERERS.size():
		var who: Array = TitleGathering.WANDERERS[i]
		var box := gathering.figure_rect(TitleGathering.CAST.size() + i)
		doing[who[5]] = true
		assert_lt(who[1], 0.6, "散在山水裡的人離得遠、畫得小")
		assert_false(box.intersects(menu), "不擋標題與選單：%s" % box)
		assert_true(Rect2(Vector2.ZERO, Vector2(FLOW_VIEW)).encloses(box), "整個人在畫面內：%s" % box)
		for j in TitleGathering.roster().size():
			if j != TitleGathering.CAST.size() + i:
				assert_gt(box.get_center().distance_to(gathering.figure_rect(j).get_center()), FLOW_VIEW.y * 0.2, "各在各的地方，不擠在一起：%s 與 %s" % [i, j])
	assert_eq(doing.size(), TitleGathering.WANDERERS.size(), "各做各的事，腳下的地方都不一樣")
	var seen := {"sex": {}, "age": {}, "wealth": {}, "school": {}}
	for who: Array in TitleGathering.roster():
		for key: String in seen:
			seen[key][who[4][key]] = true
	assert_eq(seen["sex"].size(), 2, "男女都有")
	assert_eq(seen["age"].size(), 3, "老少都有")
	assert_eq(seen["wealth"].size(), 3, "貧富都有")
	assert_eq(seen["school"].size(), 4, "儒釋道與無門派都有")


func test_gdd_9_1_backdrop_frames_mountain_plain_and_water() -> void:
	var framing := SceneFraming.new(_title._world)
	var z: float = _title._world.grid_size().x * SceneFraming.VIEW_Z_FRAC
	var got := framing.shares(_title.backdrop_focus(), SceneFraming.ground_extent(z, float(FLOW_VIEW.x) / FLOW_VIEW.y))
	assert_gt(got.x, 0.1, "有山：%s" % got)
	assert_gt(got.y, 0.15, "有平地：%s" % got)
	assert_gt(got.z, 0.1, "有水：%s" % got)


func test_gdd_9_1_cliff_cracks_stay_inside_the_cliff_wall() -> void:
	var wall := TitleGathering.cliff_wall()
	var cracks := TitleGathering.cliff_cracks()
	assert_gt(cracks.size(), 5, "崖壁有幾道裂理")
	for crack in cracks:
		for i in crack.size() - 1:
			var middle := crack[i].lerp(crack[i + 1], 0.5)
			assert_true(Geometry2D.is_point_in_polygon(middle, wall), "裂理不超出崖壁：%s" % middle)


func test_gdd_9_1_pine_is_drawn_back_to_front() -> void:
	var order := TitleGathering.PINE_ORDER
	assert_lt(order.find(&"far_needles"), order.find(&"branches"), "遠處的松針在枝後面")
	assert_lt(order.find(&"branches"), order.find(&"trunk"), "枝從幹後面伸出來，枝根被幹蓋住")
	assert_lt(order.find(&"trunk"), order.find(&"bark"), "樹皮的鱗畫在幹上")
	assert_lt(order.find(&"bark"), order.find(&"near_needles"), "近處的松針在最前面")
	for branch: Array in TitleGathering.BRANCHES:
		assert_true(Geometry2D.is_point_in_polygon(branch[0], CultivatorFigure.smooth_outline(TitleGathering.TRUNK)), "枝根在幹裡面：%s" % branch[0])


func test_gdd_9_1_backdrop_is_layered_ridges_with_the_main_peak_left_of_centre() -> void:
	var backdrop: TitleBackdrop = _title.backdrop
	assert_eq(backdrop.ridges.size(), TitleBackdrop.LAYERS, "由近到遠幾層山影")
	var peak := backdrop.main_peak()
	assert_gt(peak.x, FLOW_VIEW.x * 0.2, "主峰在中間偏左：%s" % peak)
	assert_lt(peak.x, FLOW_VIEW.x * 0.58, "主峰在中間偏左：%s" % peak)
	assert_lt(peak.y, FLOW_VIEW.y * 0.4, "主峰頂在畫面上方：%s" % peak)
	assert_gt(peak.y, 0.0, "主峰頂在畫面內")
	var logo: Rect2 = _title.logo.logo_rect()
	assert_lt(backdrop.tone_at(logo.get_center().x), backdrop.tone_at(FLOW_VIEW.x * 0.4) * 0.8, "標題與選單後面的山影淡下去")
	for column in range(1, TitleBackdrop.COLUMNS - 1):
		for layer in TitleBackdrop.LAYERS:
			var bend := absf(backdrop.crest(layer, column - 1).y + backdrop.crest(layer, column + 1).y - 2.0 * backdrop.crest(layer, column).y)
			assert_lt(bend, FLOW_VIEW.y * 0.03, "山稜平順，沒有尖刺：第 %s 層第 %s 欄" % [layer, column])
	for layer in TitleBackdrop.LAYERS - 1:
		assert_gt(backdrop.foot_y(layer), backdrop.foot_y(layer + 1), "遠的一層山腳比近的一層高")


func test_gdd_9_1_backdrop_fog_is_one_field_that_thickens_with_distance() -> void:
	var backdrop: TitleBackdrop = _title.backdrop
	for layer in TitleBackdrop.LAYERS:
		var fog := backdrop.fog_of(layer)
		assert_eq(fog.shader, TitleBackdrop.FOG, "每層山影用同一片霧場")
		assert_eq(fog.get_shader_parameter("scale"), TitleBackdrop.FOG_SCALE, "各層的霧團一樣大，是同一片霧")
		if layer > 0:
			assert_gt(fog.get_shader_parameter("depth"), backdrop.fog_of(layer - 1).get_shader_parameter("depth"), "越遠的山被霧遮得越多")
	assert_eq(backdrop.fog_of(0).get_shader_parameter("depth"), 0.0, "最近一層不另外淡掉")


func test_gdd_9_1_backdrop_ranges_run_unbroken_across_the_whole_width() -> void:
	var backdrop: TitleBackdrop = _title.backdrop
	var tallest := 0.0
	var rises: Array[float] = []
	var ranked := 0
	for segment in 8:
		var best := 0.0
		var layers_up := 0
		for layer in TitleBackdrop.LAYERS:
			var up := 0.0
			for column in range(segment * TitleBackdrop.COLUMNS / 8, (segment + 1) * TitleBackdrop.COLUMNS / 8):
				up = maxf(up, backdrop.foot_y(layer) - backdrop.crest(layer, column).y)
			best = maxf(best, up)
			if up > FLOW_VIEW.y * 0.05:
				layers_up += 1
		rises.append(best)
		tallest = maxf(tallest, best)
		ranked += int(layers_up >= 3)
	for segment in 8:
		assert_gt(rises[segment], tallest * 0.4, "山連綿橫過整幅畫面，第 %s 段也有山：%s" % [segment, rises])
	assert_gt(ranked, 5, "多數地方至少疊著三層山：%s 段" % ranked)


func test_gdd_9_1_backdrop_is_the_latest_saved_world() -> void:
	var saved := SimWorld.new()
	assert_eq(saved.create(4242, "small"), "")
	assert_eq(SaveStore.save_manual(0, saved), "")
	var other: Node = (load(TITLE) as PackedScene).instantiate()
	tree.root.add_child(other)
	await frames(2)
	assert_eq(other.backdrop_options(), saved.world_options(), "有存檔時背景是最近那份存檔的世界")
	tree.root.remove_child(other)
	other.queue_free()
	SaveStore.delete(SaveStore.latest()["path"])
	var fresh: Node = (load(TITLE) as PackedScene).instantiate()
	tree.root.add_child(fresh)
	await frames(2)
	assert_ne(fresh.backdrop_options()["seed"], 4242, "沒有存檔時是另取的種子")
	tree.root.remove_child(fresh)
	fresh.queue_free()


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
