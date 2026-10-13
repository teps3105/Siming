extends "res://tests/harness/suite.gd"
## GDD 5.13 棋子演出：事件 → 動作對照表（動作序列與長度）、四段影片的時間線、舞台（棋子出現在事件格、依序播、播完消失）、
## 同格排隊與丟棄、等鏡頭（hold／release）、兩人相向、畫面外與鏡頭太高不建立棋子、棋子上限、暫停不前進。

const INTRO := EventScript.INTRO
const HOLD := EventScript.HOLD
const OUTRO := EventScript.OUTRO

var _game: Node
var _sim: SimHost
var _camera: MapCamera
var _map: TerrainMap
var _spec: Spectacle
var _stage: EventStage
var _next_id := 9000


func suite_name() -> String:
	return "event_stage"


func _boot() -> void:
	_game = await boot(false, FLOW_VIEW)
	_sim = _game.get_node("SimHost")
	_camera = _game.get_node("MapCamera")
	_map = _game.get_node("TerrainMap")
	_spec = tree.get_first_node_in_group("spectacle")
	_stage = tree.get_first_node_in_group("event_stage")


## 合成一筆橋接事件（欄位同 drain_events）；persons 與 pos 預設給第 0、1 位與 (10, 10)。
func _event(kind: String, extra := {}) -> Dictionary:
	_next_id += 1
	var event := {"id": _next_id, "tick": 100, "kind": kind, "importance": 3, "province": 0, "cell": -1, "pos": Vector2(10.5, 10.5),
		"a_person": 0, "b_person": 1, "a": "沈青霜", "b": "韓烈", "p": "青州", "q": "", "q_province": -1, "x": "", "n": 0, "year": 9}
	event.merge(extra, true)
	return event


func _actions(plan: Dictionary) -> Array:
	var out := []
	for actor: Dictionary in plan["actors"]:
		var names := []
		for step: Dictionary in actor["steps"]:
			names.append(step["do"])
		out.append(names)
	return out


## 鏡頭落到某州郡治所，近到畫得出棋子；回傳治所座標。
func _look_at_province(id: int, frac := 0.12) -> Vector2:
	var seat: Vector2 = _sim.world.province_info(id)["seat"]
	_camera.look_at_map(seat, _map.map_size.x * frac, true)
	return seat


## 在治所擺一件事（鏡頭先落在該處）。
func _event_at_seat(kind: String, extra := {}, id := 0) -> Dictionary:
	var seat := _look_at_province(id)
	var event := _event(kind, {"pos": seat, "province": id})
	event.merge(extra, true)
	return event


# ── 對照表 ────────────────────────────────────────────────────

func test_gdd_5_13_table_maps_each_event_to_its_action_sequence() -> void:
	var cases := [
		[_event("Killed"), [["strike"], ["fall"]]],
		[_event("FeudEscalated"), [["strike"], ["fall"]]],
		[_event("ThreadResolved", {"x": "Feud"}), [["strike"], ["fall"]]],
		[_event("ThreadResolved", {"x": "Feud", "survivor_alive": true}), [["strike"], ["brace"]]],
		[_event("ThreadResolved", {"x": "Grace"}), [["bow"], ["bow"]]],
		[_event("ThreadResolved", {"x": "Love"}), [["bow"], ["bow"]]],
		[_event("ThreadResolved", {"x": "Grudge"}), [["bow"], ["bow"]]],
		[_event("Died", {"x": "Age"}), [["fall"]]],
		[_event("Died", {"x": "Plague"}), [["fall"]]],
		[_event("Breakthrough", {"n": 2}), [["seal", "rise"]]],
		[_event("Ascended"), [["rise"]]],
		[_event("FellToDemon"), [["seal", "struck"]]],
		[_event("ThreadCreated", {"x": "Mentor"}), [["bow"], ["stand"]]],
		[_event("Married"), [["bow"], ["bow"]]],
		[_event("Succession"), [["bow"]]],
		[_event("PowerFounded"), [["seal"]]],
		[_event("SectSplit"), [["seal"]]],
		[_event("Traveled", {"x": "Out", "q_province": 3}), [["walk"]]],
		[_event("Traveled", {"x": "Back", "q_province": 3}), [["walk"]]],
	]
	for entry: Array in cases:
		var event: Dictionary = entry[0]
		var plan := EventScript.plan(event)
		assert_false(plan.is_empty(), "%s 有對應動作" % event["kind"])
		assert_eq(_actions(plan), entry[1], "%s（%s）的動作序列" % [event["kind"], event["x"]])
		var total: float = INTRO + float(plan["action_len"]) + HOLD + OUTRO
		assert_true(is_equal_approx(EventStage.duration_for(event), total), "%s 的長度是四段合計" % event["kind"])
		assert_gt(EventStage.duration_for(event), INTRO + HOLD + OUTRO, "%s 的動作有長度" % event["kind"])


