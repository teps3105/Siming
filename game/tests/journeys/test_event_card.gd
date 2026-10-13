extends "res://tests/harness/suite.gd"
## 事件卡／溯因視圖（GDD 5.8）：從傳聞簽的「溯因」小印、人物歷程的一筆、X 鍵打開；因樹與漣漪；
## 節點點了換卡、人名開人物卡、返回堆疊走得回去；已修剪的事件寫「此事已不可考」；
## 兩種解析度與 150% 縮放不出畫面；鍵鼠與搖桿都走得完「傳聞→事件卡→因樹上一層→人物卡→返回」。

const DECK_VIEW := Vector2i(1280, 800)

var _game: Node
var _sim: SimHost
var _hud: Hud
var _camera: MapCamera
var _rumor: RumorSlip
## 開局後走了幾年的事件流（編號 → 事件）。
var _stream := {}


func suite_name() -> String:
	return "event_card"


func setup() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func teardown() -> void:
	GameSettings.set_value("ui_scale", 1.0)


func _boot(view: Vector2i) -> void:
	_game = await boot(false, view)
	_sim = _game.get_node("SimHost")
	_hud = _game.get_node("Hud")
	_camera = _game.get_node("MapCamera")
	_rumor = _hud.rumor()
	_sim.world.step(12 * 30)
	_stream.clear()
	for event: Dictionary in _sim.world.drain_events():
		_stream[event["id"]] = event
	_sim.world.flush_terrain()


func _card() -> EventCard:
	return _hud.event_card()


## 因樹有兩層以上、有地點、單則不併入統計的事件（照事件編號小的先找，確定性）。
func _deep_event() -> Dictionary:
	var ids: Array = _stream.keys()
	ids.sort()
	for id: int in ids:
		var event: Dictionary = _stream[id]
		if int(event["importance"]) < 3 or not event.has("pos") or not EventText.category(event).is_empty():
			continue
		# 世界每次開出的演變都不同：要選有人物可開人物卡的事件。
		if int(event["a_person"]) < 0 and (event["persons"] as PackedInt32Array).is_empty():
			continue
		for node: Dictionary in _sim.world.event_causes(id, 6):
			if int(node["level"]) >= 2:
				return event
	return {}


## 傳聞簽裡這則事件的簽序；沒有為 -1。
func _slip_index(id: int) -> int:
	var shown := _rumor.shown()
	for i in shown.size():
		if int(shown[i]["id"]) == id:
			return i
	return -1


func _trace_center(index: int) -> Vector2:
	return _rumor.position + _rumor.action_slots(index)["trace"].get_center()


## 事件卡清單裡第 n 個事件列（kind == event）的中心（全域座標），落在不是連結的位置：取日期那行。
func _event_row_point(card: EventCard, nth: int) -> Vector2:
	var list: LedgerList = card._list
	var count := 0
	for row: Dictionary in card._rows:
		if row["kind"] != "event":
			continue
		if count == nth:
			var y: float = row["top"] + 3.0 - list.scroll
			return list.get_global_rect().position + Vector2(float(row["indent"]) + 6.0, y)
		count += 1
	return Vector2(-1.0, -1.0)


func test_spec_five_eight_trace_stamp_opens_the_event_card() -> void:
	await _boot(FLOW_VIEW)
	var event := _deep_event()
	assert_false(event.is_empty(), "三十年內有因樹兩層以上的事件")
	_sim.event_posted.emit(event)
	await frames(2)
	var index := _slip_index(int(event["id"]))
	assert_gt(index, -1, "事件入簽")
	assert_true(_rumor.has_trace_slot(index), "單則有溯因入口")
	_rumor.expanded = true
	await frames(2)
	await click(_trace_center(index))
	assert_true(_card().is_open(), "點溯因小印開事件卡")
	assert_eq(_card().event, int(event["id"]))
	assert_false(_card().is_gone())
	assert_eq(_card().cause_count(), EventText.fold_tree(_sim.world.event_causes(int(event["id"]), 6), [_sim.world.event(int(event["id"]))]).size(), "因樹節點數與橋接折疊後相同")
	assert_eq(_card().ripple_count(), EventText.fold_history(_sim.world.event_ripples(int(event["id"]))).size(), "漣漪數與橋接折疊後相同")
	assert_eq(_card()._title(), "溯因")
	assert_true(_card().size.y <= _hud._last_card_limit + 0.5, "卡不高過印匣上緣之上")
	assert_false(_rumor.expanded, "開卡即收合全螢幕傳聞")
	assert_eq(_hud._open_card(), _card(), "一次只開一張")
	await key(KEY_ESCAPE)
	assert_false(_card().is_open(), "Esc 直接關卡（全螢幕已收合）")
	assert_false(_hud.menu_open(), "收卡的 Esc 不開選單")


