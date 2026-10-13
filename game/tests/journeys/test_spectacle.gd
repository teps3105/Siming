extends "res://tests/harness/suite.gd"
## 地圖奇觀（GDD 5.6，第一批：天劫、飛升、宿緣了結）：佇列上限與捨棄順序、快轉簡化、暫停不前進、
## 三種事件各自觸發且位置在事件所在州郡、時長到了自行移除、鏡頭外不放、傳聞簽飛往時重播。

var _game: Node
var _sim: SimHost
var _hud: Hud
var _camera: MapCamera
var _map: TerrainMap
var _spec: Spectacle
var _next_id := 5000


func suite_name() -> String:
	return "spectacle"


func _boot() -> void:
	_game = await boot(false, FLOW_VIEW)
	_sim = _game.get_node("SimHost")
	_hud = _game.get_node("Hud")
	_camera = _game.get_node("MapCamera")
	_map = _game.get_node("TerrainMap")
	_spec = tree.get_first_node_in_group("spectacle")


## 合成一筆橋接事件（欄位同 drain_events）。
func _event(kind: String, importance: int, extra := {}) -> Dictionary:
	_next_id += 1
	var event := {"id": _next_id, "tick": 100, "kind": kind, "importance": importance, "province": 0, "cell": -1,
		"souls": PackedInt32Array(), "a": "沈青霜", "b": "韓烈", "f": "", "g": "", "p": "青州", "q": "", "x": "", "n": 0, "m": 0, "year": 9}
	event.merge(extra, true)
	return event


func _post(event: Dictionary) -> void:
	_sim.event_posted.emit(event)


## 把鏡頭立刻移到某州郡治所，回傳它的地圖座標。
func _look_at_province(id: int) -> Vector2:
	var seat: Vector2 = _sim.world.province_info(id)["seat"]
	_camera.look_at_map(seat, -1.0, true)
	return seat


## 每個州郡的治所；找一個目前在畫面外的。
func _off_screen_seat() -> Dictionary:
	var rect := Rect2(Vector2.ZERO, Vector2(FLOW_VIEW)).grow(Spectacle.VIEW_MARGIN)
	for id in _sim.world.province_count():
		var seat: Vector2 = _sim.world.province_info(id)["seat"]
		var s := _spec.screen_of(seat)
		if not s.is_finite() or not rect.has_point(s):
			return {"id": id, "pos": seat}
	return {}


func _spec_dict(kind: String, importance: int) -> Dictionary:
	return {"id": _next_id, "kind": kind, "importance": importance, "pos": Vector2(10.0, 10.0)}


# ── 佇列 ──────────────────────────────────────────────────────

func test_queue_caps_at_twelve_and_drops_lowest_importance() -> void:
	var queue := SpectacleQueue.new()
	for i in 12:
		_next_id += 1
		assert_true(queue.add(_spec_dict("Tribulation", 3 + i % 3), false), "未滿上限時都留下")
	assert_eq(queue.count(), 12, "同時 12 個")
	_next_id += 1
	var newcomer := _next_id
	var spec := _spec_dict("Ascended", 5)
	spec["id"] = newcomer
	assert_true(queue.add(spec, false), "較高重要度的新奇觀留下")
	assert_eq(queue.count(), 12, "仍然只有 12 個")
	var threes := 0
	for item in queue.items():
		if int(item["importance"]) == 3:
			threes += 1
	assert_eq(threes, 3, "被擠掉的是重要度 3 的一個（4 個變 3 個）")
	_next_id += 1
	assert_false(queue.add(_spec_dict("Tribulation", 2), false), "比所有在放的都低的新奇觀被捨棄")
	assert_eq(queue.count(), 12)
	assert_true(queue.has_event(newcomer), "新來的在放")


func test_queue_drops_oldest_among_equal_importance() -> void:
	var queue := SpectacleQueue.new()
	var first := 0
	for i in 12:
		_next_id += 1
		if i == 0:
			first = _next_id
		var spec := _spec_dict("Tribulation", 3)
		spec["id"] = _next_id
		queue.add(spec, false)
	_next_id += 1
	var late := _spec_dict("Tribulation", 3)
	late["id"] = _next_id
	assert_true(queue.add(late, false), "同重要度時新的留下")
	assert_false(queue.has_event(first), "最先開始的那個被捨棄")
	assert_true(queue.has_event(_next_id), "新的在放")


func test_fast_speed_leaves_only_spots_below_importance_four() -> void:
	var queue := SpectacleQueue.new()
	_next_id += 1
	queue.add(_spec_dict("Tribulation", 3), true)
	assert_eq(queue.count(), 0, "快轉時重要度 3 不佔一般位置")
	assert_eq(queue.spots().size(), 1, "只留一個光點")
	assert_eq(queue.spots()[0]["kind"], "Spot")
	assert_eq(queue.spots()[0]["duration"], 0.8, "光點 0.8 秒")
	queue.add(_spec_dict("ThreadResolved", 4), true)
	queue.add(_spec_dict("Ascended", 5), true)
	assert_eq(queue.count(), 2, "重要度 4 以上照常完整表現")
	for i in 30:
		queue.add(_spec_dict("Tribulation", 3), true)
	assert_eq(queue.count(), 2, "光點不計入 12 個的上限")
	queue.add(_spec_dict("Tribulation", 3), false)
	assert_eq(queue.count(), 3, "不快轉時重要度 3 完整表現")


