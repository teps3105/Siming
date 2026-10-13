extends SceneTree
## 修士放大圖：把指定身分的幾個修士放大畫在一張紙上存成 PNG，用來看部件的細節（GDD 5.8）。
## 用法：godot --path game -s res://tests/harness/figure_zoom.gd -- --out=<檔案> [--given=<JSON>] [--vary=<欄位>:<值,值,...>]
##        [--pose=<姿勢編號>] [--height=<身高像素>] [--count=<幾個>] [--seed=<種子>] [--focus=head|body]
## given 是 traits 的指定欄位（列舉寫數字）；vary 讓每一個人的某個欄位各取一個值，沒給就每個人換一個種子。
## focus=head 只看頭與胸（紙高 420），body 看全身。

const F := preload("res://modules/ui_kit/code/cultivator_figure.gd")


class Sheet:
	extends Control

	var given := {}
	var vary_key := ""
	var vary_values: Array = []
	var pose := 0
	var height := 640.0
	var count := 6
	var first_seed := 11
	var head_only := true

	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO, size), Ink.PAPER, true)
		for column in count:
			var traits := given.duplicate()
			if vary_key != "":
				traits[vary_key] = vary_values[column % vary_values.size()]
			var look := F.traits(first_seed if vary_key != "" else first_seed + column, traits)
			var foot := Vector2((column + 0.5) * size.x / count, height * 1.1 if head_only else size.y - 30.0)
			F.draw(self, foot, height, -1.0, look, pose, 0.4, Color(Ink.INK, 0.94))


func _initialize() -> void:
	var out := "user://figure_zoom.png"
	var sheet := Sheet.new()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--given="):
			var parsed: Variant = JSON.parse_string(arg.trim_prefix("--given="))
			if parsed is Dictionary:
				for key: String in parsed:
					sheet.given[key] = int(parsed[key]) if parsed[key] is float and key != "beard" else parsed[key]
		elif arg.begins_with("--vary="):
			var pair := arg.trim_prefix("--vary=").split(":")
			sheet.vary_key = pair[0]
			for value in pair[1].split(","):
				sheet.vary_values.append(int(value))
			sheet.count = sheet.vary_values.size()
		elif arg.begins_with("--pose="):
			sheet.pose = int(arg.trim_prefix("--pose="))
		elif arg.begins_with("--height="):
			sheet.height = float(arg.trim_prefix("--height="))
		elif arg.begins_with("--count="):
			sheet.count = int(arg.trim_prefix("--count="))
		elif arg.begins_with("--seed="):
			sheet.first_seed = int(arg.trim_prefix("--seed="))
		elif arg.begins_with("--focus="):
			sheet.head_only = arg.trim_prefix("--focus=") == "head"
	# 畫在離屏的視口上，紙的大小不受視窗與螢幕限制。
	var paper := SubViewport.new()
	var cell := sheet.height * (0.36 if sheet.head_only else 0.8)
	paper.size = Vector2i(int(cell * sheet.count), int(sheet.height * 0.66) if sheet.head_only else int(sheet.height * 1.25))
	paper.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(paper)
	sheet.size = Vector2(paper.size)
	paper.add_child(sheet)
	for i in 4:
		await process_frame
	paper.get_texture().get_image().save_png(out)
	quit(0)
