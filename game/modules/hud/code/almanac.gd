class_name Almanac
extends Control
## GDD 5.10 天下冊：統計圖紙，整張全螢幕，版式與收合同傳聞簽的全螢幕。
## 紙上方是頁首與四頁籤（天下／修士／宗門／因果），下方左欄是圖表、右欄是名冊。
## 點左欄圖表的一項，右欄就列出構成那一項的州郡、人物、宗門或事件；名冊的名稱可點，州名飛往並開州郡卡、
## 人名開人物卡、宗門名飛往山門。資料在展開那一刻讀一次，展開期間不變（時間也已暫停）。
## 數字全來自橋接的唯讀統計查詢（stats_*、people_where 等），這裡不重算規則。

signal fullscreen_changed(on: bool)
signal province_requested(id: int)
signal person_requested(id: int)
signal sect_requested(map_pos: Vector2)
signal event_requested(id: int)

const PAGES: Array[String] = ["world", "people", "sect", "karma"]
const MARGIN := 24.0
const HEADER_HEIGHT := 46.0
const TAB_HEIGHT := 44.0
const SIDE := 22.0
const BOTTOM := 18.0
const COLUMN_GAP := 20.0
## 左欄占內文寬度的比例。
const LEFT_FRACTION := 0.56
const KEY_SCROLL_SPEED := 900.0
## 名冊最多列幾筆（總數照實寫在標題）。
const ROSTER_LIMIT := 600
const RECENT_YEARS := 100
const TOP_PEOPLE := 20
const PROVINCE_ROWS := 10
const SECT_ROWS := 20
const FALLEN_ROWS := 12
const ORIGIN_ROWS := 12
const BAR_STEP_GAP := 4.0
const SELECT_NONE := -1
const SELECT_DIM := -2
const SELECT_COLLAPSE := -3
const TAB_BASE := 100

## 近百年各類事件的鍵（橋接 recent_counts 的鍵）。
const RECENT_PEOPLE: Array[String] = ["tribulation_passed", "tribulation_failed", "ascended", "fell_to_demon", "fallen_high"]
const RECENT_SECT: Array[String] = ["sect_founded", "sect_split", "sect_war", "sect_fallen"]
const FAITH_COLORS: Array[Color] = [Ink.CYAN, Ink.GOLD, Ink.INK, Ink.INK_LIGHT]
const FAITH_KEYS: Array[String] = ["faith_dao", "faith_shi", "faith_ru", "almanac_faith_none"]
const WAY_COLORS: Array[Color] = [Ink.CYAN, Ink.SEAL_RED, Ink.GOLD]
const WAY_KEYS: Array[String] = ["almanac_way_righteous", "almanac_way_demonic", "almanac_way_beast"]
const AGE_KEYS: Array[String] = ["almanac_age_young", "almanac_age_adult", "almanac_age_old"]
const KARMA_KEYS: Array[String] = ["kind_feud", "kind_grace", "kind_love", "kind_mentor", "kind_grudge"]

## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像）。
var gamepad := false:
	set(value):
		gamepad = value
		if _left != null:
			_left.edge_scroll = value
			_right.edge_scroll = value
		queue_redraw()
## 是否展開為全螢幕。每次展開重讀資料；頁籤停在上次的一頁。
var expanded := false:
	set(value):
		if expanded == value:
			return
		expanded = value
		visible = value
		_hover = SELECT_NONE
		_pressed = SELECT_NONE
		if value:
			_reload()
		fullscreen_changed.emit(expanded)
		queue_redraw()
## 目前的一頁（PAGES 的序）。
var page := 0:
	set(value):
		value = clampi(value, 0, PAGES.size() - 1)
		if value == page:
			return
		page = value
		_pick = ""
		_roster = {}
		if _left != null:
			_left.scroll = 0.0
			_right.scroll = 0.0
			_relayout()
		queue_redraw()

var _sim: SimHost
var _unit := 1.0
var _left: LedgerList
var _right: LedgerList
var _hover := SELECT_NONE
var _pressed := SELECT_NONE
## 展開那一刻讀來的資料，依頁存放。
var _data := {}
var _date := ""
var _left_hits: Array[Dictionary] = []
var _right_hits: Array[Dictionary] = []
## 左欄選取的圖表項（目標字串，如 pick:realm:3）與它對應的名冊。
var _pick := ""
var _roster := {}
## 圖表項對應的名冊筆數（圖上寫了筆數的項才有）。
var _counts := {}
var _province_sort := "population"
var _sect_sort := "members"
## 名冊裡山門的座標，依宗門編號。
var _sect_pos := {}


func setup(sim: SimHost, unit: float) -> void:
	_sim = sim
	_unit = unit
	set_anchors_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	visible = false
	_left = LedgerList.new()
	_right = LedgerList.new()
	for list: LedgerList in [_left, _right]:
		add_child(list)
		list.setup(unit)
		list.activated.connect(_on_target)
	_left.painter = _paint_left
	_left.finder = func(point: Vector2) -> String: return AlmanacPen.target_at(_left_hits, point)
	_right.painter = _paint_right
	_right.finder = func(point: Vector2) -> String: return AlmanacPen.target_at(_right_hits, point)
	resized.connect(_layout_body)
	_layout_body()


# ── 版面 ─────────────────────────────────────────────────────

func _screen() -> Vector2:
	return get_viewport_rect().size if is_inside_tree() else Vector2(1920.0, 1080.0)


## 紙面：畫面四邊各留 MARGIN × 縮放。
func paper_rect() -> Rect2:
	var edge := Vector2.ONE * MARGIN * _unit
	return Rect2(edge, _screen() - edge * 2.0)


func header_rect() -> Rect2:
	var paper := paper_rect()
	return Rect2(paper.position, Vector2(paper.size.x, HEADER_HEIGHT * _unit))


func _tab_row() -> Rect2:
	var head := header_rect()
	return Rect2(Vector2(head.position.x, head.end.y), Vector2(head.size.x, TAB_HEIGHT * _unit))


