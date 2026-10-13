extends "res://tests/harness/suite.gd"
## 宗門卡（GDD 5.8）：每一處宗門名點了都開同一張宗門卡；卡上的數字與名單對得起來；掌門、門人、關係、門史裡的名字點得到、
## 返回鍵回得來；已覆滅的宗門也開得了卡；地圖山門可點又不礙地圖操作；1280×800 與 1920×1080（含 150% 縮放）排得下。

const DECK_VIEW := Vector2i(1280, 800)

var _game: Node
var _sim: SimHost
var _hud: Hud
var _camera: MapCamera


func suite_name() -> String:
	return "sect_card"


func setup() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func teardown() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func _boot(view: Vector2i) -> void:
	_game = await boot(false, view)
	_sim = _game.get_node("SimHost")
	_hud = _game.get_node("Hud")
	_camera = _game.get_node("MapCamera")
	# 走一百年，讓宗門有門史、有分有滅。
	_sim.world.step(12 * 100)
	_sim.world.flush_terrain()
	_hud._overlay.refresh_sects()


func _card() -> SectCard:
	return _hud.sect_card()


func _sects(alive: bool) -> Array:
	return _sim.world.sects().filter(func(row: Dictionary) -> bool: return row["alive"] == alive)


## 門人最多的在世宗門。
func _biggest() -> int:
	var best := -1
	var most := -1
	for row: Dictionary in _sects(true):
		if int(row["members"]) > most:
			most = row["members"]
			best = row["id"]
	return best


## 第一個有友好或敵對對象的在世宗門。
func _with_relation() -> int:
	for row: Dictionary in _sects(true):
		var info := _sim.world.sect(row["id"])
		if not (info["friends"] as Array).is_empty() or not (info["foes"] as Array).is_empty():
			return row["id"]
	return -1


## 卡片固定部分裡，第一個目標以 prefix 起頭的連結的螢幕座標；沒有回傳 INF。
func _head_point(card: SectCard, prefix: String) -> Vector2:
	var paper := card._paper_rect(card._content_height())
	var origin := card.global_position + paper.position + Vector2((LedgerCard.STRIP + 14.0) * card._unit, 0.0)
	for block: Dictionary in card._blocks:
		if block["kind"] != "para":
			continue
		for hit: Dictionary in block["paragraph"]["hits"]:
			if String(hit["target"]).begins_with(prefix):
				return origin + Vector2(0.0, float(block["y"])) + (hit["rect"] as Rect2).get_center()
	return Vector2.INF


## 清單裡第一個目標以 prefix 起頭的連結的螢幕座標（會把清單捲到看得見）；沒有回傳 INF。
func _list_point(card: SectCard, prefix: String) -> Vector2:
	var list := card.list()
	var date_height := LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, card._unit))
	for row: Dictionary in card._rows:
		if row["kind"] != "event":
			continue
		for hit: Dictionary in row["paragraph"]["hits"]:
			if String(hit["target"]).begins_with(prefix):
				list.scroll = clampf(float(row["top"]) - 8.0, 0.0, list.max_scroll())
				await wait(0.3)
				var inside := Vector2(0.0, float(row["top"]) + date_height) + (hit["rect"] as Rect2).get_center() - Vector2(0.0, list.scroll)
				return list.get_global_rect().position + inside
	return Vector2.INF


func test_person_card_sect_name_opens_the_sect_card_and_back_returns() -> void:
	await _boot(FLOW_VIEW)
	var picked := -1
	for row: Dictionary in _sects(true):
		for member: Dictionary in _sim.world.sect(row["id"])["members"]:
			if picked < 0 and not member["is_leader"]:
				picked = member["id"]
	assert_gt(picked, -1, "有宗門中人")
	var power: int = _sim.world.person(picked)["power"]
	_hud.open_person(picked)
	await wait(0.5)
	var person_card := _hud.person_card()
	var paper := person_card._paper_rect(person_card._content_height())
	var origin := person_card.global_position + Vector2(paper.position.x + (LedgerCard.STRIP + 14.0) * person_card._unit + person_card._side(), paper.position.y + float(person_card._layout["subtitle_y"]))
	var target := ""
	var point := Vector2.INF
	for hit: Dictionary in person_card._subtitle["hits"]:
		if String(hit["target"]).begins_with("sect:"):
			target = hit["target"]
			point = origin + (hit["rect"] as Rect2).get_center()
	assert_eq(target, "sect:%d" % power, "首列的宗門名連到他的宗門")
	await click(point)
	assert_true(_card().is_open(), "點首列的宗門名開宗門卡")
	assert_eq(_card().sect, power)
	assert_false(person_card.is_open(), "一次只開一張")
	assert_true(_card().can_back)
	await wait(0.5)
	assert_lt(_camera._target_focus.distance_to(_sim.world.sect(power)["pos"]), 0.01, "鏡頭飛往山門")
	await key(KEY_ESCAPE)
	assert_true(person_card.is_open() and person_card.person == picked, "Esc 回到人物卡")
	assert_false(_card().is_open())


