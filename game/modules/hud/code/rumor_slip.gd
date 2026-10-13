class_name RumorSlip
extends Control
## GDD 4.7、5.7 傳聞簽：畫面右上一列紙簽，每張是事件印、一句話（人物帶門派與境界）、原因標記與年月，新到舊，世界大事置頂。
## 支援的決定：該追哪一則——點一則鏡頭飛往該地。收合時最多 8 張（Deck 為 1 張）；展開是蓋滿畫面的一張紙（四邊各留 24 × 縮放，
## 底下的地圖壓暗），每則一列、寬度用滿，統計直接攤開名單，列出保留的全部（KEEP_MAX 則），頁首有篩選一列。
## 入簽依重要度分級（5.7「通知優先序」）：≥ 3 或世界大事入簽，2 只進展開清單，其餘不顯示；
## 同一魂同一年的多筆異常事件合併為一簽（取重要度高的）。
## 例行事件（7.2「傳聞的合成與標記」）同月同類合成一則統計，點開攤出名單；收合時異常先於例行統計。
## 內部分兩層：_records 是逐筆事件，_entries 是由它們合成的一張張簽（單筆，或同月同類的統計）。

## 點一則有地點的傳聞：飛往地圖座標 map_pos。
signal fly_requested(map_pos: Vector2)
## 點了「看人物」（或統計名單的一行）：開這個人的人物卡。
signal person_requested(person: int)
## 點了「看宗門」小印：開那一則事件所屬宗門的宗門卡。
signal sect_requested(sect: int)
## 點了「溯因」（或統計名單一行的「溯因」）：開這則事件的事件卡。
signal trace_requested(event: int)
## 全螢幕展開或收合了（Hud 據此暫停與復原時間、讓鏡頭讓出畫面）。
signal fullscreen_changed(on: bool)

## 收合時簽的寬：每筆帶「紀元名＋年＋（世界年）＋月」的日期（約 250 單位），句子才有地方放。
const WIDTH := 520.0
const ITEM_HEIGHT := 32.0
const ITEM_GAP := 5.0
const HEADER_HEIGHT := 26.0
## 收合時最多幾張；全螢幕清單最多留幾則。
const SHOWN_MAX := 8
const KEEP_MAX := 300
## 逐筆事件累積到這個數目就先合成一次（快轉時不讓它無限長）。
const RECORDS_MAX := 2000
## 入簽與只進清單的重要度門檻。
const SLIP_IMPORTANCE := 3
const NOTE_IMPORTANCE := 2
## 世界大事置頂多久（tick）：一世。
const PIN_TICKS := 360
const FADE_SECONDS := 0.35
const SEAL_SIZE := 22.0
const SEAL_WIDTH := 38.0
const PAD := 8.0
## 控制項四周留給柔影的空間，紙面從這裡起算。
const SHADOW := 8.0
## 全螢幕清單捲動：搖桿游標貼近上下緣時每秒捲的像素，方向鍵每秒捲的像素（皆乘縮放）。
const EDGE_ZONE := 26.0
const EDGE_SPEED := 320.0
const KEY_SCROLL_SPEED := 900.0
## 滾輪每格捲幾列。
const WHEEL_ROWS := 3
const SELECT_NONE := -1
const SELECT_HEADER := -2
## 點在紙外的暗處：收合。
const SELECT_DIM := -3
## 全螢幕：紙面離畫面四邊的距離、頁首與篩選列的高、每則與名單一行的高、清單離紙邊的左右內縮與底部留白（皆乘縮放）。
const FULL_MARGIN := 24.0
const FULL_HEADER_HEIGHT := 46.0
const FULL_FILTER_HEIGHT := 44.0
const FULL_ROW_HEIGHT := 44.0
const FULL_ROW_GAP := 2.0
const FULL_MEMBER_HEIGHT := 32.0
const FULL_SIDE := 22.0
const FULL_BOTTOM := 18.0
## 全螢幕名單一行相對於它所屬那一列的左內縮。
const FULL_MEMBER_INDENT := 52.0
## 全螢幕的事件印比收合時大一圈。
const FULL_SEAL_SIZE := 26.0
const FULL_SEAL_WIDTH := 46.0
## 篩選：全部與各類（歸類表在 EventText.FILTER_KINDS）。
const FILTERS: Array[String] = ["all", "world", "person", "sect", "dynasty", "karma"]
## 篩選鈕的選取碼 = FILTER_BASE + 序。
const FILTER_BASE := 20000000
## 攤開的名單一行的高度與行距；名單列的選取碼 = MEMBER_BASE + 簽序 × MEMBER_STRIDE + 名單序。
const MEMBER_HEIGHT := 22.0
const MEMBER_GAP := 2.0
const MEMBER_BASE := 1000000
const MEMBER_STRIDE := 10000
## 「看人物」小印的選取碼 = SLOT_BASE + 簽序。
const SLOT_BASE := 5000000
## 「溯因」小印的選取碼 = TRACE_BASE + 簽序；名單一行的 = MEMBER_TRACE_BASE + 簽序 × MEMBER_STRIDE + 名單序。
const TRACE_BASE := 6000000
## 「看宗門」小印的選取碼 = SECT_BASE + 簽序。
const SECT_BASE := 7000000
const MEMBER_TRACE_BASE := 10000000
## 原因標記的方框邊長。
const MARK_SIZE := 16.0
const MARK_WIDTH := 30.0
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
## 是否展開為全螢幕的清單。每次展開都從「全部」、頂端開始（點收合時的統計則捲到那一則）。
var expanded := false:
	set(value):
		if expanded == value:
			return
		expanded = value
		_shown_valid = false
		_scroll = 0.0
		_filter = 0
		_hover = SELECT_NONE
		_pressed = SELECT_NONE
		_refit()
		if expanded and not _focus_key.is_empty():
			_scroll_to(_focus_key)
		_focus_key = ""
		fullscreen_changed.emit(expanded)
