class_name SectCard
extends LedgerCard
## GDD 5.8 宗門卡（命簿）：題簽直書宗門名；首列是正邪與山門所在州郡（已覆滅者加覆滅年）；其下依序是掌門、聲望與立派年與門人數、
## 靈脈與勢力範圍、境界分布、與他宗的友好與敵對、飛升者；下方可捲動的清單先是門史（開派、繼位、爭脈、覆滅，由舊到新），再是門人名單（依境界由高到低）。
## 人名、州郡名、宗門名可點：人名開人物卡、州郡名飛往並開州郡卡、宗門名換成那一宗的卡、門史的一筆換成事件卡。已覆滅的宗門也開得了卡，沒有門人名單。
## 頁尾：飛往（鏡頭移到山門）；從別的卡點進來時另有「返回」。

signal center_requested(map_pos: Vector2)
signal person_requested(person: int)
signal province_requested(province: int)
signal event_requested(event: int)
signal sect_requested(sect: int)
signal back_requested

const ROW_GAP := 10.0
const SECTION_HEIGHT := 26.0
const MEMBER_ROW := 26.0
## 清單至少留多高（畫面再矮也不低於這個）。
const MIN_LIST := 120.0

var sect := -1
## 是否有上一張卡可以返回（Hud 設定）；有的話 Esc／B 是返回，關閉鈕不掛鍵。
var can_back := false:
	set(value):
		if can_back != value:
			can_back = value
			_relayout()
			queue_redraw()

var _info: Dictionary = {}
var _history: Array = []
var _list: LedgerList
## 固定部分各塊：kind（para、section、stroke）、y（相對於紙面上緣），另依種類帶 paragraph、color、text。
var _blocks: Array[Dictionary] = []
## 清單各列：kind（section、event、member、note）、top、height，另依種類帶 text、paragraph、seal、date、event、person、realm。
var _rows: Array[Dictionary] = []
var _rows_height := 0.0
var _fixed := 0.0
## 卡放不下時，境界分布、關係、飛升者改排進清單。
var _tight := false
var _member_rows := 0
var _history_rows := 0


func setup(world: SimWorld, unit: float, width := -1.0) -> void:
	super.setup(world, unit, width)
	_list = LedgerList.new()
	add_child(_list)
	_list.setup(unit)
	_list.painter = _paint_rows
	_list.finder = _row_target_at
	_list.activated.connect(_on_target)
	_list.visible = false


func show_sect(id: int) -> void:
	var fresh := id != sect
	sect = id
	refresh()
	if fresh and _list != null:
		_list.scroll = 0.0
	_begin_open()


func _forget() -> void:
	sect = -1


## 重新讀取宗門與門史（每月推進後呼叫）。
func refresh() -> void:
	if sect < 0:
		return
	_info = _world.sect(sect)
	_history = _world.sect_history(sect) if not _info.is_empty() else []
	_relayout()
	queue_redraw()


## 頁尾動作「飛往」：鏡頭移到山門。
func request_center() -> void:
	if is_open() and not _info.is_empty():
		center_requested.emit(_info["pos"])


func info() -> Dictionary:
	return _info


## 清單裡的門人列數、門史列數（測試與呈現用）。
func member_row_count() -> int:
	return _member_rows


func history_row_count() -> int:
	return _history_rows


func list() -> LedgerList:
	return _list


## 固定部分所有文字（測試用）。
func head_text() -> String:
	var out: Array[String] = []
	for block: Dictionary in _blocks:
		if block["kind"] == "para":
			out.append(" ".join(block["paragraph"]["lines"]))
		elif block["kind"] == "section":
			out.append(block["text"])
	for row: Dictionary in _rows:
		if row.get("extra", false):
			out.append(row["text"] if row["kind"] == "section" else " ".join(row["paragraph"]["lines"]))
	return "\n".join(out)


func _has_content() -> bool:
	return not _info.is_empty()


func _title() -> String:
	return _info["name"]


func _close_has_key() -> bool:
	return not can_back


func _back_key() -> String:
	return InputHints.label(InputActions.CANCEL, gamepad)


func _center_key() -> String:
	return InputHints.label(InputActions.CARD_CENTER, gamepad)


func _on_max_height_changed() -> void:
	if not _info.is_empty() and _unit > 0.0 and not _blocks.is_empty():
		_relayout()
		queue_redraw()


# ── 版面 ──────────────────────────────────────────────────────

