class_name PersonCard
extends LedgerCard
## GDD 5.8 人物卡（命簿）：題簽直書姓名；首列是身分（門派職分、境界）；其下基本資料與魂的來歷，
## 然後是個人歷程——這個人一生牽涉的事件，由舊到新，每筆一句話（與傳聞簽同一套 7.2 模板）。
## 歷程裡的其他人名、地名可點：人名換成那個人的卡，地名飛往並開州郡卡。已身故者仍可開卡，歷程到死為止。
## 頁尾：置中（鏡頭移到此人所在）；從別的卡點進來時另有「返回」。

signal center_requested(map_pos: Vector2)
## 點了歷程或基本資料裡的人名。
signal person_requested(person: int)
## 點了歷程或基本資料裡的地名。
signal province_requested(province: int)
## 點了歷程的一筆（人名、地名以外的部分）：開那則事件的事件卡。
signal event_requested(event: int)
signal back_requested

## 歷程每筆之間的空白。
const ROW_GAP := 10.0
## 歷程清單至少留多高（畫面再矮也不低於這個）。
const MIN_LIST := 120.0

var person := -1
## 是否有上一張卡可以返回（Hud 設定）；有的話 Esc／B 是返回，關閉鈕不掛鍵。
var can_back := false:
	set(value):
		if can_back != value:
			can_back = value
			_relayout()
			queue_redraw()

var _info: Dictionary = {}
var _list: LedgerList
## 首列身分與基本資料的段落（依序：首列、年齡、地點、資質、魂）。
var _subtitle: Dictionary = {}
var _paragraphs: Array[Dictionary] = []
## 歷程各筆：top（內容座標的上緣）、height、date、seal、paragraph。
var _rows: Array[Dictionary] = []
var _rows_height := 0.0
## 版面（相對於紙面上緣的位移，已含版面倍率）。
var _layout := {}


func setup(world: SimWorld, unit: float, width := -1.0) -> void:
	super.setup(world, unit, width)
	_list = LedgerList.new()
	add_child(_list)
	_list.setup(unit)
	_list.painter = _paint_history
	_list.finder = _history_target_at
	_list.activated.connect(_on_target)
	_list.visible = false


func show_person(id: int) -> void:
	var fresh := id != person
	person = id
	refresh()
	if fresh and _list != null:
		_list.scroll = 0.0
	_begin_open()


func _forget() -> void:
	person = -1


## 重新讀取人物與歷程（每月推進後呼叫）。
func refresh() -> void:
	if person < 0:
		return
	_info = _world.person(person)
	_relayout()
	queue_redraw()


## 頁尾動作「置中」：把鏡頭移到此人所在。
func request_center() -> void:
	if is_open() and not _info.is_empty():
		center_requested.emit(_info["pos"])


func _has_content() -> bool:
	return not _info.is_empty()


func _title() -> String:
	return _info["name"]


func _close_has_key() -> bool:
	return not can_back


## 鍵盤的 Esc、搖桿的 B（返回與關閉共用同一個動作，由 Hud 分派）。
func _back_key() -> String:
	return InputHints.label(InputActions.CANCEL, gamepad)


func _center_key() -> String:
	return InputHints.label(InputActions.CARD_CENTER, gamepad)


# ── 版面 ──────────────────────────────────────────────────────

func _province_link(word: String, id: int) -> Dictionary:
	return {"text": word, "target": "province:%d" % id}