func test_gdd_5_13_events_without_a_matching_action_have_zero_length() -> void:
	var none := [_event("Born"), _event("Tribulation", {"x": "Passed", "n": 3}), _event("Died", {"x": "Killed"}), _event("Died", {"x": "Tribulation"}),
		_event("ThreadCreated", {"x": "Feud"}), _event("ThreadResolved", {"x": "Mentor"}), _event("Traveled", {"x": "Out", "q_province": -1}),
		_event("WorldChanged"), _event("Famine")]
	for event: Dictionary in none:
		assert_eq(EventStage.duration_for(event), 0.0, "%s（%s）沒有對應動作" % [event["kind"], event["x"]])
	var no_pos := _event("Married")
	no_pos.erase("pos")
	assert_eq(EventStage.duration_for(no_pos), 0.0, "沒有位置給不出站的格")
	var nobody := _event("Died", {"x": "Age", "a_person": -1})
	assert_eq(EventStage.duration_for(nobody), 0.0, "沒有當事人")


func test_gdd_5_13_the_fall_starts_at_the_strike_hit_and_the_action_spans_both() -> void:
	var plan := EventScript.plan(_event("Killed"))
	var hit := FigureMotion.event_time("strike", "hit")
	var fall: Dictionary = plan["actors"][1]["steps"][0]
	assert_true(absf(float(fall["start"]) - hit) < 1e-4, "倒地從出手命中那一格開始")
	assert_true(is_equal_approx(float(plan["action_len"]), hit + FigureMotion.length("fall")), "動作段到倒地播完")
	var seq := EventScript.plan(_event("Breakthrough"))
	var steps: Array = seq["actors"][0]["steps"]
	assert_true(is_equal_approx(float(steps[1]["start"]), FigureMotion.length("seal")), "結印播完接浮空")


func test_gdd_5_13_accepts_filters_by_kind_and_parameter_without_building_anything() -> void:
	assert_true(EventScript.accepts({"kind": "Died", "x": "Age"}))
	assert_false(EventScript.accepts({"kind": "Died", "x": "Killed"}), "其他死因不演")
	assert_false(EventScript.accepts({"kind": "Born"}), "表裡沒有的種類")
	assert_true(EventScript.accepts({"kind": "Married"}), "不分支的種類只看種類")
	assert_false(EventScript.accepts({"kind": "ThreadCreated", "x": "Feud"}))
	assert_true(EventScript.accepts({"kind": "ThreadResolved", "x": "Feud"}))


func test_gdd_5_13_ascension_lifts_the_figure_a_body_height_and_more_by_the_end_of_the_action() -> void:
	var plan := EventScript.plan(_event("Ascended"))
	var span := float(plan["action_len"])
	assert_eq(EventScript.lift_at(plan, INTRO + span * 0.3 - 0.01), 0.0, "動作段前 30% 原地")
	assert_gt(EventScript.lift_at(plan, INTRO + span), 1.0, "動作段結束時已離地至少一個身高")
	assert_gt(EventScript.lift_at(plan, INTRO + span + 1.0), EventScript.lift_at(plan, INTRO + span), "餘韻繼續緩升")
	assert_eq(EventScript.lift_at(EventScript.plan(_event("Breakthrough")), INTRO + 3.0), 0.0, "突破的 rise 原地")


