extends "res://tests/harness/suite.gd"
## 傳聞的內容（GDD 5.7、7.2「傳聞的合成與標記」）：年月、渡劫與突破寫出境界、人物帶身分、原因標記、後果欄位、
## 例行事件同月同類合成一則統計（攤得開名單）、異常事件各自成則、收合時異常在前。

var _game: Node
var _sim: SimHost
var _hud: Hud
var _rumor: RumorSlip
var _next_id := 5000


func suite_name() -> String:
	return "rumor_content"


func setup() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func teardown() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func _boot(view: Vector2i) -> void:
	_game = await boot(false, view)
	_sim = _game.get_node("SimHost")
	_hud = _game.get_node("Hud")
	_rumor = _hud.rumor()


## 合成一筆橋接事件。
func _event(kind: String, importance: int, tick: int, extra := {}) -> Dictionary:
	_next_id += 1
	var event := {"id": _next_id, "tick": tick, "kind": kind, "importance": importance, "province": 0, "cell": -1,
		"souls": PackedInt32Array(), "a": "沈青霜", "b": "韓烈", "f": "", "g": "", "p": "青州", "q": "", "x": "", "n": 0, "m": 0, "year": tick / 12 + 1}
	event.merge(extra, true)
	return event


func _post(event: Dictionary) -> void:
	_sim.event_posted.emit(event)


## 一筆渡劫事件：realm 是新境界，當事人原是 realm − 1。
func _trib(tick: int, soul: int, realm: int, outcome := "Passed", extra := {}) -> Dictionary:
	var base := {"souls": PackedInt32Array([soul]), "a": "霍靜沉%d" % soul, "x": outcome, "n": realm, "a_role": "Rogue", "a_power": "", "a_realm": realm - 1}
	base.merge(extra, true)
	return _event("Tribulation", 3, tick, base)


func test_every_slip_shows_year_and_month() -> void:
	await _boot(FLOW_VIEW)
	# tick 以日計（每月 30 日）：tick 1 是第一年一月，tick 780 是第三年二月（GDD 2.6）。
	_post(_event("Killed", 3, 780, {"year": 3}))
	var entry := _rumor.shown()[0]
	assert_eq(entry["month"], 2, "月由 tick 算：780 → 2 月")
	_post(_event("Killed", 3, 810, {"year": 43, "world_year": 12356, "date_era": "咸寧", "souls": PackedInt32Array([7])}))
	entry = _rumor.shown()[0]
	assert_eq(entry["era"], "咸寧", "記下事件當時的紀元名")
	assert_eq(entry["world_year"], 12356, "記下事件當時的世界年")
	assert_eq(_rumor._stamp(entry), "咸寧紀元 四三年 (12356年) 三月", "傳聞簽寫紀元時間加世界時間")
	_post(_event("Killed", 3, 1080, {"year": 3, "souls": PackedInt32Array([9])}))
	assert_eq(_rumor.shown()[0]["month"], 12, "tick 1080 是 12 月")
	_post(_event("Killed", 3, 1110, {"year": 4, "month": 7, "souls": PackedInt32Array([8])}))
	assert_eq(_rumor.shown()[0]["month"], 7, "橋接帶了月份就用橋接的")


func test_tribulation_and_breakthrough_sentences_name_the_realm() -> void:
	await _boot(FLOW_VIEW)
	assert_eq(EventText.line(_trib(50, 1, 2)), "散修霍靜沉1（築基）渡三九雷劫，結成金丹", "渡劫成功寫出成就的境界")
	assert_true(EventText.line(_trib(50, 1, 3)).ends_with("渡四九雷劫，成就元嬰"))
	assert_true(EventText.line(_trib(50, 1, 4)).ends_with("渡五九雷劫，晉入化神"))
	var shed := EventText.line(_trib(50, 1, 2, "ShedBody"))
	assert_true(shed.contains("三九雷劫") and shed.contains("未成金丹") and shed.ends_with("舍身兵解"), "兵解寫清楚：" + shed)
	var gone := EventText.line(_trib(50, 1, 3, "Perished"))
	assert_true(gone.contains("未成元嬰") and gone.ends_with("形神俱滅"), "形神俱滅寫清楚：" + gone)
	var up := EventText.line(_event("Breakthrough", 3, 50, {"n": 4, "a": "沈青霜", "a_role": "Leader", "a_power": "鶴微宗", "a_realm": 4}))
	assert_eq(up, "鶴微宗掌門沈青霜（元嬰）突破至化神", "突破寫出新境界，身分的境界是突破前的")


