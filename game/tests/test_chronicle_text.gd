extends "res://tests/harness/suite.gd"
## 小傳與年表的組合（GDD 5.12）：漢字數字、折行片段、句型庫、同一個人文字逐字相同，以及 5.12 驗收的前三項。

const SEED := {"seed": 20261011, "size": "small", "rng_seed": 7}

var _world: SimWorld


func suite_name() -> String:
	return "chronicle_text"


func _make_world() -> SimWorld:
	var world := SimWorld.new()
	world.create_with(SEED)
	world.step(12 * 220)
	return world


func _lookup(pid: int) -> Dictionary:
	_ensure()
	return _world.person(pid)


func _compose(id: int) -> Dictionary:
	_ensure()
	return ChronicleText.compose(_world.person(id), _world.person_chronicle(id), _lookup)


func _ensure() -> void:
	if _world == null:
		_world = _make_world()


## 在世與已歿各取 count 位有生平可寫的人；past_min 為其中至少要有前世的人數。
func _sample(count: int) -> Dictionary:
	_ensure()
	var alive: Array[int] = []
	var dead: Array[int] = []
	var past: Array[int] = []
	var id := 0
	while not _world.person(id).is_empty():
		var info := _world.person(id)
		var facts := _world.person_chronicle(id)
		if facts.size() >= 4:
			var has_past: bool = facts[0]["kind"] == "past_life"
			if has_past and past.size() < 6:
				past.append(id)
			elif info["alive"] and alive.size() < count / 2:
				alive.append(id)
			elif not info["alive"] and dead.size() < count / 2:
				dead.append(id)
		id += 1
	return {"alive": alive, "dead": dead, "past": past}


func _people(count: int) -> Array[int]:
	var s := _sample(count)
	var out: Array[int] = []
	out.append_array(s["past"])
	out.append_array(s["alive"])
	out.append_array(s["dead"])
	return out


# ── 單元 ──────────────────────────────────────────────────────

func test_han_writes_chinese_numerals() -> void:
	assert_eq(ChronicleText.han(1), "一")
	assert_eq(ChronicleText.han(10), "十")
	assert_eq(ChronicleText.han(13), "十三")
	assert_eq(ChronicleText.han(20), "二十")
	assert_eq(ChronicleText.han(25), "二十五")
	assert_eq(ChronicleText.han(100), "一百")
	assert_eq(ChronicleText.han(102), "一百零二")
	assert_eq(ChronicleText.han(111), "一百一十一")
	assert_eq(ChronicleText.han(1000), "一千")


func test_keep_pieces_finds_year_and_month_runs() -> void:
	var pieces := ChronicleText.keep_pieces("一百零二年二月，他於十三年入門。")
	assert_true(pieces.has("一百零二年二月"))
	assert_true(pieces.has("十三年"))
	assert_eq(ChronicleText.keep_pieces("無年月").size(), 0)


func test_every_pattern_key_has_variants_and_known_placeholders() -> void:
	var lib := ChronicleText.library()
	var checked := 0
	for key: String in lib:
		if key.begins_with("_") or typeof(lib[key]) != TYPE_ARRAY:
			continue
		assert_gt(ChronicleText.variant_count(key), 0, "句型 %s 至少一種" % key)
		checked += 1
	assert_gt(checked, 20, "句型庫有相當數量的鍵")


func test_empty_facts_compose_nothing_but_the_origin() -> void:
	var info := {"id": 1, "name": "甲", "alive": true, "age": 20}
	var out := ChronicleText.compose(info, [])
	assert_eq(out["rows"].size(), 0, "沒有事實就沒有年表")


# ── 全體 ──────────────────────────────────────────────────────

func test_no_unfilled_placeholder_for_everyone() -> void:
	_ensure()
	var id := 0
	var bad := 0
	while not _world.person(id).is_empty():
		var out := _compose(id)
		for para: Dictionary in out["bio"]:
			if para["text"].contains("{") or para["text"].contains("@"):
				bad += 1
		for row: Dictionary in out["rows"]:
			if row["text"].contains("{") or row["text"].contains("@") or row["date"].contains("0"):
				bad += 1
		id += 1
	assert_gt(id, 100, "有足夠的人")
	assert_eq(bad, 0, "沒有未填的占位符")