func test_gdd_5_13_pose_at_follows_the_four_segments() -> void:
	var plan := EventScript.plan(_event("Killed"))
	var striker: Dictionary = plan["actors"][0]
	var victim: Dictionary = plan["actors"][1]
	assert_eq(EventScript.pose_at(striker, 0.2)["action"], "stand", "定場站定")
	assert_eq(EventScript.pose_at(striker, INTRO - 0.01)["action"], "stand")
	assert_eq(EventScript.pose_at(striker, INTRO + 0.5)["action"], "strike", "之後出手")
	assert_eq(EventScript.pose_at(victim, INTRO + 0.5)["action"], "stand", "被殺的人在命中前站著")
	assert_eq(EventScript.pose_at(victim, INTRO + 2.0)["action"], "fall")
	var late := EventScript.pose_at(victim, float(plan["total"]) - 0.1)
	assert_eq(late["action"], "fall", "餘韻停在動作結束的姿勢")
	assert_eq(float(late["time"]), FigureMotion.length("fall"), "倒地之後時間定格在動作長度（姿勢不變，不重畫）")
	assert_eq(EventScript.pose_at(striker, 0.1), EventScript.pose_at(striker, 1.2), "定場站定期間回傳的姿勢與時間不變")
	assert_eq(EventScript.pose_at(victim, float(plan["total"]) - 1.0), EventScript.pose_at(victim, float(plan["total"]) - 0.2), "餘韻停格期間不變")
	assert_true(FigureMotion.fps() <= 30.0, "動作段每秒重畫不超過 30 次")
	var walk: Dictionary = EventScript.plan(_event("Traveled", {"x": "Out", "q_province": 3}))["actors"][0]
	assert_gt(float(EventScript.pose_at(walk, INTRO + 4.0, true)["time"]), float(EventScript.pose_at(walk, INTRO + 3.0, true)["time"]), "行走餘韻時間照走")
	assert_eq(EventScript.opacity_at(4.0, 0.0), 0.0, "一開始全透明")
	assert_eq(EventScript.opacity_at(4.0, 1.0), 1.0, "站定後全不透明")
	assert_true(is_equal_approx(EventScript.opacity_at(4.0, 3.75), 0.5), "收場淡出")
	assert_eq(EventScript.opacity_at(4.0, 4.0), 0.0, "結束時全透明")


# ── 舞台 ──────────────────────────────────────────────────────

func test_gdd_5_13_figure_appears_on_the_event_cell_plays_in_order_and_vanishes() -> void:
	await _boot()
	var event := _event_at_seat("Succession")
	var total := EventStage.duration_for(event)
	_stage.post(event)
	assert_eq(_stage.figures().size(), 1, "事件來了立刻有棋子")
	var actor := _stage.figures()[0]
	assert_eq(actor.action(), "stand", "第一個畫面是定場的 stand")
	var seat: Vector2 = event["pos"]
	var expected := _spec.screen_of(seat)
	var shown := _camera.unproject_position(actor.global_position)
	assert_lt(shown.distance_to(expected), 4.0, "棋子的腳底在事件所在格")
	_stage.advance(INTRO + 0.3)
	assert_eq(actor.action(), "bow", "定場之後行禮")
	_stage.advance(FigureMotion.length("bow") + 0.5)
	assert_eq(actor.action(), "bow", "餘韻停在行禮結束的姿勢")
	_stage.advance(total)
	assert_eq(_stage.figures().size(), 0, "播完棋子消失")
	assert_eq(_stage.performances().size(), 0, "演出也移除")


func test_gdd_5_13_remaining_seconds_counts_down_to_zero() -> void:
	await _boot()
	var event := _event_at_seat("Married")
	var total := EventStage.duration_for(event)
	assert_eq(_stage.remaining_seconds(0), 0.0, "沒有演出為 0")
	_stage.post(event)
	assert_true(is_equal_approx(_stage.remaining_seconds(0), total), "剛開演是全長")
	_stage.advance(1.0)
	assert_true(is_equal_approx(_stage.remaining_seconds(0), total - 1.0), "隨時間遞減")
	assert_eq(_stage.remaining_seconds(1), 0.0, "別的州郡不受影響")
	_stage.advance(total)
	assert_eq(_stage.remaining_seconds(0), 0.0, "播完為 0")


func test_gdd_5_13_two_person_show_faces_each_other_on_either_side_of_the_cell() -> void:
	await _boot()
	var event := _event_at_seat("Married")
	_stage.post(event)
	var figures := _stage.figures()
	assert_eq(figures.size(), 2, "兩個棋子")
	var a := _camera.unproject_position(figures[0].global_position)
	var b := _camera.unproject_position(figures[1].global_position)
	var middle := _spec.screen_of(event["pos"])
	assert_lt(a.x, middle.x, "第一位在格的左側")
	assert_gt(b.x, middle.x, "第二位在格的右側")
	assert_lt(absf(a.y - b.y), 20.0, "兩人大致站在同一條線上（地形有起伏）")
	assert_gt(b.x - a.x, 20.0, "兩人不重疊")
	assert_true(figures[0].face_right, "左邊的面向右")
	assert_false(figures[1].face_right, "右邊的面向左")
	assert_lt(figures[0].card_scale_x(), 0.0, "朝右是面片鏡像")
	assert_gt(figures[1].card_scale_x(), 0.0)