func test_person_events_carry_sect_role_and_realm() -> void:
	await _boot(FLOW_VIEW)
	var elder := _event("Died", 3, 50, {"a": "霍靜沉", "x": "Age", "n": 412, "a_role": "Member", "a_power": "鶴微宗", "a_realm": 3})
	assert_eq(EventText.line(elder), "鶴微宗長老霍靜沉（元嬰）壽終，享年412", "門派、長老、姓名、境界")
	var disciple := _event("Died", 3, 50, {"a": "韓烈", "x": "Killed", "n": 30, "a_role": "Member", "a_power": "鶴微宗", "a_realm": 1})
	assert_true(EventText.line(disciple).begins_with("鶴微宗弟子韓烈（築基）"), "築基以下的成員是弟子")
	var ruler := _event("Died", 4, 50, {"a": "魏素忍", "x": "Age", "n": 52, "a_role": "Ruler"})
	assert_true(EventText.line(ruler).begins_with("君主魏素忍壽終"), "君主沒有境界括號")
	var bare := _event("Killed", 3, 50)
	assert_eq(EventText.line(bare), "沈青霜殺韓烈於青州", "沒有身分欄位時只寫姓名")


func test_narrow_slip_cuts_the_identity_but_never_the_realm_or_the_result() -> void:
	await _boot(FLOW_VIEW)
	var info := EventText.describe(_trib(50, 1, 2, "Passed", {"a_role": "Member", "a_power": "鶴微宗長老修行院", "a": "歐陽慧林山"}))
	var variants: Array = info["variants"]
	for width in [400.0, 250.0, 200.0, 180.0]:
		var text := _rumor.fit_variants(variants, width, 13)
		assert_true(text.contains("（築基）") and text.ends_with("金丹"), "寬 %d：境界與結果都在：%s" % [int(width), text])
	var died := EventText.describe(_event("Died", 2, 50, {"a": "慕容歐陽青", "x": "BreakthroughFailed", "n": 191, "a_role": "Rogue", "a_realm": 2}))
	var text := _rumor.fit_variants(died["variants"], 215.0, 13)
	assert_true(text.contains("（金丹）") and text.ends_with("突破失敗而亡") and not text.contains("…"), "窄時先去享年，姓名完整：" + text)
	var founded := EventText.describe(_event("PowerFounded", 3, 50, {"a": "吳隱耀", "f": "凌鶴門", "p": "墟赤郡"}))
	text = _rumor.fit_variants(founded["variants"], 150.0, 13)
	assert_true(text.ends_with("創立凌鶴門"), "窄時創立的宗門名不被截：" + text)
	var full := _rumor.fit_variants(variants, 900.0, 13)
	assert_true(full.begins_with("鶴微宗長老修行院"), "放得下就寫全")


func test_seals_and_reason_marks_are_two_character_words() -> void:
	var seals := {}
	var table: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(Locale.UI_PATH))
	for key: String in table:
		if key.begins_with("seal_") or key.begins_with("reason_"):
			assert_eq(Locale.t(key).length(), 2, "%s 是二字詞：%s" % [key, Locale.t(key)])
		if key.begins_with("seal_"):
			assert_false(seals.has(Locale.t(key)), "印不重複：" + Locale.t(key))
			seals[Locale.t(key)] = true
	assert_gt(seals.size(), 40, "字串表有事件印")


func test_reason_mark_takes_the_first_that_holds() -> void:
	assert_eq(EventText.reason(_event("WorldChanged", 5, 1, {"change": "ColdBegan", "era": ""})), "world")
	assert_eq(EventText.reason(_event("PowerFounded", 3, 1)), "lead", "開宗立派")
	assert_eq(EventText.reason(_event("Succession", 3, 1)), "lead", "掌門更替")
	assert_eq(EventText.reason(_trib(1, 1, 5, "Perished")), "high", "高境界先於身死")
	assert_eq(EventText.reason(_trib(1, 1, 3, "Perished")), "death", "渡劫失敗")
	assert_eq(EventText.reason(_event("Died", 2, 1, {"x": "BreakthroughFailed", "a_realm": 2})), "death", "突破失敗而亡")
	assert_eq(EventText.reason(_event("ThreadResolved", 4, 1, {"n": 1})), "fate", "宿緣了結")
	assert_eq(EventText.reason(_event("Killed", 3, 1)), "", "其餘不標")
	assert_eq(EventText.reason(_event("ThreadResolved", 4, 1, {"a_realm": 4})), "high", "高境界先於宿緣")


