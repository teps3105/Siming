extends "res://tests/harness/suite.gd"
## GDD 2.6 / 6.4 雙步進：細步四檔逐日前進（每秒 1／2／5／10 日）；世界快進三種終點各停在該停的那一天，
## 中止有效，快進期間不發事件訊號但事件照樣進帳，摘要與期間事件一致。

const SEED := 20261011
const SPEEDS := [1, 2, 5, 10]

var _host: SimHost


func suite_name() -> String:
	return "dual_step"


func teardown() -> void:
	_host = null


func _boot_host() -> SimHost:
	var host := SimHost.new()
	track(host)
	tree.root.add_child(host)
	var error := host.create_world(SEED, "small")
	assert_eq(error, "", "小尺寸世界應能生成")
	host.paused = false
	return host


## 以固定的每幀秒數推進 seconds 秒（0.25 是二進位小數，累積不產生誤差）。
func _run_seconds(host: SimHost, seconds: float) -> void:
	for i in int(seconds / 0.25):
		host._process(0.25)


## 一直推進到快進結束；回傳走了幾幀。
func _drain_fast(host: SimHost, max_frames := 200000) -> int:
	var count := 0
	while host.fast_active() and count < max_frames:
		host._process(0.016)
		count += 1
	return count


func test_each_speed_advances_exactly_n_times_the_days_per_second() -> void:
	var host := _boot_host()
	for i in SPEEDS.size():
		host.set_speed(i)
		assert_eq(host.days_per_second(), SPEEDS[i], "第 %d 檔每秒 %d 日" % [i, SPEEDS[i]])
		var before := host.world.tick()
		_run_seconds(host, 5.0)
		assert_eq(host.world.tick() - before, 5 * SPEEDS[i], "第 %d 檔 5 秒走 %d 日" % [i, 5 * SPEEDS[i]])


func test_paused_time_does_not_move_and_months_speeds_are_gone() -> void:
	var host := _boot_host()
	host.set_paused(true)
	var before := host.world.tick()
	_run_seconds(host, 2.0)
	assert_eq(host.world.tick(), before, "暫停時不前進")
	assert_eq(SimHost.DAYS_PER_SECOND.size(), 4, "細步四檔")
	assert_eq(host.speed_index, 0)


func test_day_steps_reach_the_same_world_as_month_steps() -> void:
	var a := _boot_host()
	var b := _boot_host()
	# 兩個世界各自開天、人物落地的亂數不同，所以只比各自的日期：逐日 360 次與一次推進 360 日同一天。
	for i in 360:
		a.advance_days(1)
	b.advance_days(360)
	assert_eq(a.world.date_parts()["month"], b.world.date_parts()["month"])
	assert_eq(a.world.date_parts()["day"], b.world.date_parts()["day"])
	assert_eq(a.world.tick(), b.world.tick(), "逐日走 360 次與一次走 360 日是同一天")


func test_day_of_month_is_reported() -> void:
	var host := _boot_host()
	var parts: Dictionary = host.world.date_parts()
	assert_true(int(parts["day"]) >= 1 and int(parts["day"]) <= 30, "日在 1 到 30 之間")
	var before := host.world.tick()
	host.advance_days(1)
	assert_eq(host.world.tick(), before + 1)


func test_fast_to_years_stops_on_the_exact_day() -> void:
	var host := _boot_host()
	var start := host.world.tick()
	assert_true(host.fast_forward(SimHost.Stop.YEARS, 3))
	assert_true(host.fast_active())
	assert_false(host.fast_forward(SimHost.Stop.YEARS, 1), "快進中不能再開一次")
	var summaries: Array[Dictionary] = []
	host.fast_forward_finished.connect(func(s: Dictionary) -> void: summaries.append(s))
	_drain_fast(host)
	assert_false(host.fast_active())
	assert_eq(host.world.tick(), start + 3 * 360, "整三年後的同一天")
	assert_eq(summaries.size(), 1, "停下出一份摘要")
	assert_eq(summaries[0]["years"], 3)
	assert_eq(summaries[0]["days"], 3 * 360)
	assert_eq(summaries[0]["end_tick"], host.world.tick())


func test_fast_to_next_world_event_stops_on_that_day() -> void:
	var host := _boot_host()
	var summaries: Array[Dictionary] = []
	host.fast_forward_finished.connect(func(s: Dictionary) -> void: summaries.append(s))
	assert_true(host.fast_forward(SimHost.Stop.WORLD_EVENT))
	_drain_fast(host)
	assert_eq(summaries.size(), 1)
	var summary: Dictionary = summaries[0]
	var hit: Dictionary = host.world.event(int(summary["hit"]))
	assert_false(hit.is_empty(), "停下的事件查得到")
	assert_eq(hit["kind"], "WorldChanged")
	assert_eq(int(hit["tick"]), host.world.tick(), "停在世界大事發生的那一天")
	var ids: Array[int] = []
	for event: Dictionary in summary["world_events"]:
		ids.append(int(event["id"]))
	assert_true(ids.has(int(summary["hit"])), "摘要的世界大事列有使它停下的那一件")


