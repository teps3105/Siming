class_name NoticeSlip
extends Control
## GDD 2.8 大事告示：世界大事發生時，畫面上方中央出現一張紙簽寫那句話，停幾秒後收起。
## 一世裡有幾件就一件接一件；只顯示不收輸入。計時用畫面時間，暫停與快轉時停留一樣久。

const HOLD_SECONDS := 4.0
const FADE_SECONDS := 0.35
const HEIGHT := 36.0
const PAD := 18.0
## 排隊的告示最多留幾張，再多就丟掉最舊的。
const QUEUE_LIMIT := 3

## 紙簽上緣中點在畫面上的位置。
var top_center := Vector2.ZERO:
	set(value):
		top_center = value
		_place()

var _unit := 1.0
var _queue: Array[String] = []
var _text := ""
var _age := 0.0


func setup(unit: float) -> void:
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	visible = false


func post(line: String) -> void:
	_queue.append(line)
	while _queue.size() > QUEUE_LIMIT:
		_queue.pop_front()
	if _text.is_empty():
		_next()


func clear() -> void:
	_queue.clear()
	_text = ""
	visible = false


func showing() -> bool:
	return visible


func text() -> String:
	return _text


## 正在顯示的與還在排隊的告示（重建器物時帶到新的告示上）。
func pending() -> Array[String]:
	var out: Array[String] = []
	if not _text.is_empty():
		out.append(_text)
	out.append_array(_queue)
	return out


func _next() -> void:
	_text = "" if _queue.is_empty() else _queue.pop_front()
	_age = 0.0
	visible = not _text.is_empty()
	_place()
	queue_redraw()


func _place() -> void:
	var width := Ink.text_size(_text, Ink.font_size(Ink.SIZE_BODY, _unit)).x + PAD * 2.0 * _unit
	size = Vector2(width, HEIGHT * _unit)
	position = (top_center + Vector2(-width * 0.5, Ink.SLIP_BAND)).round()


func _process(delta: float) -> void:
	if _text.is_empty():
		return
	_age += delta
	var life := FADE_SECONDS * 2.0 + HOLD_SECONDS
	if _age >= life:
		_next()
		return
	modulate.a = clampf(minf(_age, life - _age) / FADE_SECONDS, 0.0, 1.0)


func _draw() -> void:
	if _text.is_empty():
		return
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	var rect := Rect2(Vector2.ZERO, size)
	Ink.paper_slip(self, rect)
	# 左右各一道朱線，像貼出來的榜文。
	for x: float in [6.0 * _unit, size.x - 6.0 * _unit]:
		draw_line(Vector2(x, 6.0 * _unit), Vector2(x, size.y - 6.0 * _unit), Ink.SEAL_RED, 1.5 * _unit, true)
	var text_size := Ink.text_size(_text, body)
	Ink.text(self, ((size - text_size) * 0.5).round(), _text, body)
