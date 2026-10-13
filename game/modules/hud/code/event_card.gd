class_name EventCard
extends LedgerCard
## GDD 5.8 事件卡（溯因視圖，命簿）：題簽「溯因」；首列是年月與地點，其下是事件的一句話（7.2 模板，帶主角身分）；
## 清單依序是「因」樹（沿 causes 往上，縮排表示層，每層至多 4 筆）與「漣漪」（這件事引出的後續事件數，點了展開為清單）。
## 每一節點是一句話加年月：人名開人物卡、地名飛往並開州郡卡、節點本身把卡換成那一則事件的事件卡。
## 事件已被修剪（或編號無效）時卡仍會開，寫「此事已不可考」。頁尾：飛往（鏡頭移到事件地點）；從別的卡點進來時另有「返回」。

signal center_requested(map_pos: Vector2)
signal person_requested(person: int)
signal province_requested(province: int)
signal sect_requested(sect: int)
## 點了因樹或漣漪裡的一則事件。
signal event_requested(event: int)
signal back_requested

## 因樹往上最多幾層（GDD 5.8）。
const DEPTH := 6
const ROW_GAP := 10.0
const SECTION_HEIGHT := 26.0
## 每往深一層縮排多少。
const INDENT := 10.0
## 清單至少留多高（畫面再矮也不低於這個）。
const MIN_LIST := 120.0

var event := -1
## 是否有上一張卡可以返回（Hud 設定）；有的話 Esc／B 是返回，關閉鈕不掛鍵。
var can_back := false:
	set(value):
		if can_back != value:
			can_back = value
			_relayout()
			queue_redraw()

var _info: Dictionary = {}
## 要的事件已修剪或不存在：卡仍開著，只寫「此事已不可考」。
var _gone := false
var _meta: Dictionary = {}
var _sentence: Dictionary = {}
var _gone_text: Dictionary = {}
var _list: LedgerList
## 清單各列：kind（section、note、head、event）、top、height，另依種類帶 text、paragraph、indent、seal、date、event、level。
var _rows: Array[Dictionary] = []
var _rows_height := 0.0
var _ripple_open := false
var _ripple_count := 0
var _cause_count := 0
## 這則事件與因樹裡出現的人物編號（地圖溯因視圖用）。
var _trace_people := PackedInt32Array()
var _layout := {}


func setup(world: SimWorld, unit: float, width := -1.0) -> void:
	super.setup(world, unit, width)
	_list = LedgerList.new()
	add_child(_list)
	_list.setup(unit)
	_list.painter = _paint_rows
	_list.finder = _row_target_at
	_list.activated.connect(_on_target)
	_list.visible = false


## 開某則事件的卡；事件已不可考也開（寫明不可考）。
func show_event(id: int) -> void:
	var fresh := id != event
	event = id
	if fresh:
		_ripple_open = false
	refresh()
	if fresh and _list != null:
		_list.scroll = 0.0
	_begin_open()


func _forget() -> void:
	event = -1


## 重新讀取事件、因樹與漣漪（每月推進後呼叫）。
func refresh() -> void:
	if event < 0:
		return
	_info = _world.event(event)
	_gone = _info.is_empty()
	_relayout()
	queue_redraw()


func is_gone() -> bool:
	return _gone


## 目前的因樹節點數、漣漪數（測試與呈現用）。
func cause_count() -> int:
	return _cause_count


func ripple_count() -> int:
	return _ripple_count


func ripple_open() -> bool:
	return _ripple_open


## 展開或收起漣漪清單。
func toggle_ripples() -> void:
	if _ripple_count <= 0:
		return
	_ripple_open = not _ripple_open
	_relayout()
	queue_redraw()


## 頁尾動作「飛往」：鏡頭移到事件地點。
func request_center() -> void:
	if is_open() and _info.has("pos"):
		center_requested.emit(_info["pos"])


func _has_content() -> bool:
	return event >= 0


func _title() -> String:
	return Locale.t("card_trace")


func _close_has_key() -> bool:
	return not can_back


func _back_key() -> String:
	return InputHints.label(InputActions.CANCEL, gamepad)


