class_name MapOverlay
extends Control
## GDD 5.3「文字標籤」、5.7 停留提示、5.9 準星與按鍵提示：畫在地圖之上、器物之下的螢幕空間層。
## 標籤把世界座標投影到螢幕後寫字，依鏡頭高度分級出現，並依重要度貪婪避讓。

## 各類標籤出現的鏡頭高度上限（地圖寬的比例）與字級。
const LABEL_RULES := {
	"continent": {"z_max": 2.0, "z_min": 0.25, "size": 26},
	"province": {"z_max": 0.3, "z_min": 0.0, "size": 16},
	"river": {"z_max": 0.5, "z_min": 0.0, "size": 15},
	"range": {"z_max": 0.5, "z_min": 0.0, "size": 15},
	"node": {"z_max": 0.4, "z_min": 0.0, "size": 15},
}
## 地名依性質分色（GDD 5.1「地名設色」）；例簽用同一張表。
const KIND_COLORS := {
	"continent": Color("3C4A52"),
	"province": Color("2B2620"),
	"seat": Color("8C2A1C"),
	"river": Color("3E6379"),
	"range": Color("3F5A45"),
	"node": Color("5B3A6B"),
}
## 沿路徑排字的字距（字高的倍數）：下限與上限。
const PATH_SPACING_MIN := 1.2
const PATH_SPACING_MAX := 2.2
## 沿路徑排字時字級的下限；路徑短到連這個字級都排不下，就不顯示。
const PATH_MIN_SIZE := 11
const LABEL_OUTLINE := 4
const CENTRED_KINDS: Array[String] = ["province", "continent"]
## 治所：鏡頭高度低於地圖寬的這個比例才標出；字級。
const SEAT_Z_MAX := 0.2
const SEAT_SIZE := 13

var _world: SimWorld
var _map: TerrainMap
var _camera: MapCamera
var _unit := 1.0
var _labels: Array[Dictionary] = []
var _nodes: Array[Dictionary] = []
## 各州郡的治所：位置與治所名（治所名隨城池等級改變，由 refresh_seats 更新）。
var _seats: Array[Dictionary] = []
var _hover_province := -1
var _hover_text := ""


func setup(world: SimWorld, map: TerrainMap, camera: MapCamera, unit: float) -> void:
	_world = world
	_map = map
	_camera = camera
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	set_anchors_preset(Control.PRESET_FULL_RECT)
	_labels.assign(world.map_labels())
	_nodes.assign(world.ley_nodes())
	_labels.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a["rank"] > b["rank"])
	refresh_seats()


func set_hover(province: int) -> void:
	_hover_province = province
	_hover_text = ""
	if province >= 0:
		var info := _world.province_info(province)
		_hover_text = "%s　%s %s" % [info["name"], Locale.t("card_population"), Locale.count(info["population"])]


## 重新讀取各州郡的治所名（每年城池等級可能改變）。
func refresh_seats() -> void:
	_seats.clear()
	for id in _world.province_count():
		var info := _world.province_info(id)
		_seats.append({"pos": info["seat"], "text": info["seat_name"]})


func _process(_delta: float) -> void:
	queue_redraw()


func _draw() -> void:
	if _camera == null:
		return
	if _map.mode == TerrainMap.Mode.LINGMAI:
		_draw_ley_nodes()
	_draw_labels()
	if _camera.use_crosshair:
		_draw_crosshair()
		_draw_prompts()
	if not _hover_text.is_empty():
		_draw_hover_tip()


func _draw_labels() -> void:
	var map_w := _map.map_size.x
	var z_frac := _camera.z / map_w
	var screen := get_viewport_rect()
	var taken: Array[Rect2] = []
	for label in _labels:
		var rule: Dictionary = LABEL_RULES[label["kind"]]
		if z_frac > rule["z_max"] or z_frac < rule["z_min"]:
			continue
		var pos: Vector2 = label["pos"]
		# 極區與州界同步淡出（|緯度| 62°–78°）。
		var latitude := absf(90.0 - 180.0 * pos.y / _map.map_size.y)
		var alpha := 1.0 - smoothstep(62.0, 78.0, latitude)
		if alpha < 0.08:
			continue
		var size := int(rule["size"] * _unit)
		var color := Color(KIND_COLORS[label["kind"]] as Color, alpha)
		var text: String = label["text"]
		var path: PackedVector2Array = label["path"]
		# 江河與山脈：字沿河道或山脊一個一個排開；路徑太短就縮小字級，再排不下就不顯示。
		if path.size() >= 2 and _map.globe_morph < 0.5:
			var points := _path_on_screen(path)
			# 路徑在鏡頭後方：這個標籤不畫。
			if points.is_empty():
				continue
			# 依路徑在畫面上的長度決定字級；排不下最小字級就不顯示。
			size = path_label_size(_path_length(points), text.length(), size, int(PATH_MIN_SIZE * _unit))
			if size == 0:
				continue
			var spots := _along_path(points, text, size, screen)
			if not spots.is_empty():
				var bounds := Rect2(spots[0], Vector2.ZERO)
				for spot in spots:
					bounds = bounds.expand(spot)
				bounds = bounds.grow(size * 0.6)
				if not _blocked(bounds, taken):
					taken.append(bounds)
					for i in spots.size():
						var glyph := text[i]
						var extent := Ink.text_size(glyph, size)
						Ink.outlined_text(self, spots[i] - extent * 0.5, glyph, size, color, LABEL_OUTLINE)
			continue
		if not _visible_on_map(pos):
			continue
		var point := _camera.unproject_position(_world_position(pos))
		var extent := Ink.text_size(text, size)
		# 州郡與大洲的名字以該點置中；其餘寫在該點上方。
		var centred: bool = label["kind"] in CENTRED_KINDS
		var rect := Rect2(point - Vector2(extent.x * 0.5, extent.y * 0.5 if centred else extent.y + 4.0), extent)
		if not screen.encloses(rect) or _blocked(rect, taken):
			continue
		taken.append(rect)
		Ink.outlined_text(self, rect.position, text, size, color, LABEL_OUTLINE)
	_draw_seats(taken)


