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
	"ocean": {"z_max": 2.0, "z_min": 0.12, "size": 22},
	"sea": {"z_max": 0.6, "z_min": 0.0, "size": 18},
	"lake": {"z_max": 0.3, "z_min": 0.0, "size": 14},
}
## 地名依性質分色（GDD 5.1「地名設色」）；例簽用同一張表。
const KIND_COLORS := {
	"continent": Color("3C4A52"),
	"province": Color("2B2620"),
	"seat": Color("8C2A1C"),
	"river": Color("3E6379"),
	"range": Color("3F5A45"),
	"node": Color("5B3A6B"),
	"sect": Color("2F5560"),
	"water": Color("2E5D62"),
}
## 共用一種顏色的標籤種類：洋、海、湖都算水域。
const KIND_TONES := {"ocean": "water", "sea": "water", "lake": "water"}
## 沿路徑排字的字距（字高的倍數）：下限與上限。
const PATH_SPACING_MIN := 1.2
const PATH_SPACING_MAX := 2.2
## 沿路徑排字時字級的下限；路徑短到連這個字級都排不下，就不顯示。
const PATH_MIN_SIZE := 11
const LABEL_OUTLINE := 4
## 水域名與岸邊之間至少留這麼多像素；檢查外框的這八個點（四角與四邊中點）。
const WATER_MARGIN := 4.0
const WATER_SAMPLES: Array[Vector2] = [
	Vector2(-1, -1), Vector2(0, -1), Vector2(1, -1), Vector2(-1, 0), Vector2(1, 0), Vector2(-1, 1), Vector2(0, 1), Vector2(1, 1),
]
const CENTRED_KINDS: Array[String] = ["province", "continent", "ocean", "sea", "lake"]
## 治所：鏡頭高度低於地圖寬的這個比例才標出；字級。
const SEAT_Z_MAX := 0.2
const SEAT_SIZE := 13
## 山門（GDD 5.5）：中景才畫，鏡頭高度低於地圖寬的這個比例；名字再近一些才寫。
const SECT_Z_MAX := 0.4
const SECT_NAME_Z_MAX := 0.3
const SECT_NAME_SIZE := 13
## 山門圖樣的縮放（圖樣在 24 單位高的座標裡畫，乘此係數與介面縮放）。
const SECT_SCALE := 1.0
## 正道：白牆黛瓦；邪道：黑瓦。
const SECT_ROOF_GOOD := Color("3C4A52")
const SECT_ROOF_EVIL := Color("1E1A17")
const SECT_WALL_EVIL := Color("6B6152")
## 三教模式（GDD 5.4 F3）：道的淡彩（與州郡著色一致）、宗門勢力範圍虛線圈的顏色。
const TAO_TINT := Color(0.65, 0.82, 0.86)
const RING_COLOR := Color("23566A")
const RING_COLOR_EVIL := Color("4A2260")
## 範圍圈（GDD 5.4）由地形著色器依地圖座標畫出，虛線的疏密以螢幕長度為準：
## 每段、間隔的長度（像素，乘介面單位）、線寬與不透明度。
const RING_DASH_PX := 8.0
const RING_GAP_PX := 5.0
const RING_WIDTH_PX := 1.5
const RING_ALPHA := 0.75
## 著色器一次最多畫幾個圈（超過的宗門不畫圈）。
const RING_MAX := 96
## 山門高過中景而不畫圖樣時，在山門位置畫的圈心：半徑（像素，乘介面單位）。
const RING_CENTRE_RADIUS := 3.0

var _world: SimWorld
var _map: TerrainMap
var _camera: MapCamera
var _unit := 1.0
var _labels: Array[Dictionary] = []
var _nodes: Array[Dictionary] = []
## 各州郡的治所：位置與治所名（治所名隨城池等級改變，由 refresh_seats 更新）。
var _seats: Array[Dictionary] = []
## 還在的宗門：山門位置、名字與勢力範圍圈（由 refresh_sects 更新）。
var _sects: Array[Dictionary] = []
var _hover_province := -1
var _hover_text := ""
## 介面器物佔住的畫面範圍：地圖標籤避開這些地方，不鑽到器物底下露出半截。
var reserved: Array[Rect2] = []


func setup(world: SimWorld, map: TerrainMap, camera: MapCamera, unit: float) -> void:
	_world = world
	_map = map
	_camera = camera
	_unit = unit
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	texture_repeat = CanvasItem.TEXTURE_REPEAT_ENABLED
	set_anchors_preset(Control.PRESET_FULL_RECT)
	reload()


