class_name ProvinceCard
extends LedgerCard
## GDD 5.8 州郡卡（命簿）：題簽寫州郡名；顯示治所、地形組成、民生、氣場墨條、三教教化與在此的有名之人。
## 頁尾的置中鈕把鏡頭移到治所，右上的關閉鈕收卷——兩個鈕都是圖示加文字加實體鍵圖像。
## 值為零的墨條與沒有教化時的教化環不畫，卡片高度跟著內容走；有名之人一節超過畫面時可捲動。

signal center_requested(map_pos: Vector2)
## 點了「在此的有名之人」的一位。
signal person_requested(person: int)

## 低於這個值的氣場視為零，不畫墨條。
const BAR_EPSILON := 0.005
const BARS := [
	["card_aura", "aura", Ink.CYAN],
	["card_karma_qi", "karma_qi", Ink.KARMA],
	["card_merit", "merit", Ink.GOLD],
	["card_righteous", "righteous", Ink.INK],
]
const FAITHS := [["faith_dao", Ink.CYAN], ["faith_shi", Ink.GOLD], ["faith_ru", Ink.INK]]
## 有名之人一節最多列幾位，與每列的高度。
const PEOPLE_LIMIT := 10
const PERSON_ROW := 26.0
## 清單至少留幾列（畫面再矮也不低於這個）。
const MIN_PEOPLE_ROWS := 2

var province := -1

var _info: Dictionary = {}
var _people: Array[Dictionary] = []
var _more := 0
var _list: LedgerList


func setup(world: SimWorld, unit: float, width := -1.0) -> void:
	super.setup(world, unit, width)
	_list = LedgerList.new()
	add_child(_list)
	_list.setup(unit)
	_list.painter = _paint_people
	_list.finder = _person_at
	_list.activated.connect(_on_list_activated)
	_list.visible = false


func show_province(id: int) -> void:
	if id < 0:
		close()
		return
	province = id
	refresh()
	_begin_open()


func _forget() -> void:
	province = -1


## 重新讀取州郡數值與有名之人（每月推進後呼叫）。
func refresh() -> void:
	if province >= 0:
		_info = _world.province_info(province)
		var rows: Array = _world.people_in(province)
		_more = maxi(rows.size() - PEOPLE_LIMIT, 0)
		_people.clear()
		for i in mini(rows.size(), PEOPLE_LIMIT):
			_people.append(rows[i])
		custom_minimum_size.y = _content_height() + ROD * 2.0 * _unit
		size = custom_minimum_size
		queue_redraw()


## 頁尾動作「置中」：把鏡頭移到治所（滑鼠點鈕、鍵盤 C、搖桿 X 共用）。
func request_center() -> void:
	if is_open() and not _info.is_empty():
		center_requested.emit(_info["seat"])


func _has_content() -> bool:
	return not _info.is_empty()


func _title() -> String:
	return _info["name"]


## 要畫的墨條（字串鍵）：靈氣一律顯示，其餘只在值不為零時顯示。
func _bar_keys() -> Array[String]:
	var keys: Array[String] = []
	for entry: Array in BARS:
		if entry[0] == "card_aura" or float(_info[entry[1]]) >= BAR_EPSILON:
			keys.append(entry[0])
	return keys


func _shows_faith_ring() -> bool:
	var faith: Vector3 = _info["faith"]
	return faith.x + faith.y + faith.z >= BAR_EPSILON


## 有名之人清單的內容高度（含「另有 n 人」一列）。
func _people_content_height() -> float:
	var rows := _people.size() + (1 if _more > 0 else 0)
	return maxf(rows, 1) * PERSON_ROW * _unit


## 清單區實際的高度：畫面放得下就全列，放不下縮到剩下的高度（至少 MIN_PEOPLE_ROWS 列）。
func _list_height() -> float:
	var all := _people_content_height()
	var spare := max_height - (_fixed_height() + ROD * 2.0 * _unit)
	var floor_rows := minf(all, MIN_PEOPLE_ROWS * PERSON_ROW * _unit)
	return clampf(spare, floor_rows, all)


## 教化環畫不畫：有教化，而且畫面高度放得下（放不下時省下環，讓有名之人清單至少留兩列）。
func _ring_shown() -> bool:
	if not _shows_faith_ring():
		return false
	var with_ring := _fixed_height(true) + ROD * 2.0 * _unit + MIN_PEOPLE_ROWS * PERSON_ROW * _unit
	return with_ring <= max_height or _people.is_empty()


## 清單以外的內容高度（含綾邊）。
func _fixed_height(with_ring := false) -> float:
	var rows := 14.0 + 34.0 + 12.0 + 28.0 + 26.0 + 32.0 + 26.0 * _bar_keys().size() + 8.0 + 26.0
	if with_ring or (_shows_faith_ring() and _ring_shown()):
		rows += 34.0
	# 有名之人一節：節名一列，清單之後留一點空白。
	rows += 6.0 + 26.0
	return (rows + 8.0 + 12.0 + ACTION_RADIUS * 2.0 + 14.0) * _unit + Ink.CARD_BAND * 2.0