## 紙面右緣與上緣在畫面上的位置。
var right_edge := 0.0:
	set(value):
		right_edge = value
		_place()
var top_edge := 0.0:
	set(value):
		top_edge = value
		_place()
## 收合時清單的高度上限（畫面上緣到印匣之間）。
var max_height := 600.0:
	set(value):
		max_height = value
		_refit()

## 收合時簽最寬能多寬（畫面上簽左邊與別的器物之間的空）；比 WIDTH 窄就縮窄，不出畫面。
var max_width := 100000.0:
	set(value):
		max_width = value
		_shown_valid = false
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
## 各簽已出現的時間（滑入動畫）。
var _ages := {}
## 全螢幕目前的篩選（FILTERS 的序）；下次展開要捲到哪一簽（key，空為頂端）；右端年月欄的寬。
var _filter := 0
var _focus_key := ""
var _stamp_w := 0.0
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
		"world_year": event.get("world_year", 0),
		"era": event.get("date_era", ""),
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
		"sect": sect_of(event),
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
	for entry in _entries:
		ages[entry["key"]] = _ages.get(entry["key"], 0.0)
	_ages = ages


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
		"world_year": first["world_year"],
		"era": first["era"],
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
## 收合時只列入簽的（重要度 ≥ 3 或世界大事），異常在前、例行統計在後，並受張數上限；
## 全螢幕時列出保留的全部，並依目前的篩選留下那一類。
func shown() -> Array[Dictionary]:
	if _dirty:
		_rebuild()
		_shown_valid = false
	if _shown_valid:
		return _shown_cache
	var out: Array[Dictionary] = []
	for entry in _entries:
		if expanded:
			if _filter == 0 or EventText.filter_of(entry["kind"]) == FILTERS[_filter]:
				out.append(entry)
		elif entry["world"] or int(entry["importance"]) >= SLIP_IMPORTANCE:
			out.append(entry)
	out.sort_custom(_before if expanded else _before_collapsed)
	if not expanded:
		out.resize(mini(out.size(), 1 if compact else SHOWN_MAX))
	_shown_cache = out
	_shown_valid = true
	_layout()
	return out


## 各簽的上緣與內容總高：全螢幕時統計的名單列直接攤開在該簽下。
func _layout() -> void:
	_tops.resize(_shown_cache.size())
	var y := 0.0
	for i in _shown_cache.size():
		_tops[i] = y
		y += _row_height_of(_shown_cache[i])
		if _is_open(_shown_cache[i]):
			y += _member_gap() + _shown_cache[i]["members"].size() * (_member_height() + _member_gap()) - _member_gap()
		y += _gap()
	_content_h = _row_height() if _shown_cache.is_empty() else y - _gap()
	if expanded:
		# 右端年月欄取全部列裡最寬的，列與列的右緣對齊。
		_stamp_w = 0.0
		var font := _font()
		for entry in _shown_cache:
			_stamp_w = maxf(_stamp_w, Ink.text_size(_stamp(entry), font).x)


func entry_count() -> int:
	if _dirty:
		_rebuild()
		_shown_valid = false
	return _entries.size()


## 全螢幕時統計一律攤開名單；收合時不攤。
func _is_open(entry: Dictionary) -> bool:
	return expanded and is_group(entry)


## 第 index 張統計是否攤開。
func is_open(index: int) -> bool:
	return _is_open(shown()[index])


func _stamp(entry: Dictionary) -> String:
	return EventText.date_label(entry.get("era", ""), int(entry.get("year", 0)), int(entry.get("world_year", 0)), int(entry["month"]))


func _queue_refit() -> void:
	if _refit_queued:
		return
	_refit_queued = true
	_refit_now.call_deferred()


func _refit_now() -> void:
	_refit_queued = false
	_refit()


## 收合時簽的寬。
func _width() -> float:
	return minf(WIDTH * _unit, max_width)


## 字級：收合時用註記字級，全螢幕用內文字級。
func _font() -> int:
	return Ink.font_size(Ink.SIZE_BODY if expanded else Ink.SIZE_NOTE, _unit)


func _row_height() -> float:
	return (FULL_ROW_HEIGHT if expanded else ITEM_HEIGHT) * _unit


## 收合時這一則的列高：那句話與年月放不進一行時，年月移到第二行，列加高一行。
func _row_height_of(entry: Dictionary) -> float:
	if not _stacked(entry):
		return _row_height()
	return _row_height() + Ink.text_size(_stamp(entry), _font()).y + 4.0 * _unit