func test_queue_removes_items_when_their_duration_ends() -> void:
	var queue := SpectacleQueue.new()
	queue.add(_spec_dict("Tribulation", 3), false)
	queue.add(_spec_dict("Ascended", 5), false)
	queue.add(_spec_dict("ThreadResolved", 4), false)
	queue.advance(2.9)
	assert_eq(queue.count(), 3, "2.9 秒時三個都還在")
	queue.advance(0.2)
	assert_eq(queue.count(), 2, "3 秒了結結束")
	queue.advance(1.0)
	assert_eq(queue.count(), 1, "4 秒天劫結束")
	queue.advance(1.8)
	assert_eq(queue.count(), 1, "5.9 秒飛升還在")
	queue.advance(0.2)
	assert_eq(queue.count(), 0, "6 秒飛升結束")


# ── 事件觸發 ──────────────────────────────────────────────────

func test_tribulation_event_starts_a_tribulation_at_its_province() -> void:
	await _boot()
	var pos := _look_at_province(2)
	await frames(2)
	_post(_event("Tribulation", 3, {"province": 2, "pos": pos, "x": "Passed", "n": 3}))
	assert_eq(_spec.queue().count(), 1, "天劫放出一個奇觀")
	var item := _spec.queue().items()[0]
	assert_eq(item["kind"], "Tribulation")
	assert_eq(_map.province_at(item["pos"]), 2, "位置在事件所在州郡")
	assert_eq(item["bolts"], 4, "新境界 3 → 4 道雷")
	assert_eq(item["outcome"], "Passed")
	await frames(3)


func test_bolt_count_is_new_realm_plus_one_within_three_to_nine() -> void:
	await _boot()
	var pos := _look_at_province(1)
	await frames(2)
	var expected := {1: 3, 2: 3, 5: 6, 8: 9, 9: 9}
	for n: int in expected:
		assert_eq(_spec.spec_of(_event("Tribulation", 3, {"pos": pos, "n": n, "x": "Passed"}))["bolts"], expected[n], "新境界 %d" % n)


func test_ascension_event_starts_a_pillar_at_its_province() -> void:
	await _boot()
	var pos := _look_at_province(3)
	await frames(2)
	_post(_event("Ascended", 5, {"province": 3, "pos": pos}))
	var item := _spec.queue().items()[0]
	assert_eq(item["kind"], "Ascended")
	assert_eq(_map.province_at(item["pos"]), 3, "光柱在事件所在州郡")
	assert_eq(item["duration"], 6.0)
	await frames(3)


func test_resolved_thread_burns_between_the_provinces_of_both_ends() -> void:
	await _boot()
	var here := _look_at_province(2)
	var other: Vector2 = _sim.world.province_info(4)["seat"]
	await frames(2)
	_post(_event("ThreadResolved", 4, {"province": 2, "pos": here, "a_pos": other, "x": "Grace", "n": 1}))
	var item := _spec.queue().items()[0]
	assert_eq(item["kind"], "ThreadResolved")
	assert_true(item["ends_known"], "兩端都有位置")
	assert_eq(_map.province_at(item["ends"][0]), 4, "欠債方一端在他所在的州郡")
	assert_eq(_map.province_at(item["ends"][1]), 2, "債主一端在事件所在州郡")
	assert_eq(item["thread_kind"], 1, "恩義取第二種顏色")
	assert_eq(item["duration"], 3.0)
	await frames(3)


func test_resolved_thread_without_other_end_still_burns_around_the_event() -> void:
	await _boot()
	var here := _look_at_province(2)
	await frames(2)
	_post(_event("ThreadResolved", 4, {"province": 2, "pos": here, "x": "Feud"}))
	var item := _spec.queue().items()[0]
	assert_false(item["ends_known"], "找不到另一端時標記為未知")
	assert_ne(item["ends"][0], item["ends"][1], "兩端仍分開，線畫得出來")


func test_other_events_do_not_start_a_spectacle() -> void:
	await _boot()
	var pos := _look_at_province(2)
	await frames(2)
	_post(_event("Killed", 4, {"pos": pos}))
	_post(_event("Breakthrough", 3, {"pos": pos}))
	assert_eq(_spec.queue().count() + _spec.queue().spots().size(), 0, "第一批只有天劫、飛升、了結")


