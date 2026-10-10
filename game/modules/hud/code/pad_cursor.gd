class_name PadCursor
extends Control
## GDD 5.9 搖桿游標：朱色小環，畫在所有器物之上；只在搖桿模式顯示。

var _camera: MapCamera
var _unit := 1.0


func setup(camera: MapCamera, unit: float) -> void:
	_camera = camera
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)


func _process(_delta: float) -> void:
	queue_redraw()


func _draw() -> void:
	if _camera == null or not _camera.use_crosshair:
		return
	var center := _camera.cursor_position()
	var radius := 9.0 * _unit
	draw_arc(center, radius, 0.0, TAU, 32, Color(Ink.PAPER_LIGHT, 0.9), 3.5, true)
	draw_arc(center, radius, 0.0, TAU, 32, Ink.SEAL_RED, 1.5, true)
	draw_circle(center, 1.5 * _unit, Ink.SEAL_RED)
