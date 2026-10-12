extends "res://tests/harness/suite.gd"
## GDD 5.8 修士剪影演算法（CultivatorFigure）：種子決定一名修士的樣子，同一套規則畫出許多不同的修士——
## 男女老少、貧富、儒釋道都在裡面。

const F := preload("res://modules/ui_kit/code/cultivator_figure.gd")


func suite_name() -> String:
	return "cultivator_figure"


func test_gdd_5_8_same_seed_gives_the_same_cultivator() -> void:
	assert_eq(F.traits(7), F.traits(7))
	assert_eq(F.shapes(F.traits(7), F.Pose.STAND, 1.5), F.shapes(F.traits(7), F.Pose.STAND, 1.5))


func test_gdd_5_8_different_seeds_give_different_cultivators() -> void:
	var seen := {"sex": {}, "age": {}, "wealth": {}, "school": {}, "hair": {}, "prop": {}, "whiskers": {}}
	var outlines := {}
	for i in 300:
		var look := F.traits(1000 + i)
		for key: String in seen:
			seen[key][look[key]] = true
		outlines[str(F.shapes(look, F.Pose.STAND))] = true
	assert_eq(seen["sex"].size(), F.Sex.size(), "男女都有")
	assert_eq(seen["age"].size(), F.Age.size(), "老少都有")
	assert_eq(seen["wealth"].size(), F.Wealth.size(), "貧富都有")
	assert_eq(seen["school"].size(), F.School.size(), "散修與儒釋道都有")
	assert_eq(seen["hair"].size(), F.Hair.size(), "每種髮式都出現")
	assert_eq(seen["prop"].size(), F.Prop.size(), "每種持物都出現")
	assert_eq(seen["whiskers"].size(), F.Whiskers.size(), "每種鬍鬚樣式都出現")
	assert_eq(outlines.size(), 300, "300 個種子是 300 個不同的剪影")


func test_gdd_5_8_given_identity_is_kept_and_shapes_the_look() -> void:
	for i in 20:
		var nun := F.traits(i, {"sex": F.Sex.FEMALE, "school": F.School.SHI})
		assert_eq(nun["sex"], F.Sex.FEMALE)
		assert_eq(nun["hair"], F.Hair.SHAVEN, "僧尼落髮")
		assert_eq(nun["beard"], 0.0, "女子無鬚")
		var child := F.traits(i, {"age": F.Age.CHILD})
		assert_lt(child["stature"], 0.75, "孩童矮")
		assert_lt(child["heads"], 5.5, "孩童頭身少")
		assert_eq(child["beard"], 0.0)
		var beggar := F.traits(i, {"wealth": F.Wealth.POOR})
		var lord := F.traits(i, {"wealth": F.Wealth.RICH})
		assert_lt(beggar["sleeve"], lord["sleeve"], "貧者窄袖、富者廣袖")
		assert_false(beggar["cape"], "貧者沒有大氅")
		assert_eq(F.traits(i, {"prop": F.Prop.LOTUS})["prop"], F.Prop.LOTUS, "持物可以指定")


func test_gdd_5_8_every_identity_and_pose_draws_as_valid_polygons_within_reach() -> void:
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
							for polygon in polygons:
								assert_false(Geometry2D.triangulate_polygon(polygon).is_empty(), "多邊形畫得出來：%s 姿勢 %d" % [look, pose])
							var box := F.bounds_of(polygons)
							assert_lt(box.size.x, 1.7, "寬度在伸手與飄帶可及的範圍內")
							assert_lt(-box.position.y, 1.45, "高度不超過身高加冠與杖")
							assert_lt(box.end.y, 0.31, "腳下只有衣擺與垂穗")


func test_gdd_5_8_seated_poses_are_about_half_as_tall() -> void:
	var look := F.traits(3, {"age": F.Age.ADULT, "hair": F.Hair.TOPKNOT, "prop": F.Prop.NONE})
	var standing := F.bounds_of(F.shapes(look, F.Pose.STAND))
	var seated := F.bounds_of(F.shapes(look, F.Pose.MEDITATE))
	assert_gt(-standing.position.y, 0.95)
	assert_lt(-seated.position.y, 0.72)
	assert_gt(-seated.position.y, 0.5)


func test_gdd_5_8_the_whole_beard_is_one_layer_between_face_and_hair() -> void:
	# 鬍鬚不論樣式都是一塊一層；頷下的冠纓、笠帶在它後面，頭髮在它前面，腦後的飄帶在臉後面。
	for hair: int in [F.Hair.TOPKNOT, F.Hair.CROWN, F.Hair.HAT, F.Hair.LOOSE, F.Hair.SCHOLAR_CAP]:
		for style: int in F.Whiskers.values():
			var look := F.traits(11, {"sex": F.Sex.MALE, "age": F.Age.ELDER, "hair": hair, "prop": F.Prop.NONE, "beard": 0.2, "whiskers": style})
			for pose: int in F.Pose.values():
				for polygon in F.shapes(look, pose, 0.4):
					assert_false(Geometry2D.triangulate_polygon(polygon).is_empty(), "髮式 %d 鬍鬚 %d 姿勢 %d 畫得出來" % [hair, style, pose])
				var beards := 0
				var last := -1
				for layer: Array in F.layers(look, pose, 0.4):
					assert_true(layer[0] >= last, "各層依層次表由後往前")
					last = layer[0]
					if layer[1] == F.BEARD:
						beards += 1
						assert_eq(layer[0], F.Layer.BEARD, "鬍鬚在鬍鬚那一層")
					if layer[1] == F.HAIR:
						assert_true(layer[0] != F.Layer.FACE and layer[0] != F.Layer.BEARD, "頭髮不混在臉與鬍鬚的層次裡")
				assert_eq(beards, 0 if style == F.Whiskers.NONE else 1, "鬍鬚 %d 是一層" % style)
				var order := F.layers(look, pose, 0.4)
				assert_eq(order.filter(func(layer: Array) -> bool: return layer[0] == F.Layer.NECK).size(), 1, "頸自成一層")
				assert_eq(order.filter(func(layer: Array) -> bool: return layer[0] == F.Layer.COLLAR).size(), 2, "衣領兩襟各一層")
				assert_true(F.Layer.ROBE < F.Layer.NECK and F.Layer.NECK < F.Layer.COLLAR and F.Layer.COLLAR < F.Layer.FACE, "頸在袍前面，衣領圍在頸外面，臉在衣領前面")
