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
	PersonCard.show_placeholder = false


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
		var count := _sim.world.person_chronicle(id).size()
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
	_card().toggle_timeline()
	await wait(0.2)
	var list := _list(_card())
	assert_gt(list.max_scroll(), 0.0, "Steam Deck 上展開年表放不下，要捲動")
	var rect := list.get_global_rect()
	_camera.set_cursor_position(Vector2(rect.get_center().x, rect.end.y - 6.0))
	await wait(0.6)
	assert_gt(list.scroll, 0.0, "游標停在清單下緣，清單自己往下捲")


func test_card_shows_the_biography_and_a_collapsed_timeline() -> void:
	await _boot(FLOW_VIEW)
	var found := _find_people()
	for key_name in ["alive", "dead"]:
		var id: int = found[key_name]
		_hud.open_person(id)
		await wait(0.5)
		var card := _card()
		var out := _compose(id)
		var bios := _items(card, "bio")
		assert_eq(bios.size(), out["bio"].size(), "小傳的段數與組合結果相同")
		assert_eq("".join(bios[0]["paragraph"]["lines"]), out["bio"][0]["text"], "第一段是出身")
		assert_true(out["bio"][0]["text"].begins_with(_sim.world.person(id)["name"]), "出身以姓名起句")
		assert_eq(_items(card, "toggle").size(), 1, "有一列「年表」")
		assert_eq(_items(card, "row").size(), 0, "年表預設收合")
		assert_true(String(_items(card, "toggle")[0]["label"]).contains(str(out["rows"].size())), "年表列寫明條數")
		var last_top := -1.0
		for row: Dictionary in card._rows:
			assert_gt(row["top"], last_top, "由上往下排")
			last_top = row["top"]
		assert_eq(card._title(), _sim.world.person(id)["name"], "題簽寫姓名")
		var identity := PersonText.identity_line(_sim.world.person(id))
		assert_eq("".join(card._subtitle["lines"]), identity, "首列是身分一行")


func _compose(id: int) -> Dictionary:
	return ChronicleText.compose(_sim.world.person(id), _sim.world.person_chronicle(id), (func(p: int) -> Dictionary: return _sim.world.person(p)), _sim.world.history_years())


func _items(card: PersonCard, kind: String) -> Array:
	return card._rows.filter(func(r: Dictionary) -> bool: return r["kind"] == kind)


## 清單頂端往下 offset 處的螢幕座標。
func _list_point(card: PersonCard, offset: Vector2) -> Vector2:
	return _list(card).get_global_rect().position + offset


func test_clicking_the_timeline_row_expands_and_collapses_it() -> void:
	await _boot(FLOW_VIEW)
	var id: int = _find_people()["alive"]
	_hud.open_person(id)
	await wait(0.5)
	var card := _card()
	var list := _list(card)
	var out := _compose(id)
	list.scroll = float(_items(card, "toggle")[0]["top"])
	await click(_list_point(card, Vector2(30.0, float(_items(card, "toggle")[0]["top"]) - list.scroll + 8.0 * _hud._unit)))
	assert_eq(_items(card, "row").size(), out["rows"].size(), "點「年表」展開，每條一列")
	assert_true(String(_items(card, "toggle")[0]["label"]).contains(Locale.t("card_timeline_close").substr(0, 2)))
	var tops: Array = _items(card, "row").map(func(r: Dictionary) -> float: return r["top"])
	var sorted_tops := tops.duplicate()
	sorted_tops.sort()
	assert_eq(tops, sorted_tops, "年表由舊到新")
	assert_eq(_items(card, "row")[0]["date"], out["rows"][0]["date"], "第一條是最舊的")
	list.scroll = float(_items(card, "toggle")[0]["top"])
	await click(_list_point(card, Vector2(30.0, float(_items(card, "toggle")[0]["top"]) - list.scroll + 8.0 * _hud._unit)))
	assert_eq(_items(card, "row").size(), 0, "再點一次收合")
	# 換人收合。
	card.toggle_timeline()
	_hud.open_person(_find_people()["dead"])
	await wait(0.5)
	assert_eq(_items(_card(), "row").size(), 0, "開另一個人時年表是收合的")


