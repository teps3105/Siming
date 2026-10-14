class_name WorldHistoryBar
extends RefCounted
## GDD 5.10 天下冊首頁頂端的世界簡史橫條：開天到今天一整條，各紀元依長短分段，下方一道細帶標出成住壞空，
## 右端是現在。全遊戲只有這裡寫開天以來的年數（世界年）。
## 量與畫同一組函式：item 為 null 時只量。座標是 Almanac 的畫面座標，band 是整塊橫條的範圍。

## 一元（一輪成住壞空）與四段的起訖（2.6「成住壞空」）；世界時間跨元一路累加，所以第 129,601 年又是新一輪的成。
const CYCLE_YEARS := 129600
const PHASES: Array[Dictionary] = [
	{"key": "history_phase_cheng", "from": 0, "to": 27000},
	{"key": "history_phase_zhu", "from": 27000, "to": 113400},
	{"key": "history_phase_huai", "from": 113400, "to": 118800},
	{"key": "history_phase_kong", "from": 118800, "to": 129600},
]
const PHASE_TINTS: Array[Color] = [Ink.CYAN, Ink.GOLD, Ink.SEAL_RED, Ink.INK_LIGHT]
## 起因性質的鍵（era_table 的 kind）對應的字串鍵。
const KIND_KEYS := {
	"genesis": "history_kind_genesis", "cold": "history_kind_cold", "warm": "history_kind_warm",
	"decline": "history_kind_decline", "revival": "history_kind_revival", "sundering": "history_kind_sundering",
	"cataclysm": "history_kind_cataclysm", "restoration": "history_kind_restoration", "aftermath": "history_kind_aftermath",
	"peace": "history_kind_peace",
}

const TITLE_HEIGHT := 24.0
const BAR_HEIGHT := 28.0
const STRIP_HEIGHT := 18.0
const LINE_HEIGHT := 20.0
const GAP := 4.0
## 一段紀元再短也要畫出這麼寬，才點得到。
const MIN_SEGMENT := 6.0
const SEGMENT_GAP := 1.0

## era_table() 的每一筆（name、kind、start_year、end_year、event）。
var rows: Array = []
## 選取的一段；沒有紀元時為 -1。預設是現在的紀元。
var selected := -1
var hover := -1
## 橫條標題右端的操作提示（滑鼠與鍵盤、搖桿各一種寫法），由天下冊依輸入裝置設定。
var hint := ""


func load_rows(table: Array) -> void:
	rows = table
	selected = rows.size() - 1


## 開天至今幾年（最後一個紀元的最後一年）。
func total_years() -> int:
	return int(rows[rows.size() - 1]["end_year"]) if not rows.is_empty() else 0


func era_count() -> int:
	return rows.size()


func era_years(index: int) -> int:
	var row: Dictionary = rows[index]
	return int(row["end_year"]) - int(row["start_year"]) + 1


## 千分位：12356 → 12,356。
static func thousands(n: int) -> String:
	var text := str(absi(n))
	var out := ""
	for i in text.length():
		if i > 0 and (text.length() - i) % 3 == 0:
			out += ","
		out += text[i]
	return ("-" if n < 0 else "") + out


## 橫條標題：「開天至今 12,356 年、5 個紀元」。
func headline() -> String:
	return Locale.t("history_headline").format({"years": thousands(total_years()), "eras": str(era_count())})


## 各段紀元的寬度（總和 = width，最短 min_width；紀元多到每段都排不下最短時平均分）。
static func segment_widths(lengths: Array, width: float, min_width: float) -> Array[float]:
	var out: Array[float] = []
	var count := lengths.size()
	if count == 0:
		return out
	if min_width * count >= width:
		for i in count:
			out.append(width / count)
		return out
	var fixed: Array[bool] = []
	fixed.resize(count)
	fixed.fill(false)
	var widths: Array[float] = []
	widths.resize(count)
	widths.fill(0.0)
	# 比例算出來小於最短的段釘在最短，剩下的寬度再按比例分給其餘各段，直到沒有新的段被釘住。
	for _round in count:
		var free_width := width
		var free_length := 0.0
		for i in count:
			if fixed[i]:
				free_width -= min_width
			else:
				free_length += float(lengths[i])
		var changed := false
		for i in count:
			if fixed[i]:
				widths[i] = min_width
				continue
			widths[i] = free_width * float(lengths[i]) / free_length if free_length > 0.0 else free_width / count
			if widths[i] < min_width:
				fixed[i] = true
				changed = true
		if not changed:
			break
	return widths


