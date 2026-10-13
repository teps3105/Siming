extends "res://tests/harness/suite.gd"
## 小傳與年表的寫法規則（GDD 5.12），用手寫的事實清單斷言輸出的句子：不依賴模擬世界。

## 開局前年數：使「咸寧紀元一二三年」對到世界年 12356。
const HISTORY := 12233

var _next_id := 100


func suite_name() -> String:
	return "chronicle_rules"


func _tick(year: int, month: int) -> int:
	return 360 * (year - 1) + 30 * (month - 1) + 1


## 一筆事實：預設欄位與橋接相同，extra 覆蓋。
func _fact(kind: String, year: int, month: int, extra := {}) -> Dictionary:
	_next_id += 1
	var f := {"id": _next_id, "tick": _tick(year, month), "era": "咸寧", "year": year, "month": month, "kind": kind, "role": "self",
		"other": -1, "other_name": "", "other_bond": -1, "other_bond_name": "", "province": -1, "province_name": "",
		"power": -1, "power_name": "", "power_from": -1, "power_from_name": "", "why": "", "detail": "", "n": 0, "thread": "",
		"target": -1, "target_name": "", "end_reason": "", "group": _next_id}
	f.merge(extra, true)
	if not extra.has("group"):
		f["group"] = f["id"]
	return f


func _info(name := "霍青廉", alive := true) -> Dictionary:
	return {"id": 1, "name": name, "alive": alive, "home": 5, "home_name": "丙郡", "role": "Cultivator", "power": -1, "power_name": "", "gender": "Male", "title": "", "is_leader": false}


func _past(person: int, name: String) -> Dictionary:
	return {"id": -1, "tick": _tick(149, 3), "era": "安黃", "year": 149, "month": 3, "kind": "past_life", "role": "self", "person": person, "name": name,
		"title": "", "identity": "Cultivator", "realm": 2, "fruit": -1, "power": -1, "power_name": "", "cause": "Killed", "age": 50,
		"province": 9, "province_name": "乙郡", "killer": 31, "killer_name": "馬谷法", "path": "", "reborn_tick": _tick(150, 1),
		"reborn_era": "安黃", "reborn_year": 150, "reborn_month": 1, "reborn_province": 5, "reborn_province_name": "丙郡"}


func _because(event: int, me: int, me_name: String, other: int, other_name: String, thread: String, year := 149) -> Dictionary:
	return {"event": event, "tick": _tick(year, 2), "era": "安黃", "year": year, "month": 2, "me": me, "me_name": me_name,
		"other": other, "other_name": other_name, "thread": thread, "lives": 1}


func _compose(facts: Array, name := "霍青廉", alive := true, lookup := Callable()) -> Dictionary:
	return ChronicleText.compose(_info(name, alive), facts, lookup, HISTORY)


func _para_with(out: Dictionary, needle: String) -> String:
	for para: Dictionary in out["bio"]:
		if para["text"].contains(needle):
			return para["text"]
	return ""


## 一次報仇：殺 袁凰 報前世之仇，前因是前世林默為馬谷法（今袁凰）所殺。
func _revenge_facts(because_me: int, because_name: String) -> Array:
	var kill := _fact("killed", 91, 1, {"other": 30, "other_name": "袁凰", "province": 9, "province_name": "乙郡", "why": "vendetta"})
	var resolved := _fact("thread_resolved", 91, 1, {"group": kill["id"], "tick": kill["tick"], "thread": "feud", "role": "victim", "other": 30,
		"other_name": "袁凰", "other_bond": 31, "other_bond_name": "馬谷法", "because": _because(50, because_me, because_name, 31, "馬谷法", "feud")})
	return [_past(7, "林默"), kill, resolved]


# ── 日期（2.6）──────────────────────────────────────────────

