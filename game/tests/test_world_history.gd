extends "res://tests/harness/suite.gd"
## GDD 5.10 世界簡史橫條的計算：各段寬度與紀元長度成比例、容得下 1 到約 15 個紀元、成住壞空的分段、標題的數字，
## 以及橋接的紀元表與世界一致。畫面與操作在 test_almanac。

const SEED := 20261011


func suite_name() -> String:
	return "world_history"


## 依各紀元長度（年）排出一張紀元表：首尾相接，起因類別輪流。
func _rows(lengths: Array) -> Array:
	var kinds := ["genesis", "cold", "warm", "decline", "revival", "peace"]
	var out: Array = []
	var year := 1
	for i in lengths.size():
		out.append({"name": "第%d" % i, "kind": kinds[i % kinds.size()], "start_year": year, "end_year": year + int(lengths[i]) - 1, "event": -1})
		year += int(lengths[i])
	return out


func _sum(values: Array) -> float:
	var total := 0.0
	for value in values:
		total += float(value)
	return total


func test_segment_widths_follow_era_lengths_and_fill_the_bar() -> void:
	var lengths := [8000, 15000, 22000, 15000]
	var widths := WorldHistoryBar.segment_widths(lengths, 1000.0, 6.0)
	assert_eq(widths.size(), 4)
	assert_true(absf(_sum(widths) - 1000.0) < 0.001, "各段加起來等於橫條寬")
	for i in 4:
		assert_true(absf(widths[i] - 1000.0 * lengths[i] / 60000.0) < 0.001, "第 %d 段寬度與長度成比例" % i)


func test_one_era_fills_the_whole_bar() -> void:
	var widths := WorldHistoryBar.segment_widths([3000], 800.0, 6.0)
	assert_eq(widths.size(), 1)
	assert_true(absf(widths[0] - 800.0) < 0.001)


func test_fifteen_eras_each_get_a_clickable_width() -> void:
	var lengths: Array = []
	for i in 15:
		lengths.append(8000 + i * 1500)
	var widths := WorldHistoryBar.segment_widths(lengths, 600.0, 6.0)
	assert_true(absf(_sum(widths) - 600.0) < 0.001)
	for width in widths:
		assert_true(width >= 6.0, "再窄的一段也點得到")


func test_a_very_short_era_is_pinned_to_the_minimum_and_the_rest_shrink() -> void:
	var widths := WorldHistoryBar.segment_widths([100000, 5, 100000], 600.0, 6.0)
	assert_true(absf(widths[1] - 6.0) < 0.001, "極短的一段釘在最短寬度")
	assert_true(absf(_sum(widths) - 600.0) < 0.001)
	assert_true(absf(widths[0] - widths[2]) < 0.001, "其餘兩段仍依比例")


func test_phases_split_one_cycle_into_cheng_zhu_huai_kong() -> void:
	var spans := WorldHistoryBar.phase_spans(129600)
	assert_eq(spans.size(), 4)
	var want := [[0, 27000], [27000, 113400], [113400, 118800], [118800, 129600]]
	for i in 4:
		assert_eq([spans[i]["from"], spans[i]["to"]], want[i], "第 %d 段的起訖年" % i)
	assert_eq(spans[0]["key"], "history_phase_cheng")
	assert_eq(spans[3]["key"], "history_phase_kong")


func test_a_young_world_only_shows_the_early_phases() -> void:
	var spans := WorldHistoryBar.phase_spans(5000)
	assert_eq(spans.size(), 1, "幾千年的世界還在成劫")
	assert_eq(spans[0]["to"], 5000, "只畫到今天")
	var older := WorldHistoryBar.phase_spans(40000)
	assert_eq(older.size(), 2, "四萬年進了住劫")
	assert_eq(older[1]["to"], 40000)


