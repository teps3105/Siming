extends SceneTree
## 世界快進（GDD 2.6 / 5.7）的畫面檢查：1920×1080 與 1280×800 各存天時儀加終點選單、快進進行中、停下的摘要。
## 要開視窗才畫得出來；開發時手動執行，不屬於測試套件。

const Suite := preload("res://tests/harness/suite.gd")
const VIEWS := [
	["full", Vector2i(1920, 1080)],
	["deck", Vector2i(1280, 800)],
]

var _out := "user://fast_shots"
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
	for view: Array in VIEWS:
		await _view(view[0], view[1])
	await _suite._finish_case()
	Suite.wipe_storage()
	quit(0)


func _shot(name: String) -> void:
	await _suite.wait(0.4)
	root.get_texture().get_image().save_png(_out.path_join("%s.png" % name))
	print("截圖：%s" % name)


func _view(prefix: String, view: Vector2i) -> void:
	GameSettings.use_file("user://harness/fast_shots.cfg", true)
	GameSettings.set_value("tutorial_done", true)
	var game: Node = await _suite.boot(false, view)
	root.size = view
	await _suite.frames(4)
	var hud: Hud = game.get_node("Hud")
	var sim: SimHost = game.get_node("SimHost")
	# 天時儀加終點選單。
	await _suite.key(KEY_R)
	await _shot("%s-1-menu" % prefix)
	await _suite.key(KEY_ESCAPE)
	# 快進進行中：一千年，跑 0.4 秒就拍。
	sim.fast_forward(SimHost.Stop.YEARS, 1000)
	await _suite.wait(0.4)
	await _shot("%s-2-fast" % prefix)
	sim.fast_stop()
	await _suite.frames(3)
	await _shot("%s-3-summary-stopped" % prefix)
	hud.fast_panel().close()
	# 往下一件世界大事快進，停下的摘要。
	sim.fast_forward(SimHost.Stop.YEARS, 100)
	while sim.fast_active():
		await _suite.frames(5)
	await _shot("%s-4-summary-100y" % prefix)
	root.remove_child(game)
	game.queue_free()
	await _suite.frames(2)