func _body_rect() -> Rect2:
	var paper := paper_rect()
	var top := _tab_row().end.y + 6.0 * _unit
	var side := SIDE * _unit
	return Rect2(Vector2(paper.position.x + side, top), Vector2(paper.size.x - side * 2.0, paper.end.y - BOTTOM * _unit - top))


func _left_rect() -> Rect2:
	var body := _body_rect()
	var width := (body.size.x - COLUMN_GAP * _unit) * LEFT_FRACTION
	return Rect2(body.position, Vector2(width, body.size.y))


func _right_rect() -> Rect2:
	var body := _body_rect()
	var left := _left_rect()
	var x := left.end.x + COLUMN_GAP * _unit
	return Rect2(Vector2(x, body.position.y), Vector2(body.end.x - x, body.size.y))


func _layout_body() -> void:
	if _left == null:
		return
	var left := _left_rect()
	var right := _right_rect()
	_left.position = left.position
	_left.size = left.size
	_right.position = right.position
	_right.size = right.size
	_relayout()


func tab_label(index: int) -> String:
	return Locale.t("almanac_tab_" + PAGES[index])


func tab_rect(index: int) -> Rect2:
	var row := _tab_row()
	var font := Ink.font_size(Ink.SIZE_BODY, _unit)
	var pad := 22.0 * _unit
	var x := row.position.x + SIDE * _unit
	var height := row.size.y - 10.0 * _unit
	var rect := Rect2()
	for i in index + 1:
		var width := Ink.text_size(tab_label(i), font).x + pad * 2.0
		rect = Rect2(Vector2(x, row.position.y + (row.size.y - height) * 0.5), Vector2(width, height))
		x += width + 8.0 * _unit
	return rect


func _collapse_rect() -> Rect2:
	var head := header_rect()
	var font := Ink.font_size(Ink.SIZE_BODY, _unit)
	var width := Ink.text_size(Locale.t("almanac_collapse"), font).x + Ink.key_glyph_width(_collapse_key(), _unit) + 24.0 * _unit
	return Rect2(Vector2(head.end.x - SIDE * _unit - width, head.position.y), Vector2(width, head.size.y))


## 收合的鍵：鍵鼠是 B（再按一次），搖桿是 B 鍵（取消）。
func _collapse_key() -> String:
	if gamepad:
		return InputHints.label(InputActions.CANCEL, true)
	return InputHints.label(InputActions.ALMANAC, false)


func left_list() -> LedgerList:
	return _left


func right_list() -> LedgerList:
	return _right


# ── 資料 ─────────────────────────────────────────────────────

func _world() -> SimWorld:
	return _sim.world


## 展開時重讀：清掉各頁的快取與選取。
func _reload() -> void:
	_data.clear()
	_pick = ""
	_roster = {}
	_date = _world().date_short()
	_sect_pos.clear()
	_layout_body()
	_left.scroll = 0.0
	_right.scroll = 0.0
	_relayout()


func _page_data() -> Dictionary:
	var key: String = PAGES[page]
	if _data.has(key):
		return _data[key]
	var w := _world()
	var d := {}
	match key:
		"world":
			d["world"] = w.stats_world()
			d["high"] = w.stats_provinces(_province_sort, PROVINCE_ROWS, true)
			d["low"] = w.stats_provinces(_province_sort, PROVINCE_ROWS, false)
		"people":
			d["people"] = w.stats_people()
			d["recent"] = w.recent_counts(RECENT_YEARS)
			d["top"] = w.people_where("top", 0, TOP_PEOPLE)
		"sect":
			d["sects"] = w.stats_sects()
			d["recent"] = w.recent_counts(RECENT_YEARS)
			for row: Dictionary in d["sects"]["alive"]:
				_sect_pos[int(row["id"])] = row["pos"]
		"karma":
			d["karma"] = w.stats_karma()
			d["provinces"] = w.stats_provinces("karma_qi", PROVINCE_ROWS, true)
	_data[key] = d
	return d


func set_page(index: int) -> void:
	page = index


## 目前頁的名稱鍵（world、people、sect、karma）。
func page_key() -> String:
	return PAGES[page]


## 目前選取的圖表項；沒有為空字串。
func picked() -> String:
	return _pick


func roster() -> Dictionary:
	return _roster


## 名冊標題上寫的總數；沒有名冊為 -1。
func roster_total() -> int:
	return int(_roster.get("total", -1))


func roster_rows() -> Array:
	return _roster.get("rows", [])


## 圖表上某一項寫出的筆數（應等於名冊總數）；圖上沒寫筆數的項為 -1。
func chart_count(target: String) -> int:
	return int(_counts.get(target, -1))


## 左欄全部可點的目標。
func left_targets() -> Array[String]:
	var out: Array[String] = []
	for hit in _left_hits:
		out.append(hit["target"])
	return out


func right_targets() -> Array[String]:
	var out: Array[String] = []
	for hit in _right_hits:
		out.append(hit["target"])
	return out


## 某個目標在 Almanac 座標上的中心點（先把該欄捲到看得見）；找不到回傳 (-1, -1)。
func target_point(target: String, in_right := false) -> Vector2:
	var list := _right if in_right else _left
	var hits := _right_hits if in_right else _left_hits
	for hit in hits:
		if hit["target"] != target:
			continue
		var rect: Rect2
		if hit.has("rect"):
			rect = hit["rect"]
		else:
			var arc: Dictionary = hit["arc"]
			var mid: float = (float(arc["from"]) + float(arc["to"])) * 0.5
			var radius: float = (float(arc["inner"]) + float(arc["outer"])) * 0.5
			rect = Rect2((arc["center"] as Vector2) + Vector2.from_angle(mid) * radius, Vector2.ZERO)
		var center := rect.get_center()
		if center.y < list.scroll or center.y > list.scroll + list.size.y:
			list.scroll = center.y - list.size.y * 0.5
		return list.position + Vector2(center.x, center.y - list.scroll)
	return Vector2(-1.0, -1.0)


# ── 重排與繪製 ───────────────────────────────────────────────