func test_every_year_is_written_with_era_and_world_year() -> void:
	var out := _compose([_fact("breakthrough", 123, 11, {"n": 1}), _fact("breakthrough", 130, 2, {"n": 2})])
	assert_eq(out["rows"][0]["date"], "咸寧紀元 一二三年 (12356年) 十一月", "年表日期是題簽寫法：紀元與世界年雙表、月照念法")
	assert_eq(out["rows"][1]["date"], "咸寧紀元 一三〇年 (12363年) 二月", "零寫〇")
	var para := _para_with(out, "築基")
	assert_true(para.contains("咸寧紀元一二三年（12356年）"), "小傳的年份雙表：" + para)
	assert_eq(para.count("咸寧紀元"), 2, "同一紀元連續出現也每次都寫紀元名：" + para)
	assert_eq(ChronicleText.digits(105), "一〇五")
	assert_eq(ChronicleText.digits(10), "一〇")
	assert_false(para.contains(" "), "句中日期不留空格")


func test_keep_pieces_covers_the_whole_double_date() -> void:
	var pieces := ChronicleText.keep_pieces("咸寧紀元一二三年（12356年）十一月，某事。咸寧紀元一〇年（12243年），某事。")
	assert_true(pieces.has("咸寧紀元一二三年（12356年）十一月"))
	assert_true(pieces.has("咸寧紀元一〇年（12243年）"))


# ── 恩怨的合併（規則 1、2）─────────────────────────────────────

func _many_kills(n: int, feud_first: int) -> Array:
	var facts: Array = []
	for i in n:
		var kill := _fact("killed", 100 + i, 3, {"other": 200 + i, "other_name": "魔%d" % (200 + i), "province": 9, "province_name": "乙郡", "why": "slay_demon"})
		facts.append(kill)
		if i < feud_first:
			facts.append(_fact("thread_created", 100 + i, 3, {"group": kill["id"], "tick": kill["tick"], "thread": "feud", "role": "killer", "other": 200 + i, "other_name": "魔%d" % (200 + i)}))
	return facts


func test_many_new_kills_become_one_sentence_with_totals() -> void:
	var facts := _many_kills(25, 5)
	var lookup := func(pid: int) -> Dictionary: return {"realm": 5 if pid == 220 else 1}
	var out := _compose(facts, "霍青廉", true, lookup)
	var para := _para_with(out, "乙郡")
	assert_ne(para, "", "有恩怨一段")
	assert_true(para.contains("二十四人"), "合併句寫總數（二十五人扣掉境界最高的一人）：" + para)
	assert_true(para.contains("多在乙郡"), para)
	assert_true(para.contains("其中五人結下死仇"), "死仇改成總數：" + para)
	assert_true(para.contains("魔220"), "對方境界最高的一人單獨寫：" + para)
	assert_false(para.contains("魔205"), "其餘不逐一列名：" + para)
	assert_lt(para.count("結下死仇"), 3, "不再逐筆寫結下死仇：" + para)
	assert_eq(out["rows"].size(), 25, "年表仍逐條")


func test_merged_kill_sentence_without_place_or_feud_leaves_no_placeholder() -> void:
	var facts: Array = []
	for i in 6:
		facts.append(_fact("killed", 100 + i, 3, {"other": 200 + i, "other_name": "魔%d" % (200 + i), "province": -1, "why": "slay_demon"}))
	var out := _compose(facts)
	for para: Dictionary in out["bio"]:
		assert_false(String(para["text"]).contains("{"), "合併句沒有地點與死仇時不留佔位：" + para["text"])


func test_same_life_cause_reads_ji_shen_yu() -> void:
	var out := _compose(_revenge_facts(1, "霍青廉"))
	for row: Dictionary in out["rows"]:
		assert_false(row["text"].contains("己身，"), row["text"])


