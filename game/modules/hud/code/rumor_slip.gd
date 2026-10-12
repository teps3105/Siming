class_name RumorSlip
extends Control
## GDD 4.7、5.7 傳聞簽：畫面右上一列紙簽，每張是事件印、一句話（人物帶門派與境界）、原因標記與年月，新到舊，世界大事置頂。
## 支援的決定：該追哪一則——點一則鏡頭飛往該地。收合時最多 8 張（Deck 為 1 張），展開為近 50 則的捲動清單。
## 入簽依重要度分級（5.7「通知優先序」）：≥ 3 或世界大事入簽，2 只進展開清單，其餘不顯示；
## 同一魂同一年的多筆異常事件合併為一簽（取重要度高的）。
## 例行事件（7.2「傳聞的合成與標記」）同月同類合成一則統計，點開攤出名單；收合時異常先於例行統計。
## 內部分兩層：_records 是逐筆事件，_entries 是由它們合成的一張張簽（單筆，或同月同類的統計）。

## 點一則有地點的傳聞：飛往地圖座標 map_pos。
signal fly_requested(map_pos: Vector2)
## 點了「看人物」（或統計名單的一行）：開這個人的人物卡。
signal person_requested(person: int)
## 點了「溯因」（或統計名單一行的「溯因」）：開這則事件的事件卡。
signal trace_requested(event: int)

const WIDTH := 300.0
const EXPANDED_WIDTH := 360.0
const ITEM_HEIGHT := 32.0
const ITEM_GAP := 5.0
const HEADER_HEIGHT := 26.0
## 收合時最多幾張；展開清單最多留幾則。
const SHOWN_MAX := 8
const KEEP_MAX := 50
## 逐筆事件累積到這個數目就先合成一次（快轉時不讓它無限長）。
const RECORDS_MAX := 2000
## 入簽與只進清單的重要度門檻。
const SLIP_IMPORTANCE := 3
const NOTE_IMPORTANCE := 2
## 世界大事置頂多久（tick）：一世。
const PIN_TICKS := 360
const FADE_SECONDS := 0.35
const SEAL_SIZE := 22.0
const PAD := 8.0
## 控制項四周留給柔影的空間，紙面從這裡起算。
const SHADOW := 8.0
## 展開清單捲動：游標貼近上下緣時每秒捲的像素，與滾輪每格的列數。
const EDGE_ZONE := 26.0
const EDGE_SPEED := 320.0
const SELECT_NONE := -1
const SELECT_HEADER := -2
## 攤開的名單一行的高度與行距；名單列的選取碼 = MEMBER_BASE + 簽序 × MEMBER_STRIDE + 名單序。
const MEMBER_HEIGHT := 22.0
const MEMBER_GAP := 2.0
const MEMBER_BASE := 1000000
const MEMBER_STRIDE := 10000
## 「看人物」小印的選取碼 = SLOT_BASE + 簽序。
const SLOT_BASE := 3000000
## 「溯因」小印的選取碼 = TRACE_BASE + 簽序；名單一行的 = MEMBER_TRACE_BASE + 簽序 × MEMBER_STRIDE + 名單序。
const TRACE_BASE := 4000000
const MEMBER_TRACE_BASE := 5000000
## 原因標記的方框邊長。
const MARK_SIZE := 16.0
## 「看人物」「溯因」入口的寬度與間距。
const SLOT_WIDTH := 44.0
const SLOT_GAP := 4.0

## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像）。
var gamepad := false:
	set(value):
		gamepad = value
		queue_redraw()
## Deck 版面：收合時只留最新一張（GDD 5.7）。
var compact := false:
	set(value):
		compact = value
		_shown_valid = false
		_refit()
## 是否展開為近 50 則的清單。
var expanded := false:
	set(value):
		if expanded == value:
			return
		expanded = value
		_shown_valid = false
		_scroll = 0.0
		_refit()
## 紙面右緣與上緣在畫面上的位置。
var right_edge := 0.0:
	set(value):
		right_edge = value
		_place()
var top_edge := 0.0:
	set(value):
		top_edge = value
		_place()
## 展開清單的高度上限（畫面上緣到印匣之間）。
var max_height := 600.0:
	set(value):
		max_height = value
		_refit()

var _unit := 1.0
## 逐筆事件，依到達順序，新的在後。
var _records: Array[Dictionary] = []
## 由 _records 合成的簽（單筆或統計），_dirty 時要重算。
var _entries: Array[Dictionary] = []
var _dirty := false
var _refit_queued := false
## 目前畫出來的簽（依顯示順序）與各簽在內容座標的上緣、內容總高。
var _shown_cache: Array[Dictionary] = []
var _shown_valid := false
var _tops := PackedFloat32Array()
var _content_h := 0.0
## 各簽已出現的時間（滑入動畫）與攤開的統計。
var _ages := {}
var _open := {}
var _latest_tick := 0
var _hover := SELECT_NONE
var _pressed := SELECT_NONE
var _scroll := 0.0
var _pointer := Vector2(-1.0, -1.0)


func setup(unit: float) -> void:
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_STOP
	clip_contents = true
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	_refit()