## 重新讀取文字標籤、洞天福地與治所（開局，以及一世結算改了地圖之後）。
func reload() -> void:
	_labels.assign(_world.map_labels())
	_nodes.assign(_world.ley_nodes())
	_labels.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a["rank"] > b["rank"])
	refresh_seats()
	refresh_sects()


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


## 重新讀取宗門（新立與覆滅都會改變山門）；只留還在的。
func refresh_sects() -> void:
	_sects.clear()
	for sect: Dictionary in _world.sects():
		if sect["alive"]:
			_sects.append(sect)


## 目前畫面上的山門標記（還在的宗門）。
func sect_marks() -> Array[Dictionary]:
	return _sects


## 山門此刻是否畫出（中景以內，GDD 5.5）。
func sect_gates_visible() -> bool:
	return _camera != null and _camera.z <= _map.map_size.x * SECT_Z_MAX


## 目前要畫的勢力範圍圈數（三教模式，GDD 5.4 F3）；其他模式為 0。
func range_ring_count() -> int:
	return _sects.size() if _map.mode == TerrainMap.Mode.SANJIAO else 0


func _process(_delta: float) -> void:
	_update_rings()
	queue_redraw()


## 把範圍圈交給地形著色器：圈心、半徑、顏色，以及每圈的虛線段數（讓整圈剛好排滿整數段）。
## 虛線一段加一個間隔在螢幕上約 RING_DASH_PX + RING_GAP_PX 個像素：取圈上最靠近鏡頭焦點的那一點，
## 量該處每格地面對應的螢幕像素，換算成每段的世界長度。
func _update_rings() -> void:
	if _camera == null:
		return
	if _map.mode != TerrainMap.Mode.SANJIAO:
		_map.clear_range_rings()
		return
	var data := PackedVector4Array()
	var colors := PackedVector3Array()
	var width := RING_WIDTH_PX * _unit
	for sect in _sects:
		if data.size() >= RING_MAX:
			break
		var centre: Vector2 = sect["pos"]
		var radius: float = sect["range_radius"]
		var segments := maxf(roundf(TAU * radius / ring_period(sect)), 4.0)
		data.append(Vector4(centre.x, centre.y, radius, segments))
		var color := RING_COLOR_EVIL if sect["evil"] else RING_COLOR
		colors.append(Vector3(color.r, color.g, color.b))
	_map.set_range_rings(data, colors, width, RING_DASH_PX / (RING_DASH_PX + RING_GAP_PX), RING_ALPHA)


## 畫面上每一格地面約多少像素：在圈上最靠近鏡頭焦點的那一點量；量不到（在鏡頭後方）回傳 0。
func ring_pixels_per_cell(sect: Dictionary) -> float:
	var centre: Vector2 = sect["pos"]
	var radius: float = sect["range_radius"]
	var away := Vector2(_camera.focus.x - centre.x, _camera.focus.y - centre.y)
	away.x -= roundf(away.x / _map.map_size.x) * _map.map_size.x
	var dir := away.normalized() if away.length() > 1e-3 else Vector2.RIGHT
	var spot := centre + dir * radius
	spot.y = clampf(spot.y, 0.5, _map.map_size.y - 0.5)
	var step := 0.5
	var along := spot + Vector2(-dir.y, dir.x) * step
	var a := _world_position(spot)
	var b := _world_position(along)
	if _camera.is_position_behind(a) or _camera.is_position_behind(b):
		return 0.0
	return _camera.unproject_position(a).distance_to(_camera.unproject_position(b)) / step


## 一段虛線加一個間隔在圈上的世界長度（格）：螢幕上約 (RING_DASH_PX + RING_GAP_PX) × 介面單位像素。
func ring_period(sect: Dictionary) -> float:
	var px := ring_pixels_per_cell(sect)
	if px < 1e-4:
		return 1.0e6
	return (RING_DASH_PX + RING_GAP_PX) * _unit / px


func _draw() -> void:
	if _camera == null:
		return
	if _map.mode == TerrainMap.Mode.LINGMAI:
		_draw_ley_nodes()
	elif _map.mode == TerrainMap.Mode.SANJIAO and not sect_gates_visible():
		_draw_ring_centres()
	_draw_labels()
	if _camera.use_crosshair:
		_draw_prompts()
	if not _hover_text.is_empty():
		_draw_hover_tip()