func test_timeline_row_opens_the_event_card() -> void:
	await _boot(FLOW_VIEW)
	var id: int = _find_people()["alive"]
	_hud.open_person(id)
	await wait(0.5)
	var card := _card()
	card.toggle_timeline()
	await wait(0.2)
	var list := _list(card)
	var chosen := {}
	for row: Dictionary in _items(card, "row"):
		if row["has_event"] and row["paragraph"]["hits"].is_empty():
			chosen = row
			break
	assert_false(chosen.is_empty(), "有一條對得到事件、又沒有連結的年表列")
	list.scroll = float(chosen["top"])
	var date_h := LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, _hud._unit))
	await click(_list_point(card, Vector2(4.0, date_h * 0.5)))
	assert_true(_hud.event_card().is_open(), "點整列開事件卡")
	assert_eq(_hud.event_card().event, int(chosen["event"]))


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
		line += "".join(para["lines"]) + "\n"
	assert_true(line.contains(Locale.t("death_" + info["death_cause"])), "寫明死因")
	assert_true(line.contains("享年 %d" % info["age"]), "寫明享年")
	var bios := _items(_card(), "bio")
	var last_bio := "".join(bios[-1]["paragraph"]["lines"])
	assert_true(last_bio.contains("享年" + ChronicleText.han(int(info["age"]))), "小傳最後寫明享年")
	assert_false(_items(_card(), "toggle").is_empty())


func test_clicking_a_name_in_the_biography_opens_that_person_and_back_returns() -> void:
	await _boot(FLOW_VIEW)
	var owner := -1
	var id := 0
	# 找一位有前世的人：小傳第一段點得到前世的那個人。
	while owner < 0 and not _sim.world.person(id).is_empty():
		var facts: Array = _sim.world.person_chronicle(id)
		if not facts.is_empty() and facts[0]["kind"] == "past_life":
			owner = id
		id += 1
	assert_gt(owner, -1, "有前世的人")
	_hud.open_person(owner)
	await wait(0.5)
	var card := _card()
	var para: Dictionary = _items(card, "bio")[0]["paragraph"]
	var hit_target := ""
	var hit_point := Vector2.ZERO
	for hit: Dictionary in para["hits"]:
		if String(hit["target"]).begins_with("person:"):
			hit_target = hit["target"]
			hit_point = hit["rect"].get_center()
			break
	assert_ne(hit_target, "", "出身與前世一段有可點的人名")
	await click(_list_point(card, hit_point))
	assert_eq(card.person, int(hit_target.substr(7)), "點人名換成那個人的卡")
	assert_true(card.can_back, "可以返回")
	await key(KEY_ESCAPE)
	assert_eq(card.person, owner, "Esc 回到上一位")
	assert_false(card.can_back, "最前面一張沒有返回")


func test_clicking_a_place_in_the_biography_flies_and_opens_the_province_card() -> void:
	await _boot(FLOW_VIEW)
	var found := _find_people()
	_hud.open_person(found["alive"])
	await wait(0.5)
	var card := _card()
	var hit_point := Vector2.ZERO
	var hit_target := ""
	for hit: Dictionary in _items(card, "bio")[0]["paragraph"]["hits"]:
		if String(hit["target"]).begins_with("province:"):
			hit_target = hit["target"]
			hit_point = hit["rect"].get_center()
			break
	assert_ne(hit_target, "", "出身一段有可點的籍貫")
	await click(_list_point(card, hit_point))
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
		assert_true(inner.position.y >= card._paragraph_origin(last_para).y + card._paragraphs[last_para]["height"] - 1.0, "小傳清單在基本資料之下：" + label)
		# 首列的文字不碰右上的關閉鈕。
		var close: Rect2 = card._close_rect()
		var sub_end: float = card._paper_rect(card._content_height()).position.x + (LedgerCard.STRIP + 14.0) * card._unit + card._inner_width() - card._close_cluster_width(card._close_has_key()) - 10.0 * card._unit - close.size.x
		assert_true(sub_end <= close.position.x, "首列不壓到關閉鈕：" + label)
		if bounds.y <= 800.0:
			for base: int in [Ink.SIZE_NOTE, Ink.SIZE_BODY, Ink.SIZE_SECTION]:
				assert_gt(Ink.font_size(base, _hud._unit), 15, "Deck 內文至少 16")


func test_timeline_dates_are_double_dated() -> void:
	await _boot(FLOW_VIEW)
	var found := _find_people()
	for key_name in ["alive", "dead"]:
		var id: int = found[key_name]
		_hud.open_person(id)
		await wait(0.5)
		var card := _card()
		card.toggle_timeline()
		await wait(0.2)
		var rows := _items(card, "row")
		assert_false(rows.is_empty())
		for row: Dictionary in rows:
			assert_false(String(row["text"]).contains("{"), "沒有留下未填的模板：" + row["text"])
			assert_true(String(row["date"]).ends_with("月"), "年月到月：" + row["date"])
			assert_true(String(row["date"]).contains("紀元") and String(row["date"]).contains("年（"), "年月雙表：" + row["date"])
		card.toggle_timeline()