func test_statistic_rows_have_no_trace_but_each_member_does() -> void:
	await _boot(FLOW_VIEW)
	var members: Array[Dictionary] = []
	for event in _stream.values():
		if EventText.category(event) == "Died" and int(event["importance"]) >= 2:
			members.append(event)
		if members.size() >= 3:
			break
	assert_eq(members.size(), 3, "找得到三則例行身死")
	var tick: int = members[0]["tick"]
	for m in members:
		var copy: Dictionary = m.duplicate()
		copy["tick"] = tick
		_sim.event_posted.emit(copy)
	_rumor.expanded = true
	await frames(2)
	var group := -1
	for i in _rumor.shown().size():
		if _rumor.is_group(_rumor.shown()[i]):
			group = i
	assert_gt(group, -1)
	assert_false(_rumor.has_trace_slot(group), "合成的統計沒有單一事件，溯因印不出現")
	assert_true(_rumor.is_open(group), "全螢幕的統計直接攤開名單")
	var member_events: Array = _rumor.shown()[group]["members"]
	var slots := _rumor.member_slots(group, 0)
	var rect := _rumor.member_rect(group, 0)
	assert_true(rect.encloses(slots["trace"]), "名單每一行有溯因印，且在行內")
	await click(_rumor.position + slots["trace"].get_center())
	assert_true(_card().is_open())
	assert_eq(_card().event, int(member_events[0]["id"]), "名單一行的溯因開那一人的事件")


func test_cause_node_replaces_the_card_and_back_returns() -> void:
	await _boot(FLOW_VIEW)
	var event := _deep_event()
	_hud.open_event(int(event["id"]))
	await wait(0.5)
	# 卡上的因樹套了人物歷程的折疊規則，所以以卡上第一個事件列為準，不用原始因樹的第一個直接因。
	var direct_id := -1
	for row: Dictionary in _card()._rows:
		if row["kind"] == "event":
			direct_id = int(row["event"])
			break
	assert_gt(direct_id, -1)
	assert_ne(direct_id, int(event["id"]))
	# 點清單的第一個事件列。
	await click(_event_row_point(_card(), 0))
	assert_eq(_card().event, direct_id, "點節點把卡換成那一則事件的事件卡")
	assert_true(_card().can_back, "換卡後卡上有返回")
	assert_eq(_hud._back_stack.size(), 1)
	await key(KEY_ESCAPE)
	assert_eq(_card().event, int(event["id"]), "Esc 回到原本那一則")
	await key(KEY_ESCAPE)
	assert_false(_card().is_open(), "再按 Esc 收起")