## 量一次兩欄：記下可點的範圍與內容高度（不畫）。
func _relayout() -> void:
	if _left == null or _sim == null or not expanded:
		return
	_counts.clear()
	var left_pen := AlmanacPen.new(null, _unit)
	_left.content_height = _compose_left(left_pen, _left.size.x - 10.0 * _unit)
	_left_hits = left_pen.hits
	var right_pen := AlmanacPen.new(null, _unit)
	_right.content_height = _compose_right(right_pen, _right.size.x - 10.0 * _unit)
	_right_hits = right_pen.hits
	_left.queue_redraw()
	_right.queue_redraw()


func _paint_left(list: LedgerList) -> void:
	var pen := AlmanacPen.new(list, _unit)
	pen.hover = list.hover
	pen.selected = _pick
	_compose_left(pen, list.size.x - 10.0 * _unit)


func _paint_right(list: LedgerList) -> void:
	var pen := AlmanacPen.new(list, _unit)
	pen.hover = list.hover
	_compose_right(pen, list.size.x - 10.0 * _unit)


func _draw() -> void:
	if not expanded:
		return
	var font := Ink.font_size(Ink.SIZE_BODY, _unit)
	draw_rect(Rect2(Vector2.ZERO, size), Color(Ink.INK, 0.62), true)
	var paper := paper_rect()
	Ink.paper_slip(self, paper, Ink.CARD_BAND)
	var head := header_rect()
	var title_size := Ink.font_size(Ink.SIZE_TITLE, _unit)
	var title := "%s　%s" % [Locale.t("almanac_title"), _date]
	Ink.text(self, Vector2(head.position.x + SIDE * _unit, head.position.y + (head.size.y - Ink.text_size(title, title_size).y) * 0.5), title, title_size, Ink.INK)
	# 右端：收合鍵與提示。
	var collapse := _collapse_rect()
	if _hover == SELECT_COLLAPSE:
		draw_rect(collapse.grow(-1.0), Color(Ink.INK, 0.1 if _pressed != SELECT_COLLAPSE else 0.2), true)
	var label := Locale.t("almanac_collapse")
	var key := _collapse_key()
	var label_x := collapse.end.x - 8.0 * _unit - Ink.text_size(label, font).x
	Ink.text(self, Vector2(label_x, collapse.position.y + (collapse.size.y - Ink.text_size(label, font).y) * 0.5), label, font, Ink.INK)
	if not key.is_empty():
		Ink.key_glyph(self, Vector2(label_x - 8.0 * _unit - Ink.key_glyph_width(key, _unit), collapse.position.y + (collapse.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), key, _unit, not gamepad)
	# 頁籤一列。
	var row := _tab_row()
	for i in PAGES.size():
		var rect := tab_rect(i)
		var code := TAB_BASE + i
		var ink := Ink.plate(self, rect, i == page, true, _hover == code, _pressed == code)
		var text := tab_label(i)
		Ink.text(self, rect.get_center() - Ink.text_size(text, font) * 0.5, text, font, ink)
	# 換頁的鍵提示在頁籤列右端。
	var turn_key := Ink.PAD_DPAD_HORIZONTAL if gamepad else Ink.KEY_ARROWS_HORIZONTAL
	var turn_text := Locale.t("almanac_turn")
	var turn_x := row.end.x - SIDE * _unit - Ink.text_size(turn_text, font).x
	Ink.text(self, Vector2(turn_x, row.position.y + (row.size.y - Ink.text_size(turn_text, font).y) * 0.5), turn_text, font, Ink.INK_LIGHT)
	Ink.key_glyph(self, Vector2(turn_x - 8.0 * _unit - Ink.key_glyph_width(turn_key, _unit), row.position.y + (row.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), turn_key, _unit, not gamepad)
	draw_line(Vector2(head.position.x + SIDE * _unit, row.end.y), Vector2(head.end.x - SIDE * _unit, row.end.y), Color(Ink.GOLD, 0.7), 1.0)
	# 兩欄之間一道細線。
	var gap_x := _left_rect().end.x + COLUMN_GAP * _unit * 0.5
	var body := _body_rect()
	draw_line(Vector2(gap_x, body.position.y), Vector2(gap_x, body.end.y), Color(Ink.INK_LIGHT, 0.3), 1.0)


# ── 各頁的圖表 ───────────────────────────────────────────────

func _item(key: String, label: String, value: float, text: String, color: Color, count := -1) -> Dictionary:
	if count >= 0:
		_counts[key] = count
	return {"key": key, "label": label, "value": value, "text": text, "color": color}


func _gap(pen: AlmanacPen) -> float:
	return AlmanacPen.SECTION_GAP * _unit


func _title(pen: AlmanacPen, y: float, key: String) -> float:
	return y + pen.section(Vector2(0.0, y), Locale.t("almanac_" + key))


func _compose_left(pen: AlmanacPen, width: float) -> float:
	var d := _page_data()
	match PAGES[page]:
		"world":
			return _compose_world(pen, width, d)
		"people":
			return _compose_people(pen, width, d)
		"sect":
			return _compose_sect(pen, width, d)
		_:
			return _compose_karma(pen, width, d)


func _compose_world(pen: AlmanacPen, width: float, d: Dictionary) -> float:
	var w: Dictionary = d["world"]
	var y := 0.0
	# 總數。
	y = _title(pen, y, "total")
	y += pen.stats(Vector2(0.0, y), width, [
		{"label": Locale.t("card_population"), "value": Locale.count(w["population"])},
		{"label": Locale.t("card_capacity"), "value": Locale.count(w["capacity"])},
		{"label": Locale.t("almanac_alive"), "value": Locale.count(w["alive"])},
		{"label": Locale.t("almanac_provinces"), "value": str(w["provinces"])},
		{"label": Locale.t("almanac_years"), "value": Locale.t("almanac_years_value").format({"n": w["years"]})},
	]) + _gap(pen)
	# 大洲。
	y = _title(pen, y, "continent")
	var continents: Array = w["continents"]
	var items: Array = []
	for i in continents.size():
		var c: Dictionary = continents[i]
		var label: String = c["name"] if not String(c["name"]).is_empty() else Locale.t("almanac_isles")
		var text := "%s　%s" % [Locale.count(c["population"]), Locale.t("almanac_province_count").format({"n": c["provinces"]})]
		items.append(_item("pick:continent:%d" % i, label, c["population"], text, Ink.SILK, c["provinces"]))
	y += pen.bars(Vector2(0.0, y), width, items) + _gap(pen)
	# 州郡。
	y = _title(pen, y, "provinces_rank")
	y += pen.note(Vector2(0.0, y), Locale.t("almanac_sort_hint"), width) + 4.0 * _unit
	y += _province_table(pen, y, width, Locale.t("almanac_high"), d["high"]) + 8.0 * _unit
	y += _province_table(pen, y, width, Locale.t("almanac_low"), d["low"]) + _gap(pen)
	# 地形。
	y = _title(pen, y, "terrain")
	var terrain: PackedInt64Array = w["terrain"]
	var rings: Array = []
	for code in terrain.size():
		if terrain[code] <= 0:
			continue
		var key: String = TerrainSwatches.KEYS[code]
		rings.append({"key": "pick:terrain:%d" % code, "label": Locale.t(key), "value": float(terrain[code]), "color": TerrainSwatches.color(key)})
	y += pen.ring(Vector2(0.0, y), width, rings) + _gap(pen)
	# 教化。
	y = _title(pen, y, "faith")
	var faith: PackedFloat64Array = w["faith"]
	var faith_rings: Array = []
	for i in faith.size():
		faith_rings.append({"key": "pick:faith:%d" % i, "label": Locale.t(FAITH_KEYS[i]), "value": faith[i], "color": FAITH_COLORS[i]})
	y += pen.ring(Vector2(0.0, y), width, faith_rings) + _gap(pen)
	# 天地。
	y = _title(pen, y, "heaven")
	y += pen.stats(Vector2(0.0, y), width, [
		{"label": Locale.t("almanac_climate"), "value": Locale.t("climate_" + String(w["climate"]))},
		{"label": Locale.t("almanac_load"), "value": "%s / %s" % [Locale.count(w["load"]), Locale.count(w["limit"])]},
		{"label": Locale.t("almanac_cataclysm"), "value": _cataclysm_text(w)},
	])
	return y + _gap(pen)


func _cataclysm_text(w: Dictionary) -> String:
	if w["in_cataclysm"]:
		return Locale.t("almanac_cataclysm_now")
	if int(w["years_since_cataclysm"]) < 0:
		return Locale.t("almanac_cataclysm_never")
	return Locale.t("almanac_years_value").format({"n": w["years_since_cataclysm"]})


## 州郡排行的一塊：名稱、人口與四項氣數；表頭可點，依該欄由大到小重排。
func _province_table(pen: AlmanacPen, y: float, width: float, caption: String, block: Dictionary) -> float:
	var rows: Array = []
	for r: Dictionary in block["rows"]:
		rows.append({"cells": [
			{"text": r["name"], "target": "go:province:%d" % r["id"]},
			{"text": Locale.count(r["population"])},
			{"text": _pct(r["aura"])},
			{"text": _pct(r["karma_qi"])},
			{"text": _pct(r["merit"])},
			{"text": _pct(r["righteous"])},
		]})
	var h := pen.note(Vector2(0.0, y), caption, width)
	return h + 2.0 * _unit + pen.table(Vector2(0.0, y + h + 2.0 * _unit), width, _province_columns(), rows, "sort:" + _province_sort)


func _province_columns() -> Array:
	return [
		{"label": Locale.t("almanac_province_name")},
		{"label": Locale.t("card_population"), "sort": "sort:population", "align": "right"},
		{"label": Locale.t("card_aura"), "sort": "sort:aura", "align": "right"},
		{"label": Locale.t("card_karma_qi"), "sort": "sort:karma_qi", "align": "right"},
		{"label": Locale.t("card_merit"), "sort": "sort:merit", "align": "right"},
		{"label": Locale.t("card_righteous"), "sort": "sort:righteous", "align": "right"},
	]


func _pct(value: float) -> String:
	return str(int(round(value * 100.0)))


func _compose_people(pen: AlmanacPen, width: float, d: Dictionary) -> float:
	var p: Dictionary = d["people"]
	var y := 0.0
	# 境界：由上往下大乘到煉氣，最底是尚未修行的人。
	y = _title(pen, y, "realm")
	var realms: PackedInt32Array = p["realms"]
	var layers: Array = []
	for r in range(realms.size() - 1, -1, -1):
		var tint := Ink.SILK.lerp(Ink.SEAL_RED, float(r) / float(maxi(realms.size() - 1, 1)))
		layers.append(_item("pick:realm:%d" % r, Locale.t("realm_%d" % r), realms[r], str(realms[r]), tint, realms[r]))
	layers.append(_item("pick:realm:-1", Locale.t("almanac_non_cultivator"), p["non_cultivators"], str(p["non_cultivators"]), Ink.INK_LIGHT, p["non_cultivators"]))
	y += pen.pyramid(Vector2(0.0, y), width, layers) + _gap(pen)
	# 出身。
	y = _title(pen, y, "origin")
	# 人數多的在前（散修與無門無派的凡俗也各占一列）。
	var origins := _sorted_origins(p["origins"])
	var bars: Array = []
	var other := 0
	for i in origins.size():
		var o: Dictionary = origins[i]
		if i < ORIGIN_ROWS:
			var label: String = _origin_name(o)
			bars.append(_item("pick:origin:%d" % o["origin"], label, o["count"], str(o["count"]), Ink.SILK, o["count"]))
		else:
			other += int(o["count"])
	if other > 0:
		bars.append(_item("pick:origin:other", Locale.t("almanac_other"), other, str(other), Ink.INK_LIGHT, other))
	y += pen.bars(Vector2(0.0, y), width, bars) + _gap(pen)
	# 男女與年紀。
	y = _title(pen, y, "gender_age")
	var half := (width - 12.0 * _unit) * 0.5
	var sexes := [
		{"key": "pick:gender:0", "label": Locale.t("male"), "value": float(p["male"]), "color": Ink.SILK},
		{"key": "pick:gender:1", "label": Locale.t("female"), "value": float(p["female"]), "color": Ink.SEAL_RED},
	]
	_counts["pick:gender:0"] = int(p["male"])
	_counts["pick:gender:1"] = int(p["female"])
	var ages: PackedInt32Array = p["ages"]
	var age_colors: Array[Color] = [Ink.CYAN, Ink.SILK, Ink.INK_LIGHT]
	var age_rings: Array = []
	for i in ages.size():
		age_rings.append({"key": "pick:age:%d" % i, "label": Locale.t(AGE_KEYS[i]), "value": float(ages[i]), "color": age_colors[i]})
		_counts["pick:age:%d" % i] = ages[i]
	var h1 := pen.ring(Vector2(0.0, y), half, sexes)
	var h2 := pen.ring(Vector2(half + 12.0 * _unit, y), half, age_rings)
	y += maxf(h1, h2) + 4.0 * _unit
	y += pen.note(Vector2(0.0, y), Locale.t("almanac_age_line").format({"mean": int(round(p["age_mean"])), "eldest": p["eldest_age"]}), width) + _gap(pen)
	# 道途。
	y = _title(pen, y, "way")
	var ways: PackedInt32Array = p["ways"]
	var way_bars: Array = []
	for i in ways.size():
		way_bars.append(_item("pick:way:%d" % i, Locale.t(WAY_KEYS[i]), ways[i], str(ways[i]), WAY_COLORS[i], ways[i]))
	way_bars.append(_item("pick:way:3", Locale.t("almanac_way_fallen"), p["fallen_to_demon"], str(p["fallen_to_demon"]), Ink.KARMA, p["fallen_to_demon"]))
	y += pen.bars(Vector2(0.0, y), width, way_bars) + _gap(pen)
	# 高人：境界最高的二十位。
	y = _title(pen, y, "top")
	var top_rows: Array = []
	for r: Dictionary in d["top"]["rows"]:
		top_rows.append({"cells": [
			{"text": r["name"], "target": "go:person:%d" % r["id"]},
			{"text": _realm_text(r)},
			{"text": Locale.t("almanac_age_value").format({"n": r["age"]})},
			{"text": _identity_text(r)},
		]})
	y += pen.table(Vector2(0.0, y), width, _person_columns(), top_rows) + _gap(pen)
	# 近百年。
	y = _title(pen, y, "recent")
	y += _recent_bars(pen, y, width, d["recent"], RECENT_PEOPLE) + _gap(pen)
	return y


## 出身依人數由多到少（同數依原順序）。
func _sorted_origins(origins: Array) -> Array:
	var out := origins.duplicate()
	out.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a["count"] > b["count"])
	return out


func _origin_name(o: Dictionary) -> String:
	match int(o["origin"]):
		-1:
			return Locale.t("identity_loose")
		-2:
			return Locale.t("almanac_no_sect")
	return o["name"]


func _recent_bars(pen: AlmanacPen, y: float, width: float, counts: Dictionary, keys: Array[String]) -> float:
	var items: Array = []
	for key in keys:
		var n := int(counts.get(key, 0))
		items.append(_item("pick:recent:" + key, Locale.t("almanac_recent_" + key), n, str(n), Ink.SEAL_RED if key.contains("fall") or key.contains("war") or key.contains("fail") else Ink.SILK, n))
	var h := pen.note(Vector2(0.0, y), Locale.t("almanac_recent_note"), width) + 4.0 * _unit
	return h + pen.bars(Vector2(0.0, y + h), width, items)


func _person_columns() -> Array:
	return [
		{"label": Locale.t("almanac_col_name")},
		{"label": Locale.t("almanac_col_realm")},
		{"label": Locale.t("almanac_col_age"), "align": "right"},
		{"label": Locale.t("almanac_col_identity")},
	]


func _realm_text(r: Dictionary) -> String:
	if int(r["realm"]) >= 0:
		return Locale.t("realm_%d" % int(r["realm"]))
	var key := "role_" + String(r["role"])
	return Locale.t(key) if Locale.has(key) else Locale.t("almanac_non_cultivator")


func _identity_text(r: Dictionary) -> String:
	if not String(r["power_name"]).is_empty():
		return r["power_name"]
	return Locale.t("identity_loose") if int(r["realm"]) >= 0 else "—"


func _compose_sect(pen: AlmanacPen, width: float, d: Dictionary) -> float:
	var s: Dictionary = d["sects"]
	var y := 0.0
	# 宗門榜。
	y = _title(pen, y, "sect_rank")
	var alive: Array = (s["alive"] as Array).duplicate()
	alive.sort_custom(_sect_before)
	var rows: Array = []
	for i in mini(alive.size(), SECT_ROWS):
		var r: Dictionary = alive[i]
		var tone: Color = Ink.SEAL_RED if r["evil"] else Ink.INK
		rows.append({"cells": [
			{"text": r["name"], "target": "go:sect:%d" % r["id"], "color": tone},
			{"text": str(r["members"]), "color": tone},
			{"text": Locale.t("realm_%d" % int(r["top_realm"])) if int(r["top_realm"]) >= 0 else "—", "color": tone},
			{"text": r["leader_name"] if int(r["leader"]) >= 0 else "—", "target": "go:person:%d" % r["leader"] if int(r["leader"]) >= 0 else "", "color": tone},
			{"text": Locale.t("almanac_years_value").format({"n": r["age_years"]}), "color": tone},
			{"text": r["province_name"], "target": "go:province:%d" % r["province"], "color": tone},
		]})
	y += pen.note(Vector2(0.0, y), Locale.t("almanac_sect_hint").format({"n": alive.size()}), width) + 4.0 * _unit
	y += pen.table(Vector2(0.0, y), width, [
		{"label": Locale.t("almanac_col_sect")},
		{"label": Locale.t("almanac_col_members"), "sort": "sort:members", "align": "right"},
		{"label": Locale.t("almanac_col_top_realm"), "sort": "sort:top_realm"},
		{"label": Locale.t("almanac_col_leader")},
		{"label": Locale.t("almanac_col_since"), "sort": "sort:age_years", "align": "right"},
		{"label": Locale.t("almanac_col_seat")},
	], rows, "sort:" + _sect_sort) + _gap(pen)
	# 勢力分布：各大洲有幾個宗門。
	y = _title(pen, y, "sect_power")
	var power: Array = []
	for r: Dictionary in s["by_continent"]:
		var label: String = r["name"] if not String(r["name"]).is_empty() else Locale.t("almanac_isles")
		power.append(_item("pick:sectcont:%d" % r["index"], label, r["count"], str(r["count"]), Ink.SILK, r["count"]))
	y += pen.bars(Vector2(0.0, y), width, power) + _gap(pen)
	# 興亡：已覆滅的宗門。
	y = _title(pen, y, "fallen")
	var fallen: Array = s["fallen"]
	if fallen.is_empty():
		y += pen.note(Vector2(0.0, y), Locale.t("almanac_fallen_none"), width)
	else:
		var frows: Array = []
		for i in mini(fallen.size(), FALLEN_ROWS):
			var f: Dictionary = fallen[i]
			var target := "go:event:%d" % f["event"] if int(f["event"]) >= 0 else ""
			frows.append({"cells": [
				{"text": f["name"], "target": target, "color": Ink.SEAL_RED if f["evil"] else Ink.INK},
				{"text": Locale.t("almanac_year_value").format({"n": f["founded_year"]})},
				{"text": Locale.t("almanac_year_value").format({"n": f["fallen_year"]})},
				{"text": Locale.t("almanac_years_value").format({"n": f["lasted_years"]})},
				{"text": Locale.t("almanac_cause_war") if f["cause_kind"] == "SectWar" else Locale.t("almanac_cause_scatter")},
			]})
		y += pen.note(Vector2(0.0, y), Locale.t("almanac_fallen_hint").format({"n": fallen.size()}), width) + 4.0 * _unit
		y += pen.table(Vector2(0.0, y), width, [
			{"label": Locale.t("almanac_col_sect")},
			{"label": Locale.t("almanac_col_founded"), "align": "right"},
			{"label": Locale.t("almanac_col_fell"), "align": "right"},
			{"label": Locale.t("almanac_col_lasted"), "align": "right"},
			{"label": Locale.t("almanac_col_cause")},
		], frows)
	y += _gap(pen)
	# 近百年。
	y = _title(pen, y, "recent")
	y += _recent_bars(pen, y, width, d["recent"], RECENT_SECT) + _gap(pen)
	return y


func _sect_before(a: Dictionary, b: Dictionary) -> bool:
	if _sect_sort == "top_realm":
		if a["top_realm"] != b["top_realm"]:
			return a["top_realm"] > b["top_realm"]
	elif _sect_sort == "age_years":
		if a["age_years"] != b["age_years"]:
			return a["age_years"] > b["age_years"]
	if a["members"] != b["members"]:
		return a["members"] > b["members"]
	return a["id"] < b["id"]


func _compose_karma(pen: AlmanacPen, width: float, d: Dictionary) -> float:
	var k: Dictionary = d["karma"]
	var y := 0.0
	# 未了宿緣：五類的條數。
	y = _title(pen, y, "threads")
	var counts: PackedInt32Array = k["counts"]
	var bars: Array = []
	for i in counts.size():
		bars.append(_item("pick:thread:%d" % i, Locale.t(KARMA_KEYS[i]), counts[i], str(counts[i]), KarmaLayer.KIND_COLORS[i], counts[i]))
	y += pen.bars(Vector2(0.0, y), width, bars) + _gap(pen)
	# 牽連最深：身上未了宿緣最多的魂。
	y = _title(pen, y, "tangled")
	var rows: Array = []
	for r: Dictionary in k["tangled"]:
		var named := int(r["person"]) >= 0
		rows.append({"cells": [
			{"text": r["name"] if named else "—", "target": "go:person:%d" % r["person"] if named else ""},
			{"text": str(r["open"]), "align": "right"},
			{"text": "%d" % int(round(float(r["strength"]))), "align": "right"},
		]})
	y += pen.table(Vector2(0.0, y), width, [
		{"label": Locale.t("almanac_col_name")},
		{"label": Locale.t("almanac_col_open"), "align": "right"},
		{"label": Locale.t("almanac_col_strength"), "align": "right"},
	], rows) + _gap(pen)
	# 業氣最重的州郡。
	y = _title(pen, y, "karma_qi")
	var prows: Array = []
	for r: Dictionary in d["provinces"]["rows"]:
		prows.append({"cells": [
			{"text": r["name"], "target": "go:province:%d" % r["id"]},
			{"text": _pct(r["karma_qi"]), "align": "right"},
			{"text": _pct(r["righteous"]), "align": "right"},
		]})
	y += pen.table(Vector2(0.0, y), width, [
		{"label": Locale.t("almanac_province_name")},
		{"label": Locale.t("card_karma_qi"), "align": "right"},
		{"label": Locale.t("card_righteous"), "align": "right"},
	], prows) + _gap(pen)
	# 歲時：尚未開放。
	y = _title(pen, y, "chronicle")
	y += pen.note(Vector2(0.0, y), Locale.t("almanac_chronicle_closed"), width)
	return y + _gap(pen)


# ── 名冊 ─────────────────────────────────────────────────────

func _compose_right(pen: AlmanacPen, width: float) -> float:
	if _roster.is_empty():
		return pen.note(Vector2(0.0, 0.0), Locale.t("almanac_roster_hint"), width)
	var y := 0.0
	var title: String = "%s　%s" % [_roster["title"], Locale.t("almanac_roster_total").format({"n": _roster["total"], "unit": _roster["unit"]})]
	y += pen.section(Vector2(0.0, y), title)
	var rows: Array = _roster["rows"]
	var table_rows: Array = []
	var columns: Array = []
	match String(_roster["type"]):
		"province":
			columns = [
				{"label": Locale.t("almanac_province_name")},
				{"label": Locale.t("card_population"), "align": "right"},
				{"label": Locale.t("card_aura"), "align": "right"},
				{"label": Locale.t("card_karma_qi"), "align": "right"},
				{"label": Locale.t("card_merit"), "align": "right"},
				{"label": Locale.t("card_righteous"), "align": "right"},
			]
			for r: Dictionary in rows:
				table_rows.append({"cells": [
					{"text": r["name"], "target": "go:province:%d" % r["id"]},
					{"text": Locale.count(r["population"])}, {"text": _pct(r["aura"])}, {"text": _pct(r["karma_qi"])},
					{"text": _pct(r["merit"])}, {"text": _pct(r["righteous"])},
				]})
		"person":
			columns = _person_columns()
			columns.append({"label": Locale.t("almanac_col_seat")})
			for r: Dictionary in rows:
				table_rows.append({"cells": [
					{"text": r["name"], "target": "go:person:%d" % r["id"]},
					{"text": _realm_text(r)},
					{"text": Locale.t("almanac_age_value").format({"n": r["age"]})},
					{"text": _identity_text(r)},
					{"text": r["province_name"], "target": "go:province:%d" % r["province"]},
				]})
		"sect":
			columns = [
				{"label": Locale.t("almanac_col_sect")},
				{"label": Locale.t("almanac_col_members"), "align": "right"},
				{"label": Locale.t("almanac_col_top_realm")},
				{"label": Locale.t("almanac_col_leader")},
				{"label": Locale.t("almanac_col_seat")},
			]
			for r: Dictionary in rows:
				var tone: Color = Ink.SEAL_RED if r["evil"] else Ink.INK
				table_rows.append({"cells": [
					{"text": r["name"], "target": "go:sect:%d" % r["id"], "color": tone},
					{"text": str(r["members"])},
					{"text": Locale.t("realm_%d" % int(r["top_realm"])) if int(r["top_realm"]) >= 0 else "—"},
					{"text": r["leader_name"] if int(r["leader"]) >= 0 else "—", "target": "go:person:%d" % r["leader"] if int(r["leader"]) >= 0 else ""},
					{"text": r["province_name"], "target": "go:province:%d" % r["province"]},
				]})
		"event":
			columns = [{"label": Locale.t("almanac_col_event")}, {"label": Locale.t("almanac_col_when"), "align": "right"}]
			for r: Dictionary in rows:
				table_rows.append({"target": "go:event:%d" % r["id"], "cells": [{"text": r["line"]}, {"text": r["when"]}]})
		"thread":
			columns = [
				{"label": Locale.t("almanac_col_owes")},
				{"label": Locale.t("almanac_col_owed")},
				{"label": Locale.t("almanac_col_lives"), "align": "right"},
				{"label": Locale.t("almanac_col_strength"), "align": "right"},
			]
			for r: Dictionary in rows:
				table_rows.append({"cells": [
					_person_cell(int(r["debtor"]), r["debtor_name"]),
					_person_cell(int(r["creditor"]), r["creditor_name"]),
					{"text": str(r["lives"])},
					{"text": "%d" % int(round(float(r["strength"])))},
				]})
	y += 4.0 * _unit
	if table_rows.is_empty():
		return y + pen.note(Vector2(0.0, y), Locale.t("almanac_roster_empty"), width) + AlmanacPen.SECTION_GAP * _unit
	y += pen.table(Vector2(0.0, y), width, columns, table_rows)
	if int(_roster["total"]) > rows.size():
		y += 6.0 * _unit + pen.note(Vector2(0.0, y + 6.0 * _unit), Locale.t("almanac_roster_more").format({"n": rows.size(), "unit": _roster["unit"]}), width)
	return y + AlmanacPen.SECTION_GAP * _unit


func _person_cell(id: int, who: String) -> Dictionary:
	return {"text": who if id >= 0 else "—", "target": "go:person:%d" % id if id >= 0 else ""}


## 依圖表項的目標字串列出名冊。
func _build_roster(target: String) -> void:
	var parts := target.split(":")
	var kind := parts[1]
	var value := parts[2]
	var w := _world()
	var label := _label_of(target)
	var d := _page_data()
	match kind:
		"continent", "terrain", "faith":
			var r: Dictionary = w.province_roster(kind, int(value), ROSTER_LIMIT)
			_roster = {"type": "province", "title": label, "total": r["total"], "rows": r["rows"], "unit": Locale.t("almanac_unit_province")}
		"realm", "gender", "age", "way":
			var r: Dictionary = w.people_where(kind, int(value), ROSTER_LIMIT)
			_roster = {"type": "person", "title": label, "total": r["total"], "rows": r["rows"], "unit": Locale.t("almanac_unit_person")}
		"origin":
			if value == "other":
				_roster = _other_origin_roster(label, _sorted_origins(d["people"]["origins"]))
			else:
				var r: Dictionary = w.people_where("origin", int(value), ROSTER_LIMIT)
				_roster = {"type": "person", "title": label, "total": r["total"], "rows": r["rows"], "unit": Locale.t("almanac_unit_person")}
		"recent":
			var r: Dictionary = w.recent_events(value, RECENT_YEARS, ROSTER_LIMIT)
			var rows: Array = []
			for id in r["ids"]:
				var event: Dictionary = w.event(id)
				if event.is_empty():
					continue
				rows.append({"id": id, "line": EventText.line(event), "when": EventText.stamp(int(event.get("year", 0)), EventText.month_of(event))})
			_roster = {"type": "event", "title": label, "total": r["total"], "rows": rows, "unit": Locale.t("almanac_unit_event")}
		"sectcont":
			var rows: Array = []
			for r: Dictionary in d["sects"]["alive"]:
				if int(r["continent"]) == int(value):
					rows.append(r)
			rows.sort_custom(_sect_before)
			_roster = {"type": "sect", "title": label, "total": rows.size(), "rows": rows, "unit": Locale.t("almanac_unit_sect")}
		"thread":
			var r: Dictionary = w.karma_roster(int(value))
			_roster = {"type": "thread", "title": label, "total": r["total"], "rows": r["rows"], "unit": Locale.t("almanac_unit_thread")}
	_right.scroll = 0.0


## 「其他」出身：排在前十二名之後的各宗門合併，依修為排序。
func _other_origin_roster(label: String, origins: Array) -> Dictionary:
	var total := 0
	var merged: Array = []
	for i in range(ORIGIN_ROWS, origins.size()):
		var r: Dictionary = _world().people_where("origin", int(origins[i]["origin"]), ROSTER_LIMIT)
		total += int(r["total"])
		merged.append_array(r["rows"])
	merged.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		if a["realm"] != b["realm"]:
			return a["realm"] > b["realm"]
		return a["id"] < b["id"])
	return {"type": "person", "title": label, "total": total, "rows": merged.slice(0, ROSTER_LIMIT), "unit": Locale.t("almanac_unit_person")}


