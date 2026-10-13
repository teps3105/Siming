class_name DirectorCard
extends Control
## GDD 5.13 字卡：導演鏡頭到位時出在畫面下方中央的一張紙簽，州郡名加一句事件（傳聞的句子），
## 人名、州名、宗門名可點。長句依帳冊文字的規則折成兩行。離開時淡出。

signal person_requested(person: int)
signal province_requested(province: int)
signal sect_requested(sect: int)
## 玩家點了字卡（不論點到名字與否）。
signal acted

const PAD_X := 18.0
const PAD_Y := 9.0
const FADE_SECONDS := 0.35
const MAX_LINES := 2
## 字卡最寬佔畫面寬度的比例。
const MAX_WIDTH_FRAC := 0.6
const MARGIN := 12.0

var _unit := 1.0
var _event: Dictionary = {}
var _para: Dictionary = {}
var _shown := false
var _hover := ""
var _pressed := ""
var _bounds := Vector2(1920.0, 1080.0)
var _bottom := 1000.0
var _right_limit := 1900.0
var _avoid: Array[Rect2] = []
var _dirty := true


func setup(unit: float) -> void:
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_PASS
	modulate.a = 0.0
	visible = false


## 目前顯示的事件；沒有為空字典。
func event() -> Dictionary:
	return _event if _shown else {}


## 字卡上的句子（全部行接起來）。
func text() -> String:
	return "".join(_para.get("lines", []))


func showing() -> bool:
	return _shown


func show_event(event: Dictionary) -> void:
	_event = event
	_shown = not event.is_empty()
	_hover = ""
	_pressed = ""
	_dirty = true
	_rebuild()
	if _shown:
		visible = true


func hide_card() -> void:
	_shown = false


## 擺放：畫面大小、字卡下緣、右側不能越過的位置，以及要避開的矩形（時間盤、傳聞簽、模式列等）。
func layout(bounds: Vector2, bottom: float, right_limit: float, avoid: Array[Rect2]) -> void:
	if bounds == _bounds and bottom == _bottom and right_limit == _right_limit and avoid == _avoid and not _dirty:
		return
	_dirty = false
	_bounds = bounds
	_bottom = bottom
	_right_limit = right_limit
	_avoid = avoid
	_rebuild()


func _rebuild() -> void:
	if _event.is_empty():
		return
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	var band_height := LedgerText.line_height(body) * MAX_LINES + PAD_Y * 2.0 * _unit
	var left := MARGIN
	var right := _right_limit
	var mid := _bounds.x * 0.5
	for rect in _avoid:
		if rect.end.y < _bottom - band_height - 4.0 or rect.position.y > _bottom + 4.0:
			continue
		if rect.get_center().x < mid:
			left = maxf(left, rect.end.x + MARGIN)
		else:
			right = minf(right, rect.position.x - MARGIN)
	var width := minf(_bounds.x * MAX_WIDTH_FRAC, right - left) - (PAD_X * 2.0 + Ink.SLIP_BAND * 2.0) * _unit
	var links := EventCard.links_of(_event)
	var prefix := String(_event.get("p", ""))
	prefix = prefix + "　" if not prefix.is_empty() else ""
	var info := EventText.describe(_event)
	var best: Dictionary = {}
	for variant in info["variants"]:
		best = LedgerText.paragraph(prefix + String(variant[0]) + String(variant[1]), links, width, body)
		if (best["lines"] as Array).size() <= MAX_LINES:
			break
	_para = best
	var inner := Vector2(0.0, 0.0)
	for line: String in _para["lines"]:
		inner.x = maxf(inner.x, Ink.text_size(line, body).x)
	inner.y = float(_para["height"])
	size = (inner + Vector2(PAD_X, PAD_Y) * 2.0 * _unit).round()
	var x := clampf(mid - size.x * 0.5, left, maxf(right - size.x, left))
	position = Vector2(x, _bottom - size.y).round()
	queue_redraw()


func _origin() -> Vector2:
	return Vector2(PAD_X, PAD_Y) * _unit


func _process(delta: float) -> void:
	var want := 1.0 if _shown else 0.0
	modulate.a = move_toward(modulate.a, want, delta / FADE_SECONDS)
	if not _shown and modulate.a <= 0.0 and visible:
		visible = false
		_event = {}
		_para = {}


func _has_point(point: Vector2) -> bool:
	return _shown and Rect2(Vector2.ZERO, size).grow(Ink.SLIP_BAND).has_point(point)


func _gui_input(event: InputEvent) -> void:
	if not _shown or _para.is_empty():
		return
	if event is InputEventMouseMotion:
		var now := LedgerText.target_at(_origin(), _para, event.position)
		if now != _hover:
			_hover = now
			queue_redraw()
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		if event.pressed:
			_pressed = LedgerText.target_at(_origin(), _para, event.position)
		else:
			var hit := LedgerText.target_at(_origin(), _para, event.position)
			if not hit.is_empty() and hit == _pressed:
				activate(hit)
			else:
				acted.emit()
			_pressed = ""
		accept_event()
		queue_redraw()


## 開啟一個名字連結：target 形如 person:12、province:3、sect:5。
func activate(target: String) -> void:
	var parts := target.split(":")
	if parts.size() != 2:
		return
	acted.emit()
	match parts[0]:
		"person":
			person_requested.emit(int(parts[1]))
		"province":
			province_requested.emit(int(parts[1]))
		"sect":
			sect_requested.emit(int(parts[1]))


## 字卡上某個目標的螢幕位置（測試用）；沒有為 (-1, -1)。
func target_center(target: String) -> Vector2:
	for hit: Dictionary in _para.get("hits", []):
		if hit["target"] == target:
			return position + _origin() + (hit["rect"] as Rect2).get_center()
	return Vector2(-1.0, -1.0)


func targets() -> Array[String]:
	var out: Array[String] = []
	for hit: Dictionary in _para.get("hits", []):
		out.append(hit["target"])
	return out


func _draw() -> void:
	if _para.is_empty():
		return
	Ink.paper_slip(self, Rect2(Vector2.ZERO, size))
	LedgerText.draw(self, _origin(), _para, Ink.INK, _hover, _pressed)