func _draw_labels() -> void:
	var map_w := _map.map_size.x
	var z_frac := _camera.z / map_w
	var screen := get_viewport_rect()
	var taken: Array[Rect2] = reserved.duplicate()
	_draw_sects(taken)
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
		var color := Color(KIND_COLORS[KIND_TONES.get(label["kind"], label["kind"])] as Color, alpha)
		var text: String = label["text"]
		var path: PackedVector2Array = label["path"]
		# 江河與山脈：字沿河道或山脊一個一個排開；路徑太短就縮小字級，再排不下就不顯示。
		if path.size() >= 2:
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
		var world_pos := _world_position(pos)
		var point := _camera.unproject_position(world_pos)
		# 水域名要整個落在水面上：放不進去就縮小字級，縮到下限仍放不下就不顯示。
		if KIND_TONES.has(label["kind"]) and _map.globe_morph < 0.5:
			size = _fit_on_water(point, world_pos.y, text, size)
			if size == 0:
				continue
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
	if _camera.is_position_behind(_world_position(pos)):
		return false
	return _map.faces_camera(pos, _camera.global_position)


## 沿路徑排字的字級：路徑在畫面上長 total 像素、要排 count 個字。
## 字距不得小於 PATH_SPACING_MIN 個字高；以 base 排不下就縮小字級，縮到 min_size 還排不下則回傳 0（不顯示）。
static func path_label_size(total: float, count: int, base: int, min_size: int) -> int:
	if count < 2:
		return 0
	var fit := int(floor(total / (PATH_SPACING_MIN * (count - 1))))
	var size := mini(base, fit)
	return size if size >= min_size else 0


## 路徑各點的螢幕位置，依閱讀方向排列（由左到右；近乎垂直時由上到下）。
## 有點在鏡頭後方、或星球視角下轉到背面時回傳空陣列。
func _path_on_screen(path: PackedVector2Array) -> PackedVector2Array:
	var points := PackedVector2Array()
	# 整條路徑用同一個東西副本（跟著第一個點選）：逐點各自選最近的副本時，
	# 離鏡頭約半個地圖寬的路徑會被拆到兩邊，折線就橫跨整個畫面。
	var shift := MapView.nearest_copy_x(path[0].x, _camera.focus.x, _map.map_size.x) - path[0].x
	var on_globe := _map.globe_morph > 0.5
	for p in path:
		var world_pos := _world_position(p) if on_globe else _map.shown_position(p.x + shift, p)
		if _camera.is_position_behind(world_pos) or not _map.faces_camera(p, _camera.global_position):
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


## 水域名的字級：由 base 往下試到字級下限，取第一個讓名字的外框（四角與四邊中點，外加留白）
## 全部落在水面上的字級；都不行回傳 0。plane_y 是水面的高度。
func _fit_on_water(point: Vector2, plane_y: float, text: String, base: int) -> int:
	var floor_size := int(PATH_MIN_SIZE * _unit)
	var size := base
	while size >= floor_size:
		var half := Ink.text_size(text, size) * 0.5 + Vector2(WATER_MARGIN, WATER_MARGIN) * _unit
		var fits := true
		for offset: Vector2 in WATER_SAMPLES:
			if not _is_water_at(point + offset * half, plane_y):
				fits = false
				break
		if fits:
			return size
		size -= 2
	return 0


## 螢幕上的一點對到水面（高度 plane_y 的平面）後，是不是水：不屬於任何州郡的格就是水。
func _is_water_at(screen: Vector2, plane_y: float) -> bool:
	var origin := _camera.project_ray_origin(screen)
	var dir := _camera.project_ray_normal(screen)
	if absf(dir.y) < 1e-5:
		return false
	var hit := _map.ground_position(origin + dir * ((plane_y - origin.y) / dir.y))
	if hit.y < 0.0 or hit.y >= _map.map_size.y:
		return false
	return _map.province_at(Vector2(fposmod(hit.x, _map.map_size.x), hit.y)) < 0


## 治所（GDD 5.3）：一個墨點，右邊寫治所名；名字與已放的標籤重疊時只畫點。
func _draw_seats(taken: Array[Rect2]) -> void:
	if _camera.z > _map.map_size.x * SEAT_Z_MAX:
		return
	var screen := get_viewport_rect()
	var size := Ink.font_size(SEAT_SIZE, _unit)
	var radius := 3.0 * _unit
	for seat in _seats:
		if not _visible_on_map(seat["pos"]):
			continue
		var world_pos := _world_position(seat["pos"])
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


