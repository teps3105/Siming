extends "res://tests/harness/suite.gd"
## GDD 8.11 人物模型與動作：同一副骨架、依樣子顯示的部件、三個動作（立、迎雷、受雷）、
## 從鏡頭看恆為 30° 俯角、畫面高度 80–160 px、鏡頭拉遠就不畫；天劫奇觀裡當事人立在事件所在格。

const CF := preload("res://modules/ui_kit/code/cultivator_figure.gd")
const GLB_PATH := "res://modules/figure/assets/figure.glb"


func suite_name() -> String:
	return "figure_actor"


func _actor(figure_seed: int, given := {}) -> FigureActor:
	var actor := FigureActor.new()
	tree.root.add_child(actor)
	track(actor)
	actor.setup(FigureActor.look_of(figure_seed, given))
	return actor


func test_gdd_8_11_skeleton_and_file_are_within_budget() -> void:
	var actor := _actor(1)
	assert_eq(actor.skeleton().get_bone_count(), 32, "32 根骨頭")
	assert_true(actor.skeleton().get_bone_count() <= 40, "骨頭上限 40")
	assert_true(FileAccess.get_file_as_bytes(GLB_PATH).size() <= 2 * 1024 * 1024, "檔案 2 MB 以內")
	var total := 0
	for key in actor.part_names():
		var node := actor.skeleton().find_child(key, true, false) as MeshInstance3D
		total += node.mesh.get_faces().size() / 3
	assert_true(total <= 20000, "整個檔案三角形 2 萬以內")
	var names := actor.animation_names()
	for action in ["stand", "brace", "struck"]:
		assert_true(names.has(action), "有動作 " + action)


func test_gdd_8_11_every_identity_shows_existing_parts() -> void:
	for sex in [CF.Sex.MALE, CF.Sex.FEMALE]:
		for age in [CF.Age.CHILD, CF.Age.ADULT, CF.Age.ELDER]:
			for school in [CF.School.NONE, CF.School.RU, CF.School.SHI, CF.School.DAO]:
				for s in 3:
					var actor := _actor(40 + s, {"sex": sex, "age": age, "school": school})
					var look := FigureActor.look_of(40 + s, {"sex": sex, "age": age, "school": school})
					assert_true(actor.shown.has("body.base"), "一定有身體")
					for part in actor.shown:
						assert_true(actor.part_names().has(FigureActor._key(part)), "顯示的部件在檔案裡：" + part)
					for part in FigureActor.parts_for(look):
						assert_true(actor.shown.has(part) or actor.missing.has(part), "每個想要的部件不是顯示就是記為缺：" + part)


func test_gdd_8_11_every_action_poses_every_look() -> void:
	for s in 12:
		var actor := _actor(300 + s)
		for action in ["stand", "brace", "struck"]:
			for age in [0.0, 0.3, 1.1, 5.0]:
				actor.play(action, age)
				for i in actor.skeleton().get_bone_count():
					var pose := actor.skeleton().get_bone_pose(i)
					assert_true(pose.origin.is_finite() and pose.basis.is_finite(), "骨頭變換是有限數")


func test_gdd_8_11_same_look_and_age_give_the_same_pose() -> void:
	var a := _actor(77)
	var b := _actor(77)
	for action in ["stand", "brace", "struck"]:
		a.play(action, 0.4)
		b.play(action, 0.4)
		assert_eq(a.pose_signature(), b.pose_signature(), "同一個 look 與 age 擺出相同的骨頭：" + action)
	a.play("brace", 0.4)
	var first := a.pose_signature()
	a.play("brace", 0.9)
	assert_ne(a.pose_signature(), first, "age 不同姿勢不同（動作是活的）")
	a.play("brace", 0.4)
	assert_eq(a.pose_signature(), first, "回到同一個 age 回到同一個姿勢（不依引擎時間）")


func test_gdd_8_11_struck_holds_its_last_pose_and_loops_wrap() -> void:
	var actor := _actor(5)
	var length := actor.action_length("struck")
	assert_gt(length, 0.5, "受雷有長度")
	actor.play("struck", length + 3.0)
	var end := actor.pose_signature()
	actor.play("struck", length + 9.0)
	assert_eq(actor.pose_signature(), end, "受雷不循環，停在最後一格")
	var loop := actor.action_length("stand")
	actor.play("stand", 0.3)
	var at := actor.pose_signature()
	actor.play("stand", 0.3 + loop)
	assert_eq(actor.pose_signature(), at, "站立循環，過一圈回到原姿")


