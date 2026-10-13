extends "res://tests/harness/suite.gd"
## GDD 8.11 地圖人物：2D 棋子（CultivatorFigure）+ 3D 設計、匯出成關節資料的動作（actions.json）；
## 棋子立在 3D 場景裡一塊後仰的面片上，從鏡頭看恆為 30° 俯角、畫面高度 80–160 px、鏡頭拉遠就不畫；天劫奇觀裡當事人立在事件所在格。

const CF := preload("res://modules/ui_kit/code/cultivator_figure.gd")
const ACTIONS_PATH := "res://modules/figure/resources/actions.json"
const ACTIONS := ["stand", "brace", "struck"]
const JOINTS := ["hip", "neck", "elbow_f", "hand_f", "elbow_b", "hand_b"]


func suite_name() -> String:
	return "figure_actor"


func _actor(figure_seed: int, given := {}) -> FigureActor:
	var actor := FigureActor.new()
	tree.root.add_child(actor)
	track(actor)
	actor.setup(FigureActor.look_of(figure_seed, given))
	return actor


func test_gdd_8_11_action_data_loads_with_every_frame_complete() -> void:
	assert_true(FileAccess.file_exists(ACTIONS_PATH), "動作資料在遊戲資源裡")
	for action in ACTIONS:
		assert_true(FigureMotion.names().has(action), "有動作 " + action)
		var count := FigureMotion.frame_count(action)
		assert_gt(count, 10, action + " 有逐格資料")
		for i in count + 1:
			var pose := FigureMotion.frame_pose(action, i)
			for key in JOINTS:
				assert_true(pose.has(key) and pose[key] is Vector2, "%s 第 %d 格有關節 %s" % [action, i, key])
			for key in ["head_roll", "wind", "seated", "point", "anchors"]:
				assert_true(pose.has(key), "%s 第 %d 格有 %s" % [action, i, key])


func test_gdd_8_11_every_joint_stays_in_a_sane_range() -> void:
	for action in ACTIONS:
		for i in FigureMotion.frame_count(action) + 1:
			var pose := FigureMotion.frame_pose(action, i)
			for key in JOINTS:
				var p: Vector2 = pose[key]
				assert_true(p.is_finite() and absf(p.x) <= 0.6 and p.y >= -1.3 and p.y <= 0.0, "%s 第 %d 格 %s 在身高 1 的範圍內：%s" % [action, i, key, p])
			var neck: Vector2 = pose["neck"]
			var hip: Vector2 = pose["hip"]
			assert_true(neck.y < hip.y - 0.2, "%s 第 %d 格頸在腰上方" % [action, i])
			assert_true(absf(float(pose["head_roll"])) <= 1.3, "%s 第 %d 格頭的後仰不超過 75°" % [action, i])
			assert_true(float(pose["wind"]) >= 0.0 and float(pose["wind"]) <= 0.4, "風量在範圍內")
			for key: String in pose["anchors"]:
				assert_true((pose["anchors"][key] as Vector2).is_finite(), "定位點是有限數：" + key)


func test_gdd_8_11_stand_first_frame_is_the_cover_stand_pose() -> void:
	var cover: Dictionary = CF.POSES[CF.Pose.STAND]
	var first := FigureMotion.frame_pose("stand", 0)
	for key in JOINTS:
		assert_true((first[key] as Vector2).distance_to(cover[key]) < 1e-4, "立第 0 格的 %s 與封面立姿相同" % key)
	assert_eq(first["seated"], false)


func test_gdd_8_11_timing_notes_become_events_and_lengths() -> void:
	# sources/figure/direct/struck/timing.md：受雷建議排 36–40 格；雷擊中在開頭，後仰極點在中段，蹲得最低在反彈。
	var frames := FigureMotion.frame_count("struck")
	assert_true(frames >= 36 and frames <= 40, "受雷 36–40 格（timing.md），實得 %d" % frames)
	assert_true(absf(FigureMotion.length("struck") - frames / 24.0) < 1e-6, "每秒 24 格")
	assert_false(FigureMotion.loops("struck"), "受雷不循環")
	var hit := FigureMotion.event_time("struck", "hit")
	var peak := FigureMotion.event_time("struck", "peak")
	var lowest := FigureMotion.event_time("struck", "lowest")
	assert_true(hit >= 0.0 and hit < peak and peak < lowest and lowest < FigureMotion.length("struck"), "受擊 < 後仰極點 < 蹲最低 < 結束")
	# brace：進場後停住循環；timing.md 發力到極點 6 格、本動作進場 12 格含預備。
	assert_true(FigureMotion.loops("brace"))
	assert_gt(FigureMotion.event_time("brace", "raised"), 0.2)
	assert_true(FigureMotion.loop_start("brace") >= FigureMotion.event_time("brace", "raised") - 1e-6, "舉臂到頂之後才循環")
	assert_eq(FigureMotion.event_time("stand", "hit"), -1.0, "沒有的事件是 −1")