func _blocked(rect: Rect2, taken: Array[Rect2]) -> bool:
	for other in taken:
		if other.intersects(rect.grow(4.0)):
			return true
	return false


## 該地圖座標在畫面上是否看得到：在鏡頭前方，且星球視角時位於朝向鏡頭的半球。
func _visible_on_map(pos: Vector2) -> bool:
	var world_pos := _world_position(pos)
	if _camera.is_position_behind(world_pos):
		return false
	if _map.globe_morph > 0.5:
		var outward := (world_pos - _map.globe_xform.origin).normalized()
		return outward.dot((_camera.global_position - world_pos).normalized()) > 0.25
	return true


## 沿路徑排字的字級：路徑在畫面上長 total 像素、要排 count 個字。
## 字距不得小於 PATH_SPACING_MIN 個字高；以 base 排不下就縮小字級，縮到 min_size 還排不下則回傳 0（不顯示）。
static func path_label_size(total: float, count: int, base: int, min_size: int) -> int:
	if count < 2:
		return 0
	var fit := int(floor(total / (PATH_SPACING_MIN * (count - 1))))
	var size := mini(base, fit)
	return size if size >= min_size else 0


## 路徑各點的螢幕位置，依閱讀方向排列（由左到右；近乎垂直時由上到下）。有點在鏡頭後方時回傳空陣列。
func _path_on_screen(path: PackedVector2Array) -> PackedVector2Array:
	var points := PackedVector2Array()
	for p in path:
		var world_pos := _world_position(p)
		if _camera.is_position_behind(world_pos):
			return PackedVector2Array()
		points.append(_camera.unproject_position(world_pos))
	var delta := points[points.size() - 1] - points[0]
	if (absf(delta.x) >= absf(delta.y) and delta.x < 0.0) or (absf(delta.x) < absf(delta.y) and delta.y < 0.0):
		points.reverse()
	return points


func _path_length(points: PackedVector2Array) -> float:
	var total := 0.0
	for i in range(1, points.size()):
		total += points[i].distance_to(points[i - 1])
	return total


## 把 text 的每個字沿螢幕上的折線 points 排開，回傳各字中心的位置；有字落在畫面外則回傳空陣列。
## 字距介於 PATH_SPACING_MIN 到 PATH_SPACING_MAX 個字高，整段置中於路徑。
func _along_path(points: PackedVector2Array, text: String, size: int, screen: Rect2) -> PackedVector2Array:
	var lengths := PackedFloat32Array([0.0])
	for i in range(1, points.size()):
		lengths.append(lengths[i - 1] + points[i].distance_to(points[i - 1]))
	var total := lengths[lengths.size() - 1]
	var count := text.length()
	var spacing := clampf(total / (count + 1), size * PATH_SPACING_MIN, size * PATH_SPACING_MAX)
	var start := (total - spacing * (count - 1)) * 0.5
	var spots := PackedVector2Array()
	var segment := 1
	for i in count:
		var target := start + spacing * i
		while segment < lengths.size() - 1 and lengths[segment] < target:
			segment += 1
		var span := maxf(lengths[segment] - lengths[segment - 1], 1e-4)
		var spot := points[segment - 1].lerp(points[segment], (target - lengths[segment - 1]) / span)
		if not screen.has_point(spot):
			return PackedVector2Array()
		spots.append(spot)
	return spots


