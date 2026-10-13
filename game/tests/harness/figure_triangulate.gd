extends SceneTree
## 掃描：各種樣子 × 各動作的每一格，找出併起來後無法三角化（draw_polygon 會報 triangulation failed）的色塊。
## 封面的列舉姿勢一併掃，當作既有的底線。用法：godot --headless --path game -s res://tests/harness/figure_triangulate.gd -- [--looks=160]

const CF := preload("res://modules/ui_kit/code/cultivator_figure.gd")


func _bad(look: Dictionary, pose: Variant, t: float) -> Array:
	var fills: Array[PackedVector2Array] = []
	var lines: Array[PackedVector2Array] = []
	var bands: Array = []
	var cuts: Array = []
	CF._build(look, pose, t, fills, lines, bands, cuts)
	var found := []
	for cut: Array in CF._ordered(cuts):
		for piece in CF._unite(fills.slice(cut[3], cut[4])):
			if Geometry2D.triangulate_polygon(piece).is_empty():
				found.append([cut[1], cut[2] % CF.OPEN])
	return found


func _initialize() -> void:
	var looks := 160
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--looks="):
			looks = int(arg.trim_prefix("--looks="))
	var base := 0
	var base_total := 0
	var by_action := {}
	var first := {}
	for s in looks:
		var look := FigureActor.look_of(s)
		for pose in CF.POSES:
			base_total += 1
			if not _bad(look, pose, 0.7).is_empty():
				base += 1
		for action in FigureMotion.names():
			for f in FigureMotion.frame_count(action) + 1:
				var t := f / 24.0
				var bad := _bad(look, FigureMotion.pose(action, t), t)
				by_action[action] = by_action.get(action, 0) + 1
				if not bad.is_empty():
					by_action[action + "_bad"] = by_action.get(action + "_bad", 0) + 1
					if not first.has(action + str(f)) and first.size() < 24:
						first[action + str(f)] = "種子 %d 第 %d 格 層/部位 %s" % [s, f, bad[0]]
	print("列舉姿勢（底線）：", base, " / ", base_total)
	print("動作格：", by_action)
	for key in first:
		print("  ", first[key])
	quit()