## 成住壞空在 [0, total_years] 內的各段：每筆 key、tint、from、to（年，0 起算），世界時間跨元時一輪接一輪。
static func phase_spans(total: int) -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	var cycle := 0
	while cycle * CYCLE_YEARS < total:
		for i in PHASES.size():
			var from: int = cycle * CYCLE_YEARS + int(PHASES[i]["from"])
			var to: int = cycle * CYCLE_YEARS + int(PHASES[i]["to"])
			if from >= total:
				break
			out.append({"key": PHASES[i]["key"], "tint": PHASE_TINTS[i], "from": from, "to": mini(to, total)})
		cycle += 1
	return out


## 第 year 年（1 起算）落在成住壞空的哪一段：0 成、1 住、2 壞、3 空。
static func phase_at(year: int) -> int:
	var offset := (maxi(year, 1) - 1) % CYCLE_YEARS
	for i in PHASES.size():
		if offset < int(PHASES[i]["to"]):
			return i
	return PHASES.size() - 1


# ── 版面 ─────────────────────────────────────────────────────

## 整塊橫條要多高（含下方兩行說明）。
static func band_height(unit: float) -> float:
	return (TITLE_HEIGHT + GAP + BAR_HEIGHT + GAP + STRIP_HEIGHT + GAP * 1.5 + LINE_HEIGHT * 2.0 + GAP * 2.0) * unit


func bar_rect(band: Rect2, unit: float) -> Rect2:
	return Rect2(band.position + Vector2(0.0, (TITLE_HEIGHT + GAP) * unit), Vector2(band.size.x, BAR_HEIGHT * unit))


func strip_rect(band: Rect2, unit: float) -> Rect2:
	var bar := bar_rect(band, unit)
	return Rect2(Vector2(bar.position.x, bar.end.y + GAP * unit), Vector2(bar.size.x, STRIP_HEIGHT * unit))


## 每段紀元的範圍（畫面座標）。
func segment_rects(band: Rect2, unit: float) -> Array[Rect2]:
	var bar := bar_rect(band, unit)
	var lengths: Array = []
	for i in rows.size():
		lengths.append(era_years(i))
	var widths := segment_widths(lengths, bar.size.x, MIN_SEGMENT * unit)
	var out: Array[Rect2] = []
	var x := bar.position.x
	for width in widths:
		out.append(Rect2(Vector2(x, bar.position.y), Vector2(width, bar.size.y)))
		x += width
	return out


## 點在第幾段紀元上；不在橫條上為 -1。
func era_at(point: Vector2, band: Rect2, unit: float) -> int:
	var bar := bar_rect(band, unit)
	if not bar.grow(2.0 * unit).has_point(point):
		return -1
	var rects := segment_rects(band, unit)
	for i in rects.size():
		if point.x >= rects[i].position.x and point.x < rects[i].end.x:
			return i
	return rects.size() - 1 if not rects.is_empty() and point.x >= rects[rects.size() - 1].end.x else -1


## 選取的鄰近一段（delta 為 ±1），停在兩端。
func step(delta: int) -> void:
	if rows.is_empty():
		return
	selected = clampi(selected + delta, 0, rows.size() - 1)


# ── 文字 ─────────────────────────────────────────────────────

func kind_text(index: int) -> String:
	return Locale.t(KIND_KEYS.get(String(rows[index]["kind"]), "history_kind_peace"))


## 一段紀元的摘要：「鴻蒙紀元　第 1 至 3,500 年（共 3,500 年）　起因：開天」。
func era_text(index: int) -> String:
	var row: Dictionary = rows[index]
	return Locale.t("history_era").format({
		"name": row["name"], "from": thousands(int(row["start_year"])), "to": thousands(int(row["end_year"])),
		"years": thousands(era_years(index)), "kind": kind_text(index),
	})


## 這一段起因的世界大事 id；開天、開局前的紀元與已被帳修剪的事件為 -1。
func event_id(index: int) -> int:
	return int(rows[index]["event"]) if index >= 0 and index < rows.size() else -1


## 目前該顯示的一段：停留的優先，其次是選取的。
func shown() -> int:
	return hover if hover >= 0 else selected


# ── 繪製 ─────────────────────────────────────────────────────