func _center_key() -> String:
	return InputHints.label(InputActions.CARD_CENTER, gamepad)


# ── 版面 ──────────────────────────────────────────────────────

func _province_link(word: String, id: int) -> Dictionary:
	return {"text": word, "target": "province:%d" % id}


## 一則事件那句話裡可點的人名與地名。
static func links_of(entry: Dictionary) -> Array[Dictionary]:
	var links: Array[Dictionary] = []
	if int(entry.get("a_person", -1)) >= 0 and not String(entry["a"]).is_empty():
		links.append({"text": entry["a"], "target": "person:%d" % entry["a_person"]})
	if int(entry.get("b_person", -1)) >= 0 and not String(entry["b"]).is_empty():
		links.append({"text": entry["b"], "target": "person:%d" % entry["b_person"]})
	if int(entry.get("province", -1)) >= 0 and not String(entry["p"]).is_empty():
		links.append({"text": entry["p"], "target": "province:%d" % entry["province"]})
	if int(entry.get("q_province", -1)) >= 0 and not String(entry["q"]).is_empty():
		links.append({"text": entry["q"], "target": "province:%d" % entry["q_province"]})
	links.append_array(sect_links(entry))
	return links


## 一則事件那句話裡可點的宗門名：事件的第一、第二個勢力（`f`／`g`）與主角當時所屬的宗門（`a_power`）。
static func sect_links(entry: Dictionary) -> Array[Dictionary]:
	var links: Array[Dictionary] = []
	for pair: Array in [["f", "f_power"], ["g", "g_power"], ["a_power", "a_power_id"]]:
		if int(entry.get(pair[1], -1)) >= 0 and not String(entry.get(pair[0], "")).is_empty():
			links.append({"text": entry[pair[0]], "target": "sect:%d" % entry[pair[1]]})
	return links


## 依目前資料重算所有段落與高度；改了資料、寬度或「可返回」時呼叫。
func _relayout() -> void:
	if event < 0 or _unit <= 0.0:
		return
	var u := _unit
	var body := Ink.font_size(Ink.SIZE_BODY, u)
	var note := Ink.font_size(Ink.SIZE_NOTE, u)
	var width := _inner_width()
	var no_links: Array[Dictionary] = []
	var sub_width := width - _close_cluster_width(_close_has_key()) - 10.0 * u - CLOSE_RADIUS * 2.0 * u
	_rows.clear()
	_cause_count = 0
	_ripple_count = 0
	_trace_people = PackedInt32Array()
	_meta = {}
	_sentence = {}
	_gone_text = {}
	var y := 14.0 * u
	if _gone:
		_gone_text = LedgerText.paragraph(Locale.t("event_gone"), no_links, sub_width, body)
		_layout["meta_y"] = y
		y += maxf(34.0 * u, float(_gone_text["height"]) + 10.0 * u)
		_layout["stroke_y"] = y
		y += 12.0 * u
		_layout["sentence_y"] = y
		_sentence = LedgerText.paragraph(Locale.t("event_gone_note"), no_links, width, note)
		y += float(_sentence["height"]) + 8.0 * u
	else:
		var place_links: Array[Dictionary] = []
		var place := String(_info["p"])
		var place_text := ""
		if int(_info["province"]) >= 0 and not place.is_empty():
			place_text = " · %s %s" % [Locale.t("event_place"), place]
			place_links.append(_province_link(place, _info["province"]))
		_meta = LedgerText.paragraph(EventText.stamp(int(_info["year"]), int(_info["month"])) + place_text, place_links, sub_width, note)
		_layout["meta_y"] = y
		y += maxf(34.0 * u, float(_meta["height"]) + 10.0 * u)
		_layout["stroke_y"] = y
		y += 12.0 * u
		_layout["sentence_y"] = y
		_sentence = LedgerText.paragraph(EventText.line(_info), links_of(_info), width, body)
		y += float(_sentence["height"]) + 8.0 * u
		_build_rows(width - 8.0 * u)
	_layout["fixed"] = y + (8.0 + 12.0 + ACTION_RADIUS * 2.0 + 14.0) * u + Ink.CARD_BAND * 2.0
	custom_minimum_size.y = _content_height() + ROD * 2.0 * u
	size = custom_minimum_size
	if _list != null:
		_list.content_height = maxf(_rows_height, 1.0)