## 治所（GDD 5.3）：一個墨點，右邊寫治所名；名字與已放的標籤重疊時只畫點。
func _draw_seats(taken: Array[Rect2]) -> void:
	if _camera.z > _map.map_size.x * SEAT_Z_MAX or _map.globe_morph > 0.5:
		return
	var screen := get_viewport_rect()
	var size := Ink.font_size(SEAT_SIZE, _unit)
	var radius := 3.0 * _unit
	for seat in _seats:
		var world_pos := _world_position(seat["pos"])
		if _camera.is_position_behind(world_pos):
			continue
		var point := _camera.unproject_position(world_pos)
		if not screen.has_point(point):
			continue
		draw_circle(point, radius + 1.5, Color(Ink.PAPER_LIGHT, 0.9))
		draw_circle(point, radius, KIND_COLORS["seat"])
		var extent := Ink.text_size(seat["text"], size)
		var rect := Rect2(point + Vector2(radius + 4.0 * _unit, -extent.y * 0.5), extent)
		if not screen.encloses(rect) or _blocked(rect, taken):
			continue
		taken.append(rect)
		Ink.outlined_text(self, rect.position, seat["text"], size, KIND_COLORS["seat"], LABEL_OUTLINE)


## 靈脈模式的洞天福地：金點加青環，大小依強度（GDD 5.4）。
func _draw_ley_nodes() -> void:
	for node in _nodes:
		if not _visible_on_map(node["pos"]):
			continue
		var point := _camera.unproject_position(_world_position(node["pos"]))
		var radius := (3.0 + 4.0 * (node["strength"] as float)) * _unit
		draw_circle(point, radius, Ink.GOLD)
		draw_arc(point, radius + 2.0 * _unit, 0.0, TAU, 24, Ink.CYAN, 1.5, true)


## 地圖座標 → 世界座標：平面取離鏡頭最近的東西副本並貼地；星球視角換算到球面。
func _world_position(pos: Vector2) -> Vector3:
	if _map.globe_morph > 0.5:
		var lon := pos.x / _map.map_size.x * TAU
		var lat := (0.5 - pos.y / _map.map_size.y) * PI
		return _map.globe_xform * (MapView.sphere_dir(lon, lat) * MapView.globe_radius(_map.map_size.x))
	var x := MapView.nearest_copy_x(pos.x, _camera.focus.x, _map.map_size.x)
	return Vector3(x, _map.surface_height(pos.x, pos.y), pos.y)


func _draw_crosshair() -> void:
	var center := get_viewport_rect().size * 0.5
	var radius := 9.0 * _unit
	draw_arc(center, radius, 0.0, TAU, 32, Color(Ink.PAPER_LIGHT, 0.9), 3.5, true)
	draw_arc(center, radius, 0.0, TAU, 32, Ink.SEAL_RED, 1.5, true)
	draw_circle(center, 1.5 * _unit, Ink.SEAL_RED)


func _draw_hover_tip() -> void:
	var size := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var extent := Ink.text_size(_hover_text, size)
	var anchor := get_viewport_rect().size * 0.5 if _camera.use_crosshair else get_local_mouse_position()
	var slip := Rect2(anchor + Vector2(16.0, 18.0) * _unit, extent + Vector2(20.0, 12.0) * _unit)
	var bounds := get_viewport_rect().size
	slip.position.x = minf(slip.position.x, bounds.x - slip.size.x - 8.0)
	slip.position.y = minf(slip.position.y, bounds.y - slip.size.y - 8.0)
	Ink.paper_slip(self, slip)
	Ink.text(self, slip.position + Vector2(10.0, 6.0) * _unit, _hover_text, size)


## 搖桿模式的按鍵提示（GDD 5.9）：右下、印匣上方。
func _draw_prompts() -> void:
	# 其他操作的鍵畫在各自的器物上，這裡只列地圖本身的兩個。
	var entries := [[InputHints.label(InputActions.SELECT, true), "prompt_select"], [InputHints.label(InputActions.CANCEL, true), "prompt_back"]]
	var size := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var width := 10.0 * _unit
	for entry: Array in entries:
		width += Ink.key_glyph_width(entry[0], _unit) + 5.0 * _unit + Ink.text_size(Locale.t(entry[1]), size).x + 14.0 * _unit
	var bounds := get_viewport_rect().size
	var height := Ink.KEY_HEIGHT * _unit + 10.0 * _unit
	var slip := Rect2(Vector2(15.0 * _unit, bounds.y - height - 15.0 * _unit), Vector2(width - 4.0 * _unit, height))
	Ink.paper_slip(self, slip)
	var cursor := slip.position.x + 10.0 * _unit
	for entry: Array in entries:
		cursor += Ink.key_glyph(self, Vector2(cursor, slip.position.y + 5.0 * _unit), entry[0], _unit) + 5.0 * _unit
		var label := Locale.t(entry[1])
		Ink.text(self, Vector2(cursor, slip.position.y + (slip.size.y - Ink.text_size(label, size).y) * 0.5), label, size)
		cursor += Ink.text_size(label, size).x + 14.0 * _unit