func test_person_history_sect_name_opens_the_sect_card() -> void:
	await _boot(FLOW_VIEW)
	var picked := -1
	var wanted := -1
	for row: Dictionary in _sects(true):
		var members: Array = _sim.world.sect(row["id"])["members"]
		for member: Dictionary in members:
			if picked >= 0:
				break
			for entry: Dictionary in _sim.world.person_history(member["id"]):
				if int(entry["f_power"]) >= 0 and entry["kind"] != "Died":
					picked = member["id"]
					wanted = entry["f_power"]
					break
	assert_gt(picked, -1, "有歷程提到宗門的人")
	_hud.open_person(picked)
	await wait(0.5)
	var card := _hud.person_card()
	var list: LedgerList = card._list
	var point := Vector2.INF
	for row: Dictionary in card._rows:
		var base := Vector2(0.0, float(row["top"]) + LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, card._unit)))
		for hit: Dictionary in row["paragraph"]["hits"]:
			if hit["target"] == "sect:%d" % wanted:
				list.scroll = maxf(float(row["top"]) - 8.0, 0.0)
				await frames(2)
				point = list.get_global_rect().position + base + (hit["rect"] as Rect2).get_center() - Vector2(0.0, list.scroll)
				break
		if point != Vector2.INF:
			break
	assert_ne(point, Vector2.INF, "歷程的句子裡宗門名可點")
	await click(point)
	assert_true(_card().is_open() and _card().sect == wanted, "點歷程裡的宗門名開那一宗的卡")


func test_event_card_sect_name_opens_the_sect_card() -> void:
	await _boot(FLOW_VIEW)
	var id := _biggest()
	var founding := {}
	for entry: Dictionary in _sim.world.sect_history(id):
		if entry["kind"] == "PowerFounded" or entry["kind"] == "SectSplit":
			founding = entry
	assert_false(founding.is_empty(), "最大的宗門有開派的事件")
	_hud.open_event(founding["id"])
	await wait(0.5)
	var card := _hud.event_card()
	var paper := card._paper_rect(card._content_height())
	var origin := card.global_position + paper.position + Vector2((LedgerCard.STRIP + 14.0) * card._unit, float(card._layout["sentence_y"]))
	var point := Vector2.INF
	for hit: Dictionary in card._sentence["hits"]:
		if hit["target"] == "sect:%d" % id:
			point = origin + (hit["rect"] as Rect2).get_center()
	assert_ne(point, Vector2.INF, "事件卡那句話裡的宗門名可點")
	await click(point)
	assert_true(_card().is_open() and _card().sect == id, "點事件卡的宗門名開宗門卡")
	await key(KEY_ESCAPE)
	assert_true(card.is_open(), "Esc 回到事件卡")


func test_rumor_slip_has_a_sect_slot_that_opens_the_card() -> void:
	await _boot(FLOW_VIEW)
	var id := _biggest()
	var entry := {}
	for item: Dictionary in _sim.world.sect_history(id):
		if item["kind"] == "PowerFounded" or item["kind"] == "SectSplit":
			entry = item
	assert_false(entry.is_empty())
	var rumor := _hud.rumor()
	assert_true(rumor.post(entry), "開派的事件入簽")
	var index := -1
	for i in rumor.shown().size():
		if int(rumor.shown()[i]["id"]) == int(entry["id"]):
			index = i
	assert_gt(index, -1)
	assert_true(rumor.has_sect_slot(index), "這一則有「看宗門」入口")
	assert_eq(int(rumor.shown()[index]["sect"]), RumorSlip.sect_of(entry))
	rumor.activate(RumorSlip.SECT_BASE + index)
	assert_true(_card().is_open() and _card().sect == RumorSlip.sect_of(entry), "點「看宗門」開宗門卡")