## 依目前資料重算所有段落與高度；改了資料、寬度或「可返回」時呼叫。
func _relayout() -> void:
	if _info.is_empty() or _unit <= 0.0:
		return
	var u := _unit
	var body := Ink.font_size(Ink.SIZE_BODY, u)
	var note := Ink.font_size(Ink.SIZE_NOTE, u)
	var width := _inner_width()
	var alive: bool = _info["alive"]
	var no_links: Array[Dictionary] = []

	# 首列：身分（避開右上的關閉鈕與它的文字、鍵）。
	var sub_width := width - _close_cluster_width(_close_has_key()) - 10.0 * u - CLOSE_RADIUS * 2.0 * u
	_subtitle = LedgerText.paragraph(PersonText.identity_line(_info), no_links, sub_width, body)

	# 基本資料。
	var gender := PersonText.gender_word(_info["gender"])
	var line1: String
	if alive:
		line1 = Locale.t("card_age_line").format({"G": gender, "N": str(_info["age"]), "L": str(_info["lifespan"])})
	else:
		line1 = Locale.t("card_dead_line").format({"G": gender, "N": str(_info["age"])})
	if not String(_info["title"]).is_empty():
		line1 = "%s · %s" % [_info["title"], line1]
	var place_label := Locale.t("card_place" if alive else "card_died_at")
	var line2 := "%s %s · %s %s" % [place_label, _info["province_name"], Locale.t("card_home"), _info["home_name"]]
	var places: Array[Dictionary] = [_province_link(_info["province_name"], _info["province"]), _province_link(_info["home_name"], _info["home"])]
	var attrs := []
	for key: String in ["root", "insight", "talent", "virtue", "ferocity"]:
		attrs.append("%s %d" % [Locale.t("attr_" + key), int(round(float(_info[key])))])
	var line3 := " · ".join(attrs)
	var line4 := "%s %s · %s %d · %s %d · %s %d" % [
		Locale.t("card_soul"), Locale.t("card_life_nth").format({"N": str(_info.get("life_index", 1))}),
		Locale.t("card_merit"), int(round(float(_info.get("merit", 0.0)))),
		Locale.t("card_sin"), int(round(float(_info.get("sin", 0.0)))),
		Locale.t("card_wisdom"), int(round(float(_info.get("wisdom", 0.0)))),
	]
	_paragraphs = [LedgerText.paragraph(line1, no_links, width, body), LedgerText.paragraph(line2, places, width, body)]
	if not alive:
		var cause := Locale.t("card_death_cause").format({"X": PersonText.death_word(_info["death_cause"])})
		_paragraphs.append(LedgerText.paragraph(cause, no_links, width, body))
	_paragraphs.append(LedgerText.paragraph(line3, no_links, width, note))
	_paragraphs.append(LedgerText.paragraph(line4, no_links, width, note))
	for i in _paragraphs.size():
		_paragraphs[i]["strong"] = i < (2 if alive else 3)

	# 歷程各筆。
	_rows.clear()
	var history: Array = EventText.fold_history(_world.person_history(person))
	var list_width := width - 8.0 * u
	var date_height := LedgerText.line_height(note)
	var top := 0.0
	var era_shown := ""
	for entry: Dictionary in history:
		var sentence := EventText.history_line(entry, person)
		var links: Array[Dictionary] = []
		if int(entry["a_person"]) >= 0 and int(entry["a_person"]) != person:
			links.append({"text": entry["a"], "target": "person:%d" % entry["a_person"]})
		if int(entry["b_person"]) >= 0 and int(entry["b_person"]) != person:
			links.append({"text": entry["b"], "target": "person:%d" % entry["b_person"]})
		if int(entry["province"]) >= 0 and not String(entry["p"]).is_empty():
			links.append(_province_link(entry["p"], entry["province"]))
		if int(entry["q_province"]) >= 0 and not String(entry["q"]).is_empty():
			links.append(_province_link(entry["q"], entry["q_province"]))
		var para := LedgerText.paragraph(sentence, links, list_width, body)
		# 同一紀元內只有第一筆與換紀元的那筆帶紀元名。
		var date := EventText.history_date(entry, entry["era"] != era_shown)
		era_shown = entry["era"]
		var height: float = date_height + float(para["height"]) + ROW_GAP * u
		_rows.append({"top": top, "height": height, "event": int(entry["id"]), "date": date, "seal": EventText.seal(entry["kind"]), "paragraph": para})
		top += height
	_rows_height = top

	# 版面位移。
	var y := 14.0 * u
	_layout["subtitle_y"] = y
	y += maxf(34.0 * u, float(_subtitle["height"]) + 10.0 * u)
	_layout["stroke_y"] = y
	y += 12.0 * u
	_layout["para_y"] = []
	for para: Dictionary in _paragraphs:
		_layout["para_y"].append(y)
		y += float(para["height"]) + 6.0 * u
	y += 4.0 * u
	_layout["section_y"] = y
	y += 26.0 * u
	_layout["fixed"] = y + (8.0 + 12.0 + ACTION_RADIUS * 2.0 + 14.0) * u + Ink.CARD_BAND * 2.0
	custom_minimum_size.y = _content_height() + ROD * 2.0 * u
	size = custom_minimum_size
	if _list != null:
		_list.content_height = maxf(_rows_height, 1.0)


func _fixed_height() -> float:
	return float(_layout.get("fixed", 0.0))


## 卡片（含軸頭）至少要多高：固定部分加上歷程清單的最低高度；畫面矮於這個就只能出界或壓到印匣。
func min_height() -> float:
	var all := maxf(_rows_height, LedgerText.line_height(Ink.font_size(Ink.SIZE_BODY, _unit)))
	return _fixed_height() + ROD * 2.0 * _unit + minf(all, MIN_LIST * _unit)


## 歷程清單的高度：畫面放得下就全列，放不下縮到剩下的高度（至少 MIN_LIST）。
func _list_height() -> float:
	var all := maxf(_rows_height, LedgerText.line_height(Ink.font_size(Ink.SIZE_BODY, _unit)))
	var spare := max_height - (_fixed_height() + ROD * 2.0 * _unit)
	return clampf(spare, minf(all, MIN_LIST * _unit), all)


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
	_list.visible = unrolled
	_list.edge_scroll = gamepad
	if unrolled:
		var rect := _list_rect()
		_list.position = rect.position
		_list.size = rect.size
		_list.content_height = maxf(_rows_height, 1.0)


# ── 頁尾 ──────────────────────────────────────────────────────

func _center_rect() -> Rect2:
	return _action_rect((STRIP + 14.0) * _unit)


## 「置中」整顆（圓鈕、字、鍵）佔的寬度。
func _center_width() -> float:
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	return ACTION_RADIUS * 2.0 * _unit + 8.0 * _unit + Ink.text_size(Locale.t("act_center"), body).x + 8.0 * _unit + Ink.key_glyph_width(_center_key(), _unit)