func test_gdd_8_11_figure_leans_back_to_a_fixed_30_degree_view() -> void:
	# 正交鏡頭：俯角 60°（MapView 現行）、方位角 0° 與 ±45°；人物的上軸與視線夾 60°（= 90° − 30°），倒向畫面上方。
	for azimuth in [0.0, 45.0, -45.0]:
		var camera_basis := Basis(Vector3.UP, deg_to_rad(azimuth)) * Basis(Vector3.RIGHT, -MapView.pitch())
		var v := camera_basis.z
		var basis := FigureActor.stand_basis(v, Vector3.UP, camera_basis.y, 0.0)
		var up_axis := basis.y.normalized()
		assert_true(absf(rad_to_deg(up_axis.angle_to(v)) - 60.0) < 0.01, "上軸與視線夾 60°（方位 %s）" % azimuth)
		assert_true(up_axis.dot(camera_basis.y) > 0.0, "往畫面上方倒")
		assert_true(absf(up_axis.dot(camera_basis.x)) < 1e-4, "轉軸是鏡頭右軸：上軸沒有左右分量")
		assert_true(absf(basis.determinant()) > 0.99 and absf(basis.determinant()) < 1.01, "只轉動不變形")
	# 透視視線與星球上方向：上方向不在「視線與畫面上方」的平面內（球面曲率），在這個平面內量的俯角仍是 30°。
	var up := Vector3(0.3, 0.9, -0.3).normalized()
	var sight := Vector3(0.2, 0.55, 0.8).normalized()
	var screen_up := (Vector3.UP - sight * sight.dot(Vector3.UP)).normalized()
	var tilted := FigureActor.stand_basis(sight, up, screen_up, 0.0)
	var in_plane := (tilted.y - screen_up.cross(sight).normalized() * tilted.y.dot(screen_up.cross(sight).normalized())).normalized()
	assert_true(absf(rad_to_deg(in_plane.angle_to(sight)) - 60.0) < 0.01, "星球上方向：平面內上軸與視線夾 60°")
	var flat := FigureActor.stand_basis(sight, screen_up, screen_up, 0.0)
	assert_true(absf(rad_to_deg(flat.y.angle_to(sight)) - 60.0) < 0.01, "上方向在平面內時上軸與視線夾 60°")
	# 正俯視（上方向與視線同向）：倒向畫面上方，不是亂指的方向。
	var nadir := FigureActor.stand_basis(Vector3.UP, Vector3.UP, Vector3.FORWARD, 0.0)
	assert_true(nadir.y.dot(Vector3.FORWARD) > 0.8, "正俯視也倒向畫面上方")
	var looking_down := FigureActor.stand_basis(Vector3.UP, Vector3.UP, Vector3.FORWARD, 0.0)
	assert_true(looking_down.is_finite(), "正俯視也算得出")


func test_gdd_8_11_height_is_80_to_160_pixels() -> void:
	var unit := 1.0
	for ppu: float in [3.0, 10.0, 25.0, 60.0, 400.0]:
		var height := FigureActor.world_height(ppu, unit)
		var px: float = height * ppu * cos(deg_to_rad(FigureActor.VIEW_PITCH_DEG))
		assert_true(px >= 80.0 - 0.01 and px <= 160.0 + 0.01, "畫面高 %s 在 80–160 內（每單位 %s 像素）" % [px, ppu])
	assert_eq(FigureActor.world_height(20.0, 2.0) * 20.0 * cos(deg_to_rad(30.0)) >= 159.99, true, "介面單位放大時範圍跟著放大")


func test_gdd_8_11_hidden_when_the_camera_is_high() -> void:
	assert_false(FigureActor.too_far(100.0, 400.0), "高度小於地圖寬 0.4 倍：畫")
	assert_true(FigureActor.too_far(161.0, 400.0), "高度超過地圖寬 0.4 倍：不畫")
	var actor := _actor(2)
	var camera := Camera3D.new()
	tree.root.add_child(camera)
	track(camera)
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 20.0
	actor.place(Vector3.ZERO, Vector3.UP, camera, 1.0, true)
	assert_false(actor.visible, "hidden 為真就不顯示")
	actor.place(Vector3.ZERO, Vector3.UP, camera, 1.0, false)
	assert_true(actor.visible, "hidden 為假就顯示")


