extends "res://tests/harness/suite.gd"
## 流程：引導簽（GDD 5.7、9.1）——一次一句，做到了換下一句，全部做過不再出現，可在設定重看。
## 設定寫在使用者資料夾底下另一個目錄，不碰玩家的檔案。

const ROOT := "user://test_guide"

var _game: Node
var _sim: SimHost
var _camera: MapCamera
var _hud: Hud


func suite_name() -> String:
	return "guide_journey"


func setup() -> void:
	_wipe()
	SaveStore.root = ROOT.path_join("saves")
	GameSettings.file_path = ROOT.path_join("settings.cfg")
	GameSettings.load_file()
	_game = await boot(false, FLOW_VIEW, true)
	_sim = _game.get_node("SimHost")
	_camera = _game.get_node("MapCamera")
	_hud = _game.get_node("Hud")
	await frames(2)


func teardown() -> void:
	_wipe()


func _wipe() -> void:
	var path := ProjectSettings.globalize_path(ROOT)
	if DirAccess.dir_exists_absolute(path):
		for name in DirAccess.get_files_at(path):
			DirAccess.remove_absolute(path.path_join(name))
		DirAccess.remove_absolute(path)


func _guide() -> GuideSlip:
	return _hud.guide()


func test_guide_walks_through_every_operation_once() -> void:
	assert_true(_guide().visible, "第一次進世界就出現")
	assert_eq(_guide().step(), &"pan")
	var bounds := Vector2(FLOW_VIEW)
	assert_lt(absf(_guide().position.x + _guide().size.x * 0.5 - bounds.x * 0.5), 1.0, "在畫面下方中央")
	var seals_top: float = _hud._seals.position.y + _hud._seals._badge_position(0).y
	assert_lt(absf(seals_top - (_guide().position.y + _guide().size.y) - 24.0), 1.0, "在印鈕列上方 24 px")

	await joy_axis(JOY_AXIS_LEFT_X, 1.0, 0.6)
	assert_eq(_guide().step(), &"zoom", "平移過就換下一句")
	assert_true(_guide().gamepad, "用搖桿時畫搖桿的鍵")
	await wheel(bounds * 0.5, true)
	await wheel(bounds * 0.5, true)
	await wait(0.5)
	assert_eq(_guide().step(), &"globe")
	await key(KEY_G)
	assert_eq(_guide().step(), &"time")
	await key(KEY_ESCAPE)
	await key(KEY_ESCAPE)
	assert_eq(_guide().step(), &"time", "開關選單造成的暫停不算")
	await key(KEY_SPACE)
	assert_eq(_guide().step(), &"mode")
	await key(KEY_F2)
	assert_eq(_guide().step(), &"select")
	await key(KEY_G)
	await wait(1.0)
	await click(bounds * 0.5)
	if _guide().step() == &"select":
		# 畫面正中是水域時改用鎖定最近的州郡。
		await joy_button(JOY_BUTTON_A)
	assert_eq(_guide().step(), &"", "全部做過")
	await frames(2)
	assert_false(_guide().visible, "做完就不再出現")
	assert_true(GameSettings.flag("tutorial_done"))


func test_guide_progress_survives_a_restart_and_can_be_replayed() -> void:
	GameSettings.set_value("tutorial_step", 3)
	_guide().setup(_camera, 100.0, 1.0)
	assert_eq(_guide().step(), &"time", "從上次的進度接著教")
	GameSettings.set_value("tutorial_done", true)
	_guide().setup(_camera, 100.0, 1.0)
	assert_eq(_guide().step(), &"")
	# 設定頁的「重看引導」。
	await key(KEY_ESCAPE)
	var deck := _hud.menu()
	for i in 3:
		await key(KEY_DOWN)
	await key(KEY_ENTER)
	assert_eq(deck.page(), &"settings")
	await key(KEY_UP)
	assert_eq(deck.panel().focused_id(), &"set.tutorial")
	await key(KEY_ENTER)
	assert_false(GameSettings.flag("tutorial_done"))
	await key(KEY_ESCAPE)
	await key(KEY_ESCAPE)
	await frames(2)
	assert_eq(_guide().step(), &"pan", "從頭再來")
	assert_true(_guide().visible)


func test_guide_stays_out_of_the_way_while_the_menu_is_open() -> void:
	assert_true(_guide().visible)
	await key(KEY_ESCAPE)
	await frames(2)
	assert_false(_guide().visible, "選單開著時收起來")