## 新到的事件。重要度不夠的不入簽；回傳是否留下。
func post(event: Dictionary) -> bool:
	var world: bool = event["kind"] == "WorldChanged"
	var importance: int = event["importance"]
	if importance < NOTE_IMPORTANCE and not world:
		return false
	var info := EventText.describe(event)
	var rec := {
		"id": event["id"],
		"key": "e%d" % int(event["id"]),
		"kind": event["kind"],
		"tick": event["tick"],
		"year": event.get("year", 0),
		"month": EventText.month_of(event),
		"importance": importance,
		"world": world,
		"n": event.get("n", 0),
		"x": event.get("x", ""),
		"line": info["line"],
		"variants": info["variants"],
		"seal": EventText.seal(event["kind"]),
		"category": info["category"],
		"reason": info["reason"],
		"consequence": info["consequence"],
		"who": info["who"],
		"who_variants": info["who_variants"],
		"tag": info["tag"],
		"what": info["what"],
		"souls": event.get("souls", PackedInt32Array()),
		# 可追的對象編號：人物、勢力、州郡（人物卡與事件卡實作後用）。
		"persons": event.get("persons", PackedInt32Array()),
		"a_person": event.get("a_person", -1),
		"powers": event.get("powers", PackedInt32Array()),
		"province": event.get("province", -1),
		"members": [],
	}
	if event.has("pos"):
		rec["pos"] = event["pos"]
	_latest_tick = maxi(_latest_tick, int(rec["tick"]))
	if _admit(rec):
		_dirty = true
		_shown_valid = false
		_queue_refit()
	return true


func clear() -> void:
	_records.clear()
	_entries.clear()
	_ages.clear()
	_open.clear()
	_latest_tick = 0
	_scroll = 0.0
	_dirty = false
	_shown_valid = false
	_refit()


## 目前留著的傳聞（重建器物時帶到新的簽上）。
func snapshot() -> Array[Dictionary]:
	return _records.duplicate()


func restore(entries: Array[Dictionary]) -> void:
	_records = entries.duplicate()
	_ages.clear()
	for rec in _records:
		_latest_tick = maxi(_latest_tick, int(rec["tick"]))
	_rebuild()
	for entry in _entries:
		_ages[entry["key"]] = FADE_SECONDS
	_shown_valid = false
	_refit()


## 這筆事件是否留下：突破（新境界 ≥ 金丹）緊接在渡劫之後，是同一件事，不另記；
## 成婚同時結下的宿緣也是同一件事，只留成婚；
## 同一魂同一年的異常事件合成一簽，留重要度高的（同級留先到的）。例行事件不併，統計要數得出每個人。
## 事件依時間先後到達，所以只從尾端往回找到年份不同為止。
func _admit(rec: Dictionary) -> bool:
	var souls: PackedInt32Array = rec["souls"]
	var year := _year_of(rec)
	if rec["kind"] == "Breakthrough" and int(rec["n"]) >= 2:
		for i in range(_records.size() - 1, -1, -1):
			var old := _records[i]
			if old["tick"] != rec["tick"]:
				break
			if old["kind"] == "Tribulation" and _share_soul(old["souls"], souls):
				return false
	if rec["kind"] == "ThreadCreated" or rec["kind"] == "Married":
		var twin := "Married" if rec["kind"] == "ThreadCreated" else "ThreadCreated"
		for i in range(_records.size() - 1, -1, -1):
			var old := _records[i]
			if old["tick"] != rec["tick"]:
				break
			if old["kind"] == twin and _same_souls(old["souls"], souls):
				if twin == "Married":
					return false
				_records.remove_at(i)
				break
	if rec["kind"] == "Died" and rec["category"] != "" and rec["x"] in EventText.TRIBULATION_CAUSES:
		# 渡劫失敗而亡：緊接在失敗的渡劫之後的身死是同一件事，合進身死的統計，不留那一筆單列的渡劫。
		for i in range(_records.size() - 1, -1, -1):
			var old := _records[i]
			if old["tick"] != rec["tick"]:
				break
			if old["kind"] == "Tribulation" and old["reason"] == "death" and _share_soul(old["souls"], souls):
				_records.remove_at(i)
				break
	if rec["world"] or rec["category"] != "" or souls.is_empty():
		_append(rec)
		return true
	for i in range(_records.size() - 1, -1, -1):
		var old := _records[i]
		if _year_of(old) < year:
			break
		if old["world"] or old["category"] != "" or _year_of(old) != year or not _share_soul(old["souls"], souls):
			continue
		if int(rec["importance"]) > int(old["importance"]):
			_records[i] = rec
			return true
		return false
	_append(rec)
	return true


## 加入一筆；一口氣湧入太多（快轉）時先合成一次，丟掉擠出 50 簽的舊事。
func _append(rec: Dictionary) -> void:
	_records.append(rec)
	if _records.size() > RECORDS_MAX:
		_rebuild()
		_shown_valid = false


func _share_soul(a: PackedInt32Array, b: PackedInt32Array) -> bool:
	for soul in b:
		if a.has(soul):
			return true
	return false