static func _link(word: String, target: String) -> Dictionary:
	return {"text": word, "target": target}


## 年份：與現在不同紀元才帶紀元名。
func _year_text(era: String, year: int) -> String:
	var key := "sect_year_era" if era != _info["now_era"] else "sect_year"
	return Locale.t(key).format({"E": era, "Y": str(year)})


func _add_para(y: float, text: String, links: Array[Dictionary], width: float, size: int, color: Color) -> float:
	var para := LedgerText.paragraph(text, links, width, size)
	_blocks.append({"kind": "para", "y": y, "paragraph": para, "color": color})
	return float(para["height"])


func _add_section(y: float, text: String) -> float:
	_blocks.append({"kind": "section", "y": y, "text": text})
	return SECTION_HEIGHT * _unit


## 境界分布、關係、飛升者三段，依序是 section 與 para 項（para 帶 text、links、size、color、gap）。
func _extra_items() -> Array[Dictionary]:
	var u := _unit
	var body := Ink.font_size(Ink.SIZE_BODY, u)
	var note := Ink.font_size(Ink.SIZE_NOTE, u)
	var no_links: Array[Dictionary] = []
	var items: Array[Dictionary] = []
	# 境界分布：各境界人數，由高到低。
	if _info["alive"] and int(_info["member_count"]) > 0:
		items.append({"kind": "section", "text": Locale.t("sect_realms")})
		var counts: PackedInt32Array = _info["realm_counts"]
		var cells: Array[String] = []
		for realm in range(counts.size() - 1, -1, -1):
			if counts[realm] > 0:
				cells.append(Locale.t("sect_realm_count").format({"R": Locale.t("realm_%d" % realm), "N": str(counts[realm])}))
		if int(_info["other_members"]) > 0:
			cells.append(Locale.t("sect_realm_count").format({"R": Locale.t("sect_realm_other"), "N": str(_info["other_members"])}))
		items.append({"kind": "para", "text": " · ".join(cells), "links": no_links, "size": body, "color": Ink.INK, "gap": 6.0 * u})

	# 關係：友好、敵對前五名（宗門名可點）。
	items.append({"kind": "section", "text": Locale.t("sect_relations")})
	var friends: Array = _info["friends"]
	var foes: Array = _info["foes"]
	if friends.is_empty() and foes.is_empty():
		items.append({"kind": "para", "text": Locale.t("sect_relations_none"), "links": no_links, "size": note, "color": Ink.INK_LIGHT, "gap": 6.0 * u})
	for pair: Array in [["sect_friends", friends], ["sect_foes", foes]]:
		var rows: Array = pair[1]
		if rows.is_empty():
			continue
		var names: Array[String] = []
		var links: Array[Dictionary] = []
		for entry: Dictionary in rows:
			names.append(entry["name"])
			if entry["is_sect"]:
				links.append(_link(entry["name"], "sect:%d" % entry["id"]))
		items.append({"kind": "para", "text": "%s　%s" % [Locale.t(pair[0]), Locale.t("sect_list_sep").join(names)], "links": links, "size": body, "color": Ink.INK, "gap": 4.0 * u})

	# 飛升者：人數；名錄可考的列名。
	var ascended: int = _info["ascended"]
	var listed: Array = _info["ascended_people"]
	var ascend_text := "%s　%s" % [Locale.t("sect_ascended"), Locale.t("sect_ascended_count").format({"N": str(ascended)})]
	var ascend_links: Array[Dictionary] = []
	if not listed.is_empty():
		if listed.size() < ascended:
			ascend_text += Locale.t("sect_ascended_listed")
		var who: Array[String] = []
		for entry: Dictionary in listed:
			who.append(entry["name"])
			ascend_links.append(_link(entry["name"], "person:%d" % entry["id"]))
		ascend_text += "　" + Locale.t("sect_list_sep").join(who)
	items.append({"kind": "para", "text": ascend_text, "links": ascend_links, "size": body, "color": Ink.INK, "gap": 10.0 * u})
	return items


## 依目前資料重算所有段落與高度；改了資料、寬度或「可返回」時呼叫。
func _relayout() -> void:
	if _info.is_empty() or _unit <= 0.0:
		return
	_tight = false
	_compose()
	if min_height() > max_height:
		_tight = true
		_compose()