## 收合時那句話最完整的樣子與年月是否擠不進同一行。
func _stacked(entry: Dictionary) -> bool:
	if expanded or entry["variants"].is_empty():
		return false
	var first: Array = entry["variants"][0]
	return Ink.text_size(String(first[0]) + String(first[1]), _font()).x > _line_room(entry, false)


## 收合時那句話在單行（連年月）或獨佔一行時可用的寬。
func _line_room(entry: Dictionary, stacked: bool) -> float:
	var seal_w := SEAL_WIDTH * _unit
	var room := _width() - PAD * 2.0 * _unit - seal_w - 8.0 * _unit - 8.0 * _unit
	if not stacked:
		room -= Ink.text_size(_stamp(entry), _font()).x
	if not String(entry["reason"]).is_empty():
		room -= 6.0 * _unit + _mark_size().x
	return room


## 收合時第一行（那句話所在）的高。
func _band_height() -> float:
	return _row_height()


func _gap() -> float:
	return (FULL_ROW_GAP if expanded else ITEM_GAP) * _unit


func _member_height() -> float:
	return (FULL_MEMBER_HEIGHT if expanded else MEMBER_HEIGHT) * _unit


func _member_gap() -> float:
	return MEMBER_GAP * _unit


## 全螢幕時列右端留給年月欄的寬（含與前一格的間距）。
func _stamp_reserve() -> float:
	return _stamp_w + 14.0 * _unit if expanded else 0.0


## 全螢幕時的畫面大小。
func _screen() -> Vector2:
	return get_viewport_rect().size if is_inside_tree() else Vector2(1920.0, 1080.0)


## 紙面左上角在控制項內的位置（收合時）。
func _origin() -> Vector2:
	return Vector2.ONE * SHADOW


## 全螢幕的紙面：畫面四邊各留 FULL_MARGIN × 縮放；收合時沒有這張紙，回傳空範圍。
func paper_rect() -> Rect2:
	if not expanded:
		return Rect2()
	var edge := Vector2.ONE * FULL_MARGIN * _unit
	return Rect2(edge, size - edge * 2.0)


## 收合時是簽頭；全螢幕時是紙面最上的一列（傳聞　共 N 則，右端收合鍵），點它收合。
func header_rect() -> Rect2:
	if expanded:
		var paper := paper_rect()
		return Rect2(paper.position, Vector2(paper.size.x, FULL_HEADER_HEIGHT * _unit))
	return Rect2(_origin(), Vector2(_width(), HEADER_HEIGHT * _unit))


## 全螢幕的篩選列（頁首之下）。
func _filter_row() -> Rect2:
	var head := header_rect()
	return Rect2(Vector2(head.position.x, head.end.y), Vector2(head.size.x, FULL_FILTER_HEIGHT * _unit))


func filter_label(index: int) -> String:
	return Locale.t("rumor_filter_" + FILTERS[index])


## 目前篩選的序（FILTERS）。
func filter_index() -> int:
	return _filter


## 換篩選：回到清單頂端。
func set_filter(index: int) -> void:
	index = clampi(index, 0, FILTERS.size() - 1)
	if index == _filter:
		return
	_filter = index
	_scroll = 0.0
	_shown_valid = false
	_refit()


## 第 index 個篩選鈕的範圍（控制項座標，全螢幕）：左起依序排開。
func filter_rect(index: int) -> Rect2:
	var row := _filter_row()
	var font := _font()
	var pad := 18.0 * _unit
	var x := row.position.x + FULL_SIDE * _unit
	var height := row.size.y - 10.0 * _unit
	var rect := Rect2()
	for i in index + 1:
		var width := Ink.text_size(filter_label(i), font).x + pad * 2.0
		rect = Rect2(Vector2(x, row.position.y + (row.size.y - height) * 0.5), Vector2(width, height))
		x += width + 8.0 * _unit
	return rect


## 清單的可視範圍（簽頭或篩選列下方到底）。
func _view_rect() -> Rect2:
	if expanded:
		var paper := paper_rect()
		var top_y := _filter_row().end.y + 4.0 * _unit
		var side := FULL_SIDE * _unit
		return Rect2(Vector2(paper.position.x + side, top_y), Vector2(paper.size.x - side * 2.0, paper.end.y - FULL_BOTTOM * _unit - top_y))
	var head := header_rect()
	var top_y := head.end.y + _gap()
	return Rect2(Vector2(head.position.x, top_y), Vector2(head.size.x, size.y - SHADOW - top_y))


func _content_height() -> float:
	shown()
	return _content_h


func _max_scroll() -> float:
	return maxf(0.0, _content_height() - _view_rect().size.y)


## 把清單捲到 key 那一簽的上緣（捲不到的取到底）。
func _scroll_to(key: String) -> void:
	shown()
	for i in _shown_cache.size():
		if _shown_cache[i]["key"] == key:
			_scroll = clampf(_tops[i], 0.0, _max_scroll())
			return


func _scroll_by(pixels: float) -> void:
	var before := _scroll
	_scroll = clampf(_scroll + pixels, 0.0, _max_scroll())
	if _scroll != before:
		_update_hover(_pointer)
		queue_redraw()


