extends "res://tests/harness/suite.gd"
## 命簿折行（GDD 5.8）：數字連同量詞不拆、可點的名字不拆成兩行、指定片段整段換行、頓號不落行首；
## 單位比一行還寬時才依序放寬。

const SIZE := 16
const NO_LINKS: Array[Dictionary] = []


func suite_name() -> String:
	return "ledger_text"


func _width(text: String) -> float:
	return Ink.text_size(text, SIZE).x


func _joined(para: Dictionary) -> String:
	return "".join(para["lines"])


## 每個寬度都掃一遍：任何折法下，數字加量詞都在同一行。
func test_digits_and_quantifiers_never_split() -> void:
	var text := "岳嶺（常始紀元82年–93年）、夏磊寒（93年–122年）、汪婷嬌（122年–148年）、胡震（148年–156年）、謝慎謐（156年–178年）"
	for step in range(6, 60):
		var width := _width("一二三四") * step * 0.25
		var para := LedgerText.paragraph(text, NO_LINKS, width, SIZE)
		assert_eq(_joined(para), text, "折行不丟字")
		for i in range(1, para["lines"].size()):
			var line: String = para["lines"][i]
			var before: String = para["lines"][i - 1]
			var head := line[0]
			var tail := before[before.length() - 1]
			assert_false("0123456789".contains(head) and "0123456789".contains(tail), "數字不從中間折開（寬 %s）：%s／%s" % [width, before, line])
			assert_false("年月日歲人位世".contains(head) and "0123456789".contains(tail), "量詞不離開數字（寬 %s）：%s／%s" % [width, before, line])
			assert_false("、，。）".contains(head), "標點不落行首（寬 %s）：%s" % [width, line])


func test_clickable_names_stay_on_one_line() -> void:
	var text := "於陌清州遇到謝慎謐與汪婷嬌，二人同赴鶴微宗"
	var names: Array[Dictionary] = [
		{"text": "陌清州", "target": "province:1"}, {"text": "謝慎謐", "target": "person:2"},
		{"text": "汪婷嬌", "target": "person:3"}, {"text": "鶴微宗", "target": "sect:4"}]
	for step in range(5, 40):
		var width := _width("一") * step
		var para := LedgerText.paragraph(text, names, width, SIZE)
		assert_eq(_joined(para), text)
		assert_eq(para["hits"].size(), 4, "每個名字只有一個點擊區（寬 %s）" % width)
		for hit: Dictionary in para["hits"]:
			assert_true(float(hit["rect"].size.x) > 0.0)


func test_kept_pieces_wrap_as_a_whole_with_comma_at_line_end() -> void:
	var pieces: Array[String] = ["岳嶺（常始紀元82年–93年）", "夏磊寒（93年–122年）", "汪婷嬌（122年–148年）"]
	var text := "、".join(pieces)
	var width := _width(pieces[0]) + _width("、") + 2.0
	var para := LedgerText.paragraph(text, NO_LINKS, width, SIZE, pieces)
	assert_eq(_joined(para), text)
	assert_eq(para["lines"].size(), 3, "每位各占一行：" + str(para["lines"]))
	assert_true(para["lines"][0].ends_with("、") and para["lines"][1].ends_with("、"), "頓號留在行尾")
	for piece in pieces:
		var whole := false
		for line: String in para["lines"]:
			whole = whole or line.begins_with(piece)
		assert_true(whole, "整位在同一行：" + piece)


func test_oversize_piece_breaks_once_between_name_and_bracket() -> void:
	var piece := "謝慎謐（常始紀元156年–閉易紀元178年）"
	var width := _width("（常始紀元156年–閉易紀元178年）") + 4.0
	var para := LedgerText.paragraph(piece, NO_LINKS, width, SIZE, [piece])
	assert_eq(_joined(para), piece)
	assert_eq(para["lines"][0], "謝慎謐", "只在姓名與括號之間折：" + str(para["lines"]))
	assert_eq(para["lines"].size(), 2)
	# 括號也放不下時，括號內才折，數字與量詞仍不拆。
	var narrow := _width("謝慎謐（常始紀元156年")
	var tight := LedgerText.paragraph(piece, NO_LINKS, narrow * 0.6, SIZE, [piece])
	assert_eq(_joined(tight), piece)
	for i in range(1, tight["lines"].size()):
		assert_false("年".contains(tight["lines"][i][0]), "年不離開數字：" + str(tight["lines"]))


func test_long_unbreakable_word_still_wraps_by_character() -> void:
	var text := "一二三四五六七八九十"
	var para := LedgerText.paragraph(text, [{"text": text, "target": "x"}] as Array[Dictionary], _width("一二三"), SIZE)
	assert_eq(_joined(para), text, "整段比一行寬時仍逐字折，不丟字")
	assert_true(para["lines"].size() >= 3)