func test_pending_omens_are_one_closing_clause_and_resolved_ones_are_dropped() -> void:
	var facts: Array = [_past(7, "林默")]
	facts.append(_fact("thread_omen", 10, 1, {"thread": "feud", "role": "victim", "other": 90, "other_name": "已遇者", "because": _because(50, 7, "林默", 31, "舊仇甲", "feud")}))
	var kill := _fact("killed", 20, 1, {"other": 90, "other_name": "已遇者", "province": 9, "province_name": "乙郡", "why": "vendetta"})
	facts.append(kill)
	facts.append(_fact("thread_resolved", 20, 1, {"group": kill["id"], "tick": kill["tick"], "thread": "feud", "role": "victim", "other": 90, "other_name": "已遇者",
		"because": _because(50, 7, "林默", 31, "舊仇甲", "feud")}))
	facts.append(_fact("thread_omen", 30, 1, {"thread": "feud", "role": "victim", "other": 91, "other_name": "未遇乙", "because": _because(51, 7, "林默", 32, "舊仇乙", "feud")}))
	facts.append(_fact("thread_omen", 40, 1, {"thread": "feud", "role": "victim", "other": 92, "other_name": "未遇丙", "because": _because(52, 7, "林默", 33, "舊仇丙", "feud")}))
	var out := _compose(facts)
	var text := ""
	for para: Dictionary in out["bio"]:
		text += para["text"] + "\n"
	for word in ["將與", "舊緣將現", "重逢之日將近"]:
		assert_false(text.contains(word), "小傳不寫將遇：" + text)
	assert_true(text.contains("未遇乙") and text.contains("未遇丙"), "尚未了結的列名：" + text)
	assert_false(text.contains("未遇乙") and text.contains("舊仇乙"), "段尾現況不重述前因：" + text)
	var last := out["bio"][out["bio"].size() - 1]["text"] as String
	assert_true(last.contains("未遇丙"), "現況在恩怨段的最後：" + last)
	var omen_rows := 0
	for row: Dictionary in out["rows"]:
		if row["text"].contains("將與") or row["text"].contains("舊緣將現") or row["text"].contains("重逢之日將近"):
			omen_rows += 1
	assert_eq(omen_rows, 3, "年表照舊逐條列出將遇")
	# 多於三個只寫數量。
	var many: Array = [_past(7, "林默")]
	for i in 5:
		many.append(_fact("thread_omen", 10 + i, 1, {"thread": "feud", "role": "victim", "other": 90 + i, "other_name": "未遇%d" % i, "because": _because(60 + i, 7, "林默", 40 + i, "舊仇%d" % i, "feud")}))
	var many_text := _para_with(_compose(many), "五段")
	assert_ne(many_text, "", "多於三個只寫數量")


# ── 前因併進句子（規則 3、4、5）──────────────────────────────────

func test_revenge_weaves_the_cause_in_and_names_which_life() -> void:
	var out := _compose(_revenge_facts(7, "林默"))
	for text: String in [_para_with(out, "袁凰"), out["rows"][1]["text"]]:
		assert_true(text.contains("袁凰"), text)
		assert_true(text.contains("林默"), "前因交代本人那一世的名字：" + text)
		assert_true(text.contains("袁凰的前身馬谷法"), "對方當時的名字交代是誰的前身：" + text)
		assert_true(text.contains("安黃紀元一四九年（12382年）"), "前因的年份雙表：" + text)
		assert_true(text.contains("前世"), "上一世稱前世：" + text)
		assert_false(text.contains("轉世之"), "不用「某某轉世之某」：" + text)
		assert_false(text.contains("，安黃") and not text.contains("——") and not text.contains("於安黃"), "前因不當句尾半句：" + text)
		assert_false(text.contains("霍青廉"), "不寫本人姓名：" + text)


func test_cause_is_told_once_per_paragraph_and_later_mentions_are_short() -> void:
	var facts := _revenge_facts(7, "林默")
	var kill2 := _fact("killed", 95, 1, {"other": 32, "other_name": "崔李", "province": 9, "province_name": "乙郡", "why": "vendetta"})
	facts.append(kill2)
	facts.append(_fact("feud_escalated", 95, 1, {"group": kill2["id"], "tick": kill2["tick"], "thread": "feud", "role": "victim", "other": 32, "other_name": "崔李",
		"because": _because(50, 7, "林默", 31, "馬谷法", "feud")}))
	var out := _compose(facts)
	var para := _para_with(out, "崔李")
	assert_eq(para.count("林默"), 1, "同一筆前因在同一段只完整寫一次：" + para)
	assert_eq(para.count("馬谷法"), 1, para)
	assert_true(para.contains("崔李"), para)


