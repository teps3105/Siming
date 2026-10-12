extends SceneTree
## 找出哪一個身分、姿勢的哪一個部位的多邊形畫不出來（開發用）。

const F := preload("res://modules/ui_kit/code/cultivator_figure.gd")


func _initialize() -> void:
	var seen := {}
	var index := 0
	for sex: int in F.Sex.values():
		for age: int in F.Age.values():
			for wealth: int in F.Wealth.values():
				for school: int in F.School.values():
					for prop: int in [F.traits(index)["prop"], index % F.Prop.size()]:
						index += 1
						var look := F.traits(50 + index, {"sex": sex, "age": age, "wealth": wealth, "school": school, "prop": prop})
						for pose: int in F.Pose.values():
							var polygons := F.shapes(look, pose, index * 0.7)
							for i in polygons.size():
								if Geometry2D.triangulate_polygon(polygons[i]).is_empty():
									var part := -1
									for candidate in 16:
										if F.pieces(look, pose, candidate, index * 0.7).has(polygons[i]):
											part = candidate
									var key := "pose %d part %d points %d" % [pose, part, polygons[i].size()]
									if not seen.has(key):
										seen[key] = true
										print(key, " sex ", sex, " age ", age, " wealth ", wealth, " school ", school, " hair ", look["hair"], " ", polygons[i])
	quit(0)
