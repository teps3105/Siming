extends "res://tests/harness/suite.gd"
## 人物卡（GDD 5.8）：州郡卡列出在此的有名之人，點一位開人物卡，歷程由舊到新；Esc／B 返回州郡卡；
## 鍵鼠與搖桿各走一遍；Steam Deck 與 150% 縮放排得下；已身故者仍可開卡。

const DECK_VIEW := Vector2i(1280, 800)

var _game: Node
var _sim: SimHost
var _hud: Hud
var _camera: MapCamera


func suite_name() -> String:
	return "person_card"


func setup() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func teardown() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func _boot(view: Vector2i) -> void:
	_game = await boot(false, view)
	_sim = _game.get_node("SimHost")
	_hud = _game.get_node("Hud")
	_camera = _game.get_node("MapCamera")
	# 多走幾年，讓人物有歷程、也有人身故。
	_sim.world.step(12 * 40)
	_sim.world.flush_terrain()


## 在世人數最多的州郡。
func _busiest_province() -> int:
	var best := -1
	var most := 0
	for id in _sim.world.province_count():
		var count := _sim.world.people_in(id).size()
		if count > most:
			most = count
			best = id
	return best


## 歷程最長的在世者與已歿者（走過全部人物）。
func _find_people() -> Dictionary:
	var out := {"alive": -1, "dead": -1}
	var most := {"alive": -1, "dead": -1}
	var id := 0
	while not _sim.world.person(id).is_empty():
		var key := "alive" if _sim.world.person(id)["alive"] else "dead"
		var count := _sim.world.person_history(id).size()
		if count > most[key]:
			most[key] = count
			out[key] = id
		id += 1
	return out


func _card() -> PersonCard:
	return _hud.person_card()


func _list(card: LedgerCard) -> LedgerList:
	return card._list


## 清單第 index 列的螢幕中心。
func _people_row_center(card: ProvinceCard, index: int) -> Vector2:
	var list := _list(card)
	return list.get_global_rect().position + Vector2(40.0 * _hud._unit, (index + 0.5) * ProvinceCard.PERSON_ROW * _hud._unit)


func test_province_card_lists_people_and_opens_the_person_card() -> void:
	await _boot(FLOW_VIEW)
	var province := _busiest_province()
	assert_gt(province, -1, "四十年後有州郡住著有名之人")
	var expected: Array = _sim.world.people_in(province)
	_hud._card.show_province(province)
	await wait(0.5)
	assert_true(_list(_hud._card).visible, "州郡卡有「在此的有名之人」清單")
	assert_eq(_hud._card._people.size(), mini(expected.size(), ProvinceCard.PEOPLE_LIMIT), "最多列 10 位")
	assert_eq(_hud._card._people[0]["id"], expected[0]["id"], "依境界由高到低")
	await click(_people_row_center(_hud._card, 0))
	assert_true(_card().is_open(), "點一位開人物卡")
	assert_eq(_card().person, expected[0]["id"])
	assert_false(_hud._card.is_open(), "一次只開一張命簿")
	assert_true(_card().can_back, "從州郡卡進來，卡上有返回")
	await wait(0.5)
	assert_lt(_camera._target_focus.distance_to(_sim.world.person(expected[0]["id"])["pos"]), 0.01, "鏡頭飛往此人所在")
	await key(KEY_ESCAPE)
	assert_false(_card().is_open(), "Esc 離開人物卡")
	assert_true(_hud._card.is_open(), "回到州郡卡")
	assert_eq(_hud._card.province, province)
	await key(KEY_ESCAPE)
	assert_false(_hud._card.is_open(), "再按 Esc 收起州郡卡")
	assert_false(_hud.menu_open(), "收卡的 Esc 不開選單")


func test_gamepad_opens_the_person_card_and_b_goes_back() -> void:
	await _boot(FLOW_VIEW)
	var province := _busiest_province()
	_hud._card.show_province(province)
	await wait(0.5)
	await joy_button(JOY_BUTTON_LEFT_STICK)
	await frames(2)
	assert_true(_hud._card.gamepad, "搖桿模式切過去")
	_camera.set_cursor_position(_people_row_center(_hud._card, 0))
	await frames(2)
	await joy_axis(JOY_AXIS_TRIGGER_LEFT, 1.0, 0.05)
	assert_true(_card().is_open(), "游標移到一位按 LT 開人物卡")
	assert_true(_card().gamepad)
	assert_eq(_card()._back_key(), "B", "返回鍵畫搖桿的 B")
	await joy_button(JOY_BUTTON_B)
	assert_true(_hud._card.is_open() and not _card().is_open(), "B 返回州郡卡")
	await joy_button(JOY_BUTTON_B)
	assert_false(_hud._card.is_open(), "B 再收起州郡卡")
	assert_false(_hud.menu_open(), "搖桿的 B 不開選單")