## 第 index 張的紙面（控制項座標）；捲動後可能落在可視範圍外。
func row_rect(index: int) -> Rect2:
	var view := _view_rect()
	shown()
	var top := _tops[index] if index < _tops.size() else 0.0
	var height := _row_height_of(_shown_cache[index]) if index < _shown_cache.size() else _row_height()
	return Rect2(view.position + Vector2(0.0, top - _scroll), Vector2(view.size.x, height))


## 第 index 張統計攤開後的第 member 行（控制項座標）；全螢幕時縮進在該列的印之下。
func member_rect(index: int, member: int) -> Rect2:
	var row := row_rect(index)
	var top := row.end.y + _member_gap() + member * (_member_height() + _member_gap())
	var indent := FULL_MEMBER_INDENT * _unit if expanded else 0.0
	return Rect2(Vector2(row.position.x + indent, top), Vector2(row.size.x - indent, _member_height()))


## 一則的主要人物編號：事件的主角，沒有主角取牽涉的第一位；沒有為 -1（統計也沒有）。
static func main_person(entry: Dictionary) -> int:
	var subject := int(entry.get("a_person", -1))
	if subject >= 0:
		return subject
	var persons: PackedInt32Array = entry.get("persons", PackedInt32Array())
	return persons[0] if not persons.is_empty() else -1


## 一則事件所屬的宗門編號：主角當時的宗門，沒有主角的宗門事件取事件的第一個勢力；沒有為 -1。
static func sect_of(event: Dictionary) -> int:
	if int(event.get("a_power_id", -1)) >= 0:
		return int(event["a_power_id"])
	return int(event.get("f_power", -1))


## 第 index 張是否有「看宗門」入口：單則且事件有所屬宗門。
func has_sect_slot(index: int) -> bool:
	var entry := shown()[index]
	return not is_group(entry) and int(entry.get("sect", -1)) >= 0


## 第 index 張右端的「看人物」「溯因」兩個入口，停留時取代年月（全螢幕時年月欄固定，入口排在它左邊）：
## 看人物是最右一顆小印（只有主要人物明確的單則才有），溯因在它左邊；沒有看人物的單則，溯因移到最右。
## 統計沒有單一事件，兩者都沒有。
func action_slots(index: int) -> Dictionary:
	var row := row_rect(index)
	var font := _font()
	var width := maxf(SLOT_WIDTH * _unit, Ink.text_size(Locale.t("rumor_person"), font).x + 12.0 * _unit)
	var gap := SLOT_GAP * _unit
	var right := row.end.x - PAD * _unit - _stamp_reserve()
	var person := Rect2(Vector2(right - width, row.position.y + 3.0 * _unit), Vector2(width, _band_height() - 6.0 * _unit))
	var trace_x := person.position.x - gap - SLOT_WIDTH * _unit if has_person_slot(index) else right - SLOT_WIDTH * _unit
	var trace := Rect2(Vector2(trace_x, row.position.y + 3.0 * _unit), Vector2(SLOT_WIDTH * _unit, _band_height() - 6.0 * _unit))
	var sect_width := maxf(SLOT_WIDTH * _unit, Ink.text_size(Locale.t("rumor_sect"), font).x + 12.0 * _unit)
	var sect := Rect2(Vector2(trace.position.x - gap - sect_width, row.position.y + 3.0 * _unit), Vector2(sect_width, _band_height() - 6.0 * _unit))
	return {"person": person, "trace": trace, "sect": sect}


## 第 index 張是否有「溯因」入口：單則都有，統計沒有。
func has_trace_slot(index: int) -> bool:
	return not is_group(shown()[index])


## 統計攤開後第 member 行右端的兩個入口：看人物（或飛往）的字在最右，溯因小印在它左邊。
func member_slots(index: int, member: int) -> Dictionary:
	var rect := member_rect(index, member)
	var font := _font()
	var entry: Dictionary = shown()[index]["members"][member]
	var tail := Locale.t("rumor_person" if main_person(entry) >= 0 else "rumor_fly")
	var tail_w := Ink.text_size(tail, font).x + 8.0 * _unit
	var right := rect.end.x - PAD * _unit - tail_w
	var width := SLOT_WIDTH * _unit
	var trace := Rect2(Vector2(right - width, rect.position.y + 2.0 * _unit), Vector2(width, rect.size.y - 4.0 * _unit))
	return {"tail_width": tail_w, "trace": trace}


## 第 index 張是否有「看人物」入口。
func has_person_slot(index: int) -> bool:
	var entry := shown()[index]
	return not is_group(entry) and main_person(entry) >= 0


## 第 index 張那句話的位置（不停留時）：事件印之後、原因標記與年月之前。
func line_rect(index: int) -> Rect2:
	var entry := shown()[index]
	var row := row_rect(index)
	var seal_w := (FULL_SEAL_WIDTH if expanded else SEAL_WIDTH) * _unit
	var left := row.position.x + PAD * _unit + seal_w + 8.0 * _unit
	var right := row.end.x - PAD * _unit
	if expanded:
		right -= _stamp_reserve()
	elif not _stacked(entry):
		right -= Ink.text_size(_stamp(entry), _font()).x
	if not String(entry["reason"]).is_empty():
		right -= 6.0 * _unit + _mark_size().x
	right -= 8.0 * _unit
	return Rect2(Vector2(left, row.position.y), Vector2(maxf(right - left, 0.0), row.size.y if expanded else _band_height()))