func test_almanac_sect_rank_closes_flies_and_opens_the_card() -> void:
	await _boot(FLOW_VIEW)
	var id := _biggest()
	_hud.toggle_almanac()
	await wait(0.3)
	assert_true(_hud.almanac().expanded)
	_hud.almanac().activate_target("go:sect:%d" % id)
	await wait(0.6)
	assert_false(_hud.almanac().expanded, "收起天下冊")
	assert_true(_card().is_open() and _card().sect == id, "開宗門卡")
	assert_lt(_camera._target_focus.distance_to(_sim.world.sect(id)["pos"]), 0.01, "飛往山門")


func test_card_numbers_agree_with_the_list_and_realms() -> void:
	await _boot(FLOW_VIEW)
	var id := _biggest()
	_hud.open_sect(id)
	await wait(0.5)
	var card := _card()
	var info := _sim.world.sect(id)
	assert_eq(card._title(), info["name"], "題簽寫宗門名")
	assert_eq(card.member_row_count(), info["member_count"], "門人數等於名單條數")
	assert_eq(card.member_row_count(), _sim.world.sects().filter(func(r: Dictionary) -> bool: return r["id"] == id)[0]["members"], "與宗門清單的成員數相同")
	var total := int(info["other_members"])
	for n in (info["realm_counts"] as PackedInt32Array):
		total += n
	assert_eq(total, info["member_count"], "各境界人數總和等於門人數")
	var rows := 0
	var last_realm := 100
	for row: Dictionary in card._rows:
		if row["kind"] == "member":
			rows += 1
			var realm: int = _sim.world.person(row["person"])["realm"]
			assert_true(realm <= last_realm, "依境界由高到低")
			last_realm = realm
	assert_eq(rows, info["member_count"])
	assert_eq(card.history_row_count(), _sim.world.sect_history(id).size(), "門史每一則都列出")
	var head := card.head_text()
	assert_true(head.contains(Locale.t("sect_good") if not info["evil"] else Locale.t("sect_evil")), "首列寫正邪")
	assert_true(head.contains(info["province_name"]) and head.contains(Locale.t("sect_leader")), "首列有州郡，下有掌門")
	assert_true(head.contains(Locale.t("sect_ascended")), "寫飛升者")


func test_leader_member_relation_and_history_links_work_and_back_returns() -> void:
	await _boot(FLOW_VIEW)
	var id := _with_relation()
	for _round in 8:
		if id >= 0:
			break
		_sim.world.step(12 * 50)
		id = _with_relation()
	assert_gt(id, -1, "有宗門與他宗有往來")
	_hud.open_sect(id)
	await wait(0.5)
	var card := _card()
	# 掌門。
	var leader := _head_point(card, "person:")
	assert_ne(leader, Vector2.INF, "掌門名可點")
	await click(leader)
	assert_true(_hud.person_card().is_open(), "點掌門開人物卡")
	await key(KEY_ESCAPE)
	assert_true(card.is_open() and card.sect == id, "返回宗門卡")
	# 關係裡的宗門。
	var other := _head_point(card, "sect:")
	assert_ne(other, Vector2.INF, "關係裡的宗門名可點")
	await click(other)
	assert_true(card.is_open() and card.sect != id, "點關係裡的宗門換成那一宗的卡")
	var reached := card.sect
	assert_true(_sim.world.sect(reached)["name"] == card._title())
	await key(KEY_ESCAPE)
	assert_eq(card.sect, id, "返回原來那一宗")
	await wait(0.6)
	# 門人名單第一列。
	var list := card.list()
	list.scroll = list.max_scroll()
	await frames(2)
	var member_top := -1.0
	for row: Dictionary in card._rows:
		if row["kind"] == "member":
			member_top = row["top"]
			break
	assert_gt(member_top, -1.0, "有門人名單")
	list.scroll = clampf(member_top - 4.0, 0.0, list.max_scroll())
	await frames(2)
	await click(list.get_global_rect().position + Vector2(40.0 * card._unit, member_top - list.scroll + 0.5 * SectCard.MEMBER_ROW * card._unit))
	assert_true(_hud.person_card().is_open(), "點門人開人物卡")
	await key(KEY_ESCAPE)
	assert_true(card.is_open(), "返回宗門卡")
	await wait(0.6)
	# 門史一筆（人名、宗門名之外的部分）開事件卡。
	var event_row := {}
	for row: Dictionary in card._rows:
		if row["kind"] == "event":
			event_row = row
			break
	assert_false(event_row.is_empty(), "有門史")
	list.scroll = 0.0
	await frames(2)
	var date_height := LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, card._unit))
	await click(list.get_global_rect().position + Vector2(list.size.x - 20.0 * card._unit, float(event_row["top"]) + date_height * 0.5))
	assert_true(_hud.event_card().is_open() and _hud.event_card().event == event_row["event"], "點門史一筆開事件卡")
	await key(KEY_ESCAPE)
	assert_true(card.is_open() and card.sect == id)


