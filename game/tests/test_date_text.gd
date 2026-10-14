extends "res://tests/harness/suite.gd"
## GDD 2.6「日期顯示」：日期只寫紀元時間（中文數字），世界年只出現在天下冊的世界簡史橫條。
## 兩種寫法：帶空格的（題簽、卡片欄位）與句子裡的（不留空格）。

const SEED := 20261011
## 帶空格的寫法與句子裡的寫法，各自整串吻合（只到年的月份可省）。
const LABEL_RE := "^[^\\s（）()]{2}紀元 [〇一二三四五六七八九]+年( (十[一二]?|[一二三四五六七八九])月)?$"
const INLINE_RE := "^[^\\s（）()]{2}紀元[〇一二三四五六七八九]+年((十[一二]?|[一二三四五六七八九])月)?$"


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


func test_label_style_has_spaces_and_no_world_year() -> void:
	assert_eq(EventText.date_label("咸寧", 123, 6), "咸寧紀元 一二三年 六月")
	assert_eq(EventText.year_label("咸寧", 123), "咸寧紀元 一二三年")
	assert_eq(EventText.date_label("常始", 1, 12), "常始紀元 一年 十二月")
	assert_eq(EventText.date_label("常始", 10, 11), "常始紀元 一〇年 十一月")


func test_inline_style_has_no_spaces() -> void:
	assert_eq(EventText.date_inline("咸寧", 123, 6), "咸寧紀元一二三年六月")
	assert_eq(EventText.year_inline("咸寧", 123), "咸寧紀元一二三年")
	assert_eq(EventText.year_inline("常始", 82) + "至" + EventText.year_inline("常始", 93), "常始紀元八二年至常始紀元九三年", "起訖各自完整")


func test_consecutive_dates_in_one_era_each_carry_the_era_name() -> void:
	var text := "、".join([EventText.date_inline("常始", 82, 3), EventText.date_inline("常始", 83, 4)])
	assert_eq(text.count("常始紀元"), 2, "同一紀元連續出現也每次都寫紀元名")


func test_date_spans_find_both_styles_and_only_dates() -> void:
	var text := "據傳咸寧紀元一二三年六月，某某於咸寧紀元 一二四年 壽終，享年八十年"
	var spans := EventText.date_spans(text)
	assert_eq(spans.size(), 2)
	assert_eq(text.substr(spans[0].x, spans[0].y - spans[0].x), "咸寧紀元一二三年六月")
	assert_eq(text.substr(spans[1].x, spans[1].y - spans[1].x), "咸寧紀元 一二四年")


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
		var founded := EventText.year_label(info["founded_era"], int(info["founded_year"]))
		assert_true(label.search(founded) != null, "立派年：" + founded)
	for row: Dictionary in stats["fallen"]:
		var founded := EventText.year_label(row["founded_era"], int(row["founded_era_year"]))
		var fallen := EventText.year_label(row["fallen_era"], int(row["fallen_era_year"]))
		assert_true(label.search(founded) != null and label.search(fallen) != null, "覆滅宗門表：%s／%s" % [founded, fallen])
		assert_true(int(row["fallen_year"]) >= int(row["founded_year"]), "覆滅不早於立派")


## 括號裡的阿拉伯數字加「年」就是世界年的寫法。
const WORLD_YEAR_RE := "[（(] ?[0-9][0-9,]* ?年[）)]"


## 全文掃描：介面字串與句型庫、事件文字沒有世界年的寫法，也沒有日期字串的 {W} 佔位。
func test_no_string_table_writes_a_world_year() -> void:
	var pattern := RegEx.create_from_string(WORLD_YEAR_RE)
	for path in ["res://modules/locale/resources/ui.zh-Hant.json", ChronicleText.LIB_PATH, "res://modules/locale/resources/events.zh-Hant.json"]:
		var text := FileAccess.get_file_as_string(path)
		assert_true(pattern.search(text) == null, "%s 沒有括號世界年" % path)
	assert_false(Locale.t("date_label").contains("{W}") or Locale.t("date_inline").contains("{W}"), "日期字串沒有世界年佔位")


## 全文掃描：實際跑出來的日期、事件句、小傳與年表都不含世界年。
func test_generated_texts_never_contain_a_world_year() -> void:
	var world := SimWorld.new()
	assert_eq(world.create(SEED, "small"), "")
	world.drain_events()
	var pattern := RegEx.create_from_string(WORLD_YEAR_RE)
	var texts: Array[String] = [EventText.now_label(world)]
	for round in 6:
		world.step(360 * 3)
		for event: Dictionary in world.drain_events():
			texts.append(EventText.event_label(event))
			texts.append(EventText.event_inline(event))
			texts.append(String(EventText.line(event)))
	var lookup := func(p: int) -> Dictionary: return world.person(p)
	var composed := 0
	for id in range(0, 400, 7):
		var info: Dictionary = world.person(id)
		if info.is_empty():
			continue
		var out := ChronicleText.compose(info, world.person_chronicle(id), lookup)
		for paragraph: Dictionary in out["bio"]:
			texts.append(String(paragraph["text"]))
		for row: Dictionary in out["rows"]:
			texts.append(String(row["date"]))
			texts.append(String(row["text"]))
		composed += 1
	assert_gt(composed, 10, "掃到足夠多的人")
	assert_gt(texts.size(), 100, "掃到足夠多的文字")
	for text in texts:
		assert_true(pattern.search(text) == null, "沒有世界年：" + text)
