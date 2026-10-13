class_name LedgerCard
extends Control
## GDD 5.8 命簿：所有卡片共用的容器——上下兩根軸頭、綾邊裱著的紙面、左側直書題簽、右上的關閉鈕、展卷與收卷。
## 子類別（州郡卡、人物卡）提供題簽字、內容高度、紙面上的內容與可點的目標；同時只開一張，由 Hud 管理。

signal closed

const WIDTH := 380.0
## 軸頭的高度。
const ROD := 12.0
const UNROLL_SECONDS := 0.25
const ROLL_UP_SECONDS := 0.18
## 左側直書題簽的寬度。
const STRIP := 40.0
const CLOSE_RADIUS := 13.0
const ACTION_RADIUS := 17.0

## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像）。
var gamepad := false:
	set(value):
		gamepad = value
		queue_redraw()
## 卡片（含軸頭）最高能佔多少畫面高度；清單類的內容超過時改為可捲動。
var max_height := 100000.0:
	set(value):
		if value != max_height:
			max_height = value
			_on_max_height_changed()

var _world: SimWorld
var _unit := 1.0
var _width := WIDTH
var _open := 0.0
var _target_open := 0.0
var _active := false
var _hover := ""
## 正按住的鈕（按下的回饋）。
var _pressed := ""


## width 為卡片寬度（像素）；不指定時為 380 乘版面倍率。
func setup(world: SimWorld, unit: float, width := -1.0) -> void:
	_world = world
	_unit = unit
	_width = width if width > 0.0 else WIDTH * unit
	custom_minimum_size = Vector2(_width, ROD * 2.0 * unit)
	size = custom_minimum_size
	mouse_filter = Control.MOUSE_FILTER_STOP
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	visible = false


func is_open() -> bool:
	return _active


## 收卷並通知關閉。
func close() -> void:
	if not _active:
		return
	_active = false
	_target_open = 0.0
	_forget()
	closed.emit()


## 讓位給另一張卡：立刻收起，不發出 closed（地圖上的選取維持）。
func suspend() -> void:
	_active = false
	_open = 0.0
	_target_open = 0.0
	_forget()
	visible = false
	queue_redraw()


## 展卷。
func _begin_open() -> void:
	_active = true
	visible = true
	_target_open = 1.0


func _process(delta: float) -> void:
	if _open == _target_open:
		return
	_open = move_toward(_open, _target_open, delta / (UNROLL_SECONDS if _target_open > _open else ROLL_UP_SECONDS))
	if _open <= 0.0:
		visible = false
	queue_redraw()


## 子類別：卡高上限改變了。
func _on_max_height_changed() -> void:
	pass


## 子類別：關閉時清掉自己的目標（id）；內容字典留著給收卷動畫畫完。
func _forget() -> void:
	pass


## 子類別：有東西可畫。
func _has_content() -> bool:
	return false


## 子類別：直書題簽的字。
func _title() -> String:
	return ""


## 子類別：兩根軸頭之間的高度（含綾邊）。
func _content_height() -> float:
	return 0.0


## 子類別：紙面上的內容。paper 是紙面範圍。
func _draw_body(_paper: Rect2) -> void:
	pass


## 子類別：點到的目標（"close" 由本類別處理）。
func _target_at(_pos: Vector2) -> String:
	return ""


## 子類別：按下目標時的動作。
func _activate(_target: String) -> void:
	pass


## 子類別：卡片完全展開（或收起）時，放置子控制項。
func _arrange(_unrolled: bool) -> void:
	pass


## 內容左緣到右緣的寬度（扣掉題簽與邊距）。
func _inner_width() -> float:
	var inset := Ink.CARD_BAND + 4.0 * _unit
	return size.x - inset * 2.0 - (STRIP + 14.0 + 16.0) * _unit


func _paper_rect(height: float) -> Rect2:
	var inset := Ink.CARD_BAND + 4.0 * _unit
	return Rect2(Vector2(inset, ROD * _unit + Ink.CARD_BAND), Vector2(size.x - inset * 2.0, height - Ink.CARD_BAND * 2.0))


func _close_key() -> String:
	return InputHints.label(InputActions.CANCEL, gamepad)


## 直書題簽：紙面左側的一條，名字由上往下寫（GDD 5.8）。
func _title_strip_rect() -> Rect2:
	var paper := _paper_rect(_content_height())
	return Rect2(paper.position, Vector2(STRIP * _unit, paper.size.y))


## 關閉鈕的外接方框。
func _close_rect() -> Rect2:
	var paper := _paper_rect(_content_height())
	var diameter := CLOSE_RADIUS * 2.0 * _unit
	return Rect2(Vector2(paper.end.x - 12.0 * _unit - diameter, paper.position.y + 12.0 * _unit), Vector2(diameter, diameter))


## 關閉鈕左緣再往左，文字與鍵圖像共佔的寬度（標題列要避開）。
func _close_cluster_width(show_key := true) -> float:
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var width := Ink.text_size(Locale.t("card_close"), note).x + 6.0 * _unit
	if show_key:
		width += 6.0 * _unit + Ink.key_glyph_width(_close_key(), _unit)
	return width


