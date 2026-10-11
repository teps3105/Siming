class_name GuideSlip
extends Control
## GDD 5.7 引導簽：畫面下方中央的一張紙簽，一次一句，教現有的操作；玩家做到了就換下一句，全部做過即不再出現。
## 進度存在設定裡（GDD 9.1），可在設定頁重看。不擋操作，也不能點。

## 各步教的事，依序出現。
const STEPS: Array[StringName] = [&"pan", &"zoom", &"globe", &"time", &"mode", &"select"]
## 各步旁邊畫的實體鍵：動作的清單（依目前的輸入裝置取鍵）。
const STEP_ACTIONS := {
	&"pan": [InputActions.PAN_UP, InputActions.PAN_LEFT, InputActions.PAN_DOWN, InputActions.PAN_RIGHT],
	&"zoom": [InputActions.ZOOM_OUT, InputActions.ZOOM_IN],
	&"globe": [InputActions.VIEW_TOGGLE],
	&"time": [InputActions.PAUSE],
	&"mode": [InputActions.MODE_TIANXIA, InputActions.MODE_NEXT],
	&"select": [InputActions.LOCK_ON],
}
const HEIGHT := 36.0
const PAD := 14.0
const KEY_GAP := 4.0
## 平移、縮放要超過多少才算做到：地圖寬的比例、鏡頭高度的比例。
const PAN_FRAC := 0.02
const ZOOM_RATIO := 1.08

## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像與哪一句話）。
var gamepad := false:
	set(value):
		if gamepad != value:
			gamepad = value
			_resize()

var _camera: MapCamera
var _map_width := 1.0
var _unit := 1.0
var _step := 0
## 這一步開始時的鏡頭焦點與高度；還沒記下時焦點是 Vector2.INF。
var _from_focus := Vector2.INF
var _from_z := 0.0


## enabled 為假時不出現也不記進度（測試用）。
func setup(camera: MapCamera, map_width: float, unit: float, enabled := true) -> void:
	_camera = camera
	_map_width = map_width
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	_step = STEPS.size() if not enabled or GameSettings.flag("tutorial_done") else clampi(GameSettings.value("tutorial_step"), 0, STEPS.size())
	_from_focus = Vector2.INF
	_resize()


## 目前教到哪一步；全部做過時是空名稱。
func step() -> StringName:
	return STEPS[_step] if _step < STEPS.size() else &""


## 玩家做了某件事（切視角 globe、動了時間 time、換地圖模式 mode、選了州郡 select）。
func notify(what: StringName) -> void:
	if step() == what:
		_advance()


func _process(_delta: float) -> void:
	var current := step()
	visible = current != &"" and _camera != null and not _camera.is_flying() and _camera.input_enabled
	if not visible:
		return
	if _from_focus == Vector2.INF:
		_from_focus = _camera.focus
		_from_z = _camera.z
	elif current == &"pan" and _camera.focus.distance_to(_from_focus) > _map_width * PAN_FRAC:
		_advance()
	elif current == &"zoom" and maxf(_camera.z / _from_z, _from_z / _camera.z) > ZOOM_RATIO:
		_advance()


func _advance() -> void:
	_step += 1
	_from_focus = Vector2.INF
	GameSettings.set_value("tutorial_step", _step)
	if _step >= STEPS.size():
		GameSettings.set_value("tutorial_done", true)
	_resize()


func _keys() -> Array[String]:
	var out: Array[String] = []
	for action: StringName in STEP_ACTIONS.get(step(), []):
		var label := InputHints.label(action, gamepad)
		if not label.is_empty() and not out.has(label):
			out.append(label)
	return out


## 這一步的那句話：搖桿另有說法時用搖桿的。
func _sentence() -> String:
	var key := "guide_%s" % step()
	var pad_text := Locale.t(key + "_pad")
	return pad_text if gamepad and pad_text != key + "_pad" else Locale.t(key)


func _keys_width() -> float:
	var width := 0.0
	for label in _keys():
		width += Ink.key_glyph_width(label, _unit) + KEY_GAP * _unit
	return width + (6.0 * _unit if width > 0.0 else 0.0)


func _resize() -> void:
	if step() == &"":
		visible = false
		return
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	custom_minimum_size = Vector2(PAD * 2.0 * _unit + _keys_width() + Ink.text_size(_sentence(), body).x, HEIGHT * _unit)
	size = custom_minimum_size
	resized.emit()
	queue_redraw()


func _draw() -> void:
	if step() == &"":
		return
	Ink.paper_slip(self, Rect2(Vector2.ZERO, size))
	var x := PAD * _unit
	for label in _keys():
		x += Ink.key_glyph(self, Vector2(x, (size.y - Ink.KEY_HEIGHT * _unit) * 0.5), label, _unit, not gamepad) + KEY_GAP * _unit
	if x > PAD * _unit:
		x += 6.0 * _unit
	var body := Ink.font_size(Ink.SIZE_BODY, _unit)
	var sentence := _sentence()
	Ink.text(self, Vector2(x, (size.y - Ink.text_size(sentence, body).y) * 0.5), sentence, body)
