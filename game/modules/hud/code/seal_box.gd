class_name SealBox
extends Control
## GDD 5.7 印匣與例簽：右下六枚模式鈕切換地圖模式（5.4），每枚是圖示加模式名；
## 鍵盤模式在鈕的右上角掛該模式的鍵帽，搖桿模式在整列兩側畫 LB、RB。
## 停留在某鈕上時，上方展開例簽說明該模式。支援的決定：想問的是哪一類問題。

signal mode_chosen(mode: TerrainMap.Mode)

const SEAL_SIZE := 38.0
const GAP := 6.0
const FLASH_SECONDS := 1.6
## 每枚鈕：字串鍵（模式名為 mode_<key>，圖示同名）、對應的地圖模式（-1 表示尚未開放）與快捷鍵的動作。
const SEALS := [
	{"key": "shanhe", "mode": TerrainMap.Mode.SHANHE, "action": InputActions.MODE_SHANHE},
	{"key": "tianxia", "mode": TerrainMap.Mode.TIANXIA, "action": InputActions.MODE_TIANXIA},
	{"key": "sanjiao", "mode": -1, "action": &""},
	{"key": "lingmai", "mode": TerrainMap.Mode.LINGMAI, "action": InputActions.MODE_LINGMAI},
	{"key": "yinguo", "mode": -1, "action": &""},
	{"key": "yeqi", "mode": -1, "action": &""},
]
const ICON_RADIUS := 9.0
const INNER_PAD := 9.0
## 鍵帽掛在鈕的上緣，凸出這麼多。
const BADGE_RISE := 9.0
## 山河模式的例簽列出的地形（色塊見 TerrainSwatches）。
const TERRAIN_LEGEND: Array[String] = [
	"terrain_plains", "terrain_hills", "terrain_mountain", "terrain_forest",
	"terrain_desert", "terrain_snow", "terrain_swamp", "terrain_water",
]
const LEGEND_SPACE := 160.0

var mode: TerrainMap.Mode = TerrainMap.Mode.SHANHE
## 目前的輸入裝置是否為搖桿（決定畫哪一種實體鍵圖像）。
var gamepad := false:
	set(value):
		gamepad = value
		queue_redraw()
var _unit := 1.0
var _hover := -1
## 切換模式後，例簽還要顯示多久（秒）。
var _flash := 0.0


func setup(unit: float) -> void:
	_unit = unit
	var width := _button_width() * SEALS.size() + GAP * (SEALS.size() - 1) * unit + _side() * 2.0
	custom_minimum_size = Vector2(width, (SEAL_SIZE + LEGEND_SPACE) * unit)
	size = custom_minimum_size
	mouse_filter = Control.MOUSE_FILTER_STOP
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED


func set_mode(new_mode: TerrainMap.Mode) -> void:
	if new_mode != mode:
		_flash = FLASH_SECONDS
	mode = new_mode
	queue_redraw()


func _process(delta: float) -> void:
	if _flash > 0.0:
		_flash = maxf(_flash - delta, 0.0)
		if _flash == 0.0:
			queue_redraw()


## 依序切到上一個或下一個已開放的模式（搖桿 LB／RB）。
func cycle(direction: int) -> void:
	var available: Array[int] = []
	for entry: Dictionary in SEALS:
		if entry["mode"] >= 0:
			available.append(entry["mode"])
	var index := available.find(int(mode))
	mode_chosen.emit(available[posmod(index + direction, available.size())] as TerrainMap.Mode)


## 整列兩側留給 LB、RB 的寬度。
func _side() -> float:
	return Ink.key_glyph_width("LB", _unit) + 6.0 * _unit


func _button_width() -> float:
	var text := Ink.text_size(Locale.t("mode_shanhe"), Ink.font_size(Ink.SIZE_SECTION, _unit)).x
	return (INNER_PAD * 2.0 + ICON_RADIUS * 2.0 + 6.0) * _unit + text


func _seal_rect(index: int) -> Rect2:
	var width := _button_width()
	var height := SEAL_SIZE * _unit
	return Rect2(Vector2(_side() + (width + GAP * _unit) * index, size.y - height), Vector2(width, height))


func _button_label(index: int) -> String:
	return Locale.t("mode_%s" % SEALS[index]["key"])


## 該鈕的鍵帽：鍵盤模式且已開放才有。
func _key_label(index: int) -> String:
	if gamepad or SEALS[index]["mode"] < 0:
		return ""
	return InputHints.label(SEALS[index]["action"], false)


## 搖桿切換上一個、下一個模式的鍵。
func _cycle_keys() -> Array[String]:
	return [InputHints.label(InputActions.MODE_PREV, true), InputHints.label(InputActions.MODE_NEXT, true)]


## 例簽所在區域的上緣（相對於本器物）。
func _legend_top() -> float:
	return 0.0