func test_older_life_is_not_called_previous_life() -> void:
	var out := _compose(_revenge_facts(9, "張雅"))
	var para := _para_with(out, "張雅")
	assert_true(para.contains("宿世"), "更早的一世稱宿世：" + para)
	assert_true(para.contains("其宿世張雅") or para.contains("宿世張雅"), "用當時那一世的名字：" + para)
	assert_false(para.contains("前世張雅"), para)
	assert_false(para.contains("前世的殺身仇人"), para)


func test_same_life_cause_has_no_own_name_and_uses_the_past_word() -> void:
	# 師父轉世了，本人仍是同一世：往日的師徒緣在此了結。
	var join := _fact("apprenticed", 221, 5, {"role": "disciple", "other": 60, "other_name": "潘澎肅"})
	var resolved := _fact("thread_resolved", 221, 5, {"group": join["id"], "tick": join["tick"], "thread": "mentor", "role": "disciple", "other": 61, "other_name": "李琳道",
		"other_bond": 60, "other_bond_name": "潘澎肅", "because": _because(70, 1, "霍青廉", 60, "潘澎肅", "mentor", 221)})
	var out := _compose([join, resolved])
	for text: String in [out["bio"][1]["text"], out["rows"][0]["text"]]:
		assert_true(text.contains("往日"), text)
		assert_true(text.contains("李琳道"), text)
		assert_false(text.contains("霍青廉"), "不寫本人姓名：" + text)
		assert_false(text.contains("之其人"), text)


func test_a_name_seen_in_the_previous_clause_is_replaced_whole() -> void:
	var out := _compose(_revenge_facts(7, "林默"))
	for row: Dictionary in out["rows"]:
		assert_false(row["text"].contains("之其人"), row["text"])
		assert_false(row["text"].contains("其其"), row["text"])


# ── 其他寫法 ─────────────────────────────────────────────────

func test_tribulation_omens_are_not_in_the_biography_but_stay_in_the_timeline() -> void:
	var facts: Array = [_fact("breakthrough", 100, 1, {"n": 1})]
	for i in 3:
		facts.append(_fact("tribulation_omen", 110 + i, 11))
	var out := _compose(facts)
	var text := ""
	for para: Dictionary in out["bio"]:
		text += para["text"]
	assert_false(text.contains("劫雲"), text)
	assert_false(text.contains("劫期"), text)
	assert_eq(out["rows"].size(), 4, "年表保留每一次")


func test_the_end_is_its_own_last_paragraph_for_the_dead() -> void:
	var facts: Array = _many_kills(5, 0)
	facts.append(_fact("died", 150, 6, {"detail": "Age", "n": 90, "province": 9, "province_name": "乙郡"}))
	var out := _compose(facts, "霍青廉", false)
	var paras: Array = out["bio"]
	assert_true(paras[-1]["text"].contains("享年"), paras[-1]["text"])
	assert_false(paras[-2]["text"].contains("享年"), "結局不接在恩怨段尾：" + paras[-2]["text"])
	var alive := _compose(_many_kills(5, 0), "霍青廉", true)
	assert_false(alive["bio"][-1]["text"].contains("享年"))