## 清單：因樹、漣漪（與展開後的後續事件）。
func _build_rows(list_width: float) -> void:
	var u := _unit
	var body := Ink.font_size(Ink.SIZE_BODY, u)
	var note := Ink.font_size(Ink.SIZE_NOTE, u)
	var top := 0.0
	var causes: Array = EventText.fold_tree(_world.event_causes(event, DEPTH), [_info])
	_cause_count = causes.size()
	_collect_people(_info)
	for node: Dictionary in causes:
		_collect_people(node)
	top = _add_section(top, Locale.t("event_causes_title").format({"N": str(causes.size())}))
	if causes.is_empty():
		top = _add_note(top, Locale.t("event_no_causes"), list_width, note)
	for node: Dictionary in causes:
		top = _add_event(top, node, (int(node["level"]) - 1) * INDENT * u, list_width, body, note)
	top = _add_section(top, Locale.t("event_ripples_title"))
	var raw_ripples := _world.event_ripple(event)
	var ripples: Array = []
	if raw_ripples > 0:
		ripples = EventText.fold_history(_world.event_ripples(event))
	_ripple_count = ripples.size()
	if _ripple_count <= 0:
		_ripple_open = false
		top = _add_note(top, Locale.t("event_ripples_wait"), list_width, note)
	else:
		var key := "event_ripples_count_cap" if raw_ripples >= 300 else "event_ripples_count"
		var height := LedgerText.line_height(body) + ROW_GAP * u
		_rows.append({"kind": "head", "top": top, "height": height, "text": Locale.t(key).format({"N": str(_ripple_count)}),
			"action": Locale.t("event_ripples_close" if _ripple_open else "event_ripples_open")})
		top += height
		if _ripple_open:
			for node: Dictionary in ripples:
				top = _add_event(top, node, 0.0, list_width, body, note)
	_rows_height = top


func _collect_people(entry: Dictionary) -> void:
	for id: int in entry.get("persons", PackedInt32Array()):
		if id >= 0 and not _trace_people.has(id):
			_trace_people.append(id)


## 這則事件與因樹裡出現的人物編號（事件已不可考為空）。
func trace_people() -> PackedInt32Array:
	return _trace_people if not _gone else PackedInt32Array()


func _add_section(top: float, text: String) -> float:
	var height := SECTION_HEIGHT * _unit
	_rows.append({"kind": "section", "top": top, "height": height, "text": text})
	return top + height


func _add_note(top: float, text: String, width: float, size: int) -> float:
	var no_links: Array[Dictionary] = []
	var para := LedgerText.paragraph(text, no_links, width, size)
	var height := float(para["height"]) + ROW_GAP * _unit
	_rows.append({"kind": "note", "top": top, "height": height, "paragraph": para})
	return top + height


func _add_event(top: float, node: Dictionary, indent: float, width: float, body: int, note: int) -> float:
	var para := LedgerText.paragraph(EventText.line(node), links_of(node), width - indent, body)
	var height := LedgerText.line_height(note) + float(para["height"]) + ROW_GAP * _unit
	_rows.append({"kind": "event", "top": top, "height": height, "indent": indent, "event": int(node["id"]), "level": int(node.get("level", 0)),
		"seal": EventText.seal(node["kind"]), "date": EventText.stamp(int(node["year"]), int(node["month"])), "paragraph": para})
	return top + height


func _fixed_height() -> float:
	return float(_layout.get("fixed", 0.0))


## 卡片（含軸頭）至少要多高：固定部分加上清單的最低高度。
func min_height() -> float:
	return _fixed_height() + ROD * 2.0 * _unit + minf(_rows_height, MIN_LIST * _unit)


## 清單的高度：畫面放得下就全列，放不下縮到剩下的高度（至少 MIN_LIST）。
func _list_height() -> float:
	var spare := max_height - (_fixed_height() + ROD * 2.0 * _unit)
	return clampf(spare, minf(_rows_height, MIN_LIST * _unit), _rows_height)