func test_gamepad_edge_hover_scrolls_the_history() -> void:
	await _boot(DECK_VIEW)
	var found := _find_people()
	await joy_button(JOY_BUTTON_LEFT_STICK)
	_hud.open_person(found["alive"])
	await wait(0.5)
	var list := _list(_card())
	assert_gt(list.max_scroll(), 0.0, "Steam Deck 上歷程放不下，要捲動")
	var rect := list.get_global_rect()
	_camera.set_cursor_position(Vector2(rect.get_center().x, rect.end.y - 6.0))
	await wait(0.6)
	assert_gt(list.scroll, 0.0, "游標停在清單下緣，清單自己往下捲")


func test_history_is_oldest_first_and_matches_the_bridge() -> void:
	await _boot(FLOW_VIEW)
	var found := _find_people()
	for key_name in ["alive", "dead"]:
		var id: int = found[key_name]
		_hud.open_person(id)
		await wait(0.5)
		var card := _card()
		var history: Array = _sim.world.person_history(id)
		assert_gt(history.size(), 4, "找到歷程 5 筆以上的人")
		assert_eq(card._rows.size(), history.size(), "歷程筆數與事件帳相符")
		var last_top := -1.0
		for row: Dictionary in card._rows:
			assert_gt(row["top"], last_top, "由舊到新排下來")
			last_top = row["top"]
		assert_eq(card._rows[0]["paragraph"]["lines"].size() > 0, true)
		assert_eq(card._title(), _sim.world.person(id)["name"], "題簽寫姓名")
		var identity := PersonText.identity_line(_sim.world.person(id))
		assert_eq("".join(card._subtitle["lines"]), identity, "首列是身分一行")


func test_dead_person_card_shows_cause_and_age_at_death() -> void:
	await _boot(FLOW_VIEW)
	var dead: int = _find_people()["dead"]
	assert_gt(dead, -1, "四十年後有人身故")
	_hud.open_person(dead)
	await wait(0.5)
	var info := _sim.world.person(dead)
	assert_false(info["alive"])
	assert_true(_card().is_open(), "已身故者仍可開卡")
	var line := ""
	for para: Dictionary in _card()._paragraphs:
		line += "".join(para["lines"]) + "
"
	assert_true(line.contains(Locale.t("death_" + info["death_cause"])), "寫明死因")
	assert_true(line.contains("享年 %d" % info["age"]), "寫明享年")
	for row: Dictionary in _card()._rows:
		assert_true(_sim.world.person_history(dead).size() == _card()._rows.size())
	var last: Dictionary = _sim.world.person_history(dead)[-1]
	assert_true(last["tick"] <= info["died_tick"], "歷程到死為止")


func test_clicking_a_name_in_the_history_opens_that_person_and_back_returns() -> void:
	await _boot(FLOW_VIEW)
	var target_row := {}
	var owner := -1
	# 找一位歷程前幾筆裡就有可點人名的人。
	var id := 0
	while owner < 0 and not _sim.world.person(id).is_empty():
		var history: Array = _sim.world.person_history(id)
		for i in mini(history.size(), 6):
			var entry: Dictionary = history[i]
			if int(entry["a_person"]) >= 0 and int(entry["a_person"]) != id or int(entry["b_person"]) >= 0 and int(entry["b_person"]) != id:
				owner = id
				break
		id += 1
	assert_gt(owner, -1, "有歷程提到別人的人")
	_hud.open_person(owner)
	await wait(0.5)
	var card := _card()
	var list := _list(card)
	var hit_target := ""
	var hit_point := Vector2.ZERO
	for row: Dictionary in card._rows:
		var base := Vector2(0.0, float(row["top"]) + LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, _hud._unit)))
		for hit: Dictionary in row["paragraph"]["hits"]:
			if String(hit["target"]).begins_with("person:") and base.y + hit["rect"].end.y < list.size.y:
				hit_target = hit["target"]
				hit_point = base + hit["rect"].get_center()
				break
		if not hit_target.is_empty():
			break
	assert_ne(hit_target, "", "最前面的幾筆有可點的人名")
	await click(list.get_global_rect().position + hit_point)
	assert_eq(card.person, int(hit_target.substr(7)), "點人名換成那個人的卡")
	assert_true(card.can_back, "可以返回")
	await key(KEY_ESCAPE)
	assert_eq(card.person, owner, "Esc 回到上一位")
	assert_false(card.can_back, "最前面一張沒有返回")