func _draw() -> void:
	var label_size := Ink.font_size(Ink.SIZE_SECTION, _unit)
	for i in SEALS.size():
		var entry: Dictionary = SEALS[i]
		var rect := _seal_rect(i)
		var ink := Ink.plate(self, rect, entry["mode"] == int(mode), entry["mode"] >= 0, i == _hover)
		var icon_center := Vector2(rect.position.x + (INNER_PAD + ICON_RADIUS) * _unit, rect.get_center().y)
		InkIcons.draw(self, entry["key"], icon_center, ICON_RADIUS * _unit, ink)
		var label := _button_label(i)
		var text_pos := Vector2(icon_center.x + (ICON_RADIUS + 6.0) * _unit, rect.position.y + (rect.size.y - Ink.text_size(label, label_size).y) * 0.5)
		Ink.text(self, text_pos, label, label_size, ink)
		var key := _key_label(i)
		Ink.key_glyph(self, Vector2(rect.end.x - Ink.key_glyph_width(key, _unit) - 3.0, rect.position.y - BADGE_RISE * _unit), key, _unit)
	if gamepad:
		var keys := _cycle_keys()
		var key_y := size.y - (SEAL_SIZE + Ink.KEY_HEIGHT) * 0.5 * _unit
		Ink.key_glyph(self, Vector2(0.0, key_y), keys[0], _unit)
		Ink.key_glyph(self, Vector2(size.x - Ink.key_glyph_width(keys[1], _unit), key_y), keys[1], _unit)
	if _hover >= 0:
		_draw_legend(SEALS[_hover])
	elif _flash > 0.0:
		for entry: Dictionary in SEALS:
			if entry["mode"] == int(mode):
				_draw_legend(entry)


func _draw_legend(entry: Dictionary) -> void:
	var lines: Array = []
	match entry["mode"]:
		TerrainMap.Mode.SHANHE:
			for key in TERRAIN_LEGEND:
				lines.append([key, "terrain"])
		TerrainMap.Mode.TIANXIA:
			lines = [["legend_tianxia", ""]]
		TerrainMap.Mode.LINGMAI:
			lines = [["legend_lingmai", "ramp"]]
		_:
			lines = [["mode_disabled_hint", ""]]
	var title_size := Ink.font_size(Ink.SIZE_SECTION, _unit)
	var note_size := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var row := 20.0 * _unit
	var columns := 2 if lines.size() > 4 else 1
	var rows := int(ceil(float(lines.size()) / columns))
	var height := 14.0 * _unit + title_size + 6.0 * _unit + rows * row + 8.0 * _unit
	var band := Ink.SLIP_BAND
	var slip := Rect2(Vector2(_side() + band, size.y - (SEAL_SIZE + BADGE_RISE + 14.0) * _unit - band - height), Vector2(size.x - (_side() + band) * 2.0, height))
	Ink.paper_slip(self, slip)
	var cursor := slip.position + Vector2(12.0, 10.0) * _unit
	Ink.text(self, cursor, Locale.t("mode_%s" % entry["key"]), title_size)
	cursor.y += title_size + 6.0 * _unit
	var column_width := (slip.size.x - 24.0 * _unit) / columns
	for i in lines.size():
		var pos := cursor + Vector2(column_width * (i % columns), row * (i / columns))
		var swatch: String = lines[i][1]
		var text_x := 0.0
		if swatch == "ramp":
			var steps := 5
			for k in steps:
				var t := float(k) / (steps - 1)
				var color := Ink.PAPER.lerp(Ink.CYAN, t * 2.0) if t < 0.5 else Ink.CYAN.lerp(Ink.GOLD, (t - 0.5) * 2.0)
				draw_rect(Rect2(pos + Vector2(k * 14.0 * _unit, 3.0 * _unit), Vector2(13.0, 12.0) * _unit), color, true)
			text_x = steps * 14.0 * _unit + 6.0 * _unit
		elif swatch == "terrain":
			TerrainSwatches.chip(self, Rect2(pos + Vector2(0.0, (note_size - 12.0 * _unit) * 0.5 + 2.0), Vector2(13.0, 12.0) * _unit), lines[i][0])
			text_x = 19.0 * _unit
		Ink.text(self, pos + Vector2(text_x, 0.0), Locale.t(lines[i][0]), note_size, Ink.INK)


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var index: int = _seal_at((event as InputEventMouseMotion).position)
		if index != _hover:
			_hover = index
			queue_redraw()
	elif event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		var index: int = _seal_at((event as InputEventMouseButton).position)
		if index >= 0:
			accept_event()
			if SEALS[index]["mode"] >= 0:
				mode_chosen.emit(SEALS[index]["mode"] as TerrainMap.Mode)


func _notification(what: int) -> void:
	if what == NOTIFICATION_MOUSE_EXIT and _hover >= 0:
		_hover = -1
		queue_redraw()


func _seal_at(pos: Vector2) -> int:
	for i in SEALS.size():
		if _seal_rect(i).has_point(pos):
			return i
	return -1


## 滑鼠只在印章上才攔截，例簽與空白處讓地圖收到事件。
func _has_point(point: Vector2) -> bool:
	return _seal_at(point) >= 0