## 山門（GDD 5.5）：牌坊、三重殿與山徑；正道白牆黛瓦，邪道黑瓦。中景才畫，名字寫在圖樣下方，
## 與已放的山門名重疊就只畫圖樣；圖樣與名字的範圍進 taken，其餘標籤與治所名讓開。
func _draw_sects(taken: Array[Rect2]) -> void:
	if not sect_gates_visible():
		return
	var screen := get_viewport_rect()
	var show_name := _camera.z <= _map.map_size.x * SECT_NAME_Z_MAX
	var size := Ink.font_size(SECT_NAME_SIZE, _unit)
	var scale := SECT_SCALE * _unit
	for sect in _sects:
		if not _visible_on_map(sect["pos"]):
			continue
		var point := _camera.unproject_position(_world_position(sect["pos"]))
		if not screen.has_point(point):
			continue
		var evil: bool = sect["evil"]
		_draw_gate(point, scale, evil, _map.mode == TerrainMap.Mode.KARMA)
		var mark := Rect2(point + Vector2(-8.0, -24.0) * scale, Vector2(16.0, 27.0) * scale)
		if show_name:
			var extent := Ink.text_size(sect["name"], size)
			var rect := Rect2(point + Vector2(-extent.x * 0.5, 6.0 * scale), extent)
			if screen.encloses(rect) and not _blocked(rect, taken):
				Ink.outlined_text(self, rect.position, sect["name"], size, KIND_COLORS["sect"], LABEL_OUTLINE)
				taken.append(rect)
		taken.append(mark)


## 山門圖樣：base 是山徑的落腳點（山門位置），scale 是每個圖樣單位的像素。
func _draw_gate(base: Vector2, scale: float, evil: bool, outline_only := false) -> void:
	var wall := SECT_WALL_EVIL if evil else Ink.PAPER_LIGHT
	var roof := SECT_ROOF_EVIL if evil else SECT_ROOF_GOOD
	var line := maxf(1.0, scale * 1.1)
	if outline_only:
		_draw_gate_outline(base, scale, line)
		return
	# 襯底的紙色暈，讓圖樣在山色上看得清楚。
	draw_circle(base + Vector2(0.0, -10.0) * scale, 13.0 * scale, Color(Ink.PAPER_LIGHT, 0.55))
	# 山徑：由牌坊下蜿蜒下山。
	var path := PackedVector2Array([Vector2(0.0, -1.0), Vector2(2.5, 1.0), Vector2(-1.5, 2.5), Vector2(1.0, 4.0)])
	for i in path.size():
		path[i] = base + path[i] * scale
	draw_polyline(path, Ink.INK_LIGHT, line, true)
	# 牌坊：兩柱一梁。
	for x in [-4.0, 4.0]:
		draw_line(base + Vector2(x, -1.0) * scale, base + Vector2(x, -7.0) * scale, Ink.INK, line * 1.2)
	draw_line(base + Vector2(-6.0, -7.0) * scale, base + Vector2(6.0, -7.0) * scale, Ink.INK, line * 1.6)
	# 三重殿：牆身加歇山頂，逐層收小。
	var tiers := [[5.0, 8.5, 11.5, 4.0], [3.6, 12.5, 15.5, 3.2], [2.2, 16.5, 19.0, 2.6]]
	for tier: Array in tiers:
		var half: float = tier[0]
		var foot: float = tier[1]
		var eave: float = tier[2]
		var rise: float = tier[3]
		var body := PackedVector2Array([
			Vector2(-half, -foot), Vector2(half, -foot), Vector2(half, -eave + 1.0), Vector2(-half, -eave + 1.0),
		])
		var cap := PackedVector2Array([
			Vector2(-half - 1.8, -eave + 1.0), Vector2(half + 1.8, -eave + 1.0),
			Vector2(half - 0.6, -eave - rise + 1.0), Vector2(-half + 0.6, -eave - rise + 1.0),
		])
		for shape in [body, cap]:
			for i in shape.size():
				shape[i] = base + shape[i] * scale
		draw_colored_polygon(body, wall)
		draw_colored_polygon(cap, roof)
		draw_polyline(body + PackedVector2Array([body[0]]), Ink.INK, line * 0.8, true)
		draw_polyline(cap + PackedVector2Array([cap[0]]), Ink.INK, line * 0.8, true)


