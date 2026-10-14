extends SceneTree
## 純棋子的讀圖：一列放大畫出一個動作的幾個關鍵時刻，最左邊是封面同種子的 stand 當對照（GDD 8.11「驗收」）。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy -s res://tests/harness/figure_sheet.gd -- --out=<PNG>
##        [--seed=11] [--action=struck] [--frames=0,2,6,10 或 --frames=all/每隔 N 格 --every=N] [--cols=列數] [--given=<JSON>] [--height=520] [--joints]
## 每格底下標格號與落在該格的事件名；--cols 指定每列幾格（多列排片）。
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
	var cols := 0

	func _event_label(frame: int) -> String:
		var names := []
		var events: Dictionary = FigureMotion.events(action)
		for key: String in events:
			if int(events[key]) == frame:
				names.append(key)
		return " ".join(names)

	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO, size), Ink.PAPER, true)
		var per_row := cols if cols > 0 else frames.size() + 1
		var rows := ceili(float(frames.size() + 1) / float(per_row))
		var step := size.x / float(per_row)
		var row_h := size.y / float(rows)
		var font := ThemeDB.fallback_font
		for slot in frames.size() + 1:
			var row := slot / per_row
			var col := slot % per_row
			var base := row_h * (row + 1) - 60.0
			var at := Vector2(step * (col + 0.5), base)
			draw_line(Vector2(step * col, base), Vector2(step * (col + 1), base), Color(Ink.INK, 0.25), 1.0)
			if slot == 0:
				F.draw(self, at, height, -1.0, look, F.Pose.STAND, 0.4, Color(Ink.INK, 0.94))
				draw_string(font, Vector2(at.x - 40.0, base + 40.0), "cover stand", HORIZONTAL_ALIGNMENT_LEFT, -1, 22, Ink.INK)
				continue
			var i := slot - 1
			var t: float = frames[i] / 24.0
			var pose := FigureMotion.pose(action, t)
			# flip（轉身的翻面）：負的朝反向畫；0 是側薄的一線，這裡照原朝向畫
			var flip := float(pose.get("flip", 1.0))
			F.draw(self, at, height, -1.0 * (signf(flip) if flip != 0.0 else 1.0), look, pose, t, Color(Ink.INK, 0.94))
			var label := "%s f%d" % [action, frames[i]]
			draw_string(font, Vector2(at.x - 70.0, base + 26.0), label, HORIZONTAL_ALIGNMENT_LEFT, -1, 22, Ink.INK)
			var event := _event_label(frames[i])
			if event != "":
				draw_string(font, Vector2(at.x - 70.0, base + 52.0), "* " + event, HORIZONTAL_ALIGNMENT_LEFT, -1, 22, Color(0.75, 0.15, 0.1))
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
	var cols := 0
	var every := 0
	var all_frames := false
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
		elif arg.begins_with("--cols="):
			cols = int(arg.trim_prefix("--cols="))
		elif arg.begins_with("--every="):
			every = int(arg.trim_prefix("--every="))
		elif arg == "--frames=all":
			all_frames = true
		elif arg.begins_with("--frames="):
			for part in arg.trim_prefix("--frames=").split(","):
				frames.append(int(part))
		elif arg.begins_with("--given="):
			var parsed: Variant = JSON.parse_string(arg.trim_prefix("--given="))
			if parsed is Dictionary:
				for key: String in parsed:
					given[key] = int(parsed[key]) if parsed[key] is float else parsed[key]
	if all_frames or every > 0:
		for index in range(0, FigureMotion.frame_count(action) + 1, maxi(every, 1)):
			frames.append(index)
	if frames.is_empty():
		frames = [0, 6, 12, 24, 36]
	var per_row := cols if cols > 0 else frames.size() + 1
	var rows := ceili(float(frames.size() + 1) / float(per_row))
	var size := Vector2i(int(height * 1.55 * per_row), int(height * 1.4 * rows))
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
	sheet.cols = cols
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