func test_gdd_5_13_strike_and_fall_run_on_two_figures_with_the_fall_after_the_hit() -> void:
	await _boot()
	_stage.post(_event_at_seat("Killed"))
	var figures := _stage.figures()
	_stage.advance(INTRO + 0.5)
	assert_eq(figures[0].action(), "strike")
	assert_eq(figures[1].action(), "stand", "被殺的人命中前站著")
	_stage.advance(1.0)
	assert_eq(figures[1].action(), "fall", "命中之後倒下")


func test_gdd_5_13_ascending_figure_is_above_the_cell_ground_by_a_body_height() -> void:
	await _boot()
	var event := _event_at_seat("Ascended")
	_stage.post(event)
	var actor := _stage.figures()[0]
	var plan := EventScript.plan(event)
	_stage.advance(INTRO + float(plan["action_len"]))
	var ground := _spec.world_of(event["pos"])
	var up := _spec.up_basis(ground).y
	assert_gt((actor.global_position - ground).dot(up), actor.body_height(), "動作段結束時腳底高於事件格地面至少一個身高")


func test_gdd_5_13_walk_moves_the_figure_away_from_the_cell_toward_the_destination() -> void:
	await _boot()
	var event := _event_at_seat("Traveled", {"x": "Out", "q_province": 1})
	_stage.post(event)
	var actor := _stage.figures()[0]
	var start := actor.global_position
	_stage.advance(INTRO + 2.0)
	assert_eq(actor.action(), "walk")
	assert_gt(actor.global_position.distance_to(start), 0.5 * actor.body_height(), "走出事件格")
	var target: Vector2 = _sim.world.province_info(1)["seat"]
	var toward := _spec.screen_of(target).x > _spec.screen_of(event["pos"]).x
	assert_eq(actor.face_right, toward, "朝目的州郡的方向")


func test_gdd_5_13_same_cell_shows_queue_by_importance_and_drop_beyond_three() -> void:
	await _boot()
	var first := _event_at_seat("Succession", {"importance": 2})
	_stage.post(first)
	var low := _event("Succession", {"pos": first["pos"], "importance": 1})
	var mid := _event("Succession", {"pos": first["pos"], "importance": 3})
	var high := _event("Succession", {"pos": first["pos"], "importance": 5})
	var other := _event("Succession", {"pos": first["pos"], "importance": 4})
	for event: Dictionary in [low, mid, high]:
		_stage.post(event)
	assert_eq(_stage.performances().size(), 4, "一件在播、三件排隊")
	_stage.post(other)
	assert_eq(_stage.performances().size(), 4, "排隊超過 3 件丟一件")
	var ids := []
	for perf in _stage.performances():
		ids.append(perf["id"])
	assert_false(ids.has(low["id"]), "丟的是重要度最低的")
	assert_eq(_stage.figures().size(), 1, "一次只演一件")
	_stage.advance(EventStage.duration_for(first) + 0.01)
	var playing := -1
	for perf in _stage.performances():
		if float(perf["age"]) >= 0.0:
			playing = perf["id"]
	assert_eq(playing, high["id"], "下一件演重要度最高的")


func test_gdd_5_13_different_cells_play_at_the_same_time() -> void:
	await _boot()
	var a := _event_at_seat("Succession")
	_stage.post(a)
	_stage.post(_event("Succession", {"pos": a["pos"] + Vector2(1.5, 0.0)}))
	assert_eq(_stage.figures().size(), 2, "不同格各演各的")


func test_gdd_5_13_hold_keeps_queued_shows_before_the_start_until_release() -> void:
	await _boot()
	var event := _event_at_seat("Married")
	var total := EventStage.duration_for(event)
	_stage.hold(0)
	_stage.post(event)
	assert_eq(_stage.figures().size(), 0, "等鏡頭時沒有棋子")
	_stage.advance(3.0)
	assert_eq(_stage.figures().size(), 0, "等待期間不前進")
	assert_true(is_equal_approx(_stage.remaining_seconds(0), total), "等待中回報尚未播的全長")
	_stage.release(0)
	assert_eq(_stage.figures().size(), 2, "放行後棋子出現")
	for actor in _stage.figures():
		assert_eq(actor.action(), "stand", "第一個畫面是定場的 stand")
	assert_true(is_equal_approx(_stage.remaining_seconds(0), total), "從頭開始播")