## 因果模式的山門只留輪廓（GDD 5.3「因果模式後處理」）：牌坊與三重殿的外框，淡紙色細線，不填色。
func _draw_gate_outline(base: Vector2, scale: float, line: float) -> void:
	var color := Color(Ink.PAPER_LIGHT, 0.8)
	for x in [-4.0, 4.0]:
		draw_line(base + Vector2(x, -1.0) * scale, base + Vector2(x, -7.0) * scale, color, line)
	draw_line(base + Vector2(-6.0, -7.0) * scale, base + Vector2(6.0, -7.0) * scale, color, line)
	for tier: Array in [[5.0, 8.5, 11.5, 4.0], [3.6, 12.5, 15.5, 3.2], [2.2, 16.5, 19.0, 2.6]]:
		var half: float = tier[0]
		var foot: float = tier[1]
		var eave: float = tier[2]
		var rise: float = tier[3]
		var shape := PackedVector2Array([
			Vector2(-half, -foot), Vector2(half, -foot), Vector2(half, -eave + 1.0), Vector2(half + 1.8, -eave + 1.0),
			Vector2(half - 0.6, -eave - rise + 1.0), Vector2(-half + 0.6, -eave - rise + 1.0), Vector2(-half - 1.8, -eave + 1.0),
			Vector2(-half, -eave + 1.0), Vector2(-half, -foot),
		])
		for i in shape.size():
			shape[i] = base + shape[i] * scale
		draw_polyline(shape, color, line * 0.8, true)


## 三教模式、鏡頭高過中景而不畫山門圖樣時，在山門位置畫一個小圓點（圈的圈心），
## 顏色與圈相同、墨色細邊，讓每個圈分得出屬於哪個宗門。
func _draw_ring_centres() -> void:
	var screen := get_viewport_rect()
	var radius := RING_CENTRE_RADIUS * _unit
	for sect in _sects:
		if not _visible_on_map(sect["pos"]):
			continue
		var point := _camera.unproject_position(_world_position(sect["pos"]))
		if not screen.has_point(point):
			continue
		draw_circle(point, radius + 1.0, Ink.INK)
		draw_circle(point, radius, RING_COLOR_EVIL if sect["evil"] else RING_COLOR)


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
	return _map.world_position(pos, _camera.focus.x)


func _draw_hover_tip() -> void:
	var size := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var extent := Ink.text_size(_hover_text, size)
	var anchor := _camera.pointer_position()
	var slip := Rect2(anchor + Vector2(16.0, 18.0) * _unit, extent + Vector2(20.0, 12.0) * _unit)
	var bounds := get_viewport_rect().size
	slip.position.x = minf(slip.position.x, bounds.x - slip.size.x - 8.0)
	slip.position.y = minf(slip.position.y, bounds.y - slip.size.y - 8.0)
	Ink.paper_slip(self, slip)
	Ink.text(self, slip.position + Vector2(10.0, 6.0) * _unit, _hover_text, size)


## 搖桿模式的按鍵提示（GDD 5.9）：左下。只列地圖本身的操作，其他操作的鍵畫在各自的器物上。
## 每組是該操作的全部實體鍵圖像加動作名。
func _draw_prompts() -> void:
	var entries := [
		[[InputHints.label(InputActions.CURSOR_LEFT, true)], "prompt_cursor"],
		[InputHints.labels(InputActions.LOCK_ON, true), "prompt_lock"],
		[InputHints.labels(InputActions.PAD_LEFT, true), "prompt_select"],
		[InputHints.labels(InputActions.PAD_RIGHT, true), "prompt_rotate"],
		[[InputHints.label(InputActions.ZOOM_IN, true)], "prompt_zoom"],
		[[InputHints.label(InputActions.ROTATE_LEFT, true)], "prompt_turn"],
		[InputHints.labels(InputActions.CANCEL, true), "prompt_back"],
	]
	var size := Ink.font_size(Ink.SIZE_NOTE, _unit)
	var width := 10.0 * _unit
	for entry: Array in entries:
		for key: String in entry[0]:
			width += Ink.key_glyph_width(key, _unit) + 4.0 * _unit
		width += 1.0 * _unit + Ink.text_size(Locale.t(entry[1]), size).x + 14.0 * _unit
	var bounds := get_viewport_rect().size
	var height := Ink.KEY_HEIGHT * _unit + 10.0 * _unit
	var slip := Rect2(Vector2(15.0 * _unit, bounds.y - height - 15.0 * _unit), Vector2(width - 4.0 * _unit, height))
	Ink.paper_slip(self, slip)
	var cursor := slip.position.x + 10.0 * _unit
	for entry: Array in entries:
		for key: String in entry[0]:
			cursor += Ink.key_glyph(self, Vector2(cursor, slip.position.y + 5.0 * _unit), key, _unit) + 4.0 * _unit
		cursor += 1.0 * _unit
		var label := Locale.t(entry[1])
		Ink.text(self, Vector2(cursor, slip.position.y + (slip.size.y - Ink.text_size(label, size).y) * 0.5), label, size)
		cursor += Ink.text_size(label, size).x + 14.0 * _unit
