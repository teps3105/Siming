class_name LedgerList
extends Control
## GDD 5.8 命簿裡可捲動的清單：一塊會剪裁的區域，內容由呼叫端以 painter 畫在「內容座標」上，
## 以 finder 回答某一點是哪個可點的目標。滑鼠滾輪捲動；游標（搖桿的右搖桿）貼近上下緣自動捲動（GDD 5.9）。
## 點到目標時發出 activated；沒有目標的點擊也由本控制項吃掉，不穿到地圖上。

signal activated(target: String)

## 游標離上下緣多近開始自動捲動，與每秒捲動的距離（皆乘版面倍率）。
const EDGE_ZONE := 28.0
const EDGE_SPEED := 260.0
const WHEEL_STEP := 44.0

## painter.call(list)：以內容座標作畫；list.hover、list.pressed 是目前停留與按住的目標。
var painter: Callable
## finder.call(point) -> String：內容座標上 point 的目標，沒有為空字串。
var finder: Callable
## 游標貼近上下緣自動捲動（搖桿沒有滾輪，只在搖桿模式開）。
var edge_scroll := false
var hover := ""
var pressed := ""
var scroll := 0.0:
	set(value):
		scroll = clampf(value, 0.0, max_scroll())
		queue_redraw()
var content_height := 0.0:
	set(value):
		content_height = value
		scroll = scroll
		queue_redraw()

var _unit := 1.0
var _pointer := Vector2(-1.0, -1.0)
var _inside := false


func setup(unit: float) -> void:
	_unit = unit
	clip_contents = true
	mouse_filter = Control.MOUSE_FILTER_STOP


func max_scroll() -> float:
	return maxf(content_height - size.y, 0.0)


func scroll_to_end() -> void:
	scroll = max_scroll()


func _draw() -> void:
	if not painter.is_valid():
		return
	draw_set_transform(Vector2(0.0, -scroll))
	painter.call(self)
	draw_set_transform(Vector2.ZERO)
	# 可捲動時，右緣一道細細的墨條標出目前的位置。
	if max_scroll() > 0.0:
		var ratio := size.y / content_height
		var bar_height := maxf(size.y * ratio, 18.0 * _unit)
		var top := (size.y - bar_height) * (scroll / max_scroll())
		draw_rect(Rect2(Vector2(size.x - 3.0 * _unit, top), Vector2(2.0 * _unit, bar_height)), Color(Ink.INK_LIGHT, 0.55), true)


func _process(delta: float) -> void:
	if not edge_scroll or not _inside or max_scroll() <= 0.0 or _pointer.y < 0.0 or _pointer.y > size.y:
		return
	var step := 0.0
	if _pointer.y < EDGE_ZONE * _unit:
		step = -EDGE_SPEED
	elif _pointer.y > size.y - EDGE_ZONE * _unit:
		step = EDGE_SPEED
	if step != 0.0:
		scroll += step * _unit * delta
		_set_hover(_target(_pointer))


func _target(point: Vector2) -> String:
	if not finder.is_valid():
		return ""
	return finder.call(point + Vector2(0.0, scroll))


func _set_hover(target: String) -> void:
	if target != hover:
		hover = target
		queue_redraw()


## 游標停在這裡時是否可以按（GDD 5.9 游標圖示）。
func is_hot(point: Vector2) -> bool:
	return not _target(point).is_empty()


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT:
		_inside = false
		_set_hover("")
		if not pressed.is_empty():
			pressed = ""
			queue_redraw()


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_inside = true
		_pointer = (event as InputEventMouseMotion).position
		_set_hover(_target(_pointer))
	elif event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		if button.button_index == MOUSE_BUTTON_LEFT:
			if button.pressed:
				pressed = _target(button.position)
				queue_redraw()
				if not pressed.is_empty():
					activated.emit(pressed)
			elif not pressed.is_empty():
				pressed = ""
				queue_redraw()
			accept_event()
		elif button.pressed and button.button_index == MOUSE_BUTTON_WHEEL_UP:
			scroll -= WHEEL_STEP * _unit
			_set_hover(_target(button.position))
			accept_event()
		elif button.pressed and button.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			scroll += WHEEL_STEP * _unit
			_set_hover(_target(button.position))
			accept_event()