func test_event_outside_the_camera_is_not_played_but_replays_from_the_rumor_slip() -> void:
	await _boot()
	_look_at_province(2)
	await frames(2)
	var far := _off_screen_seat()
	assert_false(far.is_empty(), "找得到畫面外的州郡")
	var event := _event("Ascended", 5, {"province": far["id"], "pos": far["pos"]})
	_post(event)
	await frames(2)
	assert_eq(_spec.queue().count(), 0, "鏡頭外不放")
	var rumor := _hud.rumor()
	assert_eq(rumor.shown().size(), 1, "傳聞簽照樣收到（不重複加傳聞）")
	rumor.activate(0)
	assert_eq(_spec.queue().count(), 1, "點傳聞飛往時重播")
	var item := _spec.queue().items()[0]
	assert_eq(item["kind"], "Ascended")
	assert_lt(float(item["age"]), 0.0, "等鏡頭飛到才開始")
	rumor.activate(0)
	assert_eq(_spec.queue().count(), 1, "已在放的不重複重播")
	_spec.step(Spectacle.REPLAY_DELAY + 0.5)
	assert_gt(float(_spec.queue().items()[0]["age"]), 0.0, "延遲過後開始前進")


func test_replay_of_a_resolved_thread_uses_the_ends_remembered_at_the_time() -> void:
	await _boot()
	_look_at_province(2)
	await frames(2)
	var far := _off_screen_seat()
	var here: Vector2 = far["pos"] + Vector2(3.0, 0.0)
	var event := _event("ThreadResolved", 4, {"province": far["id"], "pos": far["pos"], "a_pos": here, "x": "Love"})
	_post(event)
	assert_eq(_spec.queue().count(), 0, "鏡頭外不放")
	assert_true(_spec.replay({"id": event["id"], "kind": "ThreadResolved", "importance": 4, "pos": far["pos"], "x": "Love", "n": 1}), "傳聞簽記錄可重播")
	assert_eq(_spec.queue().items()[0]["ends"][0], here, "兩端用事件當下記下的")
	assert_false(_spec.replay({"id": 1, "kind": "Killed"}), "不是奇觀事件不重播")


# ── 速度與暫停 ────────────────────────────────────────────────

func test_fast_speed_turns_low_importance_events_into_spots() -> void:
	await _boot()
	var pos := _look_at_province(2)
	await frames(2)
	_sim.set_speed(3)
	assert_eq(_sim.speed_multiplier(), 20)
	_post(_event("Tribulation", 3, {"pos": pos, "x": "Passed", "n": 3}))
	assert_eq(_spec.queue().count(), 0, "20× 時重要度 3 的天劫不放完整的")
	assert_eq(_spec.queue().spots().size(), 1, "只放一個光點")
	_post(_event("Ascended", 5, {"pos": pos}))
	assert_eq(_spec.queue().count(), 1, "重要度 5 的飛升照常")
	await frames(3)


func test_paused_game_freezes_spectacles_and_running_game_advances_them() -> void:
	await _boot()
	var pos := _look_at_province(2)
	await frames(2)
	_post(_event("Ascended", 5, {"pos": pos}))
	_sim.set_paused(true)
	var before := float(_spec.queue().items()[0]["age"])
	await wait(0.25)
	assert_eq(float(_spec.queue().items()[0]["age"]), before, "暫停時奇觀不前進")
	_sim.set_paused(false)
	await wait(0.25)
	assert_gt(float(_spec.queue().items()[0]["age"]), before + 0.1, "放行後以實際秒前進")
	_sim.set_paused(true)


func test_duration_is_real_seconds_regardless_of_game_speed() -> void:
	await _boot()
	var pos := _look_at_province(2)
	await frames(2)
	_sim.set_paused(false)
	_sim.set_speed(0)
	_post(_event("Ascended", 5, {"pos": pos}))
	_sim.set_speed(2)
	var at := float(_spec.queue().items()[0]["age"])
	await wait(0.3)
	var gained := float(_spec.queue().items()[0]["age"]) - at
	assert_true(gained > 0.2 and gained < 0.6, "10× 時 0.3 秒只前進約 0.3 秒（實際 %.2f）" % gained)
	_sim.set_paused(true)


func test_finished_spectacle_removes_itself_from_the_running_game() -> void:
	await _boot()
	var pos := _look_at_province(2)
	await frames(2)
	_post(_event("Tribulation", 3, {"pos": pos, "x": "Perished", "n": 2}))
	assert_eq(_spec.queue().count(), 1)
	_spec.step(4.01)
	assert_eq(_spec.queue().count(), 0, "時長到了自行移除")


func test_all_three_spectacles_draw_through_their_whole_life_without_errors() -> void:
	await _boot()
	var pos := _look_at_province(2)
	var other: Vector2 = _sim.world.province_info(4)["seat"]
	await frames(2)
	_post(_event("Tribulation", 3, {"pos": pos, "x": "Passed", "n": 4}))
	_post(_event("Tribulation", 3, {"pos": pos, "x": "ShedBody", "n": 6}))
	_post(_event("Tribulation", 3, {"pos": pos, "x": "Perished", "n": 3}))
	_post(_event("Ascended", 5, {"pos": pos}))
	_post(_event("ThreadResolved", 4, {"pos": pos, "a_pos": other, "x": "Grudge"}))
	for i in 30:
		_spec.step(0.2)
		await frames(1)
	assert_eq(_spec.queue().count(), 0, "全部演完")