func _same_souls(a: PackedInt32Array, b: PackedInt32Array) -> bool:
	if a.size() != b.size() or a.is_empty():
		return false
	for soul in b:
		if not a.has(soul):
			return false
	return true


func _year_of(entry: Dictionary) -> int:
	return floori(float(entry["tick"]) / 12.0)


## 由逐筆事件合成簽：例行事件同月同類成一簽（只有一筆就維持單筆的樣子），其餘各自一簽；超過 50 簽丟掉最舊的。
func _rebuild() -> void:
	_dirty = false
	var singles: Array[Dictionary] = []
	var groups := {}
	var order: Array[String] = []
	for rec in _records:
		var cat: String = rec["category"]
		if cat.is_empty():
			singles.append(rec)
			continue
		var gkey := "%d:%s" % [int(rec["tick"]), cat]
		if not groups.has(gkey):
			groups[gkey] = []
			order.append(gkey)
		groups[gkey].append(rec)
	_entries = singles
	for gkey in order:
		var members: Array = groups[gkey]
		if members.size() == 1:
			_entries.append(members[0])
		else:
			_entries.append(_make_group(gkey, members))
	while _entries.size() > KEEP_MAX:
		var oldest := 0
		for i in _entries.size():
			if _older(_entries[i], _entries[oldest]):
				oldest = i
		var gone := _entries[oldest]
		var gone_members: Array = gone["members"] if gone["members"].size() > 0 else [gone]
		for m in gone_members:
			_remove_record(int(m["id"]))
		_entries.remove_at(oldest)
	var ages := {}
	var open := {}
	for entry in _entries:
		ages[entry["key"]] = _ages.get(entry["key"], 0.0)
		if _open.has(entry["key"]):
			open[entry["key"]] = true
	_ages = ages
	_open = open


func _make_group(gkey: String, members: Array) -> Dictionary:
	members.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		if a["n"] != b["n"]:
			return a["n"] > b["n"]
		return a["id"] < b["id"])
	var first: Dictionary = members[0]
	var importance := 0
	var newest := 0
	for m in members:
		importance = maxi(importance, int(m["importance"]))
		newest = maxi(newest, int(m["id"]))
	var variants := EventText.group_variants(first["category"], members)
	return {
		"id": newest,
		"key": "g" + gkey,
		"kind": first["kind"],
		"tick": first["tick"],
		"year": first["year"],
		"month": first["month"],
		"importance": importance,
		"world": false,
		"category": first["category"],
		"line": variants[0][1],
		"variants": variants,
		"seal": first["seal"],
		"reason": "",
		"consequence": "",
		"souls": PackedInt32Array(),
		"persons": PackedInt32Array(),
		"a_person": -1,
		"powers": PackedInt32Array(),
		"province": -1,
		"members": members,
	}


func _older(a: Dictionary, b: Dictionary) -> bool:
	if a["tick"] != b["tick"]:
		return a["tick"] < b["tick"]
	return a["id"] < b["id"]


func _remove_record(id: int) -> void:
	for i in _records.size():
		if int(_records[i]["id"]) == id:
			_records.remove_at(i)
			return


## 世界大事是否仍置頂：發生在最近一世之內。
func _pinned(entry: Dictionary) -> bool:
	return entry["world"] and _latest_tick - int(entry["tick"]) < PIN_TICKS


func _before(a: Dictionary, b: Dictionary) -> bool:
	var pa := _pinned(a)
	var pb := _pinned(b)
	if pa != pb:
		return pa
	if a["tick"] != b["tick"]:
		return a["tick"] > b["tick"]
	if a["importance"] != b["importance"]:
		return a["importance"] > b["importance"]
	return a["id"] > b["id"]


## 收合時的順序：置頂的世界大事、其他異常、最後才是例行統計；各自新到舊。
func _before_collapsed(a: Dictionary, b: Dictionary) -> bool:
	var pa := _pinned(a)
	var pb := _pinned(b)
	if pa != pb:
		return pa
	var ra := is_routine(a)
	var rb := is_routine(b)
	if ra != rb:
		return rb
	return _before(a, b)


## 例行（同月同類統計，或這一類的單筆）。
func is_routine(entry: Dictionary) -> bool:
	return entry["category"] != ""


## 是一則統計（攤得開名單）。
func is_group(entry: Dictionary) -> bool:
	return entry["members"].size() > 0


## 目前畫出來的傳聞，由上到下：置頂的世界大事先，再依新到舊、重要度排；
## 收合時只列入簽的（重要度 ≥ 3 或世界大事），異常在前、例行統計在後，並受張數上限，展開時全部。
func shown() -> Array[Dictionary]:
	if _dirty:
		_rebuild()
		_shown_valid = false
	if _shown_valid:
		return _shown_cache
	var out: Array[Dictionary] = []
	for entry in _entries:
		if expanded or entry["world"] or int(entry["importance"]) >= SLIP_IMPORTANCE:
			out.append(entry)
	out.sort_custom(_before if expanded else _before_collapsed)
	if not expanded:
		out.resize(mini(out.size(), 1 if compact else SHOWN_MAX))
	_shown_cache = out
	_shown_valid = true
	_layout()
	return out


