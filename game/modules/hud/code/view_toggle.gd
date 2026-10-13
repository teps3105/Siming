class_name ViewToggle
extends Control
## GDD 5.2 視角鈕：星球視角與平面地圖互換。鈕上畫的是按下去會換到的視角，旁邊是快捷鍵的圖像。

signal toggled

const HEIGHT := 30.0
const PAD := 8.0
const ICON_RADIUS := 8.0

## 目前是否為星球視角。
var globe := false:
	set(value):
		globe = value
		queue_redraw()
## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像）。
var gamepad := false:
	set(value):
		gamepad = value
		queue_redraw()
var _unit := 1.0
var _hover := false
var _pressed := false


func setup(unit: float) -> void:
	_unit = unit
	custom_minimum_size = Vector2(_width(), HEIGHT * unit)
	size = custom_minimum_size
	mouse_filter = Control.MOUSE_FILTER_STOP


## 游標停在這裡時是否可以按（GDD 5.9 游標圖示）。
func is_hot(_point: Vector2) -> bool:
	return true


func _label() -> String:
	return Locale.t("view_plane" if globe else "view_globe")


func _key() -> String:
	return InputHints.label(InputActions.VIEW_TOGGLE, gamepad)


## 寬度按兩種字樣、兩種裝置的鍵圖像中最寬者預留，切換時不變寬。
func _width() -> float:
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var text := maxf(Ink.text_size(Locale.t("view_globe"), note).x, Ink.text_size(Locale.t("view_plane"), note).x)
	var key := maxf(
		Ink.key_glyph_width(InputHints.label(InputActions.VIEW_TOGGLE, false), _unit),
		Ink.key_glyph_width(InputHints.label(InputActions.VIEW_TOGGLE, true), _unit)
	)
	return (PAD + ICON_RADIUS * 2.0 + 6.0 + PAD + PAD) * _unit + text + key


func _draw() -> void:
	var rect := Rect2(Vector2.ZERO, size)
	var ink := Ink.plate(self, rect, false, true, _hover, _pressed)
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var x := PAD * _unit
	InkIcons.draw(self, &"plane" if globe else &"globe", Vector2(x + ICON_RADIUS * _unit, size.y * 0.5), ICON_RADIUS * _unit, ink)
	x += (ICON_RADIUS * 2.0 + 6.0) * _unit
	var label := _label()
	Ink.text(self, Vector2(x, (size.y - Ink.text_size(label, note).y) * 0.5), label, note, ink)
	var key := _key()
	var key_x := size.x - PAD * _unit - Ink.key_glyph_width(key, _unit)
	Ink.key_glyph(self, Vector2(key_x, (size.y - Ink.KEY_HEIGHT * _unit) * 0.5), key, _unit)


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and not _hover:
		_hover = true
		queue_redraw()
	elif event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
		accept_event()
		_pressed = event.pressed
		queue_redraw()
		if event.pressed:
			toggled.emit()


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and (_hover or _pressed):
		_hover = false
		_pressed = false
		queue_redraw()
