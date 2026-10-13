class_name TitleLogo
extends Control
## 標題畫面的標誌（GDD 9.1）：四個行書大字橫排在右上，右下一方朱印，底下一道金線；主選單接在金線下面。
## 大小跟著畫面高度走：以 1080 高為準，矮的畫面（Steam Deck）等比縮小。

const ART: Texture2D = preload("res://app/assets/title_logo.png")
## 以下都是 1080 高畫面上的版面單位。
const GLYPH := 135.0
const GLYPH_GAP := 10.0
const RIGHT := 96.0
const TOP := 64.0
const SEAL := 56.0
## 標誌圖在字帶四周多留的邊。
const BLEED := 40.0
const LINE_GAP := 22.0
const MENU_GAP := 26.0
const MENU_INSET := 18.0
## 版面縮小的下限，與選單放大的範圍。
const SCALE_MIN := 0.62
const MENU_ZOOM_MAX := 1.3

var _unit := 1.0
var _scale := 1.0
## 標誌左緣（畫面座標）：整組靠右，右邊留 RIGHT。
var _left := 0.0


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
	set_anchors_preset(Control.PRESET_FULL_RECT)


## 依畫面大小與版面倍率排版。
func arrange(bounds: Vector2, unit: float) -> void:
	_unit = unit
	_scale = clampf(bounds.y / unit / 1080.0, SCALE_MIN, 1.0)
	_left = bounds.x - _px(RIGHT + 4.0 * GLYPH + 3.0 * GLYPH_GAP + 12.0 + SEAL)
	queue_redraw()


## 選單該放大幾倍。
func menu_zoom() -> float:
	return clampf(_scale * MENU_ZOOM_MAX, 1.0, MENU_ZOOM_MAX)


## 選單左上角的位置（畫面座標）。
func menu_origin() -> Vector2:
	return Vector2(_left + _px(MENU_INSET), _line_y() + _px(MENU_GAP)).round()


## 四個字加朱印的範圍（畫面座標）。
func logo_rect() -> Rect2:
	return Rect2(_left, _px(TOP), _px(GLYPH * 4.0 + GLYPH_GAP * 3.0 + 12.0 + SEAL), _px(GLYPH))


func _px(value: float) -> float:
	return value * _scale * _unit


func _line_y() -> float:
	return _px(TOP + GLYPH + LINE_GAP)


func _draw() -> void:
	# 四個字與朱印畫在同一張圖上（tools/title/build_logo.py）；圖的四周比字帶多留 BLEED，給出格的筆畫。
	draw_texture_rect(ART, logo_rect().grow(_px(BLEED)), false)
	var line_y := _line_y()
	draw_line(Vector2(_left + _px(4.0), line_y), Vector2(logo_rect().end.x, line_y), Color(Ink.GOLD, 0.9), maxf(_px(2.0), 1.0))