func test_history_link_in_the_sect_history_reaches_a_person_or_a_sect() -> void:
	await _boot(FLOW_VIEW)
	var found := -1
	for row: Dictionary in _sects(true):
		_hud.open_sect(row["id"])
		await frames(2)
		var point := await _list_point(_card(), "person:")
		if point != Vector2.INF:
			await click(point)
			assert_true(_hud.person_card().is_open(), "點門史句子裡的人名開人物卡 點=%s 清單=%s 捲=%s" % [point, _card().list().get_global_rect(), _card().list().scroll])
			found = row["id"]
			break
	assert_gt(found, -1, "某一宗的門史句子裡有可點的人名")
	await key(KEY_ESCAPE)
	assert_true(_card().is_open() and _card().sect == found, "返回宗門卡")


func test_fallen_sect_opens_a_card_with_fall_year_and_cause() -> void:
	await _boot(FLOW_VIEW)
	var fallen := _sects(false)
	assert_gt(fallen.size(), 0, "一百年後有宗門覆滅")
	var id: int = fallen[0]["id"]
	var info := _sim.world.sect(id)
	_hud.open_sect(id)
	await wait(0.5)
	var card := _card()
	assert_true(card.is_open() and card.sect == id, "已覆滅的宗門也開得了卡")
	assert_eq(card.member_row_count(), 0, "沒有門人名單")
	var head := card.head_text()
	assert_true(head.contains(Locale.t("sect_fell")) and head.contains(str(info["fallen_year"])), "寫明覆滅年")
	assert_true(head.contains(Locale.t("sect_fell_war")) or head.contains(Locale.t("sect_fell_scatter")), "寫明覆滅原因")
	var kinds: Array[String] = []
	for entry: Dictionary in _sim.world.sect_history(id):
		kinds.append(entry["kind"])
	assert_eq(kinds[-1], "PowerFallen", "門史以覆滅作結")
	assert_lt(_camera._target_focus.distance_to(info["pos"]), 0.01, "飛往它原本的山門")