## 各簽的上緣與內容總高：攤開的統計在簽下多出名單列。
func _layout() -> void:
	_tops.resize(_shown_cache.size())
	var y := 0.0
	for i in _shown_cache.size():
		_tops[i] = y
		y += _row_height()
		if _is_open(_shown_cache[i]):
			y += _member_gap() + _shown_cache[i]["members"].size() * (_member_height() + _member_gap()) - _member_gap()
		y += _gap()
	_content_h = _row_height() if _shown_cache.is_empty() else y - _gap()


func entry_count() -> int:
	if _dirty:
		_rebuild()
		_shown_valid = false
	return _entries.size()


func _is_open(entry: Dictionary) -> bool:
	return expanded and _open.get(entry["key"], false)


## 第 index 張統計是否攤開。
func is_open(index: int) -> bool:
	return _is_open(shown()[index])


func _queue_refit() -> void:
	if _refit_queued:
		return
	_refit_queued = true
	_refit_now.call_deferred()


func _refit_now() -> void:
	_refit_queued = false
	_refit()


func _width() -> float:
	return (EXPANDED_WIDTH if expanded else WIDTH) * _unit


func _row_height() -> float:
	return ITEM_HEIGHT * _unit


func _gap() -> float:
	return ITEM_GAP * _unit


func _member_height() -> float:
	return MEMBER_HEIGHT * _unit


func _member_gap() -> float:
	return MEMBER_GAP * _unit


## 紙面左上角在控制項內的位置。
func _origin() -> Vector2:
	return Vector2.ONE * SHADOW


func header_rect() -> Rect2:
	return Rect2(_origin(), Vector2(_width(), HEADER_HEIGHT * _unit))


## 清單的可視範圍（簽頭下方到底）。
func _view_rect() -> Rect2:
	var head := header_rect()
	var top_y := head.end.y + _gap()
	return Rect2(Vector2(head.position.x, top_y), Vector2(head.size.x, size.y - SHADOW - top_y))


func _content_height() -> float:
	shown()
	return _content_h


func _max_scroll() -> float:
	return maxf(0.0, _content_height() - _view_rect().size.y)


## 第 index 張的紙面（控制項座標）；捲動後可能落在可視範圍外。
func row_rect(index: int) -> Rect2:
	var view := _view_rect()
	shown()
	var top := _tops[index] if index < _tops.size() else 0.0
	return Rect2(view.position + Vector2(0.0, top - _scroll), Vector2(view.size.x, _row_height()))


## 第 index 張統計攤開後的第 member 行（控制項座標）。
func member_rect(index: int, member: int) -> Rect2:
	var row := row_rect(index)
	var top := row.end.y + _member_gap() + member * (_member_height() + _member_gap())
	return Rect2(Vector2(row.position.x, top), Vector2(row.size.x, _member_height()))


## 一則的主要人物編號：事件的主角，沒有主角取牽涉的第一位；沒有為 -1（統計也沒有）。
static func main_person(entry: Dictionary) -> int:
	var subject := int(entry.get("a_person", -1))
	if subject >= 0:
		return subject
	var persons: PackedInt32Array = entry.get("persons", PackedInt32Array())
	return persons[0] if not persons.is_empty() else -1


## 第 index 張右端的「看人物」「溯因」兩個入口，停留時取代年月：看人物是最右一顆小印（只有主要人物明確的單則才有），
## 溯因在它左邊；沒有看人物的單則，溯因移到最右。統計沒有單一事件，兩者都沒有。
func action_slots(index: int) -> Dictionary:
	var row := row_rect(index)
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var width := maxf(SLOT_WIDTH * _unit, Ink.text_size(Locale.t("rumor_person"), note).x + 12.0 * _unit)
	var gap := SLOT_GAP * _unit
	var right := row.end.x - PAD * _unit
	var person := Rect2(Vector2(right - width, row.position.y + 3.0 * _unit), Vector2(width, row.size.y - 6.0 * _unit))
	var trace_x := person.position.x - gap - SLOT_WIDTH * _unit if has_person_slot(index) else right - SLOT_WIDTH * _unit
	var trace := Rect2(Vector2(trace_x, row.position.y + 3.0 * _unit), Vector2(SLOT_WIDTH * _unit, row.size.y - 6.0 * _unit))
	return {"person": person, "trace": trace}


## 第 index 張是否有「溯因」入口：單則都有，統計沒有。
func has_trace_slot(index: int) -> bool:
	return not is_group(shown()[index])


## 統計攤開後第 member 行右端的兩個入口：看人物（或飛往）的字在最右，溯因小印在它左邊。
func member_slots(index: int, member: int) -> Dictionary:
	var rect := member_rect(index, member)
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var entry: Dictionary = shown()[index]["members"][member]
	var tail := Locale.t("rumor_person" if main_person(entry) >= 0 else "rumor_fly")
	var tail_w := Ink.text_size(tail, note).x + 8.0 * _unit
	var right := rect.end.x - PAD * _unit - tail_w
	var width := SLOT_WIDTH * _unit
	var trace := Rect2(Vector2(right - width, rect.position.y + 2.0 * _unit), Vector2(width, rect.size.y - 4.0 * _unit))
	return {"tail_width": tail_w, "trace": trace}