func test_gdd_8_11_person_fields_become_look_and_missing_ones_come_from_seed() -> void:
	var given := FigureActor.given_from_person({"gender": "Female", "age": 22, "lifespan": 100})
	assert_eq(given["sex"], CF.Sex.FEMALE)
	assert_eq(given["age"], CF.Age.ADULT)
	assert_false(given.has("school"), "橋接給不出門派（僧以外）：由種子決定")
	assert_false(given.has("wealth"), "橋接給不出貧富：由種子決定")
	assert_eq(FigureActor.given_from_person({"gender": "Male", "age": 70, "lifespan": 90})["age"], CF.Age.ELDER)
	assert_eq(FigureActor.given_from_person({"gender": "Male", "age": 10})["age"], CF.Age.CHILD)
	assert_eq(FigureActor.given_from_person({"gender": "Male", "age": 40, "fruit": 1})["school"], CF.School.SHI, "有果位的是僧")
	assert_eq(FigureActor.look_of(9, given)["sex"], CF.Sex.FEMALE, "given 的欄位照用")


func _stage(bolts := 4) -> TribulationStage:
	var stage := TribulationStage.new()
	tree.root.add_child(stage)
	track(stage)
	stage.setup({"id": 5, "kind": "Tribulation", "seed": 48, "bolts": bolts, "outcome": "Passed", "person": 12,
		"given": {"sex": CF.Sex.MALE, "age": CF.Age.ELDER}})
	return stage


func test_gdd_5_6_tribulation_figure_braces_then_is_struck_once_per_bolt_then_stands() -> void:
	var stage := _stage(4)
	assert_eq(stage.figure_action(0.2)["action"], "brace", "劫雲聚攏時迎雷")
	assert_eq(stage.figure_action(TribulationStage.BOLT_START - 0.01)["action"], "brace")
	var first := TribulationStage.BOLT_START
	var gap := TribulationStage.BOLT_SPAN / 4.0
	for k in 4:
		var doing := stage.figure_action(first + gap * k + 0.05)
		assert_eq(doing["action"], "struck", "第 %d 道落雷受雷" % (k + 1))
		assert_true(absf(float(doing["time"]) - 0.05) < 1e-4, "每道雷從受雷動作的開頭起算")
	assert_eq(stage.figure_action(TribulationStage.OUTCOME_START + 0.5)["action"], "stand", "收尾時站著")
	assert_eq(stage.figure_action(TribulationStage.END)["action"], "stand")


func test_gdd_5_6_tribulation_figure_stands_on_the_event_cell_and_is_hidden_when_far() -> void:
	var stage := _stage(3)
	var camera := Camera3D.new()
	tree.root.add_child(camera)
	track(camera)
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 30.0
	camera.global_transform = Transform3D(Basis(Vector3.RIGHT, deg_to_rad(-60.0)), Vector3(0.0, 20.0, 20.0))
	stage.global_position = Vector3(3.0, 1.0, -2.0)
	stage.tick(1.0, {"scale": 5.0, "u": 1.0, "px_world": 0.05, "beam_height": 6.0, "camera": camera, "up": Vector3.UP, "figure_hidden": false})
	var actor: FigureActor = null
	for child in stage.get_children():
		if child is FigureActor:
			actor = child
	assert_true(actor != null, "舞台上有小人")
	assert_true(actor.visible, "鏡頭近時畫")
	assert_true(actor.global_position.distance_to(Vector3(3.0, 1.0, -2.0)) < 1e-4, "腳底在事件所在格")
	stage.tick(1.0, {"scale": 5.0, "u": 1.0, "px_world": 0.05, "beam_height": 6.0, "camera": camera, "up": Vector3.UP, "figure_hidden": true})
	assert_false(actor.visible, "鏡頭拉遠不畫")


func test_gdd_5_6_spec_of_carries_the_principal_of_a_tribulation() -> void:
	var spectacle := Spectacle.new()
	track(spectacle)
	var by_a := spectacle.spec_of({"id": 3, "kind": "Tribulation", "pos": Vector2(4, 4), "n": 2, "x": "Passed", "a_person": 41})
	assert_eq(by_a["person"], 41, "當事人取 a_person")
	var by_list := spectacle.spec_of({"id": 4, "kind": "Tribulation", "pos": Vector2(4, 4), "n": 2, "x": "Passed", "a_person": -1,
		"persons": PackedInt32Array([8, 9])})
	assert_eq(by_list["person"], 8, "沒有 a_person 就取 persons 的第一位")
	var none := spectacle.spec_of({"id": 5, "kind": "Tribulation", "pos": Vector2(4, 4), "x": "Passed"})
	assert_eq(none["person"], -1, "都沒有就是 -1")
	assert_eq(none["given"], {}, "沒有當事人：樣子全由種子決定")