func test_gdd_8_11_three_actions_have_different_poses() -> void:
	var stand := FigureMotion.pose("stand", 0.5)
	var brace := FigureMotion.pose("brace", 1.2)
	var peak := FigureMotion.pose("struck", FigureMotion.event_time("struck", "peak"))
	var low := FigureMotion.pose("struck", FigureMotion.event_time("struck", "lowest"))
	assert_true((brace["hand_f"] as Vector2).y < (stand["hand_f"] as Vector2).y - 0.3, "迎雷時遠臂的手舉過頭頂，高過垂手")
	assert_true(float(brace["head_roll"]) > 0.25, "迎雷時頭後仰")
	assert_true((peak["neck"] as Vector2).x > (stand["neck"] as Vector2).x + 0.05, "後仰極點：頸往身後（+x）移")
	assert_true((low["neck"] as Vector2).x < (peak["neck"] as Vector2).x - 0.05, "蹲到最低時身體已前傾回來")
	assert_true((low["hip"] as Vector2).y > (stand["hip"] as Vector2).y + 0.03, "蹲低時腰比立姿低")
	var seen := {}
	for p: Dictionary in [stand, brace, peak, low]:
		seen[str([p["hand_f"], p["hand_b"], p["neck"], p["head_roll"]])] = true
	assert_eq(seen.size(), 4, "四個時刻的姿勢兩兩不同")


func test_gdd_8_11_struck_holds_its_last_pose_and_loops_wrap() -> void:
	var end := FigureMotion.length("struck")
	assert_eq(str(FigureMotion.pose("struck", end + 3.0)), str(FigureMotion.pose("struck", end + 9.0)), "受雷不循環，停在最後一格")
	assert_eq(str(FigureMotion.pose("struck", 0.0)), str(FigureMotion.pose("struck", -2.0)), "時間為負停在第 0 格")
	var loop := FigureMotion.length("stand")
	assert_eq(str(FigureMotion.pose("stand", 0.3)), str(FigureMotion.pose("stand", 0.3 + loop)), "站立循環，過一圈回到原姿")
	var start := FigureMotion.loop_start("brace")
	var span := FigureMotion.length("brace") - start
	assert_eq(str(FigureMotion.pose("brace", start + 0.4)), str(FigureMotion.pose("brace", start + 0.4 + span)), "迎雷進場後繞圈")
	assert_ne(str(FigureMotion.pose("brace", 0.0)), str(FigureMotion.pose("brace", start + span)), "進場只播一次：第 0 格不等於循環裡的任何格")


func test_gdd_8_11_same_look_and_age_draw_the_same_polygons() -> void:
	var a := _actor(77)
	var b := _actor(77)
	for action in ACTIONS:
		a.play(action, 0.4)
		b.play(action, 0.4)
		assert_eq(str(a.polygons()), str(b.polygons()), "同一個 look 與 age 畫出相同的多邊形：" + action)
	a.play("brace", 0.4)
	var first := str(a.polygons())
	a.play("brace", 0.9)
	assert_ne(str(a.polygons()), first, "age 不同畫面不同（動作是活的）")
	a.play("brace", 0.4)
	assert_eq(str(a.polygons()), first, "回到同一個 age 回到同一個畫面（不依引擎時間）")
	assert_ne(str(_actor(78).polygons()), str(_actor(77).polygons()), "換種子是另一個人")


func test_gdd_8_11_joint_dictionary_poses_match_the_enum_poses() -> void:
	# 既有列舉的行為不可改：把列舉對應的關節表當字典餵進去，畫出來的多邊形與折線逐點相同。
	for s in [3, 11, 25, 41, 90]:
		var look := FigureActor.look_of(s, {"prop": CF.Prop.NONE})
		for pose in [CF.Pose.STAND, CF.Pose.GESTURE, CF.Pose.MEDITATE]:
			assert_eq(str(CF.shapes(look, pose, 0.7)), str(CF.shapes(look, CF.POSES[pose], 0.7)), "多邊形相同：種子 %d 姿勢 %d" % [s, pose])
			assert_eq(str(CF.folds(look, pose, 0.7)), str(CF.folds(look, CF.POSES[pose], 0.7)), "折線相同")