# ── 5.12 驗收 ─────────────────────────────────────────────────

## 驗收一：二十人（在世已歿各半、至少五人有前世）。同一件事沒有出現兩次；再婚之前有上一段婚姻的結束；
## 換師之前有原因；前世的事用前世的名字。
func test_acceptance_1_no_duplicates_and_causes_precede() -> void:
	_ensure()
	var people := _people(20)
	assert_gt(people.size(), 13, "取得足夠的樣本")
	var past_count := 0
	for id in people:
		var facts := _world.person_chronicle(id)
		var out := _compose(id)
		# 同一件事不出現兩次：年表每條對到不同事件，文字也不重複。
		var seen_events := {}
		var seen_text := {}
		for row: Dictionary in out["rows"]:
			var event := int(row["event"])
			if event >= 0:
				assert_false(seen_events.has(event), "%d 號的年表事件 %d 重複" % [id, event])
				seen_events[event] = true
			assert_false(seen_text.has(row["date"] + row["text"]), "%d 號的年表有重複的句子：%s" % [id, row["text"]])
			seen_text[row["date"] + row["text"]] = true
		# 小傳段與段之間不重複。
		var paras := {}
		for para: Dictionary in out["bio"]:
			assert_false(paras.has(para["text"]), "小傳段落重複")
			paras[para["text"]] = true
		# 再婚之前，上一段婚姻已結束；換師之前，上一位師父的緣分已了結。
		var open_marriages := 0
		var open_masters := 0
		for fact: Dictionary in facts:
			match fact["kind"]:
				"married":
					assert_eq(open_marriages, 0, "%d 號再婚前上一段婚姻要先有結束" % id)
					open_marriages += 1
				"widowed":
					open_marriages = maxi(open_marriages - 1, 0)
				"apprenticed":
					if fact["role"] == "disciple":
						assert_eq(open_masters, 0, "%d 號換師前要先有了結" % id)
						open_masters += 1
				"mentor_ended":
					if fact["role"] == "disciple":
						open_masters = maxi(open_masters - 1, 0)
		# 前世的事用前世的名字。
		if not facts.is_empty() and facts[0]["kind"] == "past_life":
			past_count += 1
			assert_true(out["bio"][0]["text"].contains(facts[0]["name"]), "%d 號的小傳第一段寫前世的名字" % id)
	assert_gt(past_count, 4, "至少五人有前世")


## 驗收二：同一個人連續組三次，文字逐字相同；另建一個相同種子的世界也相同。
func test_acceptance_2_same_text_every_time_and_after_reload() -> void:
	var other := _make_world()
	for id in _people(20):
		var a := _compose(id)
		var b := _compose(id)
		var c := _compose(id)
		assert_eq(a["bio"], b["bio"], "%d 號連續兩次相同" % id)
		assert_eq(b["rows"], c["rows"], "%d 號連續三次相同" % id)
		var d := ChronicleText.compose(other.person(id), other.person_chronicle(id), func(p: int) -> Dictionary: return other.person(p))
		assert_eq(a["bio"], d["bio"], "%d 號重建世界後小傳相同" % id)
		assert_eq(a["rows"], d["rows"], "%d 號重建世界後年表相同" % id)


## 驗收三：連續看二十個人的小傳，入道與師門一段至少四種不同句型。
func test_acceptance_3_school_paragraph_uses_at_least_four_patterns() -> void:
	_ensure()
	var variants := {}
	var looked := 0
	var id := 0
	while looked < 20 and not _world.person(id).is_empty():
		var out := _compose(id)
		if out["used"].has("school") and not out["used"]["school"].is_empty():
			for key: String in out["used"]["school"]:
				variants[key] = true
			looked += 1
		id += 1
	assert_eq(looked, 20, "取到二十個有入道與師門的人")
	assert_gt(variants.size(), 3, "句型 %d 種：%s" % [variants.size(), variants.keys()])
