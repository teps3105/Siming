class_name GameCursor
extends Control
## GDD 5.9 遊戲內游標：系統游標隱藏，滑鼠與搖桿共用這一個游標，畫在所有器物之上。
## 圖示依情形更換：一般（墨色箭頭）、可按（朱色箭頭加環）、拖動地圖（四向箭頭）、旋轉鏡頭（弧形雙箭頭）。
## 熱點在箭頭尖端；拖動與旋轉的圖示以熱點為中心。

## 箭頭的輪廓（熱點在原點，單位為像素 × 版面倍率）。
const ARROW: Array[Vector2] = [
	Vector2(0.0, 0.0), Vector2(0.0, 17.0), Vector2(4.4, 13.2), Vector2(7.6, 20.0),
	Vector2(10.4, 18.7), Vector2(7.3, 12.2), Vector2(13.0, 12.0),
]
const OUTLINE := 2.5

var _camera: MapCamera
var _unit := 1.0
## 滑鼠在視窗內（搖桿模式不看這個）。
var _inside := true


func setup(camera: MapCamera, unit: float) -> void:
	_camera = camera
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)


func _enter_tree() -> void:
	Input.mouse_mode = Input.MOUSE_MODE_HIDDEN


func _exit_tree() -> void:
	Input.mouse_mode = Input.MOUSE_MODE_VISIBLE


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_MOUSE_ENTER:
		_inside = true
	elif what == NOTIFICATION_WM_MOUSE_EXIT:
		_inside = false


func _process(_delta: float) -> void:
	queue_redraw()


## 目前該畫哪一種圖示。
func kind() -> StringName:
	if _camera == null:
		return &"arrow"
	var from_camera := _camera.pointer_kind()
	if from_camera != &"arrow":
		return from_camera
	var hovered := get_viewport().gui_get_hovered_control()
	if hovered != null and hovered.has_method(&"is_hot"):
		var local := hovered.get_global_transform().affine_inverse() * _camera.pointer_position()
		if hovered.call(&"is_hot", local):
			return &"point"
	return &"arrow"


func _draw() -> void:
	if _camera == null or not (_inside or _camera.use_crosshair):
		return
	var at := _camera.pointer_position()
	match kind():
		&"drag":
			_draw_drag(at)
		&"rotate":
			_draw_rotate(at)
		&"point":
			draw_arc(at, 7.0 * _unit, 0.0, TAU, 32, Color(Ink.PAPER_LIGHT, 0.9), 3.5, true)
			draw_arc(at, 7.0 * _unit, 0.0, TAU, 32, Ink.SEAL_RED, 1.5, true)
			_draw_arrow(at, Ink.SEAL_RED)
		_:
			_draw_arrow(at, Ink.INK)


func _draw_arrow(at: Vector2, color: Color) -> void:
	var points := PackedVector2Array()
	for p in ARROW:
		points.append(at + p * _unit)
	var closed := points.duplicate()
	closed.append(points[0])
	draw_polyline(closed, Ink.PAPER_LIGHT, OUTLINE * 2.0, true)
	draw_colored_polygon(points, color)


## 四向箭頭：中間一點，四個方向各一個箭頭。
func _draw_drag(at: Vector2) -> void:
	for pass_index in 2:
		var color := Ink.PAPER_LIGHT if pass_index == 0 else Ink.INK
		var width := (1.8 + (OUTLINE if pass_index == 0 else 0.0)) * _unit
		for dir: Vector2 in [Vector2.UP, Vector2.DOWN, Vector2.LEFT, Vector2.RIGHT]:
			var tip := at + dir * 11.0 * _unit
			var side := dir.orthogonal() * 3.5 * _unit
			draw_line(at + dir * 3.5 * _unit, tip, color, width, true)
			draw_polyline(PackedVector2Array([tip - dir * 4.0 * _unit + side, tip, tip - dir * 4.0 * _unit - side]), color, width, true)
		draw_circle(at, (1.8 + (1.2 if pass_index == 0 else 0.0)) * _unit, color)


## 弧形雙箭頭：上方一道弧，兩端各一個箭頭。
func _draw_rotate(at: Vector2) -> void:
	var radius := 10.0 * _unit
	var from := deg_to_rad(200.0)
	var to := deg_to_rad(340.0)
	for pass_index in 2:
		var color := Ink.PAPER_LIGHT if pass_index == 0 else Ink.SEAL_RED
		var width := (1.8 + (OUTLINE if pass_index == 0 else 0.0)) * _unit
		draw_arc(at, radius, from, to, 24, color, width, true)
		for end: Array in [[from, -1.0], [to, 1.0]]:
			var out := Vector2.from_angle(end[0])
			var tip := at + out * radius
			var along := out.orthogonal() * (end[1] as float)
			draw_polyline(
				PackedVector2Array([tip - along * 4.5 * _unit + out * 3.0 * _unit, tip, tip - along * 4.5 * _unit - out * 3.0 * _unit]),
				color, width, true
			)
		draw_circle(at, (1.8 + (1.2 if pass_index == 0 else 0.0)) * _unit, color)
