extends SceneTree
## 肢體動作的剪影差異矩陣（不開視窗）：godot --headless --path game -s res://tests/harness/figure_silhouette_matrix.gd -- --names=a,b,c [--armed]
## 一併列出 stand、bow、seal；預設沒持物，--armed 持杖。

const S := preload("res://tests/harness/figure_silhouette.gd")


func _initialize() -> void:
	var names: Array = ["stand", "bow", "seal"]
	var armed := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--names="):
			for n in arg.trim_prefix("--names=").split(","):
				names.append(n)
		elif arg == "--armed":
			armed = true
	var masks := S.masks_of(names, armed)
	var header := "%-9s" % ""
	for n: String in names:
		header += "%6s" % n.substr(0, 5)
	print(header)
	for a: String in names:
		var line := "%-9s" % a
		for b: String in names:
			line += "%6.2f" % S.difference(masks[a], masks[b])
		print(line)
	quit()