## 第 index 張是否有「看人物」入口。
func has_person_slot(index: int) -> bool:
	var entry := shown()[index]
	return not is_group(entry) and main_person(entry) >= 0


## 目前看得到（落在可視範圍內）的傳聞張數。
func visible_row_count() -> int:
	var count := 0
	for i in shown().size():
		if _view_rect().intersects(row_rect(i)):
			count += 1
	return count


## 簽佔住的畫面範圍（地圖標籤要避開）。
func occupied_rect() -> Rect2:
	var view := _view_rect()
	var bottom := minf(view.position.y + _content_height() - _scroll, view.end.y)
	var rect := header_rect().merge(Rect2(view.position, Vector2(view.size.x, maxf(bottom - view.position.y, 0.0))))
	rect.position += position
	return rect


func _refit() -> void:
	if _unit <= 0.0:
		return
	var natural := header_rect().size.y + _gap() + _content_height()
	var height := minf(natural, max_height - SHADOW * 2.0) if expanded else natural
	custom_minimum_size = Vector2(_width(), height) + Vector2.ONE * SHADOW * 2.0
	size = custom_minimum_size
	_scroll = clampf(_scroll, 0.0, _max_scroll())
	_place()
	queue_redraw()


func _place() -> void:
	position = Vector2(right_edge - size.x + SHADOW, top_edge - SHADOW).round()


## 簽頭右端的鍵：鍵鼠是展開清單的鍵，搖桿沒有專屬鍵，以游標的左鍵（LT）點。
func _list_key() -> String:
	if gamepad:
		return InputHints.label(InputActions.PAD_LEFT, true)
	return InputHints.label(InputActions.RUMOR_LIST, false)


## 飛往的實體鍵：鍵鼠用滑鼠左鍵，沒有鍵可畫；搖桿是游標的左鍵（LT）。
func _fly_key() -> String:
	return InputHints.label(InputActions.PAD_LEFT, true) if gamepad else ""


func _process(delta: float) -> void:
	var dirty := false
	for key in _ages:
		if float(_ages[key]) < FADE_SECONDS:
			_ages[key] = float(_ages[key]) + delta
			dirty = true
	if expanded and _hover != SELECT_NONE and _max_scroll() > 0.0:
		var view := _view_rect()
		var step := 0.0
		if _pointer.y < view.position.y + EDGE_ZONE * _unit and _pointer.y >= view.position.y:
			step = -EDGE_SPEED
		elif _pointer.y > view.end.y - EDGE_ZONE * _unit and _pointer.y <= view.end.y:
			step = EDGE_SPEED
		if step != 0.0:
			_scroll = clampf(_scroll + step * _unit * delta, 0.0, _max_scroll())
			_update_hover(_pointer)
			dirty = true
	if dirty:
		queue_redraw()


func _draw() -> void:
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	_draw_header(note)
	var entries := shown()
	var view := _view_rect()
	if entries.is_empty():
		var rect := row_rect(0)
		Ink.paper_slip(self, rect)
		var text := Locale.t("rumor_empty")
		Ink.text(self, rect.position + Vector2(PAD * _unit * 1.5, (rect.size.y - Ink.text_size(text, note).y) * 0.5), text, note, Ink.INK_LIGHT)
		return
	for i in entries.size():
		var rect := row_rect(i)
		var open := _is_open(entries[i])
		var block := rect.merge(member_rect(i, entries[i]["members"].size() - 1)) if open else rect
		if not view.intersects(block):
			continue
		if view.intersects(rect):
			_draw_row(entries[i], rect, i, note)
		if open:
			for j in entries[i]["members"].size():
				var mrect := member_rect(i, j)
				if view.intersects(mrect):
					_draw_member(entries[i]["members"][j], mrect, MEMBER_BASE + i * MEMBER_STRIDE + j, note)


