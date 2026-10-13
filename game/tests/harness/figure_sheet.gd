extends SceneTree
## 純棋子的讀圖：一列放大畫出一個動作的幾個關鍵時刻，最左邊是封面同種子的 stand 當對照（GDD 8.11「驗收」）。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy -s res://tests/harness/figure_sheet.gd -- --out=<PNG>
##        [--seed=11] [--action=struck] [--frames=0,2,6,10] [--given=<JSON>] [--height=520] [--joints]
## 視窗要開（不能 --headless），畫面抓取後即結束。--joints 在每個棋子上標出關節點（紅是 _f，藍是 _b）。

const F := preload("res://modules/ui_kit/code/cultivator_figure.gd")

var _out := "user://figure_sheet.png"
var _ticks := 0
var _view: SubViewport


class Sheet:
	extends Control

	var look := {}
	var action := "stand"
	var frames: Array[int] = []
	var height := 520.0
	var joints := false

	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO, size), Ink.PAPER, true)
		var step := size.x / float(frames.size() + 1)
		var base := size.y - 60.0
		F.draw(self, Vector2(step * 0.5, base), height, -1.0, look, F.Pose.STAND, 0.4, Color(Ink.INK, 0.94))
		draw_string(ThemeDB.fallback_font, Vector2(step * 0.5 - 40.0, size.y - 20.0), "cover stand", HORIZONTAL_ALIGNMENT_LEFT, -1, 22, Ink.INK)
		for i in frames.size():
			var at := Vector2(step * (i + 1.5), base)
			var t: float = frames[i] / 24.0
			var pose := FigureMotion.pose(action, t)
			F.draw(self, at, height, -1.0, look, pose, t, Color(Ink.INK, 0.94))
			draw_string(ThemeDB.fallback_font, Vector2(at.x - 50.0, size.y - 20.0), "%s f%d" % [action, frames[i]], HORIZONTAL_ALIGNMENT_LEFT, -1, 22, Ink.INK)
			if joints:
				var grow: float = height * float(look["stature"])
				for key in ["hip", "neck", "elbow_f", "hand_f", "elbow_b", "hand_b"]:
					var p: Vector2 = pose[key]
					draw_circle(at + p * grow, 4.0, Color.RED if key.ends_with("_f") else Color.BLUE)


func _initialize() -> void:
	var action := "stand"
	var seed_value := 11
	var given := {}
	var height := 520.0
	var show_joints := false
	var frames: Array[int] = []
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--action="):
			action = arg.trim_prefix("--action=")
		elif arg.begins_with("--seed="):
			seed_value = int(arg.trim_prefix("--seed="))
		elif arg.begins_with("--height="):
			height = float(arg.trim_prefix("--height="))
		elif arg == "--joints":
			show_joints = true
		elif arg.begins_with("--frames="):
			for part in arg.trim_prefix("--frames=").split(","):
				frames.append(int(part))
		elif arg.begins_with("--given="):
			var parsed: Variant = JSON.parse_string(arg.trim_prefix("--given="))
			if parsed is Dictionary:
				for key: String in parsed:
					given[key] = int(parsed[key]) if parsed[key] is float else parsed[key]
	if frames.is_empty():
		frames = [0, 6, 12, 24, 36]
	var size := Vector2i(int(height * 1.05 * (frames.size() + 1)), int(height * 1.25))
	_view = SubViewport.new()
	_view.size = size
	_view.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(_view)
	var sheet := Sheet.new()
	sheet.look = F.traits(seed_value, given)
	sheet.action = action
	sheet.frames = frames
	sheet.height = height
	sheet.joints = show_joints
	sheet.size = Vector2(size)
	_view.add_child(sheet)
	var look: Dictionary = sheet.look
	print("樣子：", {"sex": look["sex"], "age": look["age"], "wealth": look["wealth"], "school": look["school"], "hair": look["hair"],
		"prop": look["prop"], "heads": look["heads"], "stature": look["stature"]})


func _process(_delta: float) -> bool:
	_ticks += 1
	if _ticks == 6:
		DirAccess.make_dir_recursive_absolute(_out.get_base_dir())
		_view.get_texture().get_image().save_png(_out)
		print("截圖：", _out)
		quit()
	return false
