class_name SwordFlyer
extends Control
## 標題畫面的御劍修士（GDD 9.1）：一個墨色剪影踩著飛劍，從畫面左邊的天上飛向中間上方的遠處，衣袖、髮帶與腰帶隨風擺動，
## 劍後拖一道淡淡的劍光；飛遠了變小、沒入雲裡，隔一會兒再來一趟，每一趟是另一名修士（CultivatorFigure）。

## 飛一趟的秒數與兩趟之間的間隔。
const FLIGHT_SECONDS := 34.0
const REST_SECONDS := 7.0
## 起點與終點（畫面的比例）、近處與遠處的身高（1080 高畫面上的像素）。
const FROM := Vector2(-0.05, 0.22)
const TO := Vector2(0.50, 0.06)
const NEAR_HEIGHT := 64.0
const FAR_HEIGHT := 22.0
## 劍光的長度（身高的倍數）與段數。
const TRAIL_LENGTH := 5.0
const TRAIL_STEPS := 14

## 這一趟飛到哪裡（0 起點 .. 1 終點；超過 1 是休息）。測試與截圖可以直接指定。
var progress := 0.3
## 這一趟是哪一名修士。
var figure_seed := 5:
	set(value):
		figure_seed = value
		_look = CultivatorFigure.traits(value)

var _look := CultivatorFigure.traits(5)
var _time := 0.0


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)


func _process(delta: float) -> void:
	_time += delta
	progress += delta / FLIGHT_SECONDS
	if progress > 1.0 + REST_SECONDS / FLIGHT_SECONDS:
		progress = 0.0
		figure_seed = randi()
	queue_redraw()


## 剪影此刻在畫面上的位置（腳下、劍身中段）；休息時在畫面外。
func figure_position() -> Vector2:
	return _point(progress)


## 飛行路線：略帶弧度，途中上下浮動。
func _point(t: float) -> Vector2:
	var bounds := get_viewport_rect().size
	var along := FROM.lerp(TO, t) * bounds
	var arc := sin(t * PI) * bounds.y * 0.04
	var bob := sin(_time * 0.9) * bounds.y * 0.006
	return along + Vector2(0.0, arc + bob)


func _draw() -> void:
	if progress >= 1.0:
		return
	var t := progress
	var height := lerpf(NEAR_HEIGHT, FAR_HEIGHT, smoothstep(0.0, 1.0, t)) * get_viewport_rect().size.y / 1080.0
	# 起點淡入、沒入遠處時淡出。
	var alpha := smoothstep(0.0, 0.06, t) * (1.0 - smoothstep(0.78, 1.0, t))
	var at := _point(t)
	var heading := (_point(t + 0.01) - at).normalized()
	_draw_trail(at, heading, height, alpha)
	# 人物朝右（剪影本身朝左，左右翻過來）；劍身順著飛行方向微微上揚。
	var tilt := clampf(heading.angle_to(Vector2.RIGHT) * -0.6, -0.5, 0.5)
	draw_set_transform(at, tilt, Vector2(-height, height))
	_draw_figure(Color(Ink.INK, 0.88 * alpha))
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)


## 劍光：劍尾往後一道越來越細、越來越淡的光。
func _draw_trail(at: Vector2, heading: Vector2, height: float, alpha: float) -> void:
	var side := heading.orthogonal()
	var previous := at - heading * height * 0.55
	for i in TRAIL_STEPS:
		var f := float(i + 1) / TRAIL_STEPS
		var wave := sin(_time * 2.2 - f * 5.0) * height * 0.05 * f
		var next := at - heading * height * (0.55 + TRAIL_LENGTH * f) + side * wave
		var fade := (1.0 - f) * (1.0 - f)
		draw_line(previous, next, Color(Ink.PAPER_LIGHT, 0.75 * fade * alpha), maxf(height * 0.10 * (1.0 - f * 0.7), 1.0), true)
		draw_line(previous, next, Color(Ink.CYAN, 0.30 * fade * alpha), maxf(height * 0.035 * (1.0 - f * 0.7), 1.0), true)
		previous = next


## 身高為 1 的剪影：腳下一柄飛劍（劍尖朝前），上面站著修士。
func _draw_figure(ink: Color) -> void:
	draw_colored_polygon(PackedVector2Array([
		Vector2(-0.78, 0.035), Vector2(-0.58, 0.005), Vector2(0.44, 0.012), Vector2(0.44, 0.062), Vector2(-0.58, 0.066),
	]), ink)
	draw_colored_polygon(PackedVector2Array([
		Vector2(0.44, -0.012), Vector2(0.48, -0.012), Vector2(0.48, 0.086), Vector2(0.44, 0.086),
	]), ink)
	draw_colored_polygon(PackedVector2Array([
		Vector2(0.48, 0.018), Vector2(0.64, 0.024), Vector2(0.64, 0.054), Vector2(0.48, 0.058),
	]), ink)
	CultivatorFigure.paint(self, _look, CultivatorFigure.Pose.RIDE, _time, ink, 0.03, ink.a)