func _content_height() -> float:
	return _fixed_height() + _list_height()


func _list_rect() -> Rect2:
	var paper := _paper_rect(_content_height())
	var x := paper.position.x + (STRIP + 14.0) * _unit
	var bottom := paper.end.y - (8.0 + 12.0 + ACTION_RADIUS * 2.0 + 14.0) * _unit
	var height := _list_height()
	return Rect2(Vector2(x, bottom - height), Vector2(_inner_width(), height))


func _arrange(unrolled: bool) -> void:
	if _list == null:
		return
	_list.visible = unrolled and not _gone
	_list.edge_scroll = gamepad
	if unrolled:
		var rect := _list_rect()
		_list.position = rect.position
		_list.size = rect.size
		_list.content_height = maxf(_rows_height, 1.0)


# ── 頁尾 ──────────────────────────────────────────────────────

func _fly_rect() -> Rect2:
	return _action_rect((STRIP + 14.0) * _unit)


func _fly_width() -> float:
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	return ACTION_RADIUS * 2.0 * _unit + 8.0 * _unit + Ink.text_size(Locale.t("act_fly"), body).x + 8.0 * _unit + Ink.key_glyph_width(_center_key(), _unit)


func _back_rect() -> Rect2:
	var left := (STRIP + 14.0) * _unit
	if _info.has("pos"):
		left += _fly_width() + 16.0 * _unit
	return _action_rect(left)


# ── 繪製 ──────────────────────────────────────────────────────

func _draw_body(paper: Rect2) -> void:
	var u := _unit
	var x := paper.position.x + (STRIP + 14.0) * u
	var right := paper.end.x - 16.0 * u
	var top := paper.position.y
	if _gone:
		LedgerText.draw(self, Vector2(x, top + float(_layout["meta_y"])), _gone_text, Ink.INK)
	else:
		LedgerText.draw(self, Vector2(x, top + float(_layout["meta_y"])), _meta, Ink.INK_LIGHT, _hover, _pressed)
	var stroke_y := top + float(_layout["stroke_y"])
	Ink.stroke(self, Vector2(x, stroke_y), Vector2(right, stroke_y), 2.0 * u, Color(Ink.INK, 0.7))
	if not _sentence.is_empty():
		LedgerText.draw(self, Vector2(x, top + float(_layout["sentence_y"])), _sentence, Ink.INK_LIGHT if _gone else Ink.INK, _hover, _pressed)
	if _info.has("pos"):
		_draw_action(_fly_rect(), &"center", Locale.t("act_fly"), _center_key(), "fly")
	if can_back:
		_draw_action(_back_rect(), &"back", Locale.t("act_back"), _back_key(), "back")


## 清單的畫法（內容座標）。
func _paint_rows(list: LedgerList) -> void:
	var u := _unit
	var note := Ink.font_size(Ink.SIZE_NOTE, u)
	var body := Ink.font_size(Ink.SIZE_BODY, u)
	var section := Ink.font_size(Ink.SIZE_SECTION, u)
	var date_height := LedgerText.line_height(note)
	var first := list.scroll
	var last := list.scroll + list.size.y
	for row: Dictionary in _rows:
		var top: float = row["top"]
		var height: float = row["height"]
		if top + height < first or top > last:
			continue
		match String(row["kind"]):
			"section":
				Ink.section(list, Vector2(0.0, top), row["text"], section)
			"note":
				LedgerText.draw(list, Vector2(0.0, top), row["paragraph"], Ink.INK_LIGHT)
			"head":
				var lit := list.hover == "ripples"
				if lit:
					list.draw_rect(Rect2(Vector2(-2.0, top), Vector2(list.size.x - 4.0 * u, height - ROW_GAP * u * 0.5)), Color(Ink.INK, 0.07), true)
				var text: String = row["text"]
				Ink.text(list, Vector2(0.0, top + (LedgerText.line_height(body) - Ink.text_size(text, body).y) * 0.5), text, body, Ink.INK)
				var action: String = row["action"]
				var action_w := Ink.text_size(action, note).x
				Ink.text(list, Vector2(list.size.x - 12.0 * u - action_w, top + (LedgerText.line_height(body) - Ink.text_size(action, note).y) * 0.5), action, note, Ink.SEAL_RED)
			"event":
				_paint_event_row(list, row, date_height, note)