func test_consequences_are_filled_when_the_event_alone_gives_them() -> void:
	var leader := _event("Died", 4, 1, {"x": "Killed", "a_role": "Leader", "a_power": "鶴微宗"})
	assert_eq(EventText.consequence(leader), "鶴微宗失其掌門")
	assert_eq(EventText.consequence(_trib(1, 1, 3, "Perished", {"a_role": "Leader", "a_power": "鶴微宗"})), "鶴微宗失其掌門", "掌門渡劫失敗")
	assert_eq(EventText.consequence(_event("PowerFounded", 3, 1, {"f": "太雲派", "p": "嶽渡郡"})), "嶽渡郡多一宗門")
	assert_eq(EventText.consequence(_event("Died", 2, 1, {"x": "Age"})), "", "得不出的留空")
	assert_eq(EventText.consequence(_trib(1, 1, 2)), "", "渡劫成功沒有後果欄")


func test_entries_keep_ids_to_trace_later() -> void:
	await _boot(FLOW_VIEW)
	_post(_event("Killed", 3, 100, {"persons": PackedInt32Array([4, 5]), "powers": PackedInt32Array([2]), "province": 7}))
	var entry := _rumor.shown()[0]
	assert_eq(entry["persons"], PackedInt32Array([4, 5]), "人物編號")
	assert_eq(entry["powers"], PackedInt32Array([2]), "勢力編號")
	assert_eq(entry["province"], 7, "州郡編號")
	assert_true(entry.has("consequence"), "後果欄位在")
	var slots := _rumor.action_slots(0)
	assert_true(slots["person"].size.x > 0.0 and slots["trace"].size.x > 0.0, "看人物與溯因的入口位置留著")
	assert_false(slots["person"].intersects(slots["trace"]), "兩個入口不相疊")


func test_routine_events_of_one_month_merge_into_one_statistic() -> void:
	await _boot(FLOW_VIEW)
	for i in 9:
		_post(_trib(100, i, 2))
	for i in 3:
		_post(_trib(100, 20 + i, 3))
	# 緊接著的突破是同一件事，不另記。
	_post(_event("Breakthrough", 2, 100, {"souls": PackedInt32Array([0]), "n": 2, "a": "霍靜沉0"}))
	_rumor.expanded = true
	var entries := _rumor.shown()
	assert_eq(entries.size(), 1, "同月同類合成一則")
	assert_eq(entries[0]["line"], "本月 12 人渡劫：9 人結成金丹，3 人成就元嬰", "寫出各晉到哪一階、各幾人")
	assert_eq(entries[0]["members"].size(), 12, "名單 12 人")
	_post(_trib(112, 40, 2))
	_post(_trib(112, 41, 2))
	assert_eq(_rumor.shown().size(), 2, "另一個月另成一則")
	assert_eq(_rumor.shown()[0]["line"], "本月 2 人渡劫：2 人結成金丹")


