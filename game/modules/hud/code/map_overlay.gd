class_name MapOverlay
extends Control
## GDD 5.3「文字標籤」、5.7 停留提示、5.9 準星與按鍵提示：畫在地圖之上、器物之下的螢幕空間層。
## 標籤把世界座標投影到螢幕後寫字，依鏡頭高度分級出現，並依重要度貪婪避讓。

## 各類標籤出現的鏡頭高度上限（地圖寬的比例）與字級。
const LABEL_RULES := {
	"continent": {"z_max": 2.0, "z_min": 0.25, "size": 26},
	"province": {"z_max": 0.3, "z_min": 0.0, "size": 16},
	"river": {"z_max": 0.5, "z_min": 0.0, "size": 14},
	"range": {"z_max": 0.5, "z_min": 0.0, "size": 14},
	"node": {"z_max": 0.4, "z_min": 0.0, "size": 14},
}
const KIND_COLORS := {
	"continent": Ink.INK, "province": Ink.INK, "river": Color("3E6379"), "range": Ink.INK_LIGHT, "node": Ink.CYAN,
}

var _world: SimWorld
var _map: TerrainMap
var _camera: MapCamera
var _unit := 1.0
var _labels: Array[Dictionary] = []
var _nodes: Array[Dictionary] = []
var _hover_province := -1
var _hover_text := ""
## 州郡卡是否開著（按鍵提示多列一組「置中」）。
var card_open := false


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


func set_hover(province: int) -> void:
	_hover_province = province
	_hover_text = ""
	if province >= 0:
		var info := _world.province_info(province)
		_hover_text = "%s　%s %s" % [info["name"], Locale.t("card_population"), Locale.count(info["population"])]


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
		# 江河與山脈：字沿河道或山脊一個一個排開；排不下就退回單點標籤。
		if path.size() >= 2 and _map.globe_morph < 0.5:
			var spots := _along_path(path, text, size, screen)
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
						Ink.outlined_text(self, spots[i] - extent * 0.5, glyph, size, color)
				continue
		if not _visible_on_map(pos):
			continue
		var point := _camera.unproject_position(_world_position(pos))
		var extent := Ink.text_size(text, size)
		var rect := Rect2(point - Vector2(extent.x * 0.5, extent.y + 4.0), extent)
		if not screen.encloses(rect) or _blocked(rect, taken):
			continue
		taken.append(rect)
		Ink.outlined_text(self, rect.position, text, size, color)


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


## 把 text 的每個字沿 path 排開，回傳各字中心的螢幕位置；畫面上的路徑太短或不在畫面內則回傳空陣列。
## 字距介於 1.5 到 4 個字高，整段置中於路徑；由左到右（路徑近乎垂直時由上到下）閱讀。
func _along_path(path: PackedVector2Array, text: String, size: int, screen: Rect2) -> PackedVector2Array:
	var points := PackedVector2Array()
	for p in path:
		var world_pos := _world_position(p)
		if _camera.is_position_behind(world_pos):
			return PackedVector2Array()
		points.append(_camera.unproject_position(world_pos))
	var first := points[0]
	var last := points[points.size() - 1]
	var delta := last - first
	if (absf(delta.x) >= absf(delta.y) and delta.x < 0.0) or (absf(delta.x) < absf(delta.y) and delta.y < 0.0):
		points.reverse()
	var lengths := PackedFloat32Array([0.0])
	for i in range(1, points.size()):
		lengths.append(lengths[i - 1] + points[i].distance_to(points[i - 1]))
	var total := lengths[lengths.size() - 1]
	var count := text.length()
	if count < 2 or total < size * 1.5 * (count - 1):
		return PackedVector2Array()
	var spacing := minf(total / (count + 1), size * 4.0)
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
	var entries := [["A", "prompt_select"], ["B", "prompt_back"], ["LB RB", "prompt_mode"], [Locale.t("prompt_start"), "prompt_pause"]]
	if card_open:
		entries.insert(2, ["X", "act_center"])
	var size := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var parts: Array[String] = []
	for entry: Array in entries:
		parts.append("%s %s" % [entry[0], Locale.t(entry[1])])
	var line := "　".join(parts)
	var extent := Ink.text_size(line, size)
	var bounds := get_viewport_rect().size
	var slip := Rect2(Vector2(16.0 * _unit, bounds.y - extent.y - 28.0 * _unit), extent + Vector2(20.0, 12.0) * _unit)
	Ink.paper_slip(self, slip)
	Ink.text(self, slip.position + Vector2(10.0, 6.0) * _unit, line, size)