func test_person_name_in_a_node_opens_the_person_card_and_back_returns() -> void:
	await _boot(FLOW_VIEW)
	var event := _deep_event()
	_hud.open_event(int(event["id"]))
	await wait(0.5)
	var link_row := {}
	for row: Dictionary in _card()._rows:
		if row["kind"] == "event":
			for hit: Dictionary in row["paragraph"]["hits"]:
				if String(hit["target"]).begins_with("person:"):
					link_row = {"row": row, "hit": hit}
					break
		if not link_row.is_empty():
			break
	# 主句也有人名連結：優先用主句，因樹節點裡沒有人名才用主句。
	var person_id := -1
	var point := Vector2.ZERO
	if not link_row.is_empty():
		var list: LedgerList = _card()._list
		var row: Dictionary = link_row["row"]
		var rect: Rect2 = link_row["hit"]["rect"]
		var date_height := LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, _hud._unit))
		point = list.get_global_rect().position + Vector2(float(row["indent"]), float(row["top"]) + date_height - list.scroll) + rect.get_center()
		person_id = int(String(link_row["hit"]["target"]).substr(7))
		if not list.get_global_rect().has_point(point):
			list.scroll = float(row["top"])
			await frames(2)
			point = list.get_global_rect().position + Vector2(float(row["indent"]), float(row["top"]) + date_height - list.scroll) + rect.get_center()
	assert_gt(person_id, -1, "因樹裡有人名連結")
	await click(point)
	assert_true(_hud.person_card().is_open(), "點節點裡的人名開人物卡")
	assert_eq(_hud.person_card().person, person_id)
	assert_false(_card().is_open(), "一次只開一張")
	await key(KEY_ESCAPE)
	assert_true(_card().is_open(), "Esc 回到事件卡")
	assert_eq(_card().event, int(event["id"]))


func test_timeline_row_opens_the_event_card_and_links_keep_priority() -> void:
	await _boot(FLOW_VIEW)
	var event := _deep_event()
	var person: int = event["a_person"] if int(event["a_person"]) >= 0 else int(event["persons"][0])
	_hud.open_person(person)
	await wait(0.5)
	var card := _hud.person_card()
	var list: LedgerList = card._list
	card.toggle_timeline()
	await wait(0.2)
	# 年表裡找一條對得到事件的：在行尾空白處點，開那一則的事件卡。
	var row: Dictionary = {}
	for r: Dictionary in card._rows:
		if r["kind"] == "row" and r["has_event"]:
			row = r
			break
	assert_false(row.is_empty(), "年表裡有對得到事件的一條")
	var rows_event: int = row["event"]
	var date_height := LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, _hud._unit))
	list.scroll = float(row["top"])
	await frames(2)
	var point := list.get_global_rect().position + Vector2(list.size.x - 24.0 * _hud._unit, float(row["top"]) - list.scroll + 4.0)
	await click(point)
	assert_true(_card().is_open(), "點年表的一條開事件卡")
	assert_eq(_card().event, rows_event)
	assert_true(_card().can_back)
	await key(KEY_ESCAPE)
	assert_true(card.is_open(), "Esc 回到人物卡")
	# 人名、地名的連結照舊優先：年表某一條的地名連結仍是飛往並開州郡卡。
	var linked := -1
	for i in card._rows.size():
		if card._rows[i]["kind"] != "row":
			continue
		for hit: Dictionary in card._rows[i]["paragraph"]["hits"]:
			if String(hit["target"]).begins_with("province:"):
				linked = i
				break
		if linked >= 0:
			break
	if linked >= 0:
		var lrow: Dictionary = card._rows[linked]
		var hit: Dictionary = lrow["paragraph"]["hits"][0]
		assert_eq(list.finder.call(Vector2((hit["rect"] as Rect2).get_center().x, float(lrow["top"]) + date_height + (hit["rect"] as Rect2).get_center().y)), hit["target"], "連結優先於整列")


func test_x_key_opens_the_last_event_of_the_selected_person() -> void:
	await _boot(FLOW_VIEW)
	var event := _deep_event()
	var person: int = event["a_person"] if int(event["a_person"]) >= 0 else int(event["persons"][0])
	_hud.open_person(person)
	await wait(0.5)
	var last: int = _sim.world.person(person)["last_event"]
	assert_gt(last, -1, "這個人有最近事件")
	await key(KEY_X)
	assert_true(_card().is_open(), "X 開事件卡")
	assert_eq(_card().event, last, "是目前選取人物的最近一則事件")
	assert_false(_hud.person_card().is_open())
	await key(KEY_ESCAPE)
	assert_true(_hud.person_card().is_open(), "Esc 回到人物卡")
	_hud.person_card().close()
	await key(KEY_X)
	assert_false(_card().is_open(), "沒有選取人物時 X 不開卡")