func test_history_scrolls_with_the_wheel_to_the_end() -> void:
	await _boot(DECK_VIEW)
	var found := _find_people()
	_hud.open_person(found["alive"])
	await wait(0.5)
	_card().toggle_timeline()
	await wait(0.2)
	var list := _list(_card())
	list.scroll = 0.0
	assert_gt(list.max_scroll(), 0.0)
	var center := list.get_global_rect().get_center()
	for i in 80:
		await wheel(center, false)
		if list.scroll >= list.max_scroll():
			break
	assert_eq(list.scroll, list.max_scroll(), "滾輪捲到底，最後一筆在眼前")


func test_history_folds_the_echo_of_one_happening() -> void:
	# 渡劫成功後緊接的突破、成婚同時結下的情緣，在歷程裡各是同一件事，只留一筆。
	var history := [
		{"kind": "Married", "tick": 30, "a_person": 7, "b_person": 9, "n": 0},
		{"kind": "ThreadCreated", "tick": 30, "a_person": 9, "b_person": 7, "n": 0},
		{"kind": "ThreadCreated", "tick": 31, "a_person": 7, "b_person": 11, "n": 0},
		{"kind": "Breakthrough", "tick": 40, "a_person": 7, "b_person": -1, "n": 1},
		{"kind": "Tribulation", "tick": 90, "a_person": 7, "b_person": -1, "n": 2},
		{"kind": "Breakthrough", "tick": 90, "a_person": 7, "b_person": -1, "n": 2},
	]
	var kinds: Array[String] = []
	for entry: Dictionary in EventText.fold_history(history):
		kinds.append("%s@%d" % [entry["kind"], entry["tick"]])
	assert_eq(kinds, ["Married@30", "ThreadCreated@31", "Breakthrough@40", "Tribulation@90"] as Array[String], "歷程去掉重複的那一筆")


func test_placeholder_portrait_stays_hidden_by_default() -> void:
	PersonCard.show_placeholder = false
	await _boot(FLOW_VIEW)
	_hud.open_person(_find_people()["alive"])
	await wait(0.5)
	var card := _card()
	assert_true(PartCatalog.load_class(PortraitRecipe.CLASS_ID).placeholder, "現在的頭像部件是佔位的")
	assert_false(card._portrait.visible, "佔位頭像不給玩家看")
	assert_eq(card._side(), 0.0, "基本資料滿寬")
	assert_eq(card.size.x, LedgerCard.WIDTH * card._unit, "卡寬回到沒有頭像的寬度")


func test_portrait_sits_top_left_at_160_by_200_and_the_first_rows_move_right() -> void:
	PersonCard.show_placeholder = true
	await _boot(FLOW_VIEW)
	var found := _find_people()
	for key_name in ["alive", "dead"]:
		var id: int = found[key_name]
		_hud.open_person(id)
		await wait(0.5)
		var card := _card()
		var unit: float = card._unit
		var frame := card._portrait_rect()
		assert_eq(frame.size, Vector2(160.0, 200.0) * unit, "頭像 160×200 乘介面縮放")
		assert_true(card._portrait.visible, "展開後顯示頭像")
		assert_eq(card._portrait.position, frame.position)
		assert_lt(absf(frame.position.x - (card._paper_rect(card._content_height()).position.x + (LedgerCard.STRIP + 14.0) * unit)), 0.01, "靠左，在題簽右邊")
		assert_lt(absf(frame.position.y - (card._paper_rect(card._content_height()).position.y + 14.0 * unit)), 0.01, "在紙面頂端")
		assert_gt(card._layout["para_x"][0], frame.size.x, "首列以下的基本資料排在頭像右側")
		for x: float in card._layout["para_x"]:
			assert_true(x == 0.0 or x == card._side(), "段落不是排在頭像右側就是滿寬")
		assert_true(card._layout["section_y"] >= card._layout["portrait_bottom"], "歷程標題在頭像之下")
		var info: Dictionary = _sim.world.person(id)
		assert_eq(card._portrait.recipe(), PortraitRecipe.from_person(info), "頭像是這個人的資料推出的那一串")
		assert_eq(float(card._portrait.recipe()["grade"]["fade"]) > 0.0, not info["alive"], "身故者褪成淡墨")
		assert_true(Rect2(Vector2.ZERO, Vector2(tree.root.get_visible_rect().size)).encloses(Rect2(card.position, card.size)), "卡不出畫面")
		assert_true(card.size.y <= card.max_height + 0.5, "卡高不超過上限")


func test_portrait_steps_aside_when_the_card_cannot_hold_it() -> void:
	PersonCard.show_placeholder = true
	GameSettings.set_value("ui_scale", 1.5)
	await _boot(DECK_VIEW)
	var id: int = _find_people()["alive"]
	_hud.open_person(id)
	await wait(0.5)
	var card := _card()
	assert_true(card._portrait_k < 1.0, "Steam Deck 縮放 150% 放不下，頭像縮小或收起")
	assert_true(card.min_height() > card.max_height or card._portrait_k > 0.0, "已經收起或仍放得下")
