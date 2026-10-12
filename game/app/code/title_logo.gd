class_name TitleLogo
extends Control
## 標題畫面的標誌（GDD 9.1）：四個大字橫排在右上，右下一方朱印，底下一道金線；主選單接在金線下面。
## 大小跟著畫面高度走：以 1080 高為準，矮的畫面（Steam Deck）等比縮小。

const FONT: FontFile = preload("res://modules/ui_kit/assets/fonts/genryu_logo_subset.otf")
## 以下都是 1080 高畫面上的版面單位。
const GLYPH := 135.0
const GLYPH_GAP := 10.0
const RIGHT := 96.0
const TOP := 64.0
const SEAL := 56.0
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
	var title := Locale.t("game_title")
	var size_px := int(round(_px(GLYPH)))
	var top := _px(TOP)
	for i in title.length():
		var x := _left + _px(i * (GLYPH + GLYPH_GAP))
		var baseline := Vector2(x, top + FONT.get_ascent(size_px) - (FONT.get_height(size_px) - size_px) * 0.5)
		draw_string_outline(FONT, baseline, title[i], HORIZONTAL_ALIGNMENT_LEFT, -1.0, size_px, int(round(_px(6.0))), Color(Ink.PAPER_LIGHT, 0.55))
		draw_string(FONT, baseline, title[i], HORIZONTAL_ALIGNMENT_LEFT, -1.0, size_px, Ink.INK)
	# 朱印：標題的頭尾兩個字，直排。
	var seal := Rect2(_left + _px(4.0 * GLYPH + 3.0 * GLYPH_GAP + 12.0), top + _px(GLYPH - SEAL * 0.9), _px(SEAL), _px(SEAL))
	draw_rect(seal, Ink.SEAL_RED, true)
	draw_rect(seal.grow(-_px(3.0)), Color(Ink.PAPER_LIGHT, 0.8), false, 1.0)
	var seal_px := int(round(_px(SEAL * 0.40)))
	var marks := [title[0], title[2]] if title.length() >= 3 else []
	for i in marks.size():
		var extent := FONT.get_string_size(marks[i], HORIZONTAL_ALIGNMENT_LEFT, -1.0, seal_px)
		var at := Vector2(seal.get_center().x - extent.x * 0.5, seal.position.y + seal.size.y * (0.27 + 0.46 * i) - seal_px * 0.5)
		draw_string(FONT, at + Vector2(0.0, FONT.get_ascent(seal_px) - (FONT.get_height(seal_px) - seal_px) * 0.5), marks[i], HORIZONTAL_ALIGNMENT_LEFT, -1.0, seal_px, Ink.PAPER_LIGHT)
	var line_y := _line_y()
	draw_line(Vector2(_left + _px(4.0), line_y), Vector2(seal.end.x, line_y), Color(Ink.GOLD, 0.9), maxf(_px(2.0), 1.0))