func test_missing_event_opens_a_card_that_says_it_is_unrecorded() -> void:
	await _boot(FLOW_VIEW)
	_hud.open_event(99999999)
	await wait(0.5)
	assert_true(_card().is_open(), "事件已不可考，卡仍開")
	assert_true(_card().is_gone())
	assert_eq(Locale.t("event_gone"), "此事已不可考")
	assert_eq(_card()._gone_text["lines"][0], "此事已不可考", "卡上寫明不可考")
	assert_eq(_card().cause_count(), 0)
	assert_eq(_card().ripple_count(), 0)
	assert_lt(_card().size.y, 260.0, "只有一句話，卡不高")
	await key(KEY_ESCAPE)
	assert_false(_card().is_open())


func test_ripples_expand_into_a_list() -> void:
	await _boot(FLOW_VIEW)
	var chosen := -1
	var ids: Array = _stream.keys()
	ids.sort()
	for id: int in ids:
		if int(_stream[id]["importance"]) >= 2 and _sim.world.event_ripple(id) >= 2:
			chosen = id
			break
	assert_gt(chosen, -1, "有至少兩則後果的事件")
	_hud.open_event(chosen)
	await wait(0.5)
	assert_false(_card().ripple_open())
	var before := _card()._rows.size()
	_card().toggle_ripples()
	assert_true(_card().ripple_open())
	assert_eq(_card()._rows.size(), before + EventText.fold_history(_sim.world.event_ripples(chosen)).size(), "展開後每則後果一列（成婚同時的結緣等回聲折掉）")
	_card().toggle_ripples()
	assert_eq(_card()._rows.size(), before)
	# 沒有後果的事件寫「尚無後果」，不能展開。
	var lone := -1
	for id: int in ids:
		if _sim.world.event_ripple(id) == 0 and int(_stream[id]["importance"]) >= 2:
			lone = id
			break
	assert_gt(lone, -1)
	_hud.open_event(lone)
	await frames(2)
	_card().toggle_ripples()
	assert_false(_card().ripple_open(), "沒有後果就不展開")


func test_card_fits_both_resolutions_and_scaling() -> void:
	for view in [FLOW_VIEW, DECK_VIEW]:
		for scale in [1.0, 1.5]:
			GameSettings.set_value("ui_scale", scale)
			await _boot(view)
			var event := _deep_event()
			_hud.open_event(int(event["id"]))
			await wait(0.5)
			var card := _card()
			var bounds := Rect2(Vector2.ZERO, Vector2(view))
			var rect := Rect2(card.position, card.size)
			var label := "%s 縮放 %s" % [view, scale]
			assert_true(bounds.encloses(rect), "%s：卡在畫面內" % label)
			assert_true(rect.end.y <= _hud._seals.top_edge() - Hud.MARGIN + 0.5 or card.min_height() > card.max_height, "%s：卡停在印匣之上留一個間距（放不下才只保證最低高度）" % label)
			_game.queue_free()
			await frames(2)
	GameSettings.set_value("ui_scale", 1.0)


func test_gamepad_walks_slip_to_card_to_cause_to_person_and_back() -> void:
	await _boot(FLOW_VIEW)
	var event := _deep_event()
	_sim.event_posted.emit(event)
	await joy_button(JOY_BUTTON_LEFT_STICK)
	await frames(2)
	var index := _slip_index(int(event["id"]))
	_rumor.expanded = true
	await frames(2)
	_camera.set_cursor_position(_trace_center(index))
	await frames(2)
	await joy_axis(JOY_AXIS_TRIGGER_LEFT, 1.0, 0.05)
	assert_true(_card().is_open(), "游標移到溯因印按 LT 開事件卡")
	assert_true(_card().gamepad)
	assert_false(_rumor.expanded, "開卡即收合全螢幕傳聞")
	assert_eq(_card()._back_key(), "B")
	await wait(0.5)
	_camera.set_cursor_position(_event_row_point(_card(), 0))
	await frames(2)
	await joy_axis(JOY_AXIS_TRIGGER_LEFT, 1.0, 0.05)
	assert_ne(_card().event, int(event["id"]), "按 LT 點因樹節點換卡（上一層）")
	await joy_button(JOY_BUTTON_B)
	assert_eq(_card().event, int(event["id"]), "B 回到上一張")
	_hud.open_person(int(event["a_person"]) if int(event["a_person"]) >= 0 else int(event["persons"][0]))
	await frames(2)
	assert_true(_hud.person_card().is_open())
	await joy_button(JOY_BUTTON_B)
	assert_true(_card().is_open(), "B 從人物卡回到事件卡")
	await joy_button(JOY_BUTTON_B)
	assert_false(_card().is_open(), "B 再收起")
	assert_false(_hud.menu_open())