func test_gdd_5_13_hold_times_out_after_five_seconds_and_does_not_touch_playing_shows() -> void:
	await _boot()
	var playing := _event_at_seat("Succession")
	_stage.post(playing)
	_stage.advance(0.2)
	var waiting := _event("Married", {"pos": playing["pos"] + Vector2(2.0, 0.0)})
	_stage.hold(0)
	_stage.post(waiting)
	assert_eq(_stage.figures().size(), 1, "正在播的不受影響，新的在等")
	_stage.advance(4.9)
	assert_eq(_stage.figures().size(), 1, "5 秒內還在等")
	_stage.advance(0.2)
	assert_eq(_stage.figures().size(), 3, "逾時自動開演（正在播的一枚加雙人的兩枚）")


func test_gdd_5_13_no_figures_off_screen_or_when_the_camera_is_too_high() -> void:
	await _boot()
	var event := _event_at_seat("Succession")
	_stage.post(event)
	assert_eq(_stage.figures().size(), 1)
	_camera.look_at_map(_map.map_size * 0.5, _map.map_size.x * 0.6, true)
	_stage.advance(0.1)
	assert_eq(_stage.figures().size(), 0, "鏡頭太高不畫")
	assert_gt(_stage.remaining_seconds(0), 0.0, "演出仍在計時")
	var seat := _look_at_province(0)
	_stage.advance(0.1)
	assert_eq(_stage.figures().size(), 1, "鏡頭回到近處又出現")
	var far := Vector2(seat.x + _map.map_size.x * 0.2, seat.y)
	var away := _event("Succession", {"pos": far, "province": 5})
	_stage.post(away)
	assert_false(_spec.pos_in_view(far), "測試用的遠處確實在畫面外")
	assert_eq(_stage.figures().size(), 1, "畫面外的事件不建棋子")


func test_gdd_5_13_figure_count_is_capped_and_the_least_important_lose_theirs() -> void:
	await _boot()
	var seat := _look_at_province(0)
	for i in 16:
		_stage.post(_event("Succession", {"pos": seat + Vector2(0.0, 0.06 * float(i)) + Vector2(float(i % 4) * 1.1, float(i / 4) * 1.1),
			"importance": 1 + i % 5}))
	assert_eq(_stage.figures().size(), EventStage.MAX_FIGURES, "同時最多 12 枚棋子")
	for perf in _stage.performances():
		if (perf["actors"] as Array).is_empty() and float(perf["age"]) >= 0.0:
			assert_lt(int(perf["importance"]), 5, "沒有棋子的是重要度較低的")


func test_gdd_5_13_paused_game_freezes_the_shows() -> void:
	await _boot()
	var event := _event_at_seat("Succession")
	_stage.post(event)
	var before := _stage.remaining_seconds(0)
	await wait(0.4)
	assert_eq(_stage.remaining_seconds(0), before, "暫停時不前進")
	_sim.set_paused(false)
	_sim.set_speed(0)
	await wait(0.4)
	assert_lt(_stage.remaining_seconds(0), before, "放行後前進")
	_sim.set_paused(true)
	var frozen := _stage.remaining_seconds(0)
	await wait(0.3)
	assert_eq(_stage.remaining_seconds(0), frozen, "再暫停又停住")


func test_gdd_5_13_show_time_does_not_speed_up_with_game_speed() -> void:
	await _boot()
	var event := _event_at_seat("Succession")
	_stage.post(event)
	_sim.set_paused(false)
	_sim.set_speed(_sim.SPEED_MULTIPLIERS.size() - 1)
	var before := _stage.remaining_seconds(0)
	var started := Time.get_ticks_msec()
	await wait(0.5)
	var real := float(Time.get_ticks_msec() - started) / 1000.0
	_sim.set_paused(true)
	var used := before - _stage.remaining_seconds(0)
	assert_lt(used, real + 0.2, "演出經過的時間不超過真實時間（不隨遊戲速度）")


func test_gdd_5_13_resolved_feud_shows_brace_when_the_other_side_lives() -> void:
	await _boot()
	var alive := _sim.world.person(1)
	var event := _event_at_seat("ThreadResolved", {"x": "Feud", "a_person": 1, "b_person": 0})
	_stage.post(event)
	var expected := "brace" if bool(alive.get("alive", false)) else "fall"
	_stage.advance(INTRO + 2.6)
	assert_eq(_stage.figures()[1].action(), expected, "欠債方在世就迎擊，不在世就倒下")