func _draw_header(note: int) -> void:
	var rect := header_rect()
	Ink.paper_slip(self, rect)
	if _hover == SELECT_HEADER:
		draw_rect(rect.grow(-1.0), Color(Ink.INK, 0.1 if _pressed != SELECT_HEADER else 0.2), true)
	var section := Ink.font_size(Ink.SIZE_SECTION, _unit)
	var y := rect.position.y + (rect.size.y - Ink.text_size(Locale.t("rumor_title"), section).y) * 0.5
	Ink.section(self, Vector2(rect.position.x + PAD * _unit, y), Locale.t("rumor_title"), section)
	# 右端：展開或收合的字與實體鍵。
	var label := Locale.t("rumor_collapse" if expanded else "rumor_expand")
	var key := _list_key()
	var key_width := Ink.key_glyph_width(key, _unit)
	var right := rect.end.x - PAD * _unit
	var key_x := right - key_width
	if not key.is_empty():
		Ink.key_glyph(self, Vector2(key_x, rect.position.y + (rect.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), key, _unit, not gamepad)
	var label_x := (key_x - 6.0 * _unit if not key.is_empty() else right) - Ink.text_size(label, note).x
	Ink.text(self, Vector2(label_x, rect.position.y + (rect.size.y - Ink.text_size(label, note).y) * 0.5), label, note, Ink.INK)
	# 中段：展開時寫留著幾則（收合時簽頭右端的字已寫了近 50 則）。
	var count := "%s %d %s" % [Locale.t("rumor_recent"), entry_count(), Locale.t("rumor_unit")]
	var count_x := rect.position.x + PAD * _unit + Ink.text_size(Locale.t("rumor_title"), section).x + 34.0 * _unit
	if count_x + Ink.text_size(count, note).x < label_x - 8.0 * _unit and expanded and entry_count() > 0:
		Ink.text(self, Vector2(count_x, rect.position.y + (rect.size.y - Ink.text_size(count, note).y) * 0.5), count, note, Ink.INK_LIGHT)


func _draw_row(entry: Dictionary, rect: Rect2, index: int, note: int) -> void:
	var fade := clampf(float(_ages.get(entry["key"], FADE_SECONDS)) / FADE_SECONDS, 0.0, 1.0)
	# 新到的簽從上方滑入。
	var slid := rect
	slid.position.y -= (1.0 - fade) * 10.0 * _unit
	var slot_hot := _hover == SLOT_BASE + index
	var trace_hot := _hover == TRACE_BASE + index
	var hot := _hover == index or slot_hot or trace_hot
	var pressed := _hover == index and _pressed == index
	var has_pos: bool = entry.has("pos")
	var group := is_group(entry)
	var clickable := has_pos or group
	var inked := Ink.INK if int(entry["importance"]) >= SLIP_IMPORTANCE or entry["world"] else Ink.INK_LIGHT
	Ink.paper_slip(self, slid.grow(-1.0) if pressed else slid)
	if hot:
		draw_rect(slid.grow(-1.0), Color(Ink.INK, 0.2 if pressed else 0.07), true)
		draw_rect(slid.grow(-2.0), Ink.INK if clickable else Ink.INK_LIGHT, false, 1.0)
	# 事件印：方形朱印，世界大事用靛。
	var seal_size := SEAL_SIZE * _unit
	var seal_rect := Rect2(slid.position + Vector2(PAD * _unit, (slid.size.y - seal_size) * 0.5), Vector2.ONE * seal_size)
	draw_rect(seal_rect, Color(Ink.SILK if entry["world"] else Ink.SEAL_RED, fade), true)
	draw_rect(seal_rect.grow(-2.0), Color(Ink.PAPER_LIGHT, 0.7 * fade), false, 1.0)
	var glyph_size := Ink.font_size(Ink.SIZE_SECTION, _unit)
	var glyph: String = entry["seal"]
	Ink.text(self, seal_rect.get_center() - Ink.text_size(glyph, glyph_size) * 0.5, glyph, glyph_size, Color(Ink.PAPER_LIGHT, fade))
	# 右端：停留時是「飛往」加實體鍵（統計是「攤開」「收起」），否則是年月。
	var right := slid.end.x - PAD * _unit
	var tail := ""
	var key := ""
	if hot and has_person_slot(index):
		# 最右一顆「看人物」小印，其餘靠左讓出位置。
		var slot: Rect2 = action_slots(index)["person"]
		var pressed_slot := slot_hot and _pressed == SLOT_BASE + index
		var slot_ink := Color(Ink.SEAL_RED, fade)
		draw_rect(slot, Color(Ink.SEAL_RED, 0.28 if pressed_slot else (0.14 if slot_hot else 0.0)), true)
		draw_rect(slot, slot_ink, false, 1.0)
		var slot_text := Locale.t("rumor_person")
		Ink.text(self, slot.get_center() - Ink.text_size(slot_text, note) * 0.5, slot_text, note, slot_ink)
		right = slot.position.x - 6.0 * _unit
	if hot and not group:
		# 「溯因」小印在看人物的左邊（沒有看人物時在最右）。
		var trace: Rect2 = action_slots(index)["trace"]
		var pressed_trace := trace_hot and _pressed == TRACE_BASE + index
		var trace_ink := Color(Ink.SEAL_RED, fade)
		draw_rect(trace, Color(Ink.SEAL_RED, 0.28 if pressed_trace else (0.14 if trace_hot else 0.0)), true)
		draw_rect(trace, trace_ink, false, 1.0)
		var trace_text := Locale.t("rumor_trace")
		Ink.text(self, trace.get_center() - Ink.text_size(trace_text, note) * 0.5, trace_text, note, trace_ink)
		right = trace.position.x - 6.0 * _unit
	if hot and group:
		tail = Locale.t("rumor_close" if _is_open(entry) else "rumor_open")
	elif hot and has_pos and not slot_hot and not trace_hot:
		tail = Locale.t("rumor_fly")
		key = _fly_key()
	elif slot_hot or trace_hot:
		# 游標在小印上：年月讓位，把寬度留給那句話。
		tail = ""
	else:
		tail = EventText.stamp(int(entry.get("year", 0)), int(entry["month"]))
	var key_width := Ink.key_glyph_width(key, _unit)
	if not key.is_empty():
		right -= key_width
		Ink.key_glyph(self, Vector2(right, slid.position.y + (slid.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), key, _unit)
		right -= 4.0 * _unit
	var tail_size := Ink.text_size(tail, note)
	right -= tail_size.x
	Ink.text(self, Vector2(right, slid.position.y + (slid.size.y - tail_size.y) * 0.5), tail, note, Color(Ink.SEAL_RED if hot and clickable else Ink.INK_LIGHT, fade))
	# 原因標記：小方框單字，至多一個。
	var reason: String = entry["reason"]
	if not reason.is_empty():
		var mark := MARK_SIZE * _unit
		var mark_rect := Rect2(Vector2(right - 6.0 * _unit - mark, slid.position.y + (slid.size.y - mark) * 0.5), Vector2.ONE * mark)
		var mark_color := Color(Ink.SILK if reason == "world" else Ink.SEAL_RED, fade)
		var mark_text := Locale.t("reason_" + reason)
		draw_rect(mark_rect, mark_color, false, 1.0)
		Ink.text(self, mark_rect.get_center() - Ink.text_size(mark_text, note) * 0.5, mark_text, note, mark_color)
		right = mark_rect.position.x
	# 一句話：依空間由全到簡，結果不被截。
	var text_x := seal_rect.end.x + 8.0 * _unit
	var line := fit_variants(entry["variants"], right - 8.0 * _unit - text_x, note)
	Ink.text(self, Vector2(text_x, slid.position.y + (slid.size.y - Ink.text_size(line, note).y) * 0.5), line, note, Color(inked, fade))


## 攤開的名單一行：身分與姓名，空一格接結果。
func _draw_member(member: Dictionary, rect: Rect2, code: int, note: int) -> void:
	var trace_code := code - MEMBER_BASE + MEMBER_TRACE_BASE
	var trace_hot := _hover == trace_code
	var hot := _hover == code or trace_hot
	var has_person := main_person(member) >= 0
	var has_pos: bool = member.has("pos")
	draw_rect(rect, Color(Ink.PAPER_LIGHT, 0.92), true)
	draw_rect(rect.grow(-0.5), Color(Ink.GOLD, 0.5), false, 1.0)
	if hot:
		draw_rect(rect.grow(-1.0), Color(Ink.INK, 0.2 if _pressed == code else 0.07), true)
	var right := rect.end.x - PAD * _unit
	if hot:
		if has_person or has_pos:
			var fly := Locale.t("rumor_person" if has_person else "rumor_fly")
			right -= Ink.text_size(fly, note).x
			Ink.text(self, Vector2(right, rect.position.y + (rect.size.y - Ink.text_size(fly, note).y) * 0.5), fly, note, Ink.SEAL_RED)
			right -= 8.0 * _unit
		# 「溯因」小印在看人物（飛往）字樣的左邊。
		var m := _member_of(code)
		var trace: Rect2 = member_slots(m[0], m[1])["trace"]
		draw_rect(trace, Color(Ink.SEAL_RED, 0.28 if trace_hot and _pressed == trace_code else (0.14 if trace_hot else 0.0)), true)
		draw_rect(trace, Ink.SEAL_RED, false, 1.0)
		var trace_text := Locale.t("rumor_trace")
		Ink.text(self, trace.get_center() - Ink.text_size(trace_text, note) * 0.5, trace_text, note, Ink.SEAL_RED)
		right = trace.position.x - 6.0 * _unit
	var variants: Array = []
	for head: String in member["who_variants"]:
		variants.append([head, String(member["tag"]) + "　" + String(member["what"])])
	var text := fit_variants(variants, right - (rect.position.x + PAD * 2.0 * _unit), note)
	Ink.text(self, Vector2(rect.position.x + PAD * 2.0 * _unit, rect.position.y + (rect.size.y - Ink.text_size(text, note).y) * 0.5), text, note, Ink.INK)


## 從 [首, 尾] 的版本裡取第一個放得進 width 的；都放不下則截最後一版的首（身分），尾（境界與結果）不截。
func fit_variants(variants: Array, width: float, font: int) -> String:
	for v: Array in variants:
		var text: String = v[0] + v[1]
		if Ink.text_size(text, font).x <= width:
			return text
	var last: Array = variants[variants.size() - 1]
	var head: String = last[0]
	var tail: String = last[1]
	var room := width - Ink.text_size(tail, font).x - Ink.text_size("…", font).x
	if head.is_empty() or room <= 0.0:
		return _fit(head + tail, width, font)
	var count := head.length()
	while count > 0 and Ink.text_size(head.substr(0, count), font).x > room:
		count -= 1
	return head.substr(0, count) + "…" + tail


## 把一句話截到放得進 width；截斷時結尾加「…」。
func _fit(line: String, width: float, font: int) -> String:
	if Ink.text_size(line, font).x <= width:
		return line
	var count := line.length()
	while count > 0 and Ink.text_size(line.substr(0, count) + "…", font).x > width:
		count -= 1
	return line.substr(0, count) + "…"


## 控制項座標的 point 落在哪：簽頭、第幾張、攤開名單的第幾行（選取碼），或沒有。
func _select_at(point: Vector2) -> int:
	if header_rect().has_point(point):
		return SELECT_HEADER
	if not _view_rect().has_point(point):
		return SELECT_NONE
	var entries := shown()
	for i in entries.size():
		if row_rect(i).has_point(point):
			if has_person_slot(i) and action_slots(i)["person"].has_point(point):
				return SLOT_BASE + i
			if has_trace_slot(i) and action_slots(i)["trace"].has_point(point):
				return TRACE_BASE + i
			return i
		if _is_open(entries[i]):
			for j in entries[i]["members"].size():
				if member_rect(i, j).has_point(point):
					if member_slots(i, j)["trace"].has_point(point):
						return MEMBER_TRACE_BASE + i * MEMBER_STRIDE + j
					return MEMBER_BASE + i * MEMBER_STRIDE + j
	return SELECT_NONE


## 選取碼對應的名單列：[簽序, 行序]；不是名單列為空。
func _member_of(select: int) -> Array:
	if select < MEMBER_BASE or (select >= SLOT_BASE and select < MEMBER_TRACE_BASE):
		return []
	var rest := select - (MEMBER_TRACE_BASE if select >= MEMBER_TRACE_BASE else MEMBER_BASE)
	return [rest / MEMBER_STRIDE, rest % MEMBER_STRIDE]


## 游標只在簽頭與簽上才攔截，其餘讓地圖收到事件。
func _has_point(point: Vector2) -> bool:
	return _select_at(point) != SELECT_NONE


## 游標停在這裡時是否可以按（GDD 5.9 游標圖示）：簽頭、統計，或有地點的簽與名單行。
func is_hot(point: Vector2) -> bool:
	var at := _select_at(point)
	if at == SELECT_HEADER or at >= SLOT_BASE:
		return true
	if at >= MEMBER_BASE:
		var m := _member_of(at)
		var member: Dictionary = shown()[m[0]]["members"][m[1]]
		return member.has("pos") or main_person(member) >= 0
	return at >= 0 and (shown()[at].has("pos") or is_group(shown()[at]))


func _update_hover(point: Vector2) -> void:
	var at := _select_at(point)
	if at != _hover:
		_hover = at
		queue_redraw()


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_pointer = (event as InputEventMouseMotion).position
		_update_hover(_pointer)
	elif event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		var at := _select_at(button.position)
		if button.button_index == MOUSE_BUTTON_LEFT:
			if button.pressed:
				_pressed = at
				queue_redraw()
				accept_event()
			else:
				var released := _pressed
				_pressed = SELECT_NONE
				queue_redraw()
				if released != SELECT_NONE and released == at:
					activate(at)
				accept_event()
		elif expanded and button.pressed and (button.button_index == MOUSE_BUTTON_WHEEL_UP or button.button_index == MOUSE_BUTTON_WHEEL_DOWN):
			var dir := -1.0 if button.button_index == MOUSE_BUTTON_WHEEL_UP else 1.0
			_scroll = clampf(_scroll + dir * (_row_height() + _gap()), 0.0, _max_scroll())
			_update_hover(button.position)
			queue_redraw()
			accept_event()


## 簽頭：展開或收合；一張統計：攤開或收起名單（收合時先展開清單）；
## 一張有地點的簽或一行名單：鏡頭飛往，展開的清單隨後收回。
func activate(select: int) -> void:
	if select == SELECT_HEADER:
		expanded = not expanded
	elif select >= MEMBER_TRACE_BASE:
		var t := _member_of(select)
		trace_requested.emit(int(shown()[t[0]]["members"][t[1]]["id"]))
	elif select >= TRACE_BASE:
		trace_requested.emit(int(shown()[select - TRACE_BASE]["id"]))
	elif select >= SLOT_BASE:
		person_requested.emit(main_person(shown()[select - SLOT_BASE]))
	elif select >= MEMBER_BASE:
		var m := _member_of(select)
		var member: Dictionary = shown()[m[0]]["members"][m[1]]
		if main_person(member) >= 0:
			person_requested.emit(main_person(member))
		elif member.has("pos"):
			fly_requested.emit(member["pos"])
			if expanded:
				expanded = false
	elif select >= 0:
		var entry := shown()[select]
		if is_group(entry):
			toggle_group(select)
		elif entry.has("pos"):
			fly_requested.emit(entry["pos"])
			if expanded:
				expanded = false


## 第 index 張統計攤開或收起；收合的簽先展開整個清單。
func toggle_group(index: int) -> void:
	var entry := shown()[index]
	var key: String = entry["key"]
	var was_open: bool = _open.get(key, false) and expanded
	if was_open:
		_open.erase(key)
	else:
		_open[key] = true
	if expanded:
		_shown_valid = false
		_refit()
	else:
		expanded = true


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and (_hover != SELECT_NONE or _pressed != SELECT_NONE):
		_hover = SELECT_NONE
		_pressed = SELECT_NONE
		queue_redraw()