## 圖表項的名稱，作名冊標題。
func _label_of(target: String) -> String:
	var parts := target.split(":")
	var d := _page_data()
	match parts[1]:
		"continent":
			var c: Dictionary = (d["world"]["continents"] as Array)[int(parts[2])]
			return c["name"] if not String(c["name"]).is_empty() else Locale.t("almanac_isles")
		"terrain":
			return Locale.t(TerrainSwatches.KEYS[int(parts[2])])
		"faith":
			return Locale.t(FAITH_KEYS[int(parts[2])])
		"realm":
			return Locale.t("realm_%s" % parts[2]) if int(parts[2]) >= 0 else Locale.t("almanac_non_cultivator")
		"gender":
			return Locale.t("female" if int(parts[2]) == 1 else "male")
		"age":
			return Locale.t(AGE_KEYS[int(parts[2])])
		"way":
			return Locale.t(WAY_KEYS[int(parts[2])]) if int(parts[2]) < 3 else Locale.t("almanac_way_fallen")
		"origin":
			if parts[2] == "other":
				return Locale.t("almanac_other")
			for o: Dictionary in d["people"]["origins"]:
				if int(o["origin"]) == int(parts[2]):
					return _origin_name(o)
		"recent":
			return Locale.t("almanac_recent_" + parts[2])
		"sectcont":
			for r: Dictionary in d["sects"]["by_continent"]:
				if int(r["index"]) == int(parts[2]):
					return r["name"] if not String(r["name"]).is_empty() else Locale.t("almanac_isles")
		"thread":
			return Locale.t(KARMA_KEYS[int(parts[2])])
	return target


