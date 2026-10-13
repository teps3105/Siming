extends "res://tests/harness/suite.gd"
## GDD 2.6「日期顯示」（決策 #91）：每個日期都是紀元時間（中文數字）加世界時間（阿拉伯數字，括號）。
## 兩種寫法：帶空格的（題簽、卡片欄位）與句子裡的（全形括號、不留空格）。

const SEED := 20261011
## 帶空格的寫法與句子裡的寫法，各自整串吻合（只到年的月份可省）。
const LABEL_RE := "^[^\\s（）()]{2}紀元 [〇一二三四五六七八九]+年 \\([0-9]+年\\)( (十[一二]?|[一二三四五六七八九])月)?$"
const INLINE_RE := "^[^\\s（）()]{2}紀元[〇一二三四五六七八九]+年（[0-9]+年）((十[一二]?|[一二三四五六七八九])月)?$"


func suite_name() -> String:
	return "date_text"


func test_era_year_is_written_digit_by_digit_with_zero_as_circle() -> void:
	assert_eq(EventText.cjk_digits(0), "〇")
	assert_eq(EventText.cjk_digits(1), "一")
	assert_eq(EventText.cjk_digits(10), "一〇")
	assert_eq(EventText.cjk_digits(105), "一〇五")
	assert_eq(EventText.cjk_digits(123), "一二三")
	assert_eq(EventText.cjk_digits(1000), "一〇〇〇")
	assert_eq(EventText.cjk_digits(12356), "一二三五六")


func test_months_are_read_the_usual_way() -> void:
	var expected := ["一月", "二月", "三月", "四月", "五月", "六月", "七月", "八月", "九月", "十月", "十一月", "十二月"]
	for month in range(1, 13):
		assert_eq(EventText.cjk_month(month), expected[month - 1])


func test_label_style_has_spaces_and_half_width_brackets() -> void:
	assert_eq(EventText.date_label("咸寧", 123, 12356, 6), "咸寧紀元 一二三年 (12356年) 六月")
	assert_eq(EventText.year_label("咸寧", 123, 12356), "咸寧紀元 一二三年 (12356年)")
	assert_eq(EventText.date_label("常始", 1, 1, 12), "常始紀元 一年 (1年) 十二月")
	assert_eq(EventText.date_label("常始", 10, 2310, 11), "常始紀元 一〇年 (2310年) 十一月")
	assert_eq(EventText.date_label("天地", 105, 100000, 1), "天地紀元 一〇五年 (100000年) 一月", "世界年不加千分位")


func test_inline_style_has_no_spaces_and_full_width_brackets() -> void:
	assert_eq(EventText.date_inline("咸寧", 123, 12356, 6), "咸寧紀元一二三年（12356年）六月")
	assert_eq(EventText.year_inline("咸寧", 123, 12356), "咸寧紀元一二三年（12356年）")
	assert_eq(EventText.year_inline("常始", 82, 2310) + "至" + EventText.year_inline("常始", 93, 2321), "常始紀元八二年（2310年）至常始紀元九三年（2321年）", "起訖各自完整")
	assert_eq(EventText.year_inline("常始", 82, 2310) + "起", "常始紀元八二年（2310年）起")


func test_consecutive_dates_in_one_era_each_carry_era_and_world_year() -> void:
	var text := "、".join([EventText.date_inline("常始", 82, 2310, 3), EventText.date_inline("常始", 83, 2311, 4)])
	assert_eq(text.count("常始紀元"), 2, "同一紀元連續出現也每次都寫紀元名")
	assert_eq(text.count("（2310年）") + text.count("（2311年）"), 2, "每次都寫世界年")


func test_date_spans_find_both_styles_and_only_dates() -> void:
	var text := "據傳咸寧紀元一二三年（12356年）六月，某某於咸寧紀元 一二四年 (12357年) 壽終，享年八十年"
	var spans := EventText.date_spans(text)
	assert_eq(spans.size(), 2)
	assert_eq(text.substr(spans[0].x, spans[0].y - spans[0].x), "咸寧紀元一二三年（12356年）六月")
	assert_eq(text.substr(spans[1].x, spans[1].y - spans[1].x), "咸寧紀元 一二四年 (12357年)")


## 全體掃描：橋接帶出的每一種日期（現在、事件、個人歷程、門史、覆滅宗門表），組出來的字串都符合兩種寫法之一。
func test_every_date_the_game_can_show_matches_one_of_the_two_styles() -> void:
	var world := SimWorld.new()
	assert_eq(world.create(SEED, "small"), "")
	world.drain_events()
	var label := RegEx.create_from_string(LABEL_RE)
	var inline := RegEx.create_from_string(INLINE_RE)
	var checked := 0
	var now := EventText.now_label(world)
	assert_true(label.search(now) != null, "現在：" + now)
	for round in 8:
		world.step(360 * 3)
		for event: Dictionary in world.drain_events():
			assert_true(label.search(EventText.event_label(event)) != null, "事件（帶空格）：" + EventText.event_label(event))
			assert_true(inline.search(EventText.event_inline(event)) != null, "事件（句子）：" + EventText.event_inline(event))
			assert_true(inline.search(EventText.event_year_inline(event)) != null, "事件（只到年）：" + EventText.event_year_inline(event))
			checked += 1
	assert_gt(checked, 100, "掃到足夠多的事件")
	var stats := world.stats_sects()
	for sect: Dictionary in stats["alive"]:
		for entry: Dictionary in world.sect_history(sect["id"]):
			assert_true(label.search(EventText.event_label(entry)) != null, "門史：" + EventText.event_label(entry))
			checked += 1
		var info := world.sect(sect["id"])
		var founded := EventText.year_label(info["founded_era"], int(info["founded_year"]), int(info["founded_world_year"]))
		assert_true(label.search(founded) != null, "立派年：" + founded)
	for row: Dictionary in stats["fallen"]:
		var founded := EventText.year_label(row["founded_era"], int(row["founded_era_year"]), int(row["founded_year"]))
		var fallen := EventText.year_label(row["fallen_era"], int(row["fallen_era_year"]), int(row["fallen_year"]))
		assert_true(label.search(founded) != null and label.search(fallen) != null, "覆滅宗門表：%s／%s" % [founded, fallen])
		assert_true(int(row["fallen_year"]) >= int(row["founded_year"]), "覆滅不早於立派（世界年）")