## 畫整塊橫條。event_line 是選取那段起因事件的一句話（沒有為空），event_rect 回傳那句話可點的範圍。
func draw(item: CanvasItem, band: Rect2, unit: float, event_line: String, event_rect: Array[Rect2]) -> void:
	if rows.is_empty():
		return
	var body := Ink.font_size(Ink.SIZE_BODY, unit)
	var note := Ink.font_size(Ink.SIZE_NOTE, unit)
	var section := Ink.font_size(Ink.SIZE_SECTION, unit)
	Ink.text(item, band.position, headline(), section, Ink.INK)
	var hint_x := band.end.x - Ink.text_size(hint, note).x
	Ink.text(item, Vector2(hint_x, band.position.y + (section - note) * 0.5 + 1.0), hint, note, Ink.INK_LIGHT)
	# 紀元一段一段。
	var rects := segment_rects(band, unit)
	var shown_index := shown()
	for i in rects.size():
		var rect := rects[i]
		var fill := Ink.SILK if i % 2 == 0 else Ink.INK_LIGHT
		var inner := Rect2(rect.position, Vector2(maxf(rect.size.x - SEGMENT_GAP * unit, 1.0), rect.size.y))
		item.draw_rect(inner, Color(fill, 0.78 if i == shown_index else 0.5), true)
		if i == selected:
			item.draw_rect(inner, Ink.SEAL_RED, false, maxf(2.0 * unit, 2.0))
		elif i == hover:
			item.draw_rect(inner, Ink.GOLD, false, maxf(1.5 * unit, 1.5))
		var name_text := String(rows[i]["name"])
		var name_size := Ink.text_size(name_text, note)
		if name_size.x + 6.0 * unit <= inner.size.x:
			Ink.text(item, inner.position + (inner.size - name_size) * 0.5, name_text, note, Ink.PAPER_LIGHT)
	# 成住壞空的細帶：各段依年數與紀元橫條同一把尺。
	var strip := strip_rect(band, unit)
	var total := maxi(total_years(), 1)
	for span in phase_spans(total):
		var x0 := strip.position.x + strip.size.x * float(span["from"]) / total
		var x1 := strip.position.x + strip.size.x * float(span["to"]) / total
		var zone := Rect2(Vector2(x0, strip.position.y), Vector2(maxf(x1 - x0 - SEGMENT_GAP * unit, 1.0), strip.size.y))
		item.draw_rect(zone, Color(span["tint"], 0.42), true)
		var label := Locale.t(String(span["key"]))
		var label_size := Ink.text_size(label, note)
		if label_size.x + 6.0 * unit <= zone.size.x:
			Ink.text(item, zone.position + (zone.size - label_size) * 0.5, label, note, Ink.INK)
	# 現在：橫條右端，朱色三角指著最後一段，旁邊寫「現在」。
	var line_y := strip.end.y + GAP * 1.5 * unit
	var now_text := Locale.t("history_now")
	var now_size := Ink.text_size(now_text, note)
	var tip := Vector2(strip.end.x - 6.0 * unit, strip.end.y)
	item.draw_colored_polygon(PackedVector2Array([tip, tip + Vector2(-5.0, 7.0) * unit, tip + Vector2(5.0, 7.0) * unit]), Ink.SEAL_RED)
	Ink.text(item, Vector2(tip.x - now_size.x * 0.5 + 6.0 * unit - now_size.x * 0.5, line_y + 2.0 * unit), now_text, note, Ink.SEAL_RED)
	# 說明兩行：選取（或停留）那段紀元，與它起因的世界大事。
	var index := shown()
	# 右端留給「現在」，兩行說明都不侵入。
	var width := band.size.x - now_size.x - 14.0 * unit
	Ink.text(item, Vector2(band.position.x, line_y), _fit(era_text(index), body, width), body, Ink.INK)
	var second_y := line_y + LINE_HEIGHT * unit
	event_rect.clear()
	if event_line.is_empty():
		var none_text := Locale.t("history_pruned") if index > 0 else Locale.t("history_genesis")
		Ink.text(item, Vector2(band.position.x, second_y), _fit(none_text, note, width), note, Ink.INK_LIGHT)
		return
	var shown_line := _fit(event_line, body, width - Ink.text_size(Locale.t("history_open_card"), note).x - 16.0 * unit)
	Ink.text(item, Vector2(band.position.x, second_y), shown_line, body, Ink.CYAN)
	var line_w := Ink.text_size(shown_line, body).x
	var open_text := Locale.t("history_open_card")
	Ink.text(item, Vector2(band.position.x + line_w + 10.0 * unit, second_y + 2.0 * unit), open_text, note, Ink.INK_LIGHT)
	event_rect.append(Rect2(Vector2(band.position.x, second_y), Vector2(line_w + 10.0 * unit + Ink.text_size(open_text, note).x, LINE_HEIGHT * unit)))


## 把一句話截到放得進 room；截斷時結尾加「…」。
static func _fit(text: String, size: int, room: float) -> String:
	if Ink.text_size(text, size).x <= room:
		return text
	var count := text.length()
	while count > 1 and Ink.text_size(text.substr(0, count) + "…", size).x > room:
		count -= 1
	return text.substr(0, count) + "…"