func _back_rect() -> Rect2:
	return _action_rect((STRIP + 14.0) * _unit + _center_width() + 16.0 * _unit)


# ── 繪製 ──────────────────────────────────────────────────────

func _draw_body(paper: Rect2) -> void:
	var u := _unit
	var x := paper.position.x + (STRIP + 14.0) * u
	var right := paper.end.x - 16.0 * u
	var section := Ink.font_size(Ink.SIZE_SECTION, u)
	var note := Ink.font_size(Ink.SIZE_NOTE, u)
	var top := paper.position.y
	LedgerText.draw(self, Vector2(x, top + float(_layout["subtitle_y"])), _subtitle, Ink.INK)
	var stroke_y := top + float(_layout["stroke_y"])
	Ink.stroke(self, Vector2(x, stroke_y), Vector2(right, stroke_y), 2.0 * u, Color(Ink.INK, 0.7))
	for i in _paragraphs.size():
		var color := Ink.INK if _paragraphs[i]["strong"] else Ink.INK_LIGHT
		LedgerText.draw(self, Vector2(x, top + float(_layout["para_y"][i])), _paragraphs[i], color, _hover, _pressed)
	var section_y := top + float(_layout["section_y"])
	Ink.section(self, Vector2(x, section_y), Locale.t("card_history_title").format({"N": str(_rows.size())}), section)
	if _rows.is_empty():
		Ink.text(self, Vector2(x, section_y + 28.0 * u), Locale.t("card_history_none"), note, Ink.INK_LIGHT)
	# 頁尾動作印。
	_draw_action(_center_rect(), &"center", Locale.t("act_center"), _center_key(), "center")
	if can_back:
		_draw_action(_back_rect(), &"back", Locale.t("act_back"), _back_key(), "back")


## 歷程清單的畫法（內容座標）：一行小字的年月與印章，下面是那句話。
func _paint_history(list: LedgerList) -> void:
	var u := _unit
	var note := Ink.font_size(Ink.SIZE_NOTE, u)
	var date_height := LedgerText.line_height(note)
	var first := list.scroll
	var last := list.scroll + list.size.y
	for row: Dictionary in _rows:
		var top: float = row["top"]
		if top + float(row["height"]) < first or top > last:
			continue
		if list.hover == "event:%d" % int(row["event"]):
			list.draw_rect(Rect2(Vector2(-2.0, top), Vector2(list.size.x - 4.0 * u, float(row["height"]) - ROW_GAP * u * 0.5)), Color(Ink.INK, 0.07), true)
		var seal_text: String = row["seal"]
		var seal_x := Ink.text_size(seal_text, note).x
		Ink.text(list, Vector2(0.0, top + (date_height - Ink.text_size(seal_text, note).y) * 0.5), seal_text, note, Ink.SEAL_RED)
		var date: String = row["date"]
		Ink.text(list, Vector2(seal_x + 6.0 * u, top + (date_height - Ink.text_size(date, note).y) * 0.5), date, note, Ink.INK_LIGHT)
		LedgerText.draw(list, Vector2(0.0, top + date_height), row["paragraph"], Ink.INK, list.hover, list.pressed)
		var base: float = top + float(row["height"]) - ROW_GAP * u * 0.5
		list.draw_line(Vector2(0.0, base), Vector2(list.size.x - 8.0 * u, base), Color(Ink.INK_LIGHT, 0.22), 1.0)


# ── 點擊 ──────────────────────────────────────────────────────

func _history_target_at(point: Vector2) -> String:
	var date_height := LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, _unit))
	for row: Dictionary in _rows:
		var top: float = row["top"]
		if point.y >= top and point.y < top + float(row["height"]):
			# 人名、地名的連結優先，其餘部分是這一筆事件本身。
			var link := LedgerText.target_at(Vector2(0.0, top + date_height), row["paragraph"], point)
			return link if not link.is_empty() else "event:%d" % int(row["event"])
	return ""


func _on_target(target: String) -> void:
	if target.begins_with("person:"):
		person_requested.emit(int(target.substr(7)))
	elif target.begins_with("province:"):
		province_requested.emit(int(target.substr(9)))
	elif target.begins_with("event:"):
		event_requested.emit(int(target.substr(6)))


func _activate(target: String) -> void:
	if target == "center":
		request_center()
	elif target == "back":
		back_requested.emit()
	else:
		_on_target(target)


func _paragraph_origin(index: int) -> Vector2:
	var paper := _paper_rect(_content_height())
	return Vector2(paper.position.x + (STRIP + 14.0) * _unit, paper.position.y + float(_layout["para_y"][index]))


func _target_at(pos: Vector2) -> String:
	if _info.is_empty() or _layout.is_empty():
		return ""
	if _in_round(_close_rect(), pos):
		return "close"
	if _in_round(_center_rect(), pos):
		return "center"
	if can_back and _in_round(_back_rect(), pos):
		return "back"
	for i in _paragraphs.size():
		var target := LedgerText.target_at(_paragraph_origin(i), _paragraphs[i], pos)
		if not target.is_empty():
			return target
	return ""