func test_fullscreen_lists_every_statistic_open_one_line_each() -> void:
	await _boot(FLOW_VIEW)
	for i in 5:
		_post(_trib(100, i, 2, "Passed", {"a_role": "Member", "a_power": "鶴微宗"}))
	await frames(2)
	assert_true(_rumor.is_group(_rumor.shown()[0]))
	assert_false(_rumor.is_open(0), "收合時名單不攤開")
	_rumor.expanded = true
	await frames(2)
	assert_true(_rumor.is_open(0), "全螢幕的統計直接攤開名單，不必再點")
	var members: Array = _rumor.shown()[0]["members"]
	assert_eq(members.size(), 5, "名單人數與統計相符")
	for j in members.size():
		var rect := _rumor.member_rect(0, j)
		assert_eq(rect.size.y, RumorSlip.FULL_MEMBER_HEIGHT, "名單每人一行")
		assert_true(members[j]["who"].begins_with("鶴微宗弟子霍靜沉"), "每行有身分與姓名：" + members[j]["who"])
		assert_eq(members[j]["what"], "結成金丹", "每行有結果")
	assert_gt(_rumor.member_rect(0, 1).position.y, _rumor.member_rect(0, 0).position.y)
	assert_eq(_rumor.row_rect(0).size.y, RumorSlip.FULL_ROW_HEIGHT, "全螢幕每列用內文字級的列高")
	assert_eq(_rumor._font(), Ink.font_size(Ink.SIZE_BODY, _hud._unit), "字級用內文字級")
	assert_true(_rumor.row_rect(0).size.x > 1500.0, "寬度用滿")
	await click(_rumor.position + _rumor.row_rect(0).get_center())
	assert_true(_rumor.is_open(0), "點統計那一列不收起名單")
	assert_true(_rumor.expanded, "也不收合全螢幕")
	assert_false(_rumor.is_hot(_rumor.row_rect(0).get_center()), "統計列不是按鈕")


func test_every_event_kind_belongs_to_exactly_one_filter_class() -> void:
	var seen := {}
	for cls: String in EventText.FILTER_KINDS:
		assert_true(cls in RumorSlip.FILTERS, "歸類名是篩選的一項：" + cls)
		for kind: String in EventText.FILTER_KINDS[cls]:
			assert_false(seen.has(kind), "%s 只屬一類（%s 與 %s）" % [kind, seen.get(kind, ""), cls])
			seen[kind] = cls
	var text := FileAccess.get_file_as_string("res://modules/locale/resources/ui.zh-Hant.json")
	var table: Dictionary = JSON.parse_string(text)
	var seals := 0
	for key: String in table:
		if key.begins_with("seal_") and key != "seal_unknown":
			seals += 1
			assert_true(seen.has(key.trim_prefix("seal_")), "有印的事件都有歸類：" + key)
	assert_gt(seals, 40, "掃到的印夠多")
	assert_eq(EventText.filter_of("WorldChanged"), "world", "世界大事歸大事")
	assert_eq(EventText.filter_of("Killed"), "person")
	assert_eq(EventText.filter_of("PowerFounded"), "sect")
	assert_eq(EventText.filter_of("DynastyFounded"), "dynasty")
	assert_eq(EventText.filter_of("ThreadResolved"), "karma")
	assert_eq(EventText.filter_of("NoSuchKind"), "world", "沒歸類的退到大事，不會從全螢幕消失")


func test_clicking_a_statistic_while_collapsed_opens_the_fullscreen_list_at_it() -> void:
	await _boot(FLOW_VIEW)
	for i in 6:
		_post(_event("Killed", 3, 300 + i * 20, {"souls": PackedInt32Array([i])}))
	# 重要度 2 的成婚統計只在全螢幕，把較舊的渡劫統計擠到畫面之外。
	for month in 12:
		for k in 3:
			_post(_event("Married", 2, 600 + month, {"souls": PackedInt32Array([200 + month * 10 + k])}))
	for i in 3:
		_post(_trib(100, i, 2))
	await frames(2)
	assert_false(_rumor.expanded)
	var index := -1
	for i in _rumor.shown().size():
		if _rumor.is_group(_rumor.shown()[i]):
			index = i
	assert_gt(index, -1, "重要度 3 的統計收合時也在")
	_rumor.activate(index)
	assert_true(_rumor.expanded, "收合時點統計：展開成全螢幕")
	var at := -1
	for i in _rumor.shown().size():
		if _rumor.is_group(_rumor.shown()[i]) and _rumor.shown()[i]["tick"] == 100:
			at = i
	assert_gt(at, -1)
	assert_true(_rumor.is_open(at), "並攤開這一則")
	assert_gt(_rumor._max_scroll(), 0.0, "這份清單要捲動")
	assert_true(_rumor._view_rect().intersects(_rumor.row_rect(at)), "捲到那一則")


## 世界裡實際存在的前幾位人物編號。
func _valid_people(count: int) -> Array[int]:
	var out: Array[int] = []
	var id := 0
	while out.size() < count and id < 5000:
		if not _sim.world.person(id).is_empty():
			out.append(id)
		id += 1
	return out