func test_clicking_a_place_in_the_history_flies_and_opens_the_province_card() -> void:
	await _boot(FLOW_VIEW)
	var found := _find_people()
	_hud.open_person(found["alive"])
	await wait(0.5)
	var card := _card()
	var list := _list(card)
	var hit_point := Vector2.ZERO
	var hit_target := ""
	for row: Dictionary in card._rows:
		var base := Vector2(0.0, float(row["top"]) + LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, _hud._unit)))
		for hit: Dictionary in row["paragraph"]["hits"]:
			if String(hit["target"]).begins_with("province:") and base.y + hit["rect"].end.y < list.size.y:
				hit_target = hit["target"]
				hit_point = base + hit["rect"].get_center()
				break
		if not hit_target.is_empty():
			break
	assert_ne(hit_target, "", "歷程裡有可點的地名")
	await click(list.get_global_rect().position + hit_point)
	assert_true(_hud._card.is_open(), "點地名開州郡卡")
	assert_eq(_hud._card.province, int(hit_target.substr(9)))
	assert_false(card.is_open())
	await key(KEY_ESCAPE)
	assert_true(card.is_open(), "Esc 回到人物卡")


func test_basic_info_places_are_clickable() -> void:
	await _boot(FLOW_VIEW)
	var found := _find_people()
	_hud.open_person(found["alive"])
	await wait(0.5)
	var card := _card()
	var hits: Array = card._paragraphs[1]["hits"]
	assert_gt(hits.size(), 1, "所在與故里都可點")
	var point: Vector2 = card._paragraph_origin(1) + hits[0]["rect"].get_center()
	await click(card.global_position + point)
	assert_true(_hud._card.is_open(), "點所在州郡開州郡卡")


func test_close_button_closes_everything() -> void:
	await _boot(FLOW_VIEW)
	var province := _busiest_province()
	_hud._card.show_province(province)
	await wait(0.5)
	await click(_people_row_center(_hud._card, 0))
	await wait(0.5)
	var close: Rect2 = _card()._close_rect()
	await click(_card().global_position + close.get_center())
	assert_false(_card().is_open())
	assert_false(_hud._card.is_open(), "關閉鈕收起全部，不回州郡卡")
	assert_true(_hud._back_stack.is_empty())


func test_cards_fit_on_deck() -> void:
	await _check_fit(DECK_VIEW, 1.0)


func test_cards_fit_on_deck_at_150_percent() -> void:
	await _check_fit(DECK_VIEW, 1.5)


func test_cards_fit_on_desktop_at_150_percent() -> void:
	await _check_fit(FLOW_VIEW, 1.5)