## 頁尾動作鈕的外接方框：第 index 顆（從左數），各顆之間的 offset 由子類別算好後傳入。
func _action_rect(left: float) -> Rect2:
	var paper := _paper_rect(_content_height())
	var diameter := ACTION_RADIUS * 2.0 * _unit
	return Rect2(Vector2(paper.position.x + left, paper.end.y - 14.0 * _unit - diameter), Vector2(diameter, diameter))


## 一顆頁尾動作印：圓鈕、右邊的動作名與實體鍵圖像；回傳整顆佔的右緣 x。
func _draw_action(rect: Rect2, icon: StringName, label: String, key: String, id: String) -> float:
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	Ink.round_button(self, rect.get_center(), rect.size.x * 0.5, icon, false, _hover == id, _pressed == id)
	var x := rect.end.x + 8.0 * _unit
	Ink.text(self, Vector2(x, rect.position.y + (rect.size.y - Ink.text_size(label, body).y) * 0.5), label, body)
	x += Ink.text_size(label, body).x + 8.0 * _unit
	if not key.is_empty():
		x += Ink.key_glyph(self, Vector2(x, rect.position.y + (rect.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), key, _unit)
	return x


func _draw() -> void:
	if not _has_content():
		return
	var eased := smoothstep(0.0, 1.0, _open)
	var full := _content_height()
	var body_height := full * eased
	var rod := ROD * _unit
	# 紙面由上軸頭往下拉開；下軸頭跟著紙的下緣走。
	if body_height > Ink.CARD_BAND * 2.0 + 4.0:
		Ink.paper_slip(self, _paper_rect(body_height), Ink.CARD_BAND)
	_draw_rod(0.0)
	_draw_rod(rod + body_height)
	var unrolled := eased >= 0.98
	_arrange(unrolled and _active)
	if not unrolled:
		return
	var paper := _paper_rect(full)
	var note := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var title := Ink.font_size(Ink.SIZE_TITLE, _unit)
	# 直書題簽：紙面左側較深的一條，名字由上往下寫。
	var strip := _title_strip_rect()
	draw_rect(strip, Color(Ink.PAPER_DARK, 0.55), true)
	draw_line(Vector2(strip.end.x, strip.position.y), strip.end, Color(Ink.INK_LIGHT, 0.6), 1.0)
	Ink.vertical_text(self, Vector2(strip.get_center().x, strip.position.y + 14.0 * _unit), _title(), title)
	_draw_body(paper)
	var close := _close_rect()
	Ink.round_button(self, close.get_center(), close.size.x * 0.5, &"close", true, _hover == "close", _pressed == "close")
	# 關閉鈕左邊：文字與實體鍵圖像（有「返回」可用時，Esc／B 歸返回，關閉鈕不掛鍵）。
	var close_label := Locale.t("card_close")
	var close_x := close.position.x - 6.0 * _unit - Ink.text_size(close_label, note).x
	Ink.text(self, Vector2(close_x, close.position.y + (close.size.y - Ink.text_size(close_label, note).y) * 0.5), close_label, note, Ink.INK_LIGHT)
	var close_key := _close_key() if _close_has_key() else ""
	if not close_key.is_empty():
		Ink.key_glyph(self, Vector2(close_x - 6.0 * _unit - Ink.key_glyph_width(close_key, _unit), close.position.y + (close.size.y - Ink.KEY_HEIGHT * _unit) * 0.5), close_key, _unit)


## 子類別：關閉鈕是否掛關閉鍵的圖像。
func _close_has_key() -> bool:
	return true


## 軸頭：墨色圓柱，兩端略粗。
func _draw_rod(y: float) -> void:
	var rod := ROD * _unit
	var cap := 10.0 * _unit
	draw_rect(Rect2(Vector2(cap * 0.5, y + rod * 0.2), Vector2(size.x - cap, rod * 0.6)), Ink.INK, true)
	draw_rect(Rect2(Vector2(cap * 0.5, y + rod * 0.3), Vector2(size.x - cap, 1.0)), Color(Ink.INK_LIGHT, 0.9), true)
	for cap_x: float in [0.0, size.x - cap]:
		draw_rect(Rect2(Vector2(cap_x, y), Vector2(cap, rod)), Ink.INK, true)
		draw_rect(Rect2(Vector2(cap_x, y + rod * 0.5 - 1.0), Vector2(cap, 2.0)), Color(Ink.GOLD, 0.8), true)


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var hover: String = _target_at((event as InputEventMouseMotion).position)
		if hover != _hover:
			_hover = hover
			queue_redraw()
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		_pressed = _target_at((event as InputEventMouseButton).position)
		queue_redraw()
		if _pressed == "close":
			close()
		elif not _pressed.is_empty():
			_activate(_pressed)
		accept_event()
	elif event is InputEventMouseButton and not event.pressed and not _pressed.is_empty():
		_pressed = ""
		queue_redraw()


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and not _hover.is_empty():
		_hover = ""
		queue_redraw()


## 游標停在這裡時是否可以按（GDD 5.9 游標圖示）。
func is_hot(point: Vector2) -> bool:
	return not _target_at(point).is_empty()


## 圓鈕上的命中：點在圓裡。
func _in_round(rect: Rect2, pos: Vector2) -> bool:
	return pos.distance_to(rect.get_center()) <= rect.size.x * 0.5