func test_abnormal_events_are_never_merged() -> void:
	await _boot(FLOW_VIEW)
	_post(_trib(100, 1, 3, "Perished"))
	_post(_trib(100, 2, 3, "ShedBody"))
	_post(_trib(100, 3, 5))
	_post(_trib(100, 4, 5))
	_post(_event("PowerFounded", 3, 100, {"souls": PackedInt32Array([7])}))
	_post(_event("PowerFounded", 3, 100, {"souls": PackedInt32Array([8])}))
	_post(_event("Succession", 3, 100, {"souls": PackedInt32Array([9])}))
	_post(_event("ThreadResolved", 4, 100, {"souls": PackedInt32Array([10]), "n": 1}))
	_post(_event("Killed", 3, 100, {"souls": PackedInt32Array([11])}))
	_post(_event("DynastyFounded", 4, 100, {"souls": PackedInt32Array([12])}))
	_rumor.expanded = true
	var entries := _rumor.shown()
	assert_eq(entries.size(), 10, "失敗、高境界、創派、更替、宿緣、殺、立國各自成則")
	for entry in entries:
		assert_false(_rumor.is_group(entry), "沒有被合成：" + entry["line"])


func test_collapsed_slip_puts_abnormal_before_routine_and_one_statistic_per_kind_per_month() -> void:
	await _boot(FLOW_VIEW)
	# 較舊的三件異常，之後每個月都有一組例行渡劫。
	for i in 3:
		_post(_trib(100 + i, i, 3, "Perished"))
	for month in 10:
		for i in 4:
			_post(_trib(200 + month, 100 + month * 10 + i, 2))
		_post(_event("Married", 2, 200 + month, {"souls": PackedInt32Array([500 + month])}))
	var shown := _rumor.shown()
	assert_eq(shown.size(), RumorSlip.SHOWN_MAX, "收合 8 張")
	for i in 3:
		assert_false(_rumor.is_routine(shown[i]), "異常在前：" + shown[i]["line"])
	for i in range(3, 8):
		assert_true(_rumor.is_routine(shown[i]), "其後才是例行統計")
	var seen := {}
	for entry in _rumor.shown():
		if _rumor.is_routine(entry):
			var key := "%d/%s" % [entry["tick"], entry["category"]]
			assert_false(seen.has(key), "每類每月至多一則")
			seen[key] = true
	_rumor.expanded = true
	assert_eq(_rumor.shown()[0]["tick"], 209, "展開的清單仍依時間新到舊")


func test_omens_of_one_month_merge_too() -> void:
	await _boot(FLOW_VIEW)
	for i in 4:
		_post(_event("TribulationOmen", 2, 100, {"souls": PackedInt32Array([i]), "a": "霍靜沉%d" % i, "a_role": "Rogue", "a_realm": 3}))
	_post(_event("TribulationOmen", 2, 100, {"souls": PackedInt32Array([9]), "a": "高人", "a_role": "Rogue", "a_realm": 5}))
	_rumor.expanded = true
	var entries := _rumor.shown()
	assert_eq(entries.size(), 2, "低境界的預兆合成一則，化神以上的各自成則")
	assert_eq(entries[0]["line"] if _rumor.is_group(entries[0]) else entries[1]["line"], "本月 4 人劫期將至，劫雲匯聚")


func test_omen_names_a_sect_gate_only_for_sect_members() -> void:
	var rogue := _event("TribulationOmen", 2, 100, {"a": "廖遠壯", "a_role": "Rogue", "a_realm": 3, "p": "青州"})
	assert_eq(EventText.line(rogue), "散修廖遠壯（元嬰）劫期將至，青州上空劫雲匯聚", "散修沒有山門，寫所在州郡")
	var leader := _event("TribulationOmen", 2, 100, {"a": "蔡舞虹", "a_role": "Leader", "a_power": "玄曦宗", "a_realm": 3, "p": "青州"})
	assert_eq(EventText.line(leader), "玄曦宗掌門蔡舞虹（元嬰）劫期將至，玄曦宗山門上空劫雲匯聚", "宗門中人寫自家山門")
	var nowhere := _event("TribulationOmen", 2, 100, {"a": "廖遠壯", "a_role": "Rogue", "a_realm": 3, "p": ""})
	assert_false(EventText.line(nowhere).contains("山門"), "沒有州郡也不寫山門")