func _paint_event_row(list: LedgerList, row: Dictionary, date_height: float, note: int) -> void:
	var u := _unit
	var top: float = row["top"]
	var indent: float = row["indent"]
	var level: int = row["level"]
	var height: float = row["height"]
	if list.hover == "event:%d" % int(row["event"]):
		list.draw_rect(Rect2(Vector2(indent - 2.0, top), Vector2(list.size.x - indent - 4.0 * u, height - ROW_GAP * u * 0.5)), Color(Ink.INK, 0.07), true)
	if level > 1:
		# 縮排的導線：這一列是上一層的因。
		list.draw_line(Vector2(indent - 5.0 * u, top + 2.0), Vector2(indent - 5.0 * u, top + height - ROW_GAP * u), Color(Ink.INK_LIGHT, 0.5), 1.0)
	var seal_text: String = row["seal"]
	var seal_x := Ink.text_size(seal_text, note).x
	Ink.text(list, Vector2(indent, top + (date_height - Ink.text_size(seal_text, note).y) * 0.5), seal_text, note, Ink.SEAL_RED)
	var date: String = row["date"]
	Ink.text(list, Vector2(indent + seal_x + 6.0 * u, top + (date_height - Ink.text_size(date, note).y) * 0.5), date, note, Ink.INK_LIGHT)
	LedgerText.draw(list, Vector2(indent, top + date_height), row["paragraph"], Ink.INK if level <= 1 else Ink.INK_LIGHT, list.hover, list.pressed)
	var base: float = top + height - ROW_GAP * u * 0.5
	list.draw_line(Vector2(indent, base), Vector2(list.size.x - 8.0 * u, base), Color(Ink.INK_LIGHT, 0.22), 1.0)


# ── 點擊 ──────────────────────────────────────────────────────

func _row_target_at(point: Vector2) -> String:
	var date_height := LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, _unit))
	for row: Dictionary in _rows:
		var top: float = row["top"]
		if point.y < top or point.y >= top + float(row["height"]):
			continue
		match String(row["kind"]):
			"head":
				return "ripples"
			"event":
				# 人名、地名的連結優先，其餘部分是這則事件本身。
				var link := LedgerText.target_at(Vector2(float(row["indent"]), top + date_height), row["paragraph"], point)
				return link if not link.is_empty() else "event:%d" % int(row["event"])
		return ""
	return ""


func _on_target(target: String) -> void:
	if target.begins_with("person:"):
		person_requested.emit(int(target.substr(7)))
	elif target.begins_with("province:"):
		province_requested.emit(int(target.substr(9)))
	elif target.begins_with("event:"):
		event_requested.emit(int(target.substr(6)))
	elif target.begins_with("sect:"):
		sect_requested.emit(int(target.substr(5)))
	elif target == "ripples":
		toggle_ripples()


func _activate(target: String) -> void:
	if target == "fly":
		request_center()
	elif target == "back":
		back_requested.emit()
	else:
		_on_target(target)


func _target_at(pos: Vector2) -> String:
	if event < 0 or _layout.is_empty():
		return ""
	if _in_round(_close_rect(), pos):
		return "close"
	if _info.has("pos") and _in_round(_fly_rect(), pos):
		return "fly"
	if can_back and _in_round(_back_rect(), pos):
		return "back"
	var paper := _paper_rect(_content_height())
	var origin := paper.position + Vector2((STRIP + 14.0) * _unit, 0.0)
	for pair: Array in [[_meta, "meta_y"], [_sentence, "sentence_y"]]:
		if (pair[0] as Dictionary).is_empty():
			continue
		var target := LedgerText.target_at(origin + Vector2(0.0, float(_layout[pair[1]])), pair[0], pos)
		if not target.is_empty():
			return target
	return ""