## 兩根軸頭之間的高度。
func _content_height() -> float:
	return _fixed_height() + _list_height()


## 置中鈕的外接方框。
func _center_rect() -> Rect2:
	return _action_rect((STRIP + 14.0) * _unit)


func _center_key() -> String:
	return InputHints.label(InputActions.CARD_CENTER, gamepad)


## 有名之人清單在卡片座標上的範圍。
func _list_rect() -> Rect2:
	var paper := _paper_rect(_content_height())
	var x := paper.position.x + (STRIP + 14.0) * _unit
	var bottom := paper.end.y - (8.0 + 12.0 + ACTION_RADIUS * 2.0 + 14.0) * _unit
	var height := _list_height()
	return Rect2(Vector2(x, bottom - height), Vector2(_inner_width(), height))


func _arrange(unrolled: bool) -> void:
	if _list == null:
		return
	var show := unrolled and not _people.is_empty()
	_list.visible = show
	_list.edge_scroll = gamepad
	if show:
		var rect := _list_rect()
		_list.position = rect.position
		_list.size = rect.size
		_list.content_height = _people_content_height()


func _draw_body(paper: Rect2) -> void:
	var x := paper.position.x + (STRIP + 14.0) * _unit
	var right := paper.end.x - 16.0 * _unit
	var y := paper.position.y + 14.0 * _unit
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	var section := Ink.font_size(Ink.SIZE_SECTION, _unit)
	# 首列：治所與治所名；下方一道墨劃。
	var seat_label := Locale.t("card_seat")
	Ink.text(self, Vector2(x, y + (body - note) * 0.6), seat_label, note, Ink.INK_LIGHT)
	Ink.text(self, Vector2(x + Ink.text_size(seat_label, note).x + 8.0 * _unit, y), _info["seat_name"], body)
	y += 34.0 * _unit
	Ink.stroke(self, Vector2(x, y), Vector2(right, y), 2.0 * _unit, Color(Ink.INK, 0.7))
	y += 12.0 * _unit

	# 地形組成：佔比最高的三種陸地地形，各一個色塊加百分比。
	var mix: PackedInt32Array = _info["terrain_mix"]
	var land := 0
	for code in range(2, mix.size()):
		land += mix[code]
	var order := range(2, mix.size())
	order.sort_custom(func(a: int, b: int) -> bool: return mix[a] > mix[b])
	var cursor := x
	for i in mini(3, order.size()):
		var code: int = order[i]
		if land == 0 or mix[code] == 0:
			break
		var key := TerrainSwatches.KEYS[code]
		TerrainSwatches.chip(self, Rect2(Vector2(cursor, y + (note - 12.0 * _unit) * 0.5 + 2.0), Vector2(12.0, 12.0) * _unit), key)
		var label := "%s %d%%" % [Locale.t(key), int(round(100.0 * mix[code] / land))]
		Ink.text(self, Vector2(cursor + 17.0 * _unit, y), label, note)
		cursor += 17.0 * _unit + Ink.text_size(label, note).x + 14.0 * _unit
	y += 28.0 * _unit

	Ink.section(self, Vector2(x, y), Locale.t("card_people"), section)
	y += 26.0 * _unit
	Ink.text(self, Vector2(x, y), "%s %s" % [Locale.t("card_population"), Locale.count(_info["population"])], body)
	Ink.text(self, Vector2(x + (right - x) * 0.5, y), "%s %s" % [Locale.t("card_capacity"), Locale.count(_info["capacity"])], body)
	y += 32.0 * _unit

	# 氣場墨條：數字寫在條的右端。
	var shown := _bar_keys()
	var label_width := 56.0 * _unit
	var number_width := Ink.text_size("100%", note).x + 8.0 * _unit
	for entry: Array in BARS:
		if not shown.has(entry[0]):
			continue
		var value := float(_info[entry[1]])
		Ink.text(self, Vector2(x, y), Locale.t(entry[0]), body)
		var bar := Rect2(Vector2(x + label_width, y + body * 0.5 - 1.0), Vector2(right - x - label_width - number_width, 6.0 * _unit))
		Ink.ink_bar(self, bar, value, entry[2])
		var number := "%d%%" % int(round(clampf(value, 0.0, 1.0) * 100.0))
		Ink.text(self, Vector2(right - Ink.text_size(number, note).x, y + (body - note) * 0.6), number, note, Ink.INK_LIGHT)
		y += 26.0 * _unit
	y += 8.0 * _unit

	# 教化：道、釋、儒三段弧；字用墨色，顏色只在色塊與弧上。
	var faith_top := y
	Ink.section(self, Vector2(x, y), Locale.t("card_faith"), section)
	if _shows_faith_ring():
		var ring_on := _ring_shown()
		var faith: Vector3 = _info["faith"]
		var total := faith.x + faith.y + faith.z
		var ring_radius := 18.0 * _unit
		var ring_center := Vector2(right - ring_radius - 4.0 * _unit, y + 30.0 * _unit)
		var start := -PI * 0.5
		if ring_on:
			draw_arc(ring_center, ring_radius, 0.0, TAU, 48, Color(Ink.INK_LIGHT, 0.2), 6.0 * _unit, true)
		cursor = x
		y += 28.0 * _unit
		for i in 3:
			var sweep: float = TAU * faith[i] / total
			if sweep > 0.01 and ring_on:
				draw_arc(ring_center, ring_radius, start, start + sweep, 32, FAITHS[i][1], 6.0 * _unit, true)
			start += sweep
			draw_rect(Rect2(Vector2(cursor, y + (note - 10.0 * _unit) * 0.5 + 2.0), Vector2(10.0, 10.0) * _unit), FAITHS[i][1], true)
			var label := "%s %d%%" % [Locale.t(FAITHS[i][0]), int(round(100.0 * faith[i] / total))]
			Ink.text(self, Vector2(cursor + 15.0 * _unit, y), label, note)
			cursor += 15.0 * _unit + Ink.text_size(label, note).x + 14.0 * _unit
	else:
		var heading := Ink.text_size(Locale.t("card_faith"), section).x + section * 1.1
		Ink.text(self, Vector2(x + heading + 10.0 * _unit, y + (section - note) * 0.6), Locale.t("card_faith_none"), note, Ink.INK_LIGHT)
	y = faith_top + (26.0 + (34.0 if _ring_shown() else 0.0) + 6.0) * _unit

	# 在此的有名之人：節名；清單是下面的子控制項，沒有人時寫「無」。
	Ink.section(self, Vector2(x, y), Locale.t("card_notables"), section)
	if _people.is_empty():
		var heading := Ink.text_size(Locale.t("card_notables"), section).x + section * 1.1
		Ink.text(self, Vector2(x + heading + 10.0 * _unit, y + (section - note) * 0.6), Locale.t("card_none"), note, Ink.INK_LIGHT)

	# 頁尾動作印。
	_draw_action(_center_rect(), &"center", Locale.t("act_center"), _center_key(), "center")