func _died(tick: int, soul: int, cause: String, realm: int, extra := {}) -> Dictionary:
	var base := {"souls": PackedInt32Array([soul]), "a": "霍靜沉%d" % soul, "x": cause, "n": 90 + soul, "a_role": "Rogue", "a_power": "", "a_realm": realm,
		"a_person": 100 + soul, "persons": PackedInt32Array([100 + soul])}
	base.merge(extra, true)
	return _event("Died", 2, tick, base)


func test_low_realm_deaths_of_one_month_merge_by_cause() -> void:
	await _boot(FLOW_VIEW)
	for i in 5:
		_post(_died(100, i, "BreakthroughFailed", 2))
	for i in 3:
		_post(_died(100, 10 + i, "Age", 1))
	_post(_died(100, 20, "Smite", 2))
	_post(_died(100, 21, "Smite", 2))
	# 不合成：被殺（有兇手）、掌門、君主、高境界、單獨一人的死因。
	_post(_died(100, 30, "Killed", 2))
	_post(_died(100, 31, "BreakthroughFailed", 2, {"a_role": "Leader", "a_power": "鶴微宗"}))
	_post(_died(100, 32, "Age", 2, {"a_role": "Ruler"}))
	_post(_died(100, 33, "BreakthroughFailed", 4))
	_post(_died(100, 34, "War", 1))
	_rumor.expanded = true
	var groups := {}
	var singles := 0
	for entry in _rumor.shown():
		if _rumor.is_group(entry):
			groups[entry["line"]] = entry
		else:
			singles += 1
	assert_true(groups.has("本月 5 人突破失敗而亡"), "低境界同月同死因合成一則：" + str(groups.keys()))
	assert_true(groups.has("本月 3 人壽終"))
	assert_true(groups.has("本月 2 人遭天譴而亡"))
	assert_eq(groups.size(), 3)
	assert_eq(singles, 5, "被殺、掌門、君主、高境界、單一死因各自成則")
	var members: Array = groups["本月 5 人突破失敗而亡"]["members"]
	assert_eq(members.size(), 5)
	assert_true(String(members[0]["who"]).begins_with("散修霍靜沉"), "名單有身分與姓名：" + members[0]["who"])
	assert_eq(members[0]["what"], "突破失敗而亡，享年94", "名單有死因與享年")
	assert_eq(members[0]["a_person"], 104, "名單每行帶人物編號，點了開人物卡")
	var marked := 0
	for entry in _rumor.shown():
		if not _rumor.is_group(entry) and entry["reason"] == "death":
			marked += 1
	assert_gt(marked, 0, "不合成的身死帶「隕落」標記")


func test_person_slot_and_statistic_rows_open_the_person_card() -> void:
	await _boot(FLOW_VIEW)
	var opened: Array[int] = []
	var flown: Array[Vector2] = []
	_rumor.person_requested.connect(func(id: int) -> void: opened.append(id))
	_rumor.fly_requested.connect(func(pos: Vector2) -> void: flown.append(pos))
	var people := _valid_people(4)
	_post(_event("Killed", 3, 100, {"persons": PackedInt32Array([people[0], 5]), "a_person": people[0], "pos": Vector2(10.0, 10.0), "souls": PackedInt32Array([7])}))
	_rumor.expanded = true
	await frames(2)
	var index := 0
	assert_true(_rumor.has_person_slot(index), "有主要人物的單則有「看人物」入口")
	var slot: Rect2 = _rumor.action_slots(index)["person"]
	assert_true(_rumor.row_rect(index).encloses(slot), "入口在這一則的紙面內")
	assert_true(slot.size.x >= RumorSlip.SLOT_WIDTH * _hud._unit - 0.1, "入口夠一顆小印大")
	await click(_rumor.position + slot.get_center())
	assert_eq(opened, [people[0]], "點小印開這個人的卡")
	assert_true(flown.is_empty(), "點小印不另外飛往")
	assert_false(_rumor.expanded, "開人物卡即收合全螢幕，卡片與全螢幕紙不同時在畫面上")
	# 點整則的空白處仍是飛往該地，並收起清單。
	await click(_rumor.position + _rumor.row_rect(index).position + Vector2(_rumor.row_rect(index).size.x * 0.5, 4.0))
	assert_eq(flown, [Vector2(10.0, 10.0)], "點空白處飛往")
	# 統計名單每一行開那個人的卡。
	_rumor.expanded = true
	for i in 3:
		_post(_died(120, 40 + i, "BreakthroughFailed", 2, {"a_person": people[1 + i], "persons": PackedInt32Array([people[1 + i]])}))
	await frames(2)
	var group := -1
	for i in _rumor.shown().size():
		if _rumor.is_group(_rumor.shown()[i]):
			group = i
	assert_gt(group, -1)
	assert_false(_rumor.has_person_slot(group), "統計本身沒有看人物入口")
	assert_true(_rumor.is_open(group), "全螢幕的名單本來就攤開")
	opened.clear()
	var member: Dictionary = _rumor.shown()[group]["members"][1]
	await click(_rumor.position + _rumor.member_rect(group, 1).get_center())
	assert_eq(opened, [int(member["a_person"])], "點名單的一行開那個人的卡")
	assert_false(_rumor.expanded, "開卡即收合")