# ── 操作 ─────────────────────────────────────────────────────

## 左右欄點到的目標：選取圖表項、重排，或前往州郡、人物、宗門、事件。
func activate_target(target: String) -> void:
	var parts := target.split(":")
	match parts[0]:
		"pick":
			_pick = target
			_build_roster(target)
			_relayout()
			queue_redraw()
		"sort":
			_sort_by(parts[1])
		"go":
			match parts[1]:
				"province":
					province_requested.emit(int(parts[2]))
				"person":
					person_requested.emit(int(parts[2]))
				"event":
					event_requested.emit(int(parts[2]))
				"sect":
					var id := int(parts[2])
					if _sect_pos.has(id):
						sect_requested.emit(_sect_pos[id])


func _on_target(target: String) -> void:
	activate_target(target)


func _sort_by(key: String) -> void:
	if PAGES[page] == "world":
		if key == _province_sort:
			return
		_province_sort = key
		_data.erase("world")
		_page_data()
	elif PAGES[page] == "sect":
		_sect_sort = key
	_relayout()
	queue_redraw()


func _select_at(point: Vector2) -> int:
	if not expanded:
		return SELECT_NONE
	if _collapse_rect().has_point(point):
		return SELECT_COLLAPSE
	for i in PAGES.size():
		if tab_rect(i).has_point(point):
			return TAB_BASE + i
	if not paper_rect().has_point(point):
		return SELECT_DIM
	return SELECT_NONE