func test_paragraph_length_is_capped() -> void:
	var facts: Array = [_past(7, "林默")]
	facts.append_array(_many_kills(30, 8))
	for i in 12:
		var k := _fact("killed", 130 + i, 5, {"other": 400 + i, "other_name": "尋仇者%d" % i, "province": 9, "province_name": "乙郡", "why": "vendetta"})
		facts.append(k)
		facts.append(_fact("thread_resolved", 130 + i, 5, {"group": k["id"], "tick": k["tick"], "thread": "feud", "role": "victim", "other": 400 + i, "other_name": "尋仇者%d" % i,
			"because": _because(300 + i, 7, "林默", 31, "馬谷法", "feud")}))
	for i in 6:
		facts.append(_fact("married", 40 + i * 5, 2, {"other": 500 + i, "other_name": "妻%d" % i, "why": "same_hometown"}))
		facts.append(_fact("widowed", 42 + i * 5, 2, {"other": 500 + i, "other_name": "妻%d" % i, "end_reason": "spouse_died"}))
	var out := _compose(facts)
	for para: Dictionary in out["bio"]:
		# 多次報仇的分項本身就長（每項帶完整前因），單獨成段時放寬到三項的長度。
		var limit := 400 if String(para["text"]).contains("度報") else 241
		assert_lt(String(para["text"]).length(), limit, "每段約不超過 220 字：" + para["text"])


# ── 審閱後的六點 ─────────────────────────────────────────────

func _bio_text(out: Dictionary) -> String:
	var text := ""
	for para: Dictionary in out["bio"]:
		text += para["text"] + "\n"
	return text


func test_sentences_in_a_paragraph_run_from_early_to_late() -> void:
	var out := _compose([_fact("breakthrough", 255, 1, {"n": 3}), _fact("sect_founded", 241, 1, {"power": 8, "power_name": "玉月派"})])
	var para := _para_with(out, "玉月派")
	var early := para.find("二四一年")
	var late := para.find("二五五年")
	assert_true(early >= 0 and late >= 0, para)
	assert_lt(early, late, "先寫早的：" + para)
	var mixed := _compose([_fact("apprenticed", 50, 1, {"role": "master", "other": 70, "other_name": "夏駿"}), _fact("married", 20, 1, {"other": 71, "other_name": "葉雅碧", "why": "same_hometown"})])
	var p2 := _para_with(mixed, "葉雅碧")
	assert_lt(p2.find("葉雅碧"), p2.find("夏駿"), "成婚（二〇年）在前，收徒（五〇年）在後：" + p2)


func test_a_merged_sentence_sits_at_its_earliest_year() -> void:
	var facts: Array = []
	for i in 5:
		facts.append(_fact("traveled", 100 + i, 5, {"detail": "out", "target": 20 + i, "target_name": "州%d" % i}))
	facts.append(_fact("breakthrough", 90, 1, {"n": 1}))
	facts.append(_fact("breakthrough", 120, 1, {"n": 2}))
	var all := _bio_text(_compose(facts))
	assert_true(all.contains("遊歷"), all)
	assert_lt(all.find("築基"), all.find("遊歷"), "合併句（最早一年一〇〇）在築基（九〇年）之後：" + all)
	assert_lt(all.find("遊歷"), all.find("金丹"), "也在結丹（一二〇年）之前：" + all)


func test_several_marriages_are_one_sentence_and_only_the_last_death_is_told() -> void:
	var facts: Array = [
		_fact("married", 10, 2, {"other": 61, "other_name": "葉雅碧", "why": "same_hometown"}),
		_fact("widowed", 20, 2, {"other": 61, "other_name": "葉雅碧"}),
		_fact("married", 21, 2, {"other": 62, "other_name": "上官淡"}),
		_fact("widowed", 30, 2, {"other": 62, "other_name": "上官淡"}),
		_fact("married", 31, 2, {"other": 63, "other_name": "廖冰冰"}),
		_fact("widowed", 40, 2, {"other": 63, "other_name": "廖冰冰"})]
	var para := _para_with(_compose(facts), "葉雅碧")
	assert_true(para.contains("娶葉雅碧，其後續娶上官淡、廖冰冰") or para.contains("娶葉雅碧；其後續娶上官淡、廖冰冰"), para)
	assert_eq(para.count("咸寧紀元"), 2, "只有第一次婚配與最後的亡故帶年份：" + para)
	assert_true(para.contains("其妻廖冰冰"), "最後一任先亡才寫，並點名：" + para)
	assert_false(para.contains("其妻已歿"), para)
	assert_false(para.contains("上官淡亡故"), "中間的喪偶不另寫：" + para)
	var alive_end := _compose(facts.slice(0, 5))
	assert_false(_para_with(alive_end, "葉雅碧").contains("廖冰冰亡故") or _para_with(alive_end, "葉雅碧").contains("廖冰冰去世"), "最後一任健在就不寫亡故")