func test_tribulation_death_counts_once_in_the_death_statistic() -> void:
	await _boot(FLOW_VIEW)
	# 渡劫失敗的雷劫事件與緊接著的身死是同一件事：合進身死統計，不再留一筆單列的渡劫。
	for i in 3:
		_post(_trib(100, i, 3, "Perished"))
		_post(_died(100, i, "Tribulation", 2))
	_post(_trib(100, 9, 5, "Perished"))
	_post(_died(100, 9, "Tribulation", 4))
	_rumor.expanded = true
	var lines: Array[String] = []
	for entry in _rumor.shown():
		lines.append(entry["line"])
	assert_true(lines.has("本月 3 人渡劫身隕"), "三人合成一則：" + str(lines))
	assert_eq(lines.size(), 2, "高境界渡劫失敗另成一則，沒有重複的身死：" + str(lines))


func test_real_simulation_does_not_flood_with_routine_tribulations() -> void:
	await _boot(FLOW_VIEW)
	for shi in 4:
		_sim.advance(360)
	_rumor.expanded = true
	var per_month := {}
	for entry in _rumor.shown():
		if entry["kind"] == "Tribulation" and not entry["reason"] in ["high", "death"]:
			var key := "%d" % entry["tick"]
			assert_false(per_month.has(key), "同一個月的例行渡劫只有一則")
			per_month[key] = true
		assert_false(String(entry["line"]).contains("{"), "句子沒有留下未填的模板")


func test_marriage_and_its_bond_are_one_rumor() -> void:
	await _boot(FLOW_VIEW)
	# 成婚同時結下的情緣是同一件事：只留成婚那一則，先到後到都一樣。
	_post(_event("Married", 2, 100, {"souls": PackedInt32Array([1, 2]), "a": "徐清", "b": "丁磊"}))
	_post(_event("ThreadCreated", 2, 100, {"souls": PackedInt32Array([1, 2]), "a": "徐清", "b": "丁磊", "x": "Love"}))
	_post(_event("ThreadCreated", 2, 100, {"souls": PackedInt32Array([4, 3]), "a": "石瑟", "b": "趙遠", "x": "Love"}))
	_post(_event("Married", 2, 100, {"souls": PackedInt32Array([3, 4]), "a": "趙遠", "b": "石瑟"}))
	# 沒有成婚的結緣照留。
	_post(_event("ThreadCreated", 2, 100, {"souls": PackedInt32Array([5, 6]), "a": "余菊", "b": "陳清", "x": "Grace"}))
	_rumor.expanded = true
	var kinds: Array[String] = []
	var people := 0
	for entry in _rumor.shown():
		kinds.append(entry["kind"])
		people += maxi(1, entry["members"].size())
	assert_eq(kinds.count("ThreadCreated"), 1, "只剩沒成婚的那一筆結緣：" + str(kinds))
	assert_eq(people, 3, "兩對成婚合成一則統計，加一筆結緣：" + str(kinds))