func test_gdd_8_11_every_identity_draws_every_action_without_breaking() -> void:
	for sex in [CF.Sex.MALE, CF.Sex.FEMALE]:
		for age in [CF.Age.CHILD, CF.Age.ADULT, CF.Age.ELDER]:
			for school in [CF.School.NONE, CF.School.RU, CF.School.SHI, CF.School.DAO]:
				var look := FigureActor.look_of(40 + sex * 7 + age, {"sex": sex, "age": age, "school": school})
				for action in ACTIONS:
					for t in [0.0, 0.3, 0.8, 1.4]:
						var shapes := CF.shapes(look, FigureMotion.pose(action, t), t)
						assert_gt(shapes.size(), 6, "有畫出東西")
						for polygon in shapes:
							assert_gt(polygon.size(), 2, "每塊至少三點")
							for p in polygon:
								assert_true(p.is_finite(), "座標是有限數")
						var box := CF.bounds_of(shapes)
						assert_true(FigureActor.FRAME.encloses(box), "%s 的畫面在棋子的畫框內：%s" % [action, box])


func test_gdd_8_11_figure_leans_back_to_a_fixed_30_degree_view() -> void:
	# 正交鏡頭：俯角 60°（MapView 現行）、方位角 0° 與 ±45°；棋子的上軸與視線夾 60°（= 90° − 30°），倒向畫面上方。
	for azimuth in [0.0, 45.0, -45.0]:
		var camera_basis := Basis(Vector3.UP, deg_to_rad(azimuth)) * Basis(Vector3.RIGHT, -MapView.pitch())
		var v := camera_basis.z
		var basis := FigureActor.stand_basis(v, Vector3.UP, camera_basis.y, 0.0)
		var up_axis := basis.y.normalized()
		assert_true(absf(rad_to_deg(up_axis.angle_to(v)) - 60.0) < 0.01, "上軸與視線夾 60°（方位 %s）" % azimuth)
		assert_true(up_axis.dot(camera_basis.y) > 0.0, "往畫面上方倒")
		assert_true(absf(up_axis.dot(camera_basis.x)) < 1e-4, "轉軸是鏡頭右軸：上軸沒有左右分量")
		assert_true(basis.z.dot(v) > 0.99 or basis.x.dot(camera_basis.x) > 0.99, "棋子正對鏡頭，鏡頭轉動時不轉身")
	var up := Vector3(0.3, 0.9, -0.3).normalized()
	var sight := Vector3(0.2, 0.55, 0.8).normalized()
	var screen_up := (Vector3.UP - sight * sight.dot(Vector3.UP)).normalized()
	var tilted := FigureActor.stand_basis(sight, up, screen_up, 0.0)
	var in_plane := (tilted.y - screen_up.cross(sight).normalized() * tilted.y.dot(screen_up.cross(sight).normalized())).normalized()
	assert_true(absf(rad_to_deg(in_plane.angle_to(sight)) - 60.0) < 0.01, "星球上方向：平面內上軸與視線夾 60°")
	var flat := FigureActor.stand_basis(sight, screen_up, screen_up, 0.0)
	assert_true(absf(rad_to_deg(flat.y.angle_to(sight)) - 60.0) < 0.01, "上方向在平面內時上軸與視線夾 60°")
	var nadir := FigureActor.stand_basis(Vector3.UP, Vector3.UP, Vector3.FORWARD, 0.0)
	assert_true(nadir.y.dot(Vector3.FORWARD) > 0.8, "正俯視也倒向畫面上方")
	assert_true(nadir.is_finite(), "正俯視也算得出")


func test_gdd_8_11_height_is_80_to_160_pixels() -> void:
	var unit := 1.0
	for ppu: float in [3.0, 10.0, 25.0, 60.0, 400.0]:
		var height := FigureActor.world_height(ppu, unit)
		var px: float = height * ppu * cos(deg_to_rad(FigureActor.VIEW_PITCH_DEG))
		assert_true(px >= 80.0 - 0.01 and px <= 160.0 + 0.01, "畫面高 %s 在 80–160 內（每單位 %s 像素）" % [px, ppu])
	assert_eq(FigureActor.world_height(20.0, 2.0) * 20.0 * cos(deg_to_rad(30.0)) >= 159.99, true, "介面單位放大時範圍跟著放大")


