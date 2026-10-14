extends SceneTree
## 「地圖標籤」開關（GDD 5.3、5.9）的畫面檢查：同一視角存標籤開、標籤關各一張，再存選單裡該項的畫面。
## 要開視窗才畫得出來；開發時手動執行，不屬於測試套件。

const Suite := preload("res://tests/harness/suite.gd")

var _out := "user://label_shots"
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
	GameSettings.use_file("user://harness/label_shots.cfg", true)
	GameSettings.set_value("tutorial_done", true)
	var game: Node = await _suite.boot(false, Vector2i(1920, 1080))
	root.size = Vector2i(1920, 1080)
	var camera: MapCamera = game.get_node("MapCamera")
	camera.z = (game.get_node("TerrainMap") as TerrainMap).map_size.x * 0.15
	await _suite.frames(4)
	var hud: Hud = game.get_node("Hud")
	var overlay: MapOverlay = hud._overlay
	await _shot("1-labels-on")
	print("標籤數（開）：%d" % overlay.labels_drawn())
	await _suite.key(KEY_ESCAPE)
	var deck := hud.menu()
	deck._on_chosen(&"settings")
	await _suite.frames(3)
	# 設定頁從第一列起，往下到「地圖標籤」那一列。
	while deck.panel().focused_id() != &"set.map_labels":
		await _suite.key(KEY_DOWN)
	await _suite.frames(3)
	await _shot("2-menu-row")
	await _suite.key(KEY_RIGHT)
	await _suite.key(KEY_ESCAPE)
	await _suite.key(KEY_ESCAPE)
	await _suite.frames(4)
	await _shot("3-labels-off")
	print("標籤數（關）：%d" % overlay.labels_drawn())
	await _suite._finish_case()
	Suite.wipe_storage()
	quit(0)


func _shot(name: String) -> void:
	await _suite.wait(0.4)
	root.get_texture().get_image().save_png(_out.path_join("%s.png" % name))
	print("截圖：%s" % name)