## 第 index 張那句話依空間縮到放得進的樣子。
func row_line(index: int) -> String:
	return fit_variants(shown()[index]["variants"], line_rect(index).size.x, _font())


## 原因標記的方框大小。
func _mark_size() -> Vector2:
	return Vector2(38.0 if expanded else MARK_WIDTH, 20.0 if expanded else MARK_SIZE) * _unit


## 目前看得到（落在可視範圍內）的傳聞張數。
func visible_row_count() -> int:
	var count := 0
	for i in shown().size():
		if _view_rect().intersects(row_rect(i)):
			count += 1
	return count


## 簽佔住的畫面範圍（地圖標籤要避開）；全螢幕是那張紙。
func occupied_rect() -> Rect2:
	if expanded:
		var paper := paper_rect()
		paper.position += position
		return paper
	var view := _view_rect()
	var bottom := minf(view.position.y + _content_height() - _scroll, view.end.y)
	var rect := header_rect().merge(Rect2(view.position, Vector2(view.size.x, maxf(bottom - view.position.y, 0.0))))
	rect.position += position
	return rect


## 畫面大小變了：全螢幕的紙跟著重排。
func fit_screen() -> void:
	if expanded:
		_refit()


func _refit() -> void:
	if _unit <= 0.0:
		return
	if expanded:
		custom_minimum_size = _screen()
		size = custom_minimum_size
		_scroll = clampf(_scroll, 0.0, _max_scroll())
		_place()
		queue_redraw()
		return
	var natural := header_rect().size.y + _gap() + _content_height()
	var height := natural
	custom_minimum_size = Vector2(_width(), height) + Vector2.ONE * SHADOW * 2.0
	size = custom_minimum_size
	_scroll = clampf(_scroll, 0.0, _max_scroll())
	_place()
	queue_redraw()


func _place() -> void:
	if expanded:
		position = Vector2.ZERO
		return
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
	if expanded:
		# 方向鍵、十字鍵與左搖桿上下：連續捲動；左右：換篩選。
		var axis := Input.get_axis(InputActions.MENU_UP, InputActions.MENU_DOWN)
		if axis != 0.0:
			_scroll_by(axis * KEY_SCROLL_SPEED * _unit * delta)
		if Input.is_action_just_pressed(InputActions.MENU_LEFT):
			set_filter(_filter - 1)
		elif Input.is_action_just_pressed(InputActions.MENU_RIGHT):
			set_filter(_filter + 1)
		# 搖桿游標貼近清單上下緣時自己捲（鍵鼠不會，免得點最後一列時清單跑掉）。
		if gamepad and _max_scroll() > 0.0:
			var view := _view_rect()
			var step := 0.0
			if view.has_point(_pointer):
				if _pointer.y < view.position.y + EDGE_ZONE * _unit:
					step = -EDGE_SPEED
				elif _pointer.y > view.end.y - EDGE_ZONE * _unit:
					step = EDGE_SPEED
			if step != 0.0:
				_scroll_by(step * _unit * delta)
	if dirty:
		queue_redraw()


func _unhandled_input(event: InputEvent) -> void:
	if not expanded or not (event is InputEventKey):
		return
	var key_event := event as InputEventKey
	if not key_event.pressed:
		return
	var page := _view_rect().size.y - _row_height()
	match key_event.keycode:
		KEY_PAGEDOWN:
			_scroll_by(page)
		KEY_PAGEUP:
			_scroll_by(-page)
		KEY_HOME:
			_scroll_by(-_scroll)
		KEY_END:
			_scroll_by(_max_scroll())
		_:
			return
	get_viewport().set_input_as_handled()


func _draw() -> void:
	if expanded:
		_draw_full()
		return
	var note := _font()
	_draw_header(note)
	var entries := shown()
	if entries.is_empty():
		var rect := row_rect(0)
		Ink.paper_slip(self, rect)
		var text := Locale.t("rumor_empty")
		Ink.text(self, rect.position + Vector2(PAD * _unit * 1.5, (rect.size.y - Ink.text_size(text, note).y) * 0.5), text, note, Ink.INK_LIGHT)
		return
	_draw_rows(entries, _view_rect(), note)


## 畫可視範圍內的每一簽，與攤開的名單列。
func _draw_rows(entries: Array[Dictionary], view: Rect2, font: int) -> void:
	for i in entries.size():
		var rect := row_rect(i)
		var open := _is_open(entries[i])
		var block := rect.merge(member_rect(i, entries[i]["members"].size() - 1)) if open else rect
		if not view.intersects(block):
			continue
		if view.intersects(rect):
			_draw_row(entries[i], rect, i, font)
		if open:
			for j in entries[i]["members"].size():
				var mrect := member_rect(i, j)
				if view.intersects(mrect):
					_draw_member(entries[i]["members"][j], mrect, MEMBER_BASE + i * MEMBER_STRIDE + j, font)


## 蓋一塊紙色（與整張紙同一個紋理相位），把超出清單範圍的列遮掉。
func _cover(rect: Rect2) -> void:
	if rect.size.x <= 0.0 or rect.size.y <= 0.0:
		return
	if Ink.paper_texture == null:
		draw_rect(rect, Ink.PAPER_LIGHT, true)
	else:
		draw_texture_rect_region(Ink.paper_texture, rect, Rect2(rect.position, rect.size), Ink.PAPER_LIGHT)