func _compose() -> void:
	var u := _unit
	var body := Ink.font_size(Ink.SIZE_BODY, u)
	var note := Ink.font_size(Ink.SIZE_NOTE, u)
	var width := _inner_width()
	var no_links: Array[Dictionary] = []
	var alive: bool = _info["alive"]
	_blocks.clear()
	var y := 14.0 * u

	# 首列：正邪、山門所在州郡（可點）；已覆滅者加覆滅年。避開右上的關閉鈕。
	var sub_width := width - _close_cluster_width(_close_has_key()) - 10.0 * u - CLOSE_RADIUS * 2.0 * u
	var parts: Array[String] = [Locale.t("sect_evil" if _info["evil"] else "sect_good"), _info["province_name"]]
	if not alive:
		parts.append("%s %s" % [Locale.t("sect_fell"), _year_text(_info["fallen_era"], _info["fallen_year"])])
	var head_links: Array[Dictionary] = [_link(_info["province_name"], "province:%d" % _info["province"])]
	var height := _add_para(y, " · ".join(parts), head_links, sub_width, body, Ink.INK)
	y += maxf(34.0 * u, height + 10.0 * u)
	_blocks.append({"kind": "stroke", "y": y})
	y += 12.0 * u

	# 掌門（已覆滅者改寫覆滅的原因）。
	if alive:
		var leader_links: Array[Dictionary] = []
		var text := Locale.t("sect_leader_none")
		if int(_info["leader"]) >= 0:
			text = "%s %s" % [Locale.t("sect_leader"), _info["leader_name"]]
			if int(_info["leader_realm"]) >= 0:
				text += " · " + Locale.t("realm_%d" % int(_info["leader_realm"]))
			leader_links.append(_link(_info["leader_name"], "person:%d" % _info["leader"]))
		y += _add_para(y, text, leader_links, width, body, Ink.INK) + 6.0 * u
	else:
		var cause := Locale.t("sect_fell_war" if _info["fall_cause"] == "SectWar" else "sect_fell_scatter")
		y += _add_para(y, cause, no_links, width, body, Ink.INK) + 6.0 * u

	# 聲望、立派年、門人數。
	var facts: Array[String] = []
	if alive:
		facts.append("%s %d" % [Locale.t("sect_prestige"), int(round(float(_info["prestige"])))])
	facts.append("%s %s" % [Locale.t("sect_founded"), _year_text(_info["founded_era"], _info["founded_year"])])
	if alive:
		facts.append("%s %d" % [Locale.t("sect_members"), int(_info["member_count"])])
	y += _add_para(y, " · ".join(facts), no_links, width, note, Ink.INK_LIGHT) + 6.0 * u

	# 靈脈：山門倚的洞天福地與勢力範圍。
	var ley := Locale.t("sect_ley_node").format({"N": _info["node"]}) if not String(_info["node"]).is_empty() else Locale.t("sect_ley_none")
	if alive:
		ley += " · " + Locale.t("sect_range").format({"N": str((_info["range"] as PackedInt32Array).size())})
	y += _add_para(y, "%s　%s" % [Locale.t("sect_ley"), ley], no_links, width, note, Ink.INK_LIGHT) + 8.0 * u

	# 境界分布、關係、飛升者：卡放得下就排在固定部分；放不下（Steam Deck 加大縮放）改排進清單最上面，跟著一起捲。
	if not _tight:
		for item: Dictionary in _extra_items():
			if item["kind"] == "section":
				y += _add_section(y, item["text"])
			else:
				y += _add_para(y, item["text"], item["links"], width, item["size"], item["color"]) + float(item["gap"])
	_fixed = y + (8.0 + 12.0 + ACTION_RADIUS * 2.0 + 14.0) * u + Ink.CARD_BAND * 2.0

	_build_rows(width - 8.0 * u)
	custom_minimum_size.y = _content_height() + ROD * 2.0 * u
	size = custom_minimum_size
	if _list != null:
		_list.content_height = maxf(_rows_height, 1.0)


