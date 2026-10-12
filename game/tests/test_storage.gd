extends "res://tests/harness/suite.gd"
## GDD 4.8 存檔欄位、自動存檔與分享碼；GDD 9.1 設定。測試寫在使用者資料夾底下另一個目錄，不碰玩家的檔案。

const SEED := 20261011
static var ROOT: String = TEST_STORAGE.path_join("storage")

var _world: SimWorld


func suite_name() -> String:
	return "storage"


func setup() -> void:
	_wipe()
	SaveStore.root = ROOT.path_join("saves")
	GameSettings.file_path = ROOT.path_join("settings.cfg")
	GameSettings.load_file()


func teardown() -> void:
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


func _small_world() -> SimWorld:
	if _world == null:
		_world = SimWorld.new()
		assert_eq(_world.create(SEED, "small"), "")
	return _world


func test_gdd_4_8_manual_slots_start_empty() -> void:
	var slots := SaveStore.manual_slots()
	assert_eq(slots.size(), SaveStore.MANUAL_SLOTS)
	for slot: Dictionary in slots:
		assert_true(slot.is_empty())
	assert_true(SaveStore.latest().is_empty())


func test_gdd_4_8_manual_save_records_the_world_and_loads_back() -> void:
	var world := _small_world()
	assert_eq(SaveStore.save_manual(3, world), "")
	var slots := SaveStore.manual_slots()
	assert_false(slots[3].is_empty())
	assert_true(slots[2].is_empty())
	var entry: Dictionary = slots[3]
	assert_eq(entry["name"], world.world_options()["name"])
	assert_eq(entry["date"], world.date_short())
	assert_eq(entry["tick"], world.tick())
	assert_eq(entry["kind"], "manual")
	# 摘要帶開局選項，標題畫面據此重建同一個世界。
	var options := world.world_options()
	assert_eq(entry["options"].size(), options.size())
	for key: String in options:
		assert_eq(str(entry["options"][key]), str(options[key]), key)
	assert_eq(SaveStore.read_text(entry["path"]), world.save_text())
	assert_eq(SaveStore.latest()["path"], entry["path"])

	var loaded := SimWorld.new()
	assert_eq(loaded.load_begin(SaveStore.read_text(entry["path"])), "")
	while not loaded.load_step(100000)["done"]:
		pass
	assert_eq(loaded.hash_hex(), world.hash_hex(), "存→讀後世界雜湊相同")


func test_gdd_4_8_auto_saves_keep_only_the_newest_five() -> void:
	var world := SimWorld.new()
	assert_eq(world.create(SEED, "small"), "")
	var ticks: Array[int] = []
	for i in SaveStore.AUTO_KEEP + 2:
		world.step(1)
		ticks.append(world.tick())
		assert_eq(SaveStore.save_auto(world), "")
	var autos := SaveStore.auto_saves()
	assert_eq(autos.size(), SaveStore.AUTO_KEEP)
	assert_eq(autos[0]["tick"], ticks[-1], "最新的排最前")
	assert_eq(autos[-1]["tick"], ticks[2], "最舊的兩份已刪")
	assert_eq(SaveStore.latest()["tick"], ticks[-1])


func test_gdd_4_8_delete_removes_one_save() -> void:
	assert_eq(SaveStore.save_manual(0, _small_world()), "")
	var entry: Dictionary = SaveStore.manual_slots()[0]
	SaveStore.delete(entry["path"])
	assert_true(SaveStore.manual_slots()[0].is_empty())


func test_gdd_4_8_auto_save_is_due_once_per_shi() -> void:
	assert_false(SaveStore.auto_due(1, 200))
	assert_true(SaveStore.auto_due(360, 361), "跨過世的邊界")
	assert_true(SaveStore.auto_due(100, 900))
	assert_false(SaveStore.auto_due(361, 720))


func test_gdd_4_8_share_code_round_trips() -> void:
	var text := _small_world().save_text()
	var code := SaveStore.share_code(text)
	assert_true(code.begins_with(SaveStore.SHARE_PREFIX))
	assert_false(code.contains("+") or code.contains("/") or code.contains("="), "只用網址安全的字元")
	var back := SaveStore.parse_share_code("  %s\n" % code)
	assert_eq(back["error"], "")
	assert_eq(back["text"], text)


func test_gdd_4_8_bad_share_codes_are_rejected() -> void:
	assert_ne(SaveStore.parse_share_code("")["error"], "")
	assert_ne(SaveStore.parse_share_code("hello")["error"], "")
	assert_ne(SaveStore.parse_share_code(SaveStore.SHARE_PREFIX + "!!!!")["error"], "")
	assert_ne(SaveStore.parse_share_code(SaveStore.SHARE_PREFIX + "AAAA")["error"], "")


func test_gdd_9_1_settings_have_defaults_and_persist() -> void:
	assert_eq(GameSettings.ui_scale(), 1.0)
	assert_false(GameSettings.flag("tutorial_done"))
	GameSettings.set_value("ui_scale", 1.25)
	GameSettings.set_value("tutorial_done", true)
	GameSettings.load_file()
	assert_eq(GameSettings.ui_scale(), 1.25)
	assert_true(GameSettings.flag("tutorial_done"))


func test_gdd_9_1_ui_scale_is_clamped() -> void:
	GameSettings.set_value("ui_scale", 9.0)
	assert_eq(GameSettings.ui_scale(), GameSettings.UI_SCALE_MAX)
	GameSettings.set_value("ui_scale", 0.1)
	assert_eq(GameSettings.ui_scale(), GameSettings.UI_SCALE_MIN)
