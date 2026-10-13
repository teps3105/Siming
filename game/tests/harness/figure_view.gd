extends SceneTree
## 3D 小人的讀圖：以 30° 俯角的正交鏡頭在紙色底上放大畫一個 FigureActor，左邊並排同一種子的封面 2D 人物（GDD 8.11「驗收」）。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy -s res://tests/harness/figure_view.gd -- --out=<PNG>
##        [--action=stand] [--age=0.5] [--seed=11] [--given=<JSON>] [--height=600] [--yaw=<度>] [--no2d] [--hair=<列舉>]
## 視窗要開（不能 --headless），畫面抓取後即結束。

const F := preload("res://modules/ui_kit/code/cultivator_figure.gd")

var _frames := 0
var _out := "user://figure_view.png"
var _seed := 11
var _given := {}
var _height := 600.0
var _show_2d := true


class Cover:
	extends Control

	var look := {}
	var height := 600.0

	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO, size), Ink.PAPER, true)
		F.draw(self, Vector2(size.x * 0.5, size.y - 90.0), height, -1.0, look, F.Pose.STAND, 0.4, Color(Ink.INK, 0.94))


func _initialize() -> void:
	var action := "stand"
	var age := 0.5
	var yaw := rad_to_deg(FigureActor.DEFAULT_YAW)
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--action="):
			action = arg.trim_prefix("--action=")
		elif arg.begins_with("--age="):
			age = float(arg.trim_prefix("--age="))
		elif arg.begins_with("--seed="):
			_seed = int(arg.trim_prefix("--seed="))
		elif arg.begins_with("--height="):
			_height = float(arg.trim_prefix("--height="))
		elif arg.begins_with("--yaw="):
			yaw = float(arg.trim_prefix("--yaw="))
		elif arg == "--no2d":
			_show_2d = false
		elif arg.begins_with("--given="):
			var parsed: Variant = JSON.parse_string(arg.trim_prefix("--given="))
			if parsed is Dictionary:
				for key: String in parsed:
					_given[key] = int(parsed[key]) if parsed[key] is float else parsed[key]
	var size := Vector2i(1500, 800)
	root.size = size
	var look := F.traits(_seed, _given)
	var cover := Cover.new()
	cover.look = look
	cover.height = _height * 0.85
	cover.size = Vector2(750, 800)
	if _show_2d:
		var layer := CanvasLayer.new()
		layer.add_child(cover)
		root.add_child(layer)
	var actor := FigureActor.new()
	root.add_child(actor)
	actor.setup(look)
	actor.play(action, age)
	# 正交鏡頭 30° 俯角；人物高 1 在畫面上 _height 像素（直立的人在 30° 俯角下鉛直長度乘 cos 30°）。
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = size.y * cos(deg_to_rad(30.0)) / _height
	camera.h_offset = -(0.25 * size.x) / (size.y / camera.size) if _show_2d else 0.0
	root.add_child(camera)
	camera.look_at_from_position(Vector3(0.0, 0.5 + 2.0, 3.46), Vector3(0.0, 0.5, 0.0), Vector3.UP)
	actor.rotation.y = deg_to_rad(yaw)
	for material in actor._fills:
		material.set_shader_parameter("depth_pull", 0.0)
	for material in actor._inks:
		material.set_shader_parameter("depth_pull", 0.0)
	print("樣子：", {"sex": look["sex"], "age": look["age"], "wealth": look["wealth"], "school": look["school"], "hair": look["hair"],
		"prop": look["prop"], "heads": look["heads"], "stature": look["stature"]}, " 顯示 ", actor.shown, " 缺 ", actor.missing)


func _process(_delta: float) -> bool:
	_frames += 1
	if _frames == 6:
		var image := root.get_viewport().get_texture().get_image()
		image.save_png(_out)
		print("存到 ", _out, " ", image.get_size())
		quit()
	return false