func test_hold_runs_until_stopped_and_stopping_gives_a_summary() -> void:
	var host := _boot_host()
	var start := host.world.tick()
	var summaries: Array[Dictionary] = []
	host.fast_forward_finished.connect(func(s: Dictionary) -> void: summaries.append(s))
	assert_true(host.fast_forward(SimHost.Stop.HOLD))
	for i in 20:
		host._process(0.016)
	assert_true(host.fast_active(), "沒有終點，不會自己停")
	assert_gt(host.world.tick(), start, "快進中時間前進")
	host.fast_stop()
	assert_false(host.fast_active())
	assert_eq(summaries.size(), 1)
	assert_eq(summaries[0]["days"], host.world.tick() - start)
	host.fast_stop()
	assert_eq(summaries.size(), 1, "沒有快進時中止不會再出摘要")


func test_followed_target_is_an_interface_only() -> void:
	var host := _boot_host()
	assert_false(host.can_fast_to(SimHost.Stop.FOLLOWED), "關注者的下一件事還不能選")
	assert_false(host.fast_forward(SimHost.Stop.FOLLOWED))
	assert_false(host.fast_active())
	assert_true(host.can_fast_to(SimHost.Stop.WORLD_EVENT))


func test_fast_forward_posts_no_events_but_the_ledger_still_records() -> void:
	var host := _boot_host()
	var posted := [0]
	host.event_posted.connect(func(_e: Dictionary) -> void: posted[0] += 1)
	var summaries: Array[Dictionary] = []
	host.fast_forward_finished.connect(func(s: Dictionary) -> void: summaries.append(s))
	assert_true(host.fast_forward(SimHost.Stop.YEARS, 20))
	_drain_fast(host)
	assert_eq(posted[0], 0, "快進期間不發傳聞與奇觀要用的事件")
	var total := 0
	for kind: String in summaries[0]["kinds"]:
		total += int(summaries[0]["kinds"][kind])
	assert_gt(total, 0, "事件照樣寫進帳，摘要數得到")
	# 快進之後第一次細步，不會把快進期間的事件補發出來。
	host.advance_days(1)
	assert_lt(posted[0], 50, "快進期間的事件不補發")


func test_summary_counts_match_the_events_in_the_ledger() -> void:
	var host := _boot_host()
	var summaries: Array[Dictionary] = []
	host.fast_forward_finished.connect(func(s: Dictionary) -> void: summaries.append(s))
	assert_true(host.fast_forward(SimHost.Stop.YEARS, 30))
	_drain_fast(host)
	var summary: Dictionary = summaries[0]
	assert_eq(int(summary["end_tick"]) - int(summary["start_tick"]), 30 * 360)
	for event: Dictionary in summary["world_events"]:
		assert_eq(event["kind"], "WorldChanged")
		assert_true(int(event["tick"]) > int(summary["start_tick"]) and int(event["tick"]) <= int(summary["end_tick"]), "世界大事落在快進期間")
	assert_eq(summary["kinds"].get("WorldChanged", 0), (summary["world_events"] as Array).size(), "世界大事件數與列出的相符")
	assert_gt(summary["people1"], 0)
	assert_gt(summary["population1"], 0.0)


func test_fast_forward_works_while_paused_and_stays_paused() -> void:
	var host := _boot_host()
	host.set_paused(true)
	var start := host.world.tick()
	assert_true(host.fast_forward(SimHost.Stop.YEARS, 1))
	_drain_fast(host)
	assert_eq(host.world.tick(), start + 360)
	assert_true(host.paused, "快進不動暫停狀態")


func test_mid_month_save_round_trips() -> void:
	var host := _boot_host()
	host.advance_days(47)
	var text := host.world.save_text()
	assert_false(text.is_empty())
	var tick := host.world.tick()
	var hash := host.world.hash_hex()
	var loader := SimWorld.new()
	var error := loader.load_begin(text)
	assert_eq(error, "")
	var result := {"done": false}
	while not result["done"]:
		result = loader.load_step(1000)
		assert_eq(result["error"], "")
	assert_eq(loader.tick(), tick, "月中的存檔讀回同一天")
	assert_eq(loader.hash_hex(), hash)