## 清單的畫法（內容座標）：姓名在左，身分與境界在右。
func _paint_people(list: LedgerList) -> void:
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var row := PERSON_ROW * _unit
	var width := list.size.x - 6.0 * _unit
	for i in _people.size():
		var person := _people[i]
		var target := "person:%d" % person["id"]
		var top := i * row
		var lit: bool = target == list.hover or target == list.pressed
		if lit:
			list.draw_rect(Rect2(Vector2(0.0, top + 1.0), Vector2(width, row - 2.0)), Color(Ink.INK, 0.2 if target == list.pressed else 0.1), true)
		var name_size := Ink.text_size(person["name"], body)
		Ink.text(list, Vector2(2.0 * _unit, top + (row - name_size.y) * 0.5), person["name"], body)
		list.draw_line(Vector2(2.0 * _unit, top + row - 4.0 * _unit), Vector2(2.0 * _unit + name_size.x, top + row - 4.0 * _unit), Color(Ink.SEAL_RED, 0.75 if lit else 0.45), 1.0)
		var identity := PersonText.identity_line(person)
		var room := width - name_size.x - 18.0 * _unit
		identity = _fit(identity, note, room)
		var extent := Ink.text_size(identity, note)
		Ink.text(list, Vector2(width - extent.x, top + (row - extent.y) * 0.5), identity, note, Ink.INK_LIGHT)
	if _more > 0:
		var line := Locale.t("card_people_more").format({"N": str(_more)})
		Ink.text(list, Vector2(2.0 * _unit, _people.size() * row + (row - Ink.text_size(line, note).y) * 0.5), line, note, Ink.INK_LIGHT)


## 一行放不下時，把右側的身分一行改成只留最後的境界一段（仍放不下就從左邊切到放得下為止，不加省略號以外的字）。
func _fit(text: String, size: int, room: float) -> String:
	if Ink.text_size(text, size).x <= room:
		return text
	var parts := text.split(" · ")
	while parts.size() > 1:
		parts.remove_at(0)
		var shorter := " · ".join(parts)
		if Ink.text_size(shorter, size).x <= room:
			return shorter
	return text


func _person_at(point: Vector2) -> String:
	var row := int(point.y / (PERSON_ROW * _unit))
	if point.y >= 0.0 and row < _people.size():
		return "person:%d" % _people[row]["id"]
	return ""


func _on_list_activated(target: String) -> void:
	if target.begins_with("person:"):
		person_requested.emit(int(target.substr(7)))


func _activate(target: String) -> void:
	if target == "center":
		request_center()


func _target_at(pos: Vector2) -> String:
	if _in_round(_close_rect(), pos):
		return "close"
	if _in_round(_center_rect(), pos):
		return "center"
	return ""
