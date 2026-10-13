extends "res://tests/harness/suite.gd"
## 導演模式（GDD 5.13）的決策與自動變速。時間全部是假的：測試自己推進真實秒，不等任何一秒。

const STEP := 0.25


## 假的演出節點：加入群組 event_stage，回報剩餘秒數，記下 hold 與 release。
class FakeStage:
	extends Node
	var remaining := 0.0
	var holds: Array[int] = []
	var releases: Array[int] = []

	func remaining_seconds(_province: int) -> float:
		return remaining

	func hold(province: int) -> void:
		holds.append(province)

	func release(province: int) -> void:
		releases.append(province)


var _dir: Director
var _log: Array[Array] = []


func suite_name() -> String:
	return "director"


func setup() -> void:
	_dir = Director.new()
	_dir.enabled = true
	_log = []
	_dir.go.connect(func(province: int, big: bool) -> void: _log.append([_dir.clock, province, big]))


## 推進 seconds 秒真實時間。
func _run(seconds: float, running := true) -> void:
	var left := seconds
	while left > 0.0001:
		var step := minf(STEP, left)
		_dir.update(step, running)
		left -= step


## 讓導演去 province 並到位（鏡頭瞬間完成）。
func _visit(province: int, importance := 2) -> void:
	_dir.post(province, importance, {"importance": importance, "p": "州%d" % province})
	_run(STEP)
	assert_eq(_dir.state, Director.State.TRAVEL, "選定去處後進入過場")
	_dir.arrive()
	assert_eq(_dir.state, Director.State.STAY)
	assert_eq(_dir.current, province)


func _event(importance: int, who := "") -> Dictionary:
	return {"importance": importance, "a": who}


func test_no_move_before_the_minimum_stay_even_for_a_big_event() -> void:
	_visit(1)
	_dir.post(2, Director.BIG_LEVEL, _event(5))
	_run(Director.STAY_MIN - 0.5)
	assert_eq(_log.size(), 1, "最短停留內大事也要等")
	_run(0.75)
	assert_eq(_log.size(), 2, "最短停留滿就插隊")
	assert_eq(_log[1][1], 2)
	assert_true(_log[1][2], "標為大事")


func test_after_the_normal_stay_a_better_province_wins_but_hysteresis_holds() -> void:
	_visit(1, 2)
	for i in 14:
		if i % 2 == 0:
			_dir.post(1, 2, _event(2))
		_run(1.0)
	assert_gt(_dir.dwell, Director.STAY_NORMAL, "停留已過一般停留")
	var here := _dir.score(1)
	assert_gt(here, 7, "目前所在州郡持續有事")
	var enough := int(floor(1.2 * float(here - 2) / 4.0))
	for i in enough:
		_dir.post(2, 4, _event(4))
	_run(STEP)
	assert_eq(_log.size(), 1, "別處分數沒有高出 1.2 倍，續留")
	_dir.post(2, 4, _event(4))
	_dir.post(2, 4, _event(4))
	_run(STEP)
	assert_eq(_log.size(), 2, "高出 1.2 倍就換")
	assert_eq(_log[1][1], 2)


func test_nothing_to_see_stays_put_and_no_tour() -> void:
	_visit(1)
	_run(200.0)
	assert_eq(_log.size(), 1, "全零時留在原地，不巡遊")
	assert_eq(_dir.current, 1)


func test_maximum_stay_forces_a_move_regardless_of_hysteresis() -> void:
	_visit(1, 3)
	for i in 29:
		_dir.post(1, 3, _event(3))
		_dir.post(2, 2, _event(2))
		_run(1.0)
	assert_eq(_log.size(), 1, "三十秒前不論如何都沒換（遲滯擋住）")
	_dir.post(1, 3, _event(3))
	_dir.post(2, 2, _event(2))
	_run(1.5)
	assert_eq(_log.size(), 2, "最長停留滿且別處有分數就換")
	assert_eq(_log[1][1], 2)


func test_revisit_cooldown_also_binds_a_big_event_of_the_province_just_left() -> void:
	_visit(1)
	_run(Director.STAY_MIN)
	_dir.post(2, 3, _event(3))
	_run(5.0)
	assert_eq(_dir.state, Director.State.TRAVEL, "換去 2")
	_dir.arrive()
	_run(Director.STAY_NORMAL + 1.0)
	_dir.post(1, 4, _event(4))
	_run(Director.STAY_NORMAL)
	assert_eq(_log.size(), 2, "剛離開的 1 號州郡冷卻中，不回去")
	_dir.post(1, Director.BIG_LEVEL, _event(5))
	_run(STEP)
	assert_eq(_log.size(), 2, "剛離開的州郡的大事也受冷卻")