## 州郡卡與兩種人物卡都在畫面內（任何組合都檢查，這是底線）；清單最低高度放得下的組合還要停在印匣之上，
## 並把多出的高度給清單；歷程清單在卡內、不壓到基本資料與頁尾動作印。
## 放不下的組合：Steam Deck（1280×800）縮放 150% 的人物卡——身分、基本資料與魂的來歷在這個縮放下加歷程清單
## 最低高度（120）已高過印匣上方的空間，只能保證不出畫面；州郡卡在這個組合放得下。
func _check_fit(view: Vector2i, ui_scale: float) -> void:
	GameSettings.set_value("ui_scale", ui_scale)
	await _boot(view)
	var bounds := Vector2(tree.root.get_visible_rect().size)
	var label := "%s 縮放 %s" % [view, ui_scale]
	var found := _find_people()
	_hud._card.show_province(_busiest_province())
	await wait(0.5)
	var rect := Rect2(_hud._card.position, _hud._card.size)
	assert_true(Rect2(Vector2.ZERO, bounds).encloses(rect), "州郡卡不出界：%s %s" % [label, rect])
	var seals_top: float = _hud._seals.top_edge()
	if _hud._card.min_height() <= _hud._card.max_height:
		assert_true(rect.end.y <= seals_top - Hud.MARGIN + 0.5, "州郡卡停在印匣之上留一個間距：%s %s" % [label, rect])
		if _hud._card._list_height() < _hud._card._people_content_height():
			assert_gt(rect.end.y, seals_top - Hud.MARGIN - 2.0, "清單放不下時，州郡卡長到印匣上緣之上：" + label)
	else:
		print("放不下（只保證不出畫面）：州郡卡 %s" % label)
	assert_true(Rect2(Vector2.ZERO, _hud._card.size).encloses(_list(_hud._card).get_rect()), "有名之人清單在卡內：" + label)
	assert_true(_list(_hud._card).get_rect().end.y <= _hud._card._center_rect().position.y, "清單不壓到頁尾動作印：" + label)
	for id: int in [found["alive"], found["dead"]]:
		_hud.open_person(id)
		await wait(0.5)
		var card := _card()
		var card_rect := Rect2(card.position, card.size)
		assert_true(Rect2(Vector2.ZERO, bounds).encloses(card_rect), "人物卡不出界：%s %s" % [label, card_rect])
		if card.min_height() <= card.max_height:
			assert_true(card_rect.end.y <= seals_top - Hud.MARGIN + 0.5, "人物卡停在印匣之上留一個間距：%s %s" % [label, card_rect])
			if card._list_height() < card._rows_height:
				assert_gt(card_rect.end.y, seals_top - Hud.MARGIN - 2.0, "歷程放不下時，人物卡長到印匣上緣之上：" + label)
		else:
			print("放不下（只保證不出畫面）：人物卡 %s" % label)
		var inner: Rect2 = _list(card).get_rect()
		assert_true(Rect2(Vector2.ZERO, card.size).encloses(inner), "歷程清單在卡內：" + label)
		assert_true(inner.end.y <= card._center_rect().position.y, "歷程清單不壓到頁尾動作印：" + label)
		var last_para := card._paragraphs.size() - 1
		assert_true(inner.position.y >= card._paragraph_origin(last_para).y + card._paragraphs[last_para]["height"] - 1.0, "歷程清單在基本資料之下：" + label)
		# 首列的文字不碰右上的關閉鈕。
		var close: Rect2 = card._close_rect()
		var sub_end: float = card._paper_rect(card._content_height()).position.x + (LedgerCard.STRIP + 14.0) * card._unit + card._inner_width() - card._close_cluster_width(card._close_has_key()) - 10.0 * card._unit - close.size.x
		assert_true(sub_end <= close.position.x, "首列不壓到關閉鈕：" + label)
		if bounds.y <= 800.0:
			for base: int in [Ink.SIZE_NOTE, Ink.SIZE_BODY, Ink.SIZE_SECTION]:
				assert_gt(Ink.font_size(base, _hud._unit), 15, "Deck 內文至少 16")


func test_history_sentences_drop_the_protagonist_and_date_the_era_once() -> void:
	await _boot(FLOW_VIEW)
	var found := _find_people()
	for key_name in ["alive", "dead"]:
		var id: int = found[key_name]
		var own_name: String = _sim.world.person(id)["name"]
		_hud.open_person(id)
		await wait(0.5)
		var card := _card()
		var history: Array = _sim.world.person_history(id)
		var eras := 0
		var previous_era := ""
		var with_era := 0
		for i in history.size():
			var entry: Dictionary = history[i]
			var sentence := EventText.history_line(entry, id)
			assert_false(sentence.contains("{"), "沒有留下未填的模板：" + sentence)
			assert_false(sentence.begins_with(own_name), "歷程不以主角自己的名字起句：" + sentence)
			assert_false(sentence.contains("（") and entry["kind"] in ["Tribulation", "Breakthrough"], "歷程沒有境界括號：" + sentence)
			if entry["era"] != previous_era:
				eras += 1
				previous_era = entry["era"]
			if String(card._rows[i]["date"]).begins_with(String(entry["era"])):
				with_era += 1
			if entry["kind"] == "Tribulation" and entry["x"] == "Passed":
				assert_true(sentence.contains(EventText.gain(int(entry["n"]))), "渡劫寫出成就的境界：" + sentence)
		assert_eq(with_era, eras, "紀元名只在第一筆與換紀元時出現（%d 次）" % eras)
		assert_true(String(card._rows[0]["date"]).begins_with(history[0]["era"]), "第一筆帶紀元名")


func test_history_scrolls_with_the_wheel_to_the_end() -> void:
	await _boot(DECK_VIEW)
	var found := _find_people()
	_hud.open_person(found["alive"])
	await wait(0.5)
	var list := _list(_card())
	assert_gt(list.max_scroll(), 0.0)
	assert_eq(list.scroll, 0.0, "開卡從最舊的一筆起")
	var center := list.get_global_rect().get_center()
	for i in 80:
		await wheel(center, false)
		if list.scroll >= list.max_scroll():
			break
	assert_eq(list.scroll, list.max_scroll(), "滾輪捲到底，最後一筆在眼前")