func test_two_revenges_become_one_passage_with_a_lead_sentence() -> void:
	var first := _revenge_facts(7, "林默")
	var second := _revenge_facts(7, "林默")
	second.remove_at(0)
	for f: Dictionary in second:
		f["other"] = 32
		f["other_name"] = "崔李"
		f["year"] = 95
		f["tick"] = _tick(95, 1)
	second[1]["because"]["event"] = 51
	var out := _compose(first + second)
	var para := _para_with(out, "崔李")
	assert_true(para.contains("一生二度報前世之仇：") or para.contains("前後二度報前世之仇："), para)
	assert_true(para.contains("袁凰") and para.contains("崔李"), para)
	assert_false(para.contains("殺之以報"), "分項不重複「殺之以報…之仇」：" + para)
	var one := _para_with(_compose(_revenge_facts(7, "林默")), "袁凰")
	assert_false(one.contains("度報"), one)
	assert_true(one.contains("袁凰") and one.contains("之仇"), one)


func test_the_founder_is_a_patriarch_and_a_leader_is_a_leader() -> void:
	var info := _info("高毅", false)
	info["power"] = 8
	info["power_name"] = "玉月派"
	var founded := ChronicleText.compose(info, [_fact("sect_founded", 241, 1, {"power": 8, "power_name": "玉月派"})], Callable(), HISTORY)
	assert_true(founded["bio"][0]["text"].contains("玉月派祖師"), founded["bio"][0]["text"])
	assert_false(founded["bio"][0]["text"].contains("弟子"), founded["bio"][0]["text"])
	var led := ChronicleText.compose(info, [_fact("succeeded", 241, 1, {"power": 8, "power_name": "玉月派"})], Callable(), HISTORY)
	assert_true(led["bio"][0]["text"].contains("玉月派掌門"), led["bio"][0]["text"])
	var plain := ChronicleText.compose(info, [], Callable(), HISTORY)
	assert_true(plain["bio"][0]["text"].contains("玉月派弟子"), plain["bio"][0]["text"])


func test_three_pupils_merge_with_the_earliest_year_and_two_do_not() -> void:
	var facts: Array = []
	for i in 3:
		facts.append(_fact("apprenticed", 20 + i * 5, 3, {"role": "master", "other": 80 + i, "other_name": "弟子%s" % "甲乙丙"[i]}))
	var text := _bio_text(_compose(facts))
	assert_true(text.contains("三人") or text.contains("三名"), text)
	assert_true(text.contains("咸寧紀元二〇年（12253年）"), "寫最早一年：" + text)
	assert_false(text.contains("弟子乙") or text.contains("弟子丙"), "不逐一列名：" + text)
	var two := _bio_text(_compose(facts.slice(0, 2)))
	assert_true(two.contains("弟子甲") and two.contains("弟子乙"), "兩人以內照舊逐一寫：" + two)


func test_same_month_events_each_get_their_own_row_with_full_names() -> void:
	var facts: Array = []
	for i in 2:
		facts.append(_fact("thread_omen", 100, 4, {"thread": "feud", "role": "victim", "other": 90, "other_name": "樊震", "because": _because(60 + i, 7, "林默", 40 + i, "舊仇%d" % i, "feud")}))
	var out := _compose([_past(7, "林默")] + facts)
	var rows: Array = out["rows"].slice(1)
	assert_eq(rows.size(), 2, "同一個月的兩件事各成一列")
	for row: Dictionary in rows:
		assert_true(row["text"].contains("樊震"), "每列第一次提到的人寫全名：" + row["text"])
		assert_false(row["text"].contains("此人"), row["text"])
		assert_false(row["text"].contains("；"), "不合成一句：" + row["text"])