func test_a_big_event_of_another_province_ignores_the_cooldown() -> void:
	_visit(1)
	_run(Director.STAY_MIN)
	_dir.post(2, Director.BIG_LEVEL, _event(5))
	_run(STEP)
	_dir.arrive()
	_run(Director.STAY_MIN)
	_dir.post(3, Director.BIG_LEVEL, _event(5))
	_run(STEP)
	_dir.arrive()
	_run(Director.STAY_MIN)
	assert_eq(_log.size(), 3, "三處都在 30 秒內換過")
	_dir.post(1, Director.BIG_LEVEL, _event(5))
	_run(STEP)
	assert_eq(_log.size(), 4, "1 號州郡不是剛離開的那一處，大事照去")
	assert_eq(_log[3][1], 1)


func test_a_big_event_that_waits_too_long_is_dropped() -> void:
	var stage := FakeStage.new()
	stage.add_to_group("event_stage")
	track(stage)
	tree.root.add_child(stage)
	_visit(1)
	_dir.post(2, Director.BIG_LEVEL, _event(5))
	stage.remaining = 1000.0
	_run(Director.BIG_KEEP + 2.0)
	stage.remaining = 0.0
	_run(STEP)
	assert_eq(_log.size(), 1, "超過保留時間的大事不去了")
	assert_eq(_dir.current, 1)


func test_cooldown_ends_after_thirty_seconds() -> void:
	_visit(1)
	_run(Director.STAY_MIN)
	_dir.post(2, 3, _event(3))
	_run(5.0)
	_dir.arrive()
	_run(Director.COOLDOWN + Director.STAY_NORMAL)
	_dir.post(1, 4, _event(4))
	_run(STEP)
	assert_eq(_log.size(), 3, "冷卻三十秒過後可以再選")


func test_pace_scales_the_three_timers() -> void:
	for pair: Array in [[0, 1.5], [1, 1.0], [2, 0.7]]:
		_dir.pace = pair[0]
		assert_lt(absf(_dir.stay_min() - 8.0 * float(pair[1])), 0.001, "最短停留 %d 檔" % pair[0])
		assert_lt(absf(_dir.stay_normal() - 12.0 * float(pair[1])), 0.001)
		assert_lt(absf(_dir.stay_max() - 30.0 * float(pair[1])), 0.001)
	_dir.pace = 0
	_visit(1)
	_dir.post(2, Director.BIG_LEVEL, _event(5))
	_run(11.0)
	assert_eq(_log.size(), 1, "慢檔最短停留 12 秒")
	_run(1.5)
	assert_eq(_log.size(), 2)


func test_unfinished_stage_blocks_the_move() -> void:
	var stage := FakeStage.new()
	stage.add_to_group("event_stage")
	track(stage)
	tree.root.add_child(stage)
	_visit(1)
	_dir.post(2, Director.BIG_LEVEL, _event(5))
	stage.remaining = 6.0
	_run(Director.STAY_MIN + 3.0)
	assert_eq(_log.size(), 1, "演出沒播完不換，即使最短停留已滿")
	stage.remaining = 0.0
	_run(STEP)
	assert_eq(_log.size(), 2, "播完就插隊")


func test_stage_is_held_while_travelling_and_released_on_arrival() -> void:
	var stage := FakeStage.new()
	stage.add_to_group("event_stage")
	track(stage)
	tree.root.add_child(stage)
	_dir.post(3, 3, _event(3))
	_run(STEP)
	assert_eq(stage.holds, [3] as Array[int], "開始過場的那一刻 hold 目標州郡")
	assert_true(stage.releases.is_empty())
	_dir.arrive()
	assert_eq(stage.releases, [3] as Array[int], "鏡頭到位的那一刻 release")
	assert_eq(_dir.dwell, 0.0, "停留從到位起算")


func test_player_interrupting_travel_releases_the_hold() -> void:
	var stage := FakeStage.new()
	stage.add_to_group("event_stage")
	track(stage)
	tree.root.add_child(stage)
	_dir.post(4, 3, _event(3))
	_run(STEP)
	_dir.player_acted()
	assert_eq(stage.releases, [4] as Array[int], "過場被玩家中斷，不留懸著的等待")
	assert_eq(_dir.state, Director.State.YIELDED)


func test_disabling_during_travel_releases_the_hold() -> void:
	var stage := FakeStage.new()
	stage.add_to_group("event_stage")
	track(stage)
	tree.root.add_child(stage)
	_dir.post(5, 3, _event(3))
	_run(STEP)
	_dir.enabled = false
	_run(STEP)
	assert_eq(stage.releases, [5] as Array[int], "導演讓出也放開")
	assert_eq(_dir.state, Director.State.OFF)


func test_player_action_yields_and_idle_brings_it_back() -> void:
	_visit(1)
	_dir.player_acted()
	assert_eq(_dir.state, Director.State.YIELDED)
	_run(18.0)
	_dir.post(2, 4, _event(4))
	_run(1.0)
	assert_eq(_log.size(), 1, "閒置沒滿，導演不動")
	_run(1.5)
	assert_eq(_log.size(), 2, "閒置滿 20 秒接回，去分數最高的地方")
	assert_eq(_log[1][1], 2)


