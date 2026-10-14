extends "res://tests/harness/suite.gd"
## GDD 8.11 肢體動作：跪、撫頂、盤坐、垂首、拂袖轉身、背身、執手、奉物、接過。
## 這批動作的重點是全身剪影：誇大到 64 像素高時兩兩分得出來；動作由 tools/figure/model/poses2d.py 以 2D 關節直接寫成。

const CF := preload("res://modules/ui_kit/code/cultivator_figure.gd")
const SIL := preload("res://tests/harness/figure_silhouette.gd")
const OLD := ["stand", "brace", "struck", "walk", "fall", "rise", "strike", "seal", "bow"]
const BODY := ["kneel", "bless", "sit", "slump", "turn", "away", "clasp", "offer", "receive"]
const EXPECT := {
	"kneel": {"frames": 54, "events": {"down": 22, "up": 36}},
	"bless": {"frames": 54, "events": {"touch": 22, "release": 40}},
	"sit": {"frames": 64, "events": {"settle": 16, "rise": 46}},
	"slump": {"frames": 60, "events": {"droop": 26, "still": 46}},
	"turn": {"frames": 40, "events": {"sweep": 12, "turned": 14}},
	"away": {"frames": 40, "events": {"turned": 20}},
	"clasp": {"frames": 56, "events": {"meet": 20, "release": 36}},
	"offer": {"frames": 60, "events": {"give": 30, "retreat": 38}},
	"receive": {"frames": 60, "events": {"take": 22, "hold": 40}},
}
## 剪影差異的門檻：不同像素占聯集的比例要超過這個數（現有 bow 對 seal 是 0.38，stand 對 seal 是 0.27）。
const SILHOUETTE_MIN := 0.30
const JOINTS := ["hip", "neck", "elbow_f", "hand_f", "elbow_b", "hand_b"]


func suite_name() -> String:
	return "figure_body_actions"


func test_gdd_8_11_body_actions_have_the_frames_and_named_events() -> void:
	for action: String in BODY:
		assert_true(FigureMotion.names().has(action), "有動作 " + action)
		assert_eq(FigureMotion.frame_count(action), EXPECT[action]["frames"], action + " 格數")
		assert_false(FigureMotion.loops(action), action + " 不循環")
		var events: Dictionary = FigureMotion.events(action)
		assert_eq(events.size(), (EXPECT[action]["events"] as Dictionary).size(), action + " 事件數")
		for name: String in EXPECT[action]["events"]:
			assert_eq(int(events.get(name, -1)), int(EXPECT[action]["events"][name]), "%s 的事件 %s" % [action, name])
			assert_lt(int(events[name]), FigureMotion.frame_count(action) + 1, "%s 的事件格號存在" % action)


func test_gdd_8_11_body_actions_start_from_stand_and_all_but_slump_end_near_it() -> void:
	var stand := FigureMotion.frame_pose("stand", 0)
	for action: String in BODY:
		var first := FigureMotion.frame_pose(action, 0)
		for key in JOINTS:
			assert_lt((first[key] as Vector2).distance_to(stand[key]), 0.03, "%s 起點的 %s 接得上 stand" % [action, key])
		assert_false(first["seated"], action + " 起點不是坐姿")
		if action == "slump":
			continue
		var last := FigureMotion.frame_pose(action, FigureMotion.frame_count(action))
		var tolerance := 0.05 if action in ["turn", "away"] else 0.03
		for key in JOINTS:
			assert_lt((last[key] as Vector2).distance_to(stand[key]), tolerance, "%s 終點的 %s 接得上 stand" % [action, key])
		assert_false(last["seated"], action + " 終點不是坐姿")


func test_gdd_8_11_slump_ends_seated_with_the_head_hanging() -> void:
	var last := FigureMotion.frame_pose("slump", FigureMotion.frame_count("slump"))
	assert_true(last["seated"], "垂首時坐著")
	assert_lt(float(last["head_roll"]), -1.0, "頭垂向前")
	assert_lt((last["neck"] as Vector2).y, (last["hip"] as Vector2).y - 0.25, "頸仍在腰之上")
	var held := FigureMotion.frame_pose("slump", FigureMotion.event_time("slump", "still") * 24.0 + 6.0)
	assert_lt((held["neck"] as Vector2).distance_to(last["neck"]) , 0.05, "不動之後頸只有極小的變化")


func test_gdd_8_11_kneel_goes_low_then_back_up_and_turn_flips_the_figure() -> void:
	var kneel_low := FigureMotion.frame_pose("kneel", 28)
	assert_gt((kneel_low["hip"] as Vector2).y, -0.3, "叩首時髖降到 −0.3 以下")
	assert_gt((kneel_low["neck"] as Vector2).y, -0.45, "叩首時頸也低")
	var upright := FigureMotion.frame_pose("kneel", 14)
	assert_gt((upright["hip"] as Vector2).y, -0.3, "跪直時髖低")
	assert_lt((upright["neck"] as Vector2).y, -0.5, "跪直時上身直起")
	for action: String in ["turn", "away"]:
		assert_eq(float(FigureMotion.frame_pose(action, 0)["flip"]), 1.0, action + " 起於原朝向")
		assert_eq(float(FigureMotion.frame_pose(action, FigureMotion.frame_count(action))["flip"]), -1.0, action + " 終於翻面")
		var turned := FigureMotion.event_time(action, "turned") * 24.0
		assert_lt(float(FigureMotion.frame_pose(action, int(turned))["flip"]), 0.0, action + " 的 turned 格已翻面")
	assert_lt(absf(float(FigureMotion.pose("turn", 12.0 / 24.0)["flip"])), 0.05, "轉到一半是側薄的一線")
	assert_gt(float(FigureMotion.frame_pose("turn", 12)["wind"]), 0.15, "甩袖時衣擺飄起")