func _node(id: int, kind: String, tick: int, a: int, b: int, level: int, parent: int, n := 0) -> Dictionary:
	return {"id": id, "kind": kind, "tick": tick, "a_person": a, "b_person": b, "level": level, "parent": parent, "n": n}


func test_cause_tree_folds_echoes_and_reattaches_children() -> void:
	# 成婚（層 1）底下的結緣（層 2，同月同一對，是成婚的回聲）；結緣的因（層 3）要接到成婚底下。
	var nodes := [
		_node(1, "Married", 10, 1, 2, 1, -1),
		_node(2, "ThreadCreated", 10, 1, 2, 2, 0),
		_node(3, "Born", 1, 1, -1, 3, 1),
		_node(4, "Born", 1, 2, -1, 2, 0),
	]
	var folded := EventText.fold_tree(nodes)
	assert_eq(folded.size(), 3, "成婚同時結下的宿緣不另列")
	assert_eq(folded[0]["kind"], "Married")
	assert_eq(folded[1]["id"], 3, "被去掉那一筆底下的因留著")
	assert_eq(folded[1]["parent"], 0, "接到上一層（成婚）")
	assert_eq(folded[1]["level"], 2, "層級跟著往上一層")
	assert_eq(folded[2]["parent"], 0)
	assert_eq(nodes.size(), 4, "不改原陣列")
	# 渡劫成功後緊接的突破（新境界 ≥ 金丹）也去掉；沒有渡劫的突破留著。
	var tribulation := [
		_node(1, "Tribulation", 20, 5, -1, 1, -1),
		_node(2, "Breakthrough", 20, 5, -1, 2, 0, 2),
		_node(3, "Breakthrough", 30, 6, -1, 1, -1, 2),
	]
	var out := EventText.fold_tree(tribulation)
	assert_eq(out.size(), 2)
	assert_eq(out[1]["id"], 3, "沒有渡劫相伴的突破留著")
	# 根事件也可作對照：根是成婚時，同月結緣的因節點被去掉。
	var with_root := EventText.fold_tree([_node(7, "ThreadCreated", 10, 1, 2, 1, -1)], [_node(9, "Married", 10, 1, 2, 0, -1)])
	assert_eq(with_root.size(), 0)


func test_event_card_tree_and_ripples_use_the_same_fold_rule() -> void:
	await _boot(Vector2i(1920, 1080))
	var checked := 0
	var ids: Array = _stream.keys()
	ids.sort()
	for id: int in ids:
		var raw: Array = _sim.world.event_causes(id, EventCard.DEPTH)
		if raw.is_empty():
			continue
		var event := _sim.world.event(id)
		var folded := EventText.fold_tree(raw, [event])
		_hud.open_event(id)
		await frames(1)
		assert_eq(_card().cause_count(), folded.size(), "事件 %d 的因樹筆數與折疊後一致" % id)
		for i in folded.size():
			var parent := int(folded[i]["parent"])
			assert_true(parent < i, "先根順序：上層在前")
			assert_eq(int(folded[i]["level"]), 1 if parent < 0 else int(folded[parent]["level"]) + 1, "層級連貫")
		checked += 1
		if checked >= 8:
			break
	assert_gt(checked, 0, "有因樹的事件可驗")
