extends SceneTree
## 量測：各動作、各種樣子的剪影範圍（身高為 1 的座標），決定 FigureActor.FRAME 的畫框。
## 用法：godot --headless --path game -s res://tests/harness/figure_extent.gd

func _initialize() -> void:
	var worst := {"x0": 0.0, "x1": 0.0, "y0": 0.0, "y1": 0.0}
	var who := {}
	var cost := 0.0
	var count := 0
	for s in 160:
		var look := FigureActor.look_of(s)
		for action in FigureMotion.names():
			var n := FigureMotion.frame_count(action)
			for f in range(0, n + 1, 3):
				var t := f / 24.0
				var started := Time.get_ticks_usec()
				var shapes := CultivatorFigure.shapes(look, FigureMotion.pose(action, t), t)
				cost += Time.get_ticks_usec() - started
				count += 1
				var box := CultivatorFigure.bounds_of(shapes)
				var grow: float = look["stature"]
				var x0 := box.position.x
				var x1 := box.end.x
				var y0 := box.position.y
				var y1 := box.end.y
				for key in [["x0", x0, -1], ["x1", x1, 1], ["y0", y0, -1], ["y1", y1, 1]]:
					if (key[2] < 0 and key[1] < worst[key[0]]) or (key[2] > 0 and key[1] > worst[key[0]]):
						worst[key[0]] = key[1]
						who[key[0]] = "種子 %d %s 第 %d 格 hair=%d prop=%d cape=%s" % [s, action, f, look["hair"], look["prop"], look["cape"]]
	print("最大範圍 ", worst)
	print(who)
	print("shapes 平均 %.2f ms（%d 次）" % [cost / count / 1000.0, count])
	quit()