## 全螢幕：壓暗底下的地圖，一張蓋滿畫面（留邊）的紙，頁首、篩選一列與清單。
func _draw_full() -> void:
	var font := _font()
	draw_rect(Rect2(Vector2.ZERO, size), Color(Ink.INK, 0.62), true)
	var paper := paper_rect()
	Ink.paper_slip(self, paper, Ink.CARD_BAND)
	var entries := shown()
	var view := _view_rect()
	if entries.is_empty():
		var text := Locale.t("rumor_empty")
		var extent := Ink.text_size(text, font)
		Ink.text(self, view.get_center() - extent * 0.5, text, font, Ink.INK_LIGHT)
	else:
		_draw_rows(entries, view, font)
	# 列捲出清單範圍的部分由紙色蓋掉，頁首與篩選列畫在它之上。
	var inner := paper.grow(-1.0)
	_cover(Rect2(inner.position, Vector2(inner.size.x, view.position.y - inner.position.y)))
	_cover(Rect2(Vector2(inner.position.x, view.end.y), Vector2(inner.size.x, inner.end.y - view.end.y)))
	_draw_full_header(font)


func _draw_full_header(font: int) -> void:
	var head := header_rect()
	if _hover == SELECT_HEADER:
		draw_rect(head.grow(-1.0), Color(Ink.INK, 0.1 if _pressed != SELECT_HEADER else 0.2), true)
	var title_size := Ink.font_size(Ink.SIZE_TITLE, _unit)
	var title := "%s　%s" % [Locale.t("rumor_title"), Locale.t("rumor_total").format({"n": shown().size()})]
	var title_h := Ink.text_size(title, title_size).y
	Ink.text(self, Vector2(head.position.x + FULL_SIDE * _unit, head.position.y + (head.size.y - title_h) * 0.5), title, title_size, Ink.INK)
	# 右端：收合鍵提示。
	var label := Locale.t("rumor_collapse")
	var key := _list_key()
	var key_width := Ink.key_glyph_width(key, _unit)
	var right := head.end.x - FULL_SIDE * _unit
	var key_x := right - key_width
	if not key.is_empty():
		Ink.key_glyph(self, Vector2(key_x, head.position.y + (head.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), key, _unit, not gamepad)
	var label_x := (key_x - 8.0 * _unit if not key.is_empty() else right) - Ink.text_size(label, font).x
	Ink.text(self, Vector2(label_x, head.position.y + (head.size.y - Ink.text_size(label, font).y) * 0.5), label, font, Ink.INK)
	# 篩選一列。
	var row := _filter_row()
	for i in FILTERS.size():
		var rect := filter_rect(i)
		var code := FILTER_BASE + i
		var ink := Ink.plate(self, rect, i == _filter, true, _hover == code, _pressed == code)
		var text := filter_label(i)
		Ink.text(self, rect.get_center() - Ink.text_size(text, font) * 0.5, text, font, ink)
	draw_line(Vector2(head.position.x + FULL_SIDE * _unit, row.end.y), Vector2(head.end.x - FULL_SIDE * _unit, row.end.y), Color(Ink.GOLD, 0.7), 1.0)


func _draw_header(note: int) -> void:
	var rect := header_rect()
	Ink.paper_slip(self, rect)
	if _hover == SELECT_HEADER:
		draw_rect(rect.grow(-1.0), Color(Ink.INK, 0.1 if _pressed != SELECT_HEADER else 0.2), true)
	var section := Ink.font_size(Ink.SIZE_SECTION, _unit)
	var y := rect.position.y + (rect.size.y - Ink.text_size(Locale.t("rumor_title"), section).y) * 0.5
	Ink.section(self, Vector2(rect.position.x + PAD * _unit, y), Locale.t("rumor_title"), section)
	# 右端：展開的字與實體鍵。
	var label := Locale.t("rumor_expand")
	var key := _list_key()
	var key_width := Ink.key_glyph_width(key, _unit)
	var right := rect.end.x - PAD * _unit
	var key_x := right - key_width
	if not key.is_empty():
		Ink.key_glyph(self, Vector2(key_x, rect.position.y + (rect.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), key, _unit, not gamepad)
	var label_x := (key_x - 6.0 * _unit if not key.is_empty() else right) - Ink.text_size(label, note).x
	Ink.text(self, Vector2(label_x, rect.position.y + (rect.size.y - Ink.text_size(label, note).y) * 0.5), label, note, Ink.INK)


func _draw_row(entry: Dictionary, rect: Rect2, index: int, note: int) -> void:
	var full := expanded
	var small := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var fade := clampf(float(_ages.get(entry["key"], FADE_SECONDS)) / FADE_SECONDS, 0.0, 1.0)
	# 新到的簽從上方滑入。
	var slid := rect
	slid.position.y -= (1.0 - fade) * 10.0 * _unit
	var slot_hot := _hover == SLOT_BASE + index
	var trace_hot := _hover == TRACE_BASE + index
	var sect_hot := _hover == SECT_BASE + index
	var group := is_group(entry)
	var hot := (_hover == index and not (full and group)) or slot_hot or trace_hot or sect_hot
	var pressed := _hover == index and _pressed == index and not (full and group)
	var has_pos: bool = entry.has("pos")
	var clickable := has_pos or (group and not full)
	var inked := Ink.INK if int(entry["importance"]) >= SLIP_IMPORTANCE or entry["world"] else Ink.INK_LIGHT
	if full:
		# 全螢幕的列直接寫在整張紙上，列與列之間一條淡線。
		draw_line(Vector2(slid.position.x, slid.end.y + _gap() * 0.5), Vector2(slid.end.x, slid.end.y + _gap() * 0.5), Color(Ink.INK_LIGHT, 0.28), 1.0)
		if hot:
			draw_rect(slid, Color(Ink.INK, 0.14 if pressed else 0.06), true)
			draw_rect(slid.grow(-1.0), Ink.INK if clickable else Ink.INK_LIGHT, false, 1.0)
	else:
		Ink.paper_slip(self, slid.grow(-1.0) if pressed else slid)
		if hot:
			draw_rect(slid.grow(-1.0), Color(Ink.INK, 0.2 if pressed else 0.07), true)
			draw_rect(slid.grow(-2.0), Ink.INK if clickable else Ink.INK_LIGHT, false, 1.0)
	# 事件印：橫長方形二字朱印，世界大事用靛。
	var seal_size := (FULL_SEAL_SIZE if full else SEAL_SIZE) * _unit
	var seal_width := (FULL_SEAL_WIDTH if full else SEAL_WIDTH) * _unit
	var seal_rect := Rect2(slid.position + Vector2(PAD * _unit, (slid.size.y - seal_size) * 0.5), Vector2(seal_width, seal_size))
	draw_rect(seal_rect, Color(Ink.SILK if entry["world"] else Ink.SEAL_RED, fade), true)
	draw_rect(seal_rect.grow(-2.0), Color(Ink.PAPER_LIGHT, 0.7 * fade), false, 1.0)
	var glyph_size := Ink.font_size(Ink.SIZE_SECTION, _unit)
	var glyph: String = entry["seal"]
	Ink.text(self, seal_rect.get_center() - Ink.text_size(glyph, glyph_size) * 0.5, glyph, glyph_size, Color(Ink.PAPER_LIGHT, fade))
	var right := slid.end.x - PAD * _unit
	var tail := ""
	var key := ""
	# 收合且年月擠不進同一行：第一行只放那句話，年月在第二行靠右。
	var stacked := _stacked(entry)
	var band := _band_height() if stacked else slid.size.y
	if stacked:
		var second := _stamp(entry)
		var second_size := Ink.text_size(second, note)
		Ink.text(self, Vector2(slid.end.x - PAD * _unit - second_size.x, slid.position.y + band + (slid.size.y - band - second_size.y) * 0.5), second, note, Color(Ink.INK_LIGHT, fade))
	if full:
		# 全螢幕：年月欄固定在最右，入口與「飛往」排在它左邊。
		var stamp := _stamp(entry)
		var stamp_size := Ink.text_size(stamp, note)
		Ink.text(self, Vector2(right - stamp_size.x, slid.position.y + (band - stamp_size.y) * 0.5), stamp, note, Color(Ink.INK_LIGHT, fade))
		right -= _stamp_reserve()
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
		if has_sect_slot(index):
			# 「看宗門」小印在溯因的左邊。
			var sect: Rect2 = action_slots(index)["sect"]
			var pressed_sect := sect_hot and _pressed == SECT_BASE + index
			var sect_ink := Color(Ink.SEAL_RED, fade)
			draw_rect(sect, Color(Ink.SEAL_RED, 0.28 if pressed_sect else (0.14 if sect_hot else 0.0)), true)
			draw_rect(sect, sect_ink, false, 1.0)
			var sect_text := Locale.t("rumor_sect")
			Ink.text(self, sect.get_center() - Ink.text_size(sect_text, note) * 0.5, sect_text, note, sect_ink)
			right = sect.position.x - 6.0 * _unit
	# 收合：停留時右端浮出「飛往」加實體鍵（統計是展開），否則是年月；全螢幕：只有「飛往」，年月欄不動。
	if full:
		if hot and has_pos and not slot_hot and not trace_hot and not sect_hot:
			tail = Locale.t("rumor_fly")
			key = _fly_key()
	elif hot and group:
		tail = Locale.t("rumor_expand")
	elif hot and has_pos and not slot_hot and not trace_hot and not sect_hot:
		tail = Locale.t("rumor_fly")
		key = _fly_key()
	elif slot_hot or trace_hot or sect_hot:
		# 游標在小印上：年月讓位，把寬度留給那句話。
		tail = ""
	elif not stacked:
		tail = _stamp(entry)
	var key_width := Ink.key_glyph_width(key, _unit)
	if not key.is_empty():
		right -= key_width
		Ink.key_glyph(self, Vector2(right, slid.position.y + (band - Ink.KEY_HEIGHT * _unit) * 0.5), key, _unit)
		right -= 4.0 * _unit
	var tail_size := Ink.text_size(tail, note)
	right -= tail_size.x
	Ink.text(self, Vector2(right, slid.position.y + (band - tail_size.y) * 0.5), tail, note, Color(Ink.SEAL_RED if hot and clickable else Ink.INK_LIGHT, fade))
	# 原因標記：小方框二字，至多一個。
	var reason: String = entry["reason"]
	if not reason.is_empty():
		var mark := _mark_size()
		var mark_rect := Rect2(Vector2(right - 6.0 * _unit - mark.x, slid.position.y + (band - mark.y) * 0.5), mark)
		var mark_color := Color(Ink.SILK if reason == "world" else Ink.SEAL_RED, fade)
		var mark_text := Locale.t("reason_" + reason)
		draw_rect(mark_rect, mark_color, false, 1.0)
		Ink.text(self, mark_rect.get_center() - Ink.text_size(mark_text, small) * 0.5, mark_text, small, mark_color)
		right = mark_rect.position.x
	# 一句話：依空間由全到簡，結果不被截。
	var text_x := seal_rect.end.x + 8.0 * _unit
	var line := fit_variants(entry["variants"], right - 8.0 * _unit - text_x, note)
	Ink.text(self, Vector2(text_x, slid.position.y + (band - Ink.text_size(line, note).y) * 0.5), line, note, Color(inked, fade))


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


## 控制項座標的 point 落在哪：紙外的暗處（全螢幕）、簽頭、篩選鈕、第幾張、攤開名單的第幾行（選取碼），或沒有。
func _select_at(point: Vector2) -> int:
	if expanded:
		if not paper_rect().has_point(point):
			return SELECT_DIM
		for i in FILTERS.size():
			if filter_rect(i).has_point(point):
				return FILTER_BASE + i
	if header_rect().has_point(point):
		return SELECT_HEADER
	if not _view_rect().has_point(point):
		return SELECT_NONE
	var entries := shown()
	for i in entries.size():
		if row_rect(i).has_point(point):
			if has_person_slot(i) and action_slots(i)["person"].has_point(point):
				return SLOT_BASE + i
			if has_sect_slot(i) and action_slots(i)["sect"].has_point(point):
				return SECT_BASE + i
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
	if select < MEMBER_BASE or (select >= SLOT_BASE and select < MEMBER_TRACE_BASE) or select >= FILTER_BASE:
		return []
	var rest := select - (MEMBER_TRACE_BASE if select >= MEMBER_TRACE_BASE else MEMBER_BASE)
	return [rest / MEMBER_STRIDE, rest % MEMBER_STRIDE]


## 收合時游標只在簽頭與簽上才攔截，其餘讓地圖收到事件；全螢幕時整個畫面都攔下。
func _has_point(point: Vector2) -> bool:
	return expanded or _select_at(point) != SELECT_NONE


## 游標停在這裡時是否可以按（GDD 5.9 游標圖示）：簽頭、篩選鈕、有地點的簽與名單行；
## 收合時的統計也可按（展開），全螢幕的統計列不是按鈕。
func is_hot(point: Vector2) -> bool:
	var at := _select_at(point)
	if at == SELECT_HEADER or at >= SLOT_BASE:
		return true
	if at >= MEMBER_BASE:
		var m := _member_of(at)
		var member: Dictionary = shown()[m[0]]["members"][m[1]]
		return member.has("pos") or main_person(member) >= 0
	return at >= 0 and (shown()[at].has("pos") or (is_group(shown()[at]) and not expanded))


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
			_scroll_by(dir * WHEEL_ROWS * (_row_height() + _gap()))
			_update_hover(button.position)
			accept_event()


## 簽頭：展開或收合；紙外的暗處與篩選鈕（全螢幕）；一張統計：收合時展開成全螢幕並捲到它；
## 一張有地點的簽或一行名單：鏡頭飛往，全螢幕隨後收回。
func activate(select: int) -> void:
	if select == SELECT_HEADER:
		expanded = not expanded
	elif select == SELECT_DIM:
		expanded = false
	elif select >= FILTER_BASE:
		set_filter(select - FILTER_BASE)
	elif select >= MEMBER_TRACE_BASE:
		var t := _member_of(select)
		trace_requested.emit(int(shown()[t[0]]["members"][t[1]]["id"]))
	elif select >= SECT_BASE:
		sect_requested.emit(int(shown()[select - SECT_BASE]["sect"]))
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
			_replay(member)
			if expanded:
				expanded = false
	elif select >= 0:
		var entry := shown()[select]
		if is_group(entry):
			open_group(select)
		elif entry.has("pos"):
			fly_requested.emit(entry["pos"])
			_replay(entry)
			if expanded:
				expanded = false


## 飛往之後補放該事件的地圖奇觀（GDD 5.6）；沒有奇觀模組或不是奇觀事件時什麼都不發生。
func _replay(record: Dictionary) -> void:
	if is_inside_tree():
		get_tree().call_group("spectacle", "replay", record)


## 收合時點了第 index 張統計：展開成全螢幕並捲到那一則（全螢幕的統計本來就攤開，不必再點）。
func open_group(index: int) -> void:
	if expanded:
		return
	_focus_key = shown()[index]["key"]
	expanded = true


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and (_hover != SELECT_NONE or _pressed != SELECT_NONE):
		_hover = SELECT_NONE
		_pressed = SELECT_NONE
		queue_redraw()