## 清單：門史，再是門人名單。
func _build_rows(list_width: float) -> void:
	var u := _unit
	var body := Ink.font_size(Ink.SIZE_BODY, u)
	var note := Ink.font_size(Ink.SIZE_NOTE, u)
	var date_height := LedgerText.line_height(note)
	_rows.clear()
	var top := 0.0
	if _tight:
		for item: Dictionary in _extra_items():
			if item["kind"] == "section":
				_rows.append({"kind": "section", "top": top, "height": SECTION_HEIGHT * u, "text": item["text"], "extra": true})
				top += SECTION_HEIGHT * u
			else:
				var extra := LedgerText.paragraph(item["text"], item["links"], list_width, item["size"])
				var extra_height := float(extra["height"]) + float(item["gap"])
				_rows.append({"kind": "para", "top": top, "height": extra_height, "paragraph": extra, "color": item["color"], "extra": true})
				top += extra_height
	var history: Array = _history
	_history_rows = history.size()
	_rows.append({"kind": "section", "top": top, "height": SECTION_HEIGHT * u, "text": Locale.t("sect_history_title").format({"N": str(history.size())})})
	top += SECTION_HEIGHT * u
	if history.is_empty():
		var empty := LedgerText.paragraph(Locale.t("sect_history_none"), [] as Array[Dictionary], list_width, note)
		_rows.append({"kind": "note", "top": top, "height": float(empty["height"]) + ROW_GAP * u, "paragraph": empty})
		top += float(empty["height"]) + ROW_GAP * u
	var era_shown := ""
	var self_target := "sect:%d" % sect
	for entry: Dictionary in history:
		var links: Array[Dictionary] = []
		for link: Dictionary in EventCard.links_of(entry):
			if link["target"] != self_target:
				links.append(link)
		var para := LedgerText.paragraph(EventText.history_line(entry, -1), links, list_width, body)
		var date := EventText.history_date(entry, entry["era"] != era_shown)
		era_shown = entry["era"]
		var height: float = date_height + float(para["height"]) + ROW_GAP * u
		_rows.append({"kind": "event", "top": top, "height": height, "event": int(entry["id"]), "date": date, "seal": EventText.seal(entry["kind"]), "paragraph": para})
		top += height
	var members: Array = _info["members"]
	_member_rows = members.size()
	top += 4.0 * u
	_rows.append({"kind": "section", "top": top, "height": SECTION_HEIGHT * u, "text": Locale.t("sect_roster_title").format({"N": str(members.size())})})
	top += SECTION_HEIGHT * u
	if members.is_empty():
		var none := LedgerText.paragraph(Locale.t("sect_roster_none"), [] as Array[Dictionary], list_width, note)
		_rows.append({"kind": "note", "top": top, "height": float(none["height"]) + ROW_GAP * u, "paragraph": none})
		top += float(none["height"]) + ROW_GAP * u
	for member: Dictionary in members:
		var realm := int(member["realm"])
		var stage := Locale.t("realm_%d" % realm) if realm >= 0 else ""
		if member["is_leader"]:
			stage = (Locale.t("sect_leader_mark") + " · " + stage) if not stage.is_empty() else Locale.t("sect_leader_mark")
		_rows.append({"kind": "member", "top": top, "height": MEMBER_ROW * u, "person": int(member["id"]), "name": member["name"], "stage": stage})
		top += MEMBER_ROW * u
	_rows_height = top


func _fixed_height() -> float:
	return _fixed


## 卡片（含軸頭）至少要多高：固定部分加上清單的最低高度。
func min_height() -> float:
	return _fixed_height() + ROD * 2.0 * _unit + minf(_rows_height, MIN_LIST * _unit)


## 清單的高度：畫面放得下就全列，放不下縮到剩下的高度（至少 MIN_LIST）。
func _list_height() -> float:
	var spare := max_height - (_fixed_height() + ROD * 2.0 * _unit)
	return clampf(spare, minf(_rows_height, MIN_LIST * _unit), maxf(_rows_height, 1.0))


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

func _fly_rect() -> Rect2:
	return _action_rect((STRIP + 14.0) * _unit)


func _fly_width() -> float:
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	return ACTION_RADIUS * 2.0 * _unit + 8.0 * _unit + Ink.text_size(Locale.t("act_fly"), body).x + 8.0 * _unit + Ink.key_glyph_width(_center_key(), _unit)


func _back_rect() -> Rect2:
	return _action_rect((STRIP + 14.0) * _unit + _fly_width() + 16.0 * _unit)


# ── 繪製 ──────────────────────────────────────────────────────

