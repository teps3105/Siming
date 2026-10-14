extends SceneTree
## 全動作代表格的 64 像素高並排圖與剪影差異矩陣（8.11 驗收）。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy -s res://tests/harness/figure_silhouettes.gd -- --out=<資料夾>
## 產出 silhouettes_64.png（上色）、silhouettes_64_black.png（純黑剪影），預設持杖；加 --noprop 改畫沒有持物的樣子，檔名加 _noprop；矩陣印在輸出。視窗要開。

const S := preload("res://tests/harness/figure_silhouette.gd")
const F := preload("res://modules/ui_kit/code/cultivator_figure.gd")
const CELL := Vector2i(S.COLS, S.ROWS)

var _out := ""
var _suffix := ""
var _view: SubViewport
var _ticks := 0
var _names: Array = []


class Strip:
	extends Control

	var look := {}
	var names: Array = []

	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO, size), Color.WHITE, true)
		for i in names.size():
			var pose := FigureMotion.frame_pose(names[i], int(S.REP[names[i]]))
			var flip := float(pose.get("flip", 1.0))
			var origin := Vector2(float(i * S.COLS) + (-S.X0) * S.PX, (-S.Y0) * S.PX)
			# 64 像素高：CF.draw 的 height 是成人身高像素；facing −1 是原朝向
			F.draw(self, origin, S.PX, -1.0 * signf(flip) if flip != 0.0 else -1.0, look, pose, 0.4, Color(0.1, 0.08, 0.06, 0.94))


func _initialize() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg == "--noprop":
			_suffix = "_noprop"
	DirAccess.make_dir_recursive_absolute(_out)
	_names = S.REP.keys()
	var strip := Strip.new()
	strip.look = S.look_of(_suffix == "")
	strip.names = _names
	var size := Vector2i(S.COLS * _names.size(), S.ROWS)
	strip.size = Vector2(size)
	_view = SubViewport.new()
	_view.size = size
	_view.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(_view)
	_view.add_child(strip)
	var masks := S.masks_of(_names, _suffix == "")
	var black := Image.create(size.x, size.y, false, Image.FORMAT_RGBA8)
	black.fill(Color.WHITE)
	for i in _names.size():
		var mask: PackedByteArray = masks[_names[i]]
		for r in S.ROWS:
			for c in S.COLS:
				if mask[r * S.COLS + c] == 1:
					black.set_pixel(i * S.COLS + c, r, Color.BLACK)
	black.save_png(_out.path_join("silhouettes_64_black%s.png" % _suffix))
	var header := "%-9s" % ""
	for n: String in _names:
		header += "%6s" % n.substr(0, 5)
	print(header)
	for a: String in _names:
		var line := "%-9s" % a
		for b: String in _names:
			line += "%6.2f" % S.difference(masks[a], masks[b])
		print(line)
	print("動作順序：", _names)


func _process(_delta: float) -> bool:
	_ticks += 1
	if _ticks == 6:
		_view.get_texture().get_image().save_png(_out.path_join("silhouettes_64%s.png" % _suffix))
		print("截圖：", _out)
		quit()
	return false