func test_gdd_8_11_old_actions_carry_no_flip_and_keep_their_card_unflipped() -> void:
	for action: String in OLD:
		for index in FigureMotion.frame_count(action) + 1:
			assert_false(FigureMotion.frame_pose(action, index).has("flip"), "%s 第 %d 格沒有 flip 鍵（舊動作不變）" % [action, index])
	var actor := FigureActor.new()
	tree.root.add_child(actor)
	track(actor)
	actor.setup(FigureActor.look_of(6))
	var camera := Camera3D.new()
	tree.root.add_child(camera)
	track(camera)
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = tree.root.get_visible_rect().size.y / FigureActor.ppu_for_screen_px(80.0)
	actor.play("stand", 0.0)
	actor.place(Vector3.ZERO, Vector3.UP, camera, 1.0)
	var plain := actor.card_scale_x()
	assert_gt(plain, 0.0, "站立朝左不翻")
	actor.play("turn", FigureMotion.length("turn"))
	actor.place(Vector3.ZERO, Vector3.UP, camera, 1.0)
	assert_true(absf(actor.card_scale_x() + plain) < 1e-6, "轉身終點翻面：水平縮放反號、大小不變")
	actor.play("turn", 12.0 / 24.0)
	actor.place(Vector3.ZERO, Vector3.UP, camera, 1.0)
	assert_lt(absf(actor.card_scale_x()), plain * 0.1, "轉到一半面片只剩一線")
	assert_gt(absf(actor.card_scale_x()), 0.0, "但不縮成零")


func test_gdd_8_11_clasp_palm_meets_in_the_middle_of_the_pair() -> void:
	var meet := FigureMotion.frame_pose("clasp", int(FigureMotion.events("clasp")["meet"]))
	var palm: Vector2 = meet["anchors"]["palm_f"]
	assert_lt(absf(-palm.x - EventStage.PAIR_GAP), 0.04, "meet 格的掌心落在身前 PAIR_GAP：%s" % palm)
	assert_true(palm.y < -0.45 and palm.y > -0.8, "手在胸腹的高度")


func test_gdd_8_11_body_actions_stay_inside_the_frame_and_triangulate() -> void:
	for action: String in BODY:
		for given in [{}, {"sex": CF.Sex.FEMALE, "wealth": CF.Wealth.POOR}, {"age": CF.Age.ELDER, "school": CF.School.SHI}, {"sex": CF.Sex.FEMALE, "wealth": CF.Wealth.RICH}]:
			var look := FigureActor.look_of(77, given)
			for index in FigureMotion.frame_count(action) + 1:
				var box := CF.bounds_of(CF.shapes(look, FigureMotion.frame_pose(action, index), index / 24.0))
				assert_true(FigureActor.FRAME.encloses(box), "%s 第 %d 格的畫面在畫框內：%s" % [action, index, box])
	var failed := 0
	for s in 24:
		var look := FigureActor.look_of(s)
		for action: String in BODY:
			for index in range(0, FigureMotion.frame_count(action) + 1, 2):
				if not _triangulates(look, FigureMotion.frame_pose(action, index), index / 24.0):
					failed += 1
	assert_eq(failed, 0, "24 種樣子的每兩格：袖、袍與衣擺都能三角化")


func _triangulates(look: Dictionary, pose: Dictionary, t: float) -> bool:
	var fills: Array[PackedVector2Array] = []
	var lines: Array[PackedVector2Array] = []
	var bands: Array = []
	var cuts: Array = []
	CF._build(look, pose, t, fills, lines, bands, cuts)
	for cut: Array in CF._ordered(cuts):
		for piece in CF._unite(fills.slice(cut[3], cut[4])):
			if Geometry2D.triangulate_polygon(piece).is_empty():
				return false
	return true


func test_gdd_8_11_silhouettes_of_the_body_actions_differ_at_64_px() -> void:
	var names: Array = BODY.duplicate()
	for old: String in ["bow", "seal", "stand"]:
		names.append(old)
	var masks := SIL.masks_of(names)
	for i in BODY.size():
		for j in names.size():
			if j <= i and j < BODY.size():
				continue
			var d := SIL.difference(masks[names[i]], masks[names[j]])
			assert_gt(d, SILHOUETTE_MIN, "%s 與 %s 的剪影不同像素比例 %.2f 要超過 %.2f" % [names[i], names[j], d, SILHOUETTE_MIN])


func test_gdd_5_13_receive_take_lines_up_with_the_offer_give() -> void:
	var plan := EventScript.plan({"kind": "ThreadResolved", "x": "Grace", "pos": Vector2(1, 1), "a_person": 4, "b_person": 7})
	var give: float = float(plan["actors"][0]["steps"][0]["start"]) + FigureMotion.event_time("offer", "give")
	var take: float = float(plan["actors"][1]["steps"][0]["start"]) + FigureMotion.event_time("receive", "take")
	assert_lt(absf(give - take), 1e-4, "接過的 take 與奉物的 give 同一刻")
	assert_gt(float(plan["actors"][1]["steps"][0]["start"]), 0.0, "接過比奉物晚開始")