func test_map_gate_is_clickable_without_blocking_the_map() -> void:
	await _boot(FLOW_VIEW)
	var id := _biggest()
	var pos: Vector2 = _sim.world.sect(id)["pos"]
	_camera.look_at_map(pos, _hud._map.map_size.x * 0.15, true)
	await wait(0.4)
	_hud._overlay.refresh_sects()
	await frames(3)
	assert_true(_hud._overlay.sect_gates_visible())
	var hit: Control = _hud._overlay._hits[id]
	assert_true(hit.visible, "山門點擊區在中景出現")
	assert_eq(_hud._overlay.mouse_filter, Control.MOUSE_FILTER_IGNORE, "覆蓋層本身仍不攔事件")
	assert_lt(hit.size.x, 200.0 * _hud._unit, "只有圖示與名字那一小塊")
	assert_true(hit.is_hot(Vector2.ZERO), "游標移上去是可點的指標")
	var before_focus := _camera._target_focus
	var outside := hit.get_global_rect().end + Vector2(70.0, 70.0)
	await drag(outside, outside + Vector2(160.0, 0.0))
	assert_gt(_camera._target_focus.distance_to(before_focus), 0.001, "山門以外的拖曳照常平移")
	await frames(2)
	_camera.look_at_map(pos, _hud._map.map_size.x * 0.15, true)
	await wait(0.6)
	await frames(2)
	hit = _hud._overlay._hits[id]
	assert_true(hit.visible, "拖曳後山門點擊區仍在")
	await click(hit.get_global_rect().get_center())
	assert_true(_card().is_open() and _card().sect == id, "點山門圖示開宗門卡")
	assert_false(_hud._card.is_open(), "沒有同時選中州郡")
	await key(KEY_ESCAPE)
	assert_false(_card().is_open())
	await click(hit.get_global_rect().end + Vector2(70.0, 70.0))
	assert_false(_card().is_open(), "點山門以外的地方不開宗門卡")


func test_cards_fit_on_deck() -> void:
	await _check_fit(DECK_VIEW, 1.0)


func test_cards_fit_on_deck_at_150_percent() -> void:
	await _check_fit(DECK_VIEW, 1.5)


func test_cards_fit_on_desktop() -> void:
	await _check_fit(FLOW_VIEW, 1.0)


func test_cards_fit_on_desktop_at_150_percent() -> void:
	await _check_fit(FLOW_VIEW, 1.5)


## 在世最大的與已覆滅的宗門卡都在畫面內、清單在卡內、不壓到頁尾，所有段落的每一行都放得進內寬（不截字）。
func _check_fit(view: Vector2i, ui_scale: float) -> void:
	GameSettings.set_value("ui_scale", ui_scale)
	await _boot(view)
	var bounds := Vector2(tree.root.get_visible_rect().size)
	var label := "%s 縮放 %s" % [view, ui_scale]
	var seals_top: float = _hud._seals.top_edge()
	var ids: Array[int] = [_biggest(), _sects(false)[0]["id"]]
	for id in ids:
		_hud.open_sect(id)
		await wait(0.5)
		var card := _card()
		var rect := Rect2(card.position, card.size)
		assert_true(Rect2(Vector2.ZERO, bounds).encloses(rect), "宗門卡不出界：%s %s" % [label, rect])
		if card.min_height() <= card.max_height:
			assert_true(rect.end.y <= seals_top - Hud.MARGIN + 0.5, "宗門卡停在印匣之上：%s %s" % [label, rect])
		else:
			print("放不下（只保證不出畫面）：宗門卡 %s" % label)
		var inner: Rect2 = card.list().get_rect()
		assert_true(Rect2(Vector2.ZERO, card.size).encloses(inner), "清單在卡內：" + label)
		assert_true(inner.end.y <= card._fly_rect().position.y, "清單不壓到頁尾動作印：" + label)
		var close: Rect2 = card._close_rect()
		var sub_end: float = card._paper_rect(card._content_height()).position.x + (LedgerCard.STRIP + 14.0) * card._unit + card._inner_width() - card._close_cluster_width(card._close_has_key()) - 10.0 * card._unit - close.size.x
		assert_true(sub_end <= close.position.x, "首列不壓到關閉鈕：" + label)
		var last_end := 0.0
		for block: Dictionary in card._blocks:
			if block["kind"] != "para":
				continue
			var para: Dictionary = block["paragraph"]
			var limit: float = card._inner_width()
			for line: String in para["lines"]:
				assert_true(Ink.text_size(line, para["size"]).x <= limit + 1.0, "每一行放得進內寬：" + label)
			last_end = maxf(last_end, float(block["y"]) + float(para["height"]))
		var paper := card._paper_rect(card._content_height())
		assert_true(inner.position.y >= last_end + 0.0, "清單在固定部分之下：" + label)
		if bounds.y <= 800.0:
			for base: int in [Ink.SIZE_NOTE, Ink.SIZE_BODY, Ink.SIZE_SECTION]:
				assert_gt(Ink.font_size(base, _hud._unit), 15, "Deck 內文至少 16")
		assert_true(paper.size.y > 0.0)