func test_gdd_8_11_card_texture_is_drawn_at_screen_resolution() -> void:
	# 墨線在畫面上約 1.6 px：貼圖的像素與畫面像素同尺度（不糊、不成髮絲）。
	for px: float in [80.0, 120.0, 160.0, 320.0]:
		var tex := FigureActor.texture_scale(px)
		assert_true(tex >= px and tex <= px * 1.6, "每單位 %s 畫面像素的貼圖密度 %s 在 1–1.6 倍內" % [px, tex])
		assert_eq(int(tex) % 16, 0, "貼圖密度以 16 為階，縮放時不逐像素重配")


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


func test_gdd_8_11_mirror_switch_flips_the_card_without_touching_the_pose() -> void:
	var actor := _actor(6)
	var camera := Camera3D.new()
	tree.root.add_child(camera)
	track(camera)
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = 20.0
	actor.place(Vector3.ZERO, Vector3.UP, camera, 1.0)
	var left := actor.card_scale_x()
	assert_true(left > 0.0, "預設朝左（棋子的原朝向）")
	actor.face_right = true
	actor.place(Vector3.ZERO, Vector3.UP, camera, 1.0)
	assert_true(actor.card_scale_x() < 0.0 and absf(absf(actor.card_scale_x()) - left) < 1e-6, "朝右是左右鏡像，大小不變")


func test_gdd_8_11_game_has_no_3d_figure_assets() -> void:
	assert_false(FileAccess.file_exists("res://modules/figure/assets/figure.glb"), "遊戲資源裡沒有 figure.glb")
	assert_false(FileAccess.file_exists("res://modules/figure/code/figure_ink.gdshader"), "沒有 3D 墨線著色器")
	var actor := _actor(4)
	assert_eq(actor.find_children("*", "Skeleton3D", true, false).size(), 0, "沒有即時骨架")
	assert_eq(actor.find_children("*", "AnimationPlayer", true, false).size(), 0, "沒有 AnimationPlayer")


func test_gdd_8_11_anchors_follow_the_pose() -> void:
	var actor := _actor(5)
	actor.play("stand", 0.0)
	var foot := actor.anchor("foot")
	var crown_stand := actor.anchor("crown")
	assert_true(absf(foot.y) < 1e-6 and crown_stand.y < -0.9, "腳底在地面、頭頂在身高附近")
	actor.play("brace", 1.2)
	assert_true(actor.anchor("palm_f").y < crown_stand.y - 0.05, "迎雷時遠臂的掌心高過站立時的頭頂")
	assert_eq(actor.anchor("nope"), Vector2.ZERO, "沒有的定位點是零向量")


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
	var hit := FigureMotion.event_time("struck", "hit")
	assert_eq(stage.figure_action(0.2)["action"], "brace", "劫雲聚攏時迎雷")
	assert_eq(stage.figure_action(TribulationStage.BOLT_START - 0.01)["action"], "brace")
	var first := TribulationStage.BOLT_START
	var gap := TribulationStage.BOLT_SPAN / 4.0
	for k in 4:
		var doing := stage.figure_action(first + gap * k + 0.05)
		assert_eq(doing["action"], "struck", "第 %d 道落雷受雷" % (k + 1))
		assert_true(absf(float(doing["time"]) - (hit + 0.05)) < 0.02, "每道雷從「雷擊中」那一格起算（資料裡的 hit 事件）")
	assert_eq(stage.figure_action(TribulationStage.END - 0.001)["action"], "stand", "奇觀結束前已站起來")
	var last := first + gap * 3.0
	assert_eq(stage.figure_action(last + FigureMotion.length("struck") * 2.0)["action"], "stand", "受雷播完就站著")


func test_gdd_5_6_struck_always_finishes_before_the_spectacle_ends() -> void:
	# 雷多、落得晚時，受雷縮時播完，不在奇觀結束前停在半空。
	for bolts in [1, 3, 4, 6, 8]:
		var stage := _stage(bolts)
		var end_pose := stage.figure_action(TribulationStage.END - 0.02)
		assert_eq(end_pose["action"], "stand", "%d 道雷：結束前站起" % bolts)


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
	assert_true(actor != null, "舞台上有棋子")
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