func test_busy_keeps_resetting_the_idle_clock() -> void:
	_visit(1)
	_dir.player_acted()
	_dir.busy = true
	_run(60.0)
	assert_eq(_log.size(), 1, "卡片開著不算閒置")
	_dir.busy = false
	_run(18.0)
	_dir.post(2, 4, _event(4))
	_run(1.0)
	assert_eq(_log.size(), 1)
	_run(1.5)
	assert_eq(_log.size(), 2)


func test_idle_setting_is_respected() -> void:
	_dir.idle_resume = 10.0
	_visit(1)
	_dir.player_acted()
	_run(9.0)
	_dir.post(2, 4, _event(4))
	_run(2.0)
	assert_eq(_log.size(), 2, "閒置接回秒數可調")


func test_paused_time_freezes_the_director() -> void:
	_visit(1)
	_dir.post(2, Director.BIG_LEVEL, _event(5))
	var dwell := _dir.dwell
	_run(100.0, false)
	assert_eq(_log.size(), 1, "暫停時不動")
	assert_eq(_dir.dwell, dwell, "暫停不計停留")
	_run(Director.STAY_MIN + 0.5)
	assert_eq(_log.size(), 2)


func test_scores_use_a_ten_second_real_window() -> void:
	_dir.post(1, 3, _event(3))
	_dir.post(1, 2, _event(2))
	assert_eq(_dir.score(1), 5)
	_run(10.5)
	assert_eq(_dir.score(1), 0, "十秒前的事件不再計分")


func test_two_visits_are_never_closer_than_the_minimum_stay() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	var arrivals: Array[float] = []
	_dir.go.connect(func(_p: int, _b: bool) -> void:
		arrivals.append(_dir.clock)
		_dir.arrive())
	for i in 2400:
		if rng.randf() < 0.3:
			_dir.post(rng.randi_range(0, 5), rng.randi_range(2, 5), _event(2))
		_run(0.5)
	assert_gt(arrivals.size(), 5, "有在換鏡頭")
	for i in range(1, arrivals.size()):
		assert_gt(arrivals[i] - arrivals[i - 1], Director.STAY_MIN - 0.01, "第 %d 次換鏡頭離上一次不短於最短停留" % i)


func test_caption_follows_the_best_local_event_and_holds_three_seconds() -> void:
	var captions: Array[Dictionary] = []
	_dir.caption_changed.connect(func(event: Dictionary) -> void: captions.append(event))
	_dir.post(1, 2, {"importance": 2, "tag": "舊"})
	_run(STEP)
	_dir.arrive()
	assert_eq(_dir.caption()["tag"], "舊", "到位時字卡是當地的事件")
	_dir.post(1, 4, {"importance": 4, "tag": "新"})
	assert_eq(_dir.caption()["tag"], "舊", "同一句至少顯示 3 秒")
	_run(3.5)
	assert_eq(_dir.caption()["tag"], "新", "三秒後換成更重要的新事件")
	_dir.player_acted()
	assert_true(_dir.caption().is_empty(), "讓出時字卡收起")


func test_slow_flag_follows_big_events_and_leaving() -> void:
	var flags: Array[bool] = []
	_dir.slow_changed.connect(func(on: bool) -> void: flags.append(on))
	_dir.post(1, Director.BIG_LEVEL, _event(5))
	_run(STEP)
	assert_eq(flags, [true] as Array[bool], "對準大事就放慢")
	_dir.arrive()
	_run(5.0)
	_dir.post(2, 3, _event(3))
	_run(Director.STAY_NORMAL - 4.0)
	assert_eq(flags, [true, false] as Array[bool], "離開那個鏡頭就解除")


func test_speed_slows_for_big_events_and_restores() -> void:
	var sim := SimHost.new()
	track(sim)
	sim.set_speed(3)
	sim.set_paused(false)
	var speed := DirectorSpeed.new(sim)
	speed.engage()
	assert_eq(sim.speed_index, 0, "降到一倍速")
	speed.release()
	assert_eq(sim.speed_index, 3, "離開時恢復降速前的速度")


func test_speed_is_not_restored_when_the_player_changed_it() -> void:
	var sim := SimHost.new()
	track(sim)
	sim.set_speed(2)
	sim.set_paused(false)
	var speed := DirectorSpeed.new(sim)
	speed.engage()
	sim.set_speed(1)
	speed.release()
	assert_eq(sim.speed_index, 1, "玩家改了速度，以玩家的為準")
	speed.engage()
	sim.set_paused(true)
	speed.release()
	assert_true(sim.paused, "玩家按了暫停，導演不解除")


func test_speed_unchanged_when_disabled() -> void:
	var sim := SimHost.new()
	track(sim)
	sim.set_speed(3)
	sim.set_paused(false)
	var speed := DirectorSpeed.new(sim)
	speed.enabled = false
	speed.engage()
	assert_eq(sim.speed_index, 3, "設定關閉時不變速")
