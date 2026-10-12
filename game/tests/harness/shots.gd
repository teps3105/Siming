extends SceneTree
## 介面品質檢查表（GDD 5.7）用的截圖：把標題、選單各頁與遊玩畫面，在 1920×1080 鍵鼠、1920×1080 搖桿、
## 1280×800 搖桿三種畫面下各存一張 PNG。要開視窗才畫得出來，由 tools/tests/shots.py 啟動。
## 存檔與設定寫在測試用的資料夾，不碰玩家的檔案。

const Suite := preload("res://tests/harness/suite.gd")
const TITLE := "res://app/scenes/title.tscn"
## 每一種畫面：[檔名的前綴, 畫面大小, 是否用搖桿]。
const LAYOUTS := [
	["kb", Vector2i(1920, 1080), false],
	["pad", Vector2i(1920, 1080), true],
	["deck", Vector2i(1280, 800), true],
]

var _out := "user://shots"
var _suite: Suite


func _initialize() -> void:
	_main.call_deferred()


func _main() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
	DirAccess.make_dir_recursive_absolute(_out)
	_suite = Suite.new()
	_suite.tree = self
	_suite._begin_case()
	GameSettings.load_file()
	for layout: Array in LAYOUTS:
		await _game_pages(layout[0], layout[1], layout[2])
		await _title_pages(layout[0], layout[1], layout[2])
	await _suite._finish_case()
	_wipe(Suite.TEST_STORAGE)
	quit(0)


## 刪掉截圖時寫下的存檔與設定。
func _wipe(dir: String) -> void:
	var path := ProjectSettings.globalize_path(dir)
	if not DirAccess.dir_exists_absolute(path):
		return
	for sub in DirAccess.get_directories_at(path):
		_wipe(dir.path_join(sub))
	for file in DirAccess.get_files_at(path):
		DirAccess.remove_absolute(path.path_join(file))
	DirAccess.remove_absolute(path)


func _shot(name: String) -> void:
	await _suite.wait(0.5)
	root.get_texture().get_image().save_png(_out.path_join("%s.png" % name))
	print("截圖：%s" % name)


## 讓畫面認得目前用的是哪一種輸入裝置。
func _use_device(pad: bool) -> void:
	# 用不會動到地圖的鍵，引導簽才停在第一句。
	if pad:
		await _suite.joy_button(JOY_BUTTON_B)
	else:
		await _suite.key(KEY_SHIFT)


func _menu_device(panel: MenuPanel, pad: bool) -> void:
	if pad:
		await _suite.joy_button(JOY_BUTTON_DPAD_DOWN)
		await _suite.joy_button(JOY_BUTTON_DPAD_UP)
	else:
		await _suite.key(KEY_DOWN)
		await _suite.key(KEY_UP)
	panel.queue_redraw()


func _game_pages(prefix: String, view: Vector2i, pad: bool) -> void:
	GameSettings.set_value("tutorial_done", false)
	GameSettings.set_value("tutorial_step", 0)
	var game: Node = await _suite.boot(false, view, true)
	root.size = view
	await _suite.frames(4)
	var hud: Hud = game.get_node("Hud")
	var sim: SimHost = game.get_node("SimHost")
	await _use_device(pad)
	await _shot("%s-play" % prefix)
	hud._card.show_province(0)
	await _shot("%s-card" % prefix)
	hud._card.show_province(-1)
	SaveStore.save_manual(0, sim.world)
	SaveStore.save_manual(3, sim.world)
	hud.open_menu()
	await _suite.frames(2)
	var deck := hud.menu()
	await _menu_device(deck.panel(), pad)
	await _shot("%s-pause" % prefix)
	for page: StringName in [&"save", &"settings", &"share"]:
		deck._show(page)
		await _menu_device(deck.panel(), pad)
		await _shot("%s-pause-%s" % [prefix, page])
	root.remove_child(game)
	game.queue_free()
	await _suite.frames(2)


func _title_pages(prefix: String, view: Vector2i, pad: bool) -> void:
	var title: Node = (load(TITLE) as PackedScene).instantiate()
	title.backdrop_seed = Suite.TEST_SEED
	title.framing_roll = 7
	root.add_child(title)
	await _suite.frames(2)
	AudioServer.set_bus_mute(0, true)
	root.size = view
	await _suite.frames(4)
	var deck: MenuDeck = title.deck
	await _menu_device(deck.panel(), pad)
	await _shot("%s-title" % prefix)
	deck._reset_options()
	for page: StringName in [&"new", &"load", &"settings", &"credits"]:
		deck._show(page)
		await _menu_device(deck.panel(), pad)
		await _shot("%s-title-%s" % [prefix, page])
	root.remove_child(title)
	title.queue_free()
	await _suite.frames(2)