func _has_point(_point: Vector2) -> bool:
	return expanded


## 游標停在這裡時是否可以按（GDD 5.9）：頁籤、收合與紙外的暗處；兩欄各自回答。
func is_hot(point: Vector2) -> bool:
	return _select_at(point) != SELECT_NONE


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var at := _select_at((event as InputEventMouseMotion).position)
		if at != _hover:
			_hover = at
			queue_redraw()
	elif event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		if button.button_index != MOUSE_BUTTON_LEFT:
			accept_event()
			return
		var at := _select_at(button.position)
		if button.pressed:
			_pressed = at
			queue_redraw()
		else:
			var released := _pressed
			_pressed = SELECT_NONE
			queue_redraw()
			if released != SELECT_NONE and released == at:
				_activate(at)
		accept_event()


func _activate(select: int) -> void:
	if select == SELECT_DIM or select == SELECT_COLLAPSE:
		expanded = false
	elif select >= TAB_BASE:
		set_page(select - TAB_BASE)


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and (_hover != SELECT_NONE or _pressed != SELECT_NONE):
		_hover = SELECT_NONE
		_pressed = SELECT_NONE
		queue_redraw()


## 鍵盤、十字鍵與左搖桿上下捲游標所在的那一欄；左右換頁。
func _process(delta: float) -> void:
	if not expanded:
		return
	var axis := Input.get_axis(InputActions.MENU_UP, InputActions.MENU_DOWN)
	if axis != 0.0:
		_column_under_pointer().scroll += axis * KEY_SCROLL_SPEED * _unit * delta
	if Input.is_action_just_pressed(InputActions.MENU_LEFT):
		set_page(page - 1)
	elif Input.is_action_just_pressed(InputActions.MENU_RIGHT):
		set_page(page + 1)


func _column_under_pointer() -> LedgerList:
	var point := get_local_mouse_position()
	if _right.get_rect().has_point(point):
		return _right
	return _left


func _unhandled_input(event: InputEvent) -> void:
	if not expanded or not (event is InputEventKey):
		return
	var key_event := event as InputEventKey
	if not key_event.pressed:
		return
	var list := _column_under_pointer()
	var page_step := list.size.y - 40.0 * _unit
	match key_event.keycode:
		KEY_PAGEDOWN:
			list.scroll += page_step
		KEY_PAGEUP:
			list.scroll -= page_step
		KEY_HOME:
			list.scroll = 0.0
		KEY_END:
			list.scroll_to_end()
		_:
			return
	get_viewport().set_input_as_handled()