func _draw_body(paper: Rect2) -> void:
	var u := _unit
	var x := paper.position.x + (STRIP + 14.0) * u
	var right := paper.end.x - 16.0 * u
	var section := Ink.font_size(Ink.SIZE_SECTION, u)
	var top := paper.position.y
	for block: Dictionary in _blocks:
		var y := top + float(block["y"])
		match String(block["kind"]):
			"para":
				LedgerText.draw(self, Vector2(x, y), block["paragraph"], block["color"], _hover, _pressed)
			"section":
				Ink.section(self, Vector2(x, y), block["text"], section)
			"stroke":
				Ink.stroke(self, Vector2(x, y), Vector2(right, y), 2.0 * u, Color(Ink.INK, 0.7))
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
			"para":
				LedgerText.draw(list, Vector2(0.0, top), row["paragraph"], row["color"], list.hover, list.pressed)
			"event":
				if list.hover == "event:%d" % int(row["event"]):
					list.draw_rect(Rect2(Vector2(-2.0, top), Vector2(list.size.x - 4.0 * u, height - ROW_GAP * u * 0.5)), Color(Ink.INK, 0.07), true)
				var seal_text: String = row["seal"]
				var seal_x := Ink.text_size(seal_text, note).x
				Ink.text(list, Vector2(0.0, top + (date_height - Ink.text_size(seal_text, note).y) * 0.5), seal_text, note, Ink.SEAL_RED)
				var date: String = row["date"]
				Ink.text(list, Vector2(seal_x + 6.0 * u, top + (date_height - Ink.text_size(date, note).y) * 0.5), date, note, Ink.INK_LIGHT)
				LedgerText.draw(list, Vector2(0.0, top + date_height), row["paragraph"], Ink.INK, list.hover, list.pressed)
				var base: float = top + height - ROW_GAP * u * 0.5
				list.draw_line(Vector2(0.0, base), Vector2(list.size.x - 8.0 * u, base), Color(Ink.INK_LIGHT, 0.22), 1.0)
			"member":
				_paint_member(list, row, body, note)


func _paint_member(list: LedgerList, row: Dictionary, body: int, note: int) -> void:
	var u := _unit
	var top: float = row["top"]
	var rh := MEMBER_ROW * u
	var width := list.size.x - 6.0 * u
	var target := "person:%d" % int(row["person"])
	var lit: bool = target == list.hover or target == list.pressed
	if lit:
		list.draw_rect(Rect2(Vector2(0.0, top + 1.0), Vector2(width, rh - 2.0)), Color(Ink.INK, 0.2 if target == list.pressed else 0.1), true)
	var label: String = row["name"]
	var name_size := Ink.text_size(label, body)
	Ink.text(list, Vector2(2.0 * u, top + (rh - name_size.y) * 0.5), label, body)
	list.draw_line(Vector2(2.0 * u, top + rh - 4.0 * u), Vector2(2.0 * u + name_size.x, top + rh - 4.0 * u), Color(Ink.SEAL_RED, 0.75 if lit else 0.45), 1.0)
	var stage: String = row["stage"]
	var extent := Ink.text_size(stage, note)
	Ink.text(list, Vector2(width - extent.x, top + (rh - extent.y) * 0.5), stage, note, Ink.INK_LIGHT)


# ── 點擊 ──────────────────────────────────────────────────────

func _row_target_at(point: Vector2) -> String:
	var date_height := LedgerText.line_height(Ink.font_size(Ink.SIZE_NOTE, _unit))
	for row: Dictionary in _rows:
		var top: float = row["top"]
		if point.y < top or point.y >= top + float(row["height"]):
			continue
		match String(row["kind"]):
			"para":
				return LedgerText.target_at(Vector2(0.0, top), row["paragraph"], point)
			"event":
				# 人名、地名、宗門名的連結優先，其餘部分是這一筆事件本身。
				var link := LedgerText.target_at(Vector2(0.0, top + date_height), row["paragraph"], point)
				return link if not link.is_empty() else "event:%d" % int(row["event"])
			"member":
				return "person:%d" % int(row["person"])
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


func _activate(target: String) -> void:
	if target == "fly":
		request_center()
	elif target == "back":
		back_requested.emit()
	else:
		_on_target(target)


func _target_at(pos: Vector2) -> String:
	if _info.is_empty() or _blocks.is_empty():
		return ""
	if _in_round(_close_rect(), pos):
		return "close"
	if _in_round(_fly_rect(), pos):
		return "fly"
	if can_back and _in_round(_back_rect(), pos):
		return "back"
	var paper := _paper_rect(_content_height())
	var origin := paper.position + Vector2((STRIP + 14.0) * _unit, 0.0)
	for block: Dictionary in _blocks:
		if block["kind"] != "para":
			continue
		var target := LedgerText.target_at(origin + Vector2(0.0, float(block["y"])), block["paragraph"], pos)
		if not target.is_empty():
			return target
	return ""