func test_world_time_keeps_adding_across_cycles() -> void:
	var spans := WorldHistoryBar.phase_spans(129600 + 1000)
	assert_eq(spans.size(), 5, "新一輪的第 129,601 年又是成")
	assert_eq(spans[4]["key"], "history_phase_cheng")
	assert_eq(spans[4]["from"], 129600)
	assert_eq(WorldHistoryBar.phase_at(1), 0)
	assert_eq(WorldHistoryBar.phase_at(27000), 0)
	assert_eq(WorldHistoryBar.phase_at(27001), 1)
	assert_eq(WorldHistoryBar.phase_at(113401), 2)
	assert_eq(WorldHistoryBar.phase_at(118801), 3)
	assert_eq(WorldHistoryBar.phase_at(129601), 0)


func test_headline_states_years_and_era_count() -> void:
	var bar := WorldHistoryBar.new()
	bar.load_rows(_rows([8000, 15000, 4356]))
	assert_eq(bar.total_years(), 27356)
	assert_eq(bar.era_count(), 3)
	assert_eq(bar.headline(), "世界簡史　開天至今 27,356 年、3 個紀元")
	assert_eq(WorldHistoryBar.thousands(999), "999")
	assert_eq(WorldHistoryBar.thousands(1000), "1,000")
	assert_eq(WorldHistoryBar.thousands(129600), "129,600")


func test_selection_defaults_to_the_present_and_steps_within_the_ends() -> void:
	var bar := WorldHistoryBar.new()
	bar.load_rows(_rows([8000, 15000, 4000]))
	assert_eq(bar.selected, 2, "預設選現在的紀元")
	bar.step(1)
	assert_eq(bar.selected, 2, "到了盡頭就停")
	bar.step(-1)
	bar.step(-1)
	bar.step(-1)
	assert_eq(bar.selected, 0)


func test_era_at_maps_points_to_segments() -> void:
	var bar := WorldHistoryBar.new()
	bar.load_rows(_rows([10000, 30000]))
	var band := Rect2(Vector2(100.0, 50.0), Vector2(400.0, WorldHistoryBar.band_height(1.0)))
	var bar_rect := bar.bar_rect(band, 1.0)
	assert_eq(bar.era_at(Vector2(110.0, bar_rect.get_center().y), band, 1.0), 0)
	assert_eq(bar.era_at(Vector2(400.0, bar_rect.get_center().y), band, 1.0), 1)
	assert_eq(bar.era_at(Vector2(110.0, 10.0), band, 1.0), -1, "橫條外不算")


func test_era_text_names_the_cause_kind() -> void:
	var bar := WorldHistoryBar.new()
	bar.load_rows(_rows([8000, 15000]))
	assert_eq(bar.era_text(1), "第1紀元　第 8,001 至 23,000 年（共 15,000 年）　起因：大寒")
	assert_eq(bar.era_text(0), "第0紀元　第 1 至 8,000 年（共 8,000 年）　起因：開天")


func test_the_bridge_era_table_matches_the_world() -> void:
	var world := SimWorld.new()
	assert_eq(world.create(SEED, "small"), "")
	var table := world.era_table()
	assert_gt(table.size(), 0, "至少有開天的那一個紀元")
	assert_eq(table[0]["start_year"], 1)
	assert_eq(table[0]["kind"], "genesis")
	for i in range(1, table.size()):
		assert_eq(table[i - 1]["end_year"] + 1, table[i]["start_year"], "紀元首尾相接")
	var bar := WorldHistoryBar.new()
	bar.load_rows(table)
	var now := int(world.stats_world()["years"])
	assert_eq(bar.total_years(), now, "橫條的總年數等於天下頁統計的開天至今")
	var span := 0
	for i in bar.era_count():
		span += bar.era_years(i)
	assert_eq(span, now, "各紀元長度加起來等於開天至今")
	assert_true(bar.headline().contains("%s 年、%d 個紀元" % [WorldHistoryBar.thousands(now), table.size()]))
