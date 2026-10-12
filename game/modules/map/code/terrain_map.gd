class_name TerrainMap
extends Node3D
## GDD 5.3 地圖：由 SimWorld 的資料建立地形網格、世界貼圖與河流製圖線，
## 並依鏡頭狀態更新著色參數。東西向以三份副本接成無縫。

## 世界的一世結算改了地圖後發出（GDD 2.8「畫面」）：terrain 為真表示高度場也換了，否則只換了州界、靈氣與靈脈。
signal refreshed(terrain: bool)

enum Mode { SHANHE = 0, TIANXIA = 1, SANJIAO = 2, LINGMAI = 3, KARMA = 4 }

const TERRAIN_SHADER := preload("res://modules/map/code/terrain_relief.gdshader")
const RIVER_SHADER := preload("res://modules/map/code/river_line.gdshader")
const COPIES := 3
const WATER_TEXTURE_SIZE := 256
## 天下、三教模式的州郡疊色不透明度。
const TINT_OPACITY := 0.45
## 著色器一次最多畫幾個範圍圈（與地形著色器的 ring_data 長度一致）。
const RING_MAX := 96
## GDD 5.3 過渡時間：平面 ↔ 球面 0.6 秒。
const GLOBE_MORPH_SECONDS := 0.6
## 天地演變換圖後，新舊地形之間漸變的秒數（GDD 2.8「畫面」）。
const TERRAIN_FADE_SECONDS := 3.0
const RELIEF_TEXTURES: Array[String] = ["elevation", "lake", "climate", "cover"]
## GDD 5.3 因果模式後處理：轉墨的漸變秒數；地形著色器的 ink_wash 由 0 走到 1。
const INK_WASH_SECONDS := 0.35
## 因果模式把亮度乘、彩度乘的係數（GDD 5.3），河流色用同一組換算。
const WASH_LIGHT := 0.35
const WASH_SATURATION := 0.2

## GDD 5.1 色票（sRGB）。
const PAPER := Color("EDE4CF")
const INK := Color("2B2620")
const INK_LIGHT := Color("6B6152")
const WATER := Color("8DB3B6")
const GOLD := Color("C9A246")
const CYAN := Color("4F8F9A")
## 岸線：墨。
const COAST_LINE := Color("2F3B3D")
## 海深色階（淺 → 深）。
const SEA_RAMP: Array[Color] = [
	Color("C6D2BC"), Color("ADC2AE"), Color("93B0A0"), Color("789C93"), Color("5F8785"),
]
## 沙土覆蓋的顏色（GDD 5.1 的沙漠色）。
const SAND := Color("D9C59A")
## 高度色階（青綠山水）：山腳赭 → 山腰石綠 → 高山石青 → 雪。
const ROCK_RAMP: Array[Color] = [Color("C9B58C"), Color("9FB08A"), Color("7397A3"), Color("F2F1EC")]

static var _water_texture: ImageTexture

var map_size := Vector2.ZERO
var mode: Mode = Mode.SHANHE
var globe_morph := 0.0
var globe_xform := Transform3D.IDENTITY

var _world: SimWorld
var _terrain_material: ShaderMaterial
var _river_material: ShaderMaterial
var _terrain_copies: Array[MeshInstance3D] = []
var _river_copies: Array[MeshInstance3D] = []
var _old_river_copies: Array[MeshInstance3D] = []
var _ley_material: ShaderMaterial
var _ley_copies: Array[MeshInstance3D] = []
var _province_colors: ImageTexture
var _blank_colors: ImageTexture
## 三教模式的州郡著色表（道的教化濃度），隨教化變動更新。
var _faith_colors: ImageTexture
var _ring_count := 0
## 因果模式的魂光與因果線圖層（GDD 5.4）；轉墨程度 0 是原樣、1 是全轉墨。
var _karma: KarmaLayer
var _ink_wash := 0.0
var _ui_unit := 1.0
## 高度消失點（MapView.lean）：鏡頭焦點、地面上由焦點指向鏡頭的方向與眼高；眼高為 0 時不位移。
var _view_focus := Vector2.ZERO
var _toward := Vector2(0.0, 1.0)
var _eye := 0.0
var _globe_relief := 1.0
## 全圖最高的地表高度（建圖時取一次；每次查詢要掃整張地圖）。
var _top_height := 1.0
# 漸變：0 是換圖前、1 是換完；起點與終點各記海平面、最高點的相對高度、最高的地表高度。
var _fade := 1.0
var _fade_from := Vector3.ZERO
var _fade_to := Vector3.ZERO
## 目前畫面依據的是世界的第幾版資料（SimWorld.map_revision、terrain_revision）。
var _shown_map := 0
var _shown_terrain := 0


## 以世界資料建立地圖。世界改變地形後再呼叫一次即可重建。
func build(world: SimWorld) -> void:
	_world = world
	for copy in _terrain_copies + _river_copies + _old_river_copies + _ley_copies:
		copy.queue_free()
	if _karma != null:
		_karma.queue_free()
		_karma = null
	_terrain_copies.clear()
	_river_copies.clear()
	_old_river_copies.clear()
	_ley_copies.clear()

	var grid := world.grid_size()
	map_size = Vector2(grid)
	_terrain_material = _make_terrain_material(world)
	_river_material = _make_line_material()
	_ley_material = _make_line_material()
	_ley_material.set_shader_parameter("line_color", _lin(CYAN))
	_ley_material.set_shader_parameter("level_width_px", Vector3(1.5, 2.0, 3.0))
	_ley_material.render_priority = 2

	var terrain_mesh := _mesh_from(world.terrain_mesh(), "", "uvs", "")
	var river_mesh := _mesh_from(world.river_mesh(), "tangents", "side_level", "taper")
	var ley_mesh := _mesh_from(_ley_arrays(world), "tangents", "side_level", "taper")
	# 頂點著色器會把網格搬到球面上，包圍盒以整張地圖加星球為準。
	var reach := map_size.x * 2.0
	var bounds := AABB(Vector3(-reach, -reach, -reach), Vector3.ONE * reach * 2.0 + Vector3(map_size.x, 0.0, map_size.y))
	for i in COPIES:
		_terrain_copies.append(_add_copy("Terrain%d" % i, terrain_mesh, _terrain_material, bounds))
		_river_copies.append(_add_copy("River%d" % i, river_mesh, _river_material, bounds))
		_old_river_copies.append(_add_copy("RiverOld%d" % i, null, _river_material, bounds))
		_ley_copies.append(_add_copy("Ley%d" % i, ley_mesh, _ley_material, bounds))
	_karma = KarmaLayer.new()
	_karma.name = "Karma"
	add_child(_karma)
	_karma.setup(self, world)
	_karma.set_unit(_ui_unit)
	_shown_map = world.map_revision()
	_shown_terrain = world.terrain_revision()
	_fade_from = _fade_to
	_apply_fade(1.0)
	recenter(map_size.x * 0.5)
	set_mode(mode)


## 世界在背景重算好高度場，或一世結算後州界與靈氣變了，就換上新的貼圖與線；地形網格不必重建。
## 高度場換上後約 3 秒內由舊的漸變到新的，暫停時間時照樣走完。
func _process(delta: float) -> void:
	if _world == null or not _world.is_ready():
		return
	if _fade < 1.0:
		_apply_fade(minf(_fade + delta / TERRAIN_FADE_SECONDS, 1.0))
	var wash_target := 1.0 if mode == Mode.KARMA else 0.0
	if _ink_wash != wash_target:
		_ink_wash = move_toward(_ink_wash, wash_target, delta / INK_WASH_SECONDS)
		_terrain_material.set_shader_parameter("ink_wash", smoothstep(0.0, 1.0, _ink_wash))
	_world.poll_terrain()
	if _world.map_revision() == _shown_map:
		return
	var terrain := _world.terrain_revision() != _shown_terrain
	_shown_map = _world.map_revision()
	_shown_terrain = _world.terrain_revision()
	if terrain:
		# 上一次漸變還沒走完又來一批：沒走完的部分直接到位，從這一批重新開始。
		_apply_fade(1.0)
		_fade_from = _fade_to
		for key in RELIEF_TEXTURES:
			_terrain_material.set_shader_parameter(key + "_prev_tex", _terrain_material.get_shader_parameter(key + "_tex"))
		_set_relief_textures(_terrain_material, _world, _world.relief_textures())
		var river_mesh := _mesh_from(_world.river_mesh(), "tangents", "side_level", "taper")
		for i in COPIES:
			_old_river_copies[i].mesh = _river_copies[i].mesh
			_old_river_copies[i].visible = true
			_river_copies[i].mesh = river_mesh
		_apply_fade(0.0)
	_set_data_textures(_terrain_material, _world, _world.data_textures())
	var ley_mesh := _mesh_from(_ley_arrays(_world), "tangents", "side_level", "taper")
	for copy in _ley_copies:
		copy.mesh = ley_mesh
	set_mode(mode)
	refresh_karma()
	refreshed.emit(terrain)


## 漸變走到 t：貼圖的內插比例、跟著地形變的三個數值、新舊河流的濃淡。走完後放掉舊貼圖與舊河流。
func _apply_fade(t: float) -> void:
	_fade = t
	var eased := smoothstep(0.0, 1.0, t)
	var now := _fade_from.lerp(_fade_to, eased)
	_top_height = now.z
	_terrain_material.set_shader_parameter("terrain_blend", eased)
	_terrain_material.set_shader_parameter("sea_level", now.x)
	_terrain_material.set_shader_parameter("max_elevation", now.y)
	for i in _river_copies.size():
		_river_copies[i].set_instance_shader_parameter("fade", eased)
		_old_river_copies[i].set_instance_shader_parameter("fade", 1.0 - eased)
	if t < 1.0:
		return
	for key in RELIEF_TEXTURES:
		_terrain_material.set_shader_parameter(key + "_prev_tex", _terrain_material.get_shader_parameter(key + "_tex"))
	for copy in _old_river_copies:
		copy.visible = false
		copy.mesh = null


## 把三份副本排在鏡頭焦點所在的那一份與其東西兩側。
func recenter(focus_x: float) -> void:
	var base := floorf(focus_x / map_size.x)
	for i in COPIES:
		var offset := (base + float(i - 1)) * map_size.x
		for copy: MeshInstance3D in [_terrain_copies[i], _river_copies[i], _old_river_copies[i], _ley_copies[i]]:
			copy.position.x = offset
			copy.set_instance_shader_parameter("copy_offset", offset)
		if _karma != null:
			_karma.set_copy_offset(i, offset)


func set_mode(new_mode: Mode) -> void:
	mode = new_mode
	_terrain_material.set_shader_parameter("map_mode", int(mode))
	_terrain_material.set_shader_parameter("tint_opacity", TINT_OPACITY if mode in [Mode.TIANXIA, Mode.SANJIAO] else 0.0)
	match mode:
		Mode.TIANXIA:
			_terrain_material.set_shader_parameter("pcol_tex", _province_colors)
		Mode.SANJIAO:
			refresh_faith()
		_:
			_terrain_material.set_shader_parameter("pcol_tex", _blank_colors)
	# GDD 5.3「河流」設色：山河、天下用 water 色；靈脈等以場為主的模式改用 ink-light。
	var line := WATER.darkened(0.25) if mode != Mode.LINGMAI else INK_LIGHT
	if mode == Mode.KARMA:
		line = wash_color(line)
	_river_material.set_shader_parameter("line_color", _lin(line))
	for copy in _ley_copies:
		copy.visible = mode == Mode.LINGMAI
	# 因果模式的轉墨由 _process 漸變（進入走向 1、離開走回 0）；魂光與因果線圖層隨模式顯示或全部隱藏。
	if _karma != null:
		_karma.set_active(mode == Mode.KARMA)


## 三教模式：重新讀取各州郡的主教化著色表（GDD 5.4 F3）。教化每年變動；其他模式不動作。
func refresh_faith() -> void:
	if mode != Mode.SANJIAO:
		return
	var data := _world.faith_colors()
	if _faith_colors == null:
		_faith_colors = _texture(256, 1, Image.FORMAT_RGBA8, data, false)
	else:
		_faith_colors.update(Image.create_from_data(256, 1, false, Image.FORMAT_RGBA8, data))
	_terrain_material.set_shader_parameter("pcol_tex", _faith_colors)


## 三教模式的宗門勢力範圍圈（GDD 5.4）：data 每圈 (圈心 x、y，半徑，虛線段數)，單位是格；colors 是各圈顏色（sRGB）。
## 虛線由地形著色器畫，width 是線寬像素，dash 是每段佔一段加一個間隔的比例。
func set_range_rings(data: PackedVector4Array, colors: PackedVector3Array, width: float, dash: float, alpha: float) -> void:
	var count := data.size()
	for i in count:
		var linear := Color(colors[i].x, colors[i].y, colors[i].z).srgb_to_linear()
		colors[i] = Vector3(linear.r, linear.g, linear.b)
	data.resize(RING_MAX)
	colors.resize(RING_MAX)
	_ring_count = count
	var material := _terrain_material
	material.set_shader_parameter("ring_count", count)
	material.set_shader_parameter("ring_data", data)
	material.set_shader_parameter("ring_colors", colors)
	material.set_shader_parameter("ring_width", width)
	material.set_shader_parameter("ring_dash", dash)
	material.set_shader_parameter("ring_alpha", alpha)


func clear_range_rings() -> void:
	if _ring_count == 0:
		return
	_ring_count = 0
	_terrain_material.set_shader_parameter("ring_count", 0)


## 目前交給著色器畫的範圍圈數。
func range_ring_count() -> int:
	return _ring_count


## 州郡疊色的不透明度（天下、三教模式；其他為 0）。
func tint_opacity() -> float:
	return _terrain_material.get_shader_parameter("tint_opacity")


func set_hover(province: int) -> void:
	_terrain_material.set_shader_parameter("hover_province", float(province))


func set_selected(province: int) -> void:
	_terrain_material.set_shader_parameter("selected_province", float(province))


## 每幀由鏡頭呼叫：依鏡頭高度、焦點與朝向更新高度的消失點、河流分級與星球過渡。
## globe 為真時過渡到星球視角，否則過渡回平面。
func update_view(z: float, focus: Vector2, camera_basis: Basis, delta: float, globe: bool) -> void:
	var target := 1.0 if globe else 0.0
	globe_morph = move_toward(globe_morph, target, delta / GLOBE_MORPH_SECONDS)
	var eased := smoothstep(0.0, 1.0, globe_morph)
	globe_xform = MapView.globe_frame(focus, map_size, camera_basis)
	_view_focus = focus
	var back := Vector2(camera_basis.z.x, camera_basis.z.z)
	_toward = back.normalized() if back.length() > 1e-4 else Vector2(0.0, 1.0)
	_eye = MapView.eye_height(z, _top_height)
	_globe_relief = MapView.globe_relief(z, map_size.x)
	var radius := MapView.globe_radius(map_size.x)
	var shaded: Array[ShaderMaterial] = [_terrain_material, _river_material, _ley_material]
	shaded.append_array(_karma.materials())
	for material: ShaderMaterial in shaded:
		material.set_shader_parameter("vanish_focus", _view_focus)
		material.set_shader_parameter("vanish_toward", _toward / tan(MapView.pitch()))
		material.set_shader_parameter("vanish_eye", _eye)
		material.set_shader_parameter("globe_relief", _globe_relief)
		material.set_shader_parameter("globe_morph", eased)
		material.set_shader_parameter("globe_radius", radius)
		material.set_shader_parameter("globe_top", _top_height)
		material.set_shader_parameter("globe_xform", globe_xform)
	_terrain_material.set_shader_parameter("fog_start", z * 3.0)
	_terrain_material.set_shader_parameter("fog_end", z * 7.0)
	_river_material.set_shader_parameter("level_alpha", MapView.river_alpha(z, map_size.x, eased))
	recenter(focus.x)


## 世界座標 (x, z) 的地表高度。
func surface_height(x: float, z: float) -> float:
	return _world.terrain_height_at(x, z)


## 平面地圖上，地面位置 (x, pos.y)、地圖座標 pos 的地表點畫在哪裡（含高度消失點的位移）。
## known_height 給定時不再查地形高度（大量取點、高度已快取時用）。
func shown_position(x: float, pos: Vector2, known_height := NAN) -> Vector3:
	var h := surface_height(pos.x, pos.y) if is_nan(known_height) else known_height
	var shown := MapView.lean(Vector2(x, pos.y), h, _view_focus, _toward, _eye)
	return Vector3(shown.x, h, shown.y)


## shown_position 的反算：畫在世界座標 world 的點，地面位置（x、z）在哪裡。
func ground_position(world: Vector3) -> Vector2:
	return MapView.unlean(Vector2(world.x, world.z), world.y, _view_focus, _toward, _eye)


## 地圖座標 → 世界座標：平面取離 near_x 最近的東西副本並貼地；星球視角換算到球面。
## known_height 給定時不再查地形高度。
func world_position(pos: Vector2, near_x: float, known_height := NAN) -> Vector3:
	if globe_morph > 0.5:
		var lon := pos.x / map_size.x * TAU
		var lat := (0.5 - pos.y / map_size.y) * PI
		var height := surface_height(pos.x, pos.y) if is_nan(known_height) else known_height
		var lift := MapView.globe_compress(height, _top_height) * _globe_relief
		return globe_xform * (MapView.sphere_dir(lon, lat) * (MapView.globe_radius(map_size.x) + lift))
	return shown_position(MapView.nearest_copy_x(pos.x, near_x, map_size.x), pos, known_height)


## 該地圖座標是否朝向鏡頭：平面視角恆為真；星球視角只有面向鏡頭的半球為真。
func faces_camera(pos: Vector2, camera_position: Vector3) -> bool:
	if globe_morph <= 0.5:
		return true
	var at := world_position(pos, 0.0)
	return (at - globe_xform.origin).normalized().dot((camera_position - at).normalized()) > 0.25


## 射線與地圖求交，回傳地圖座標（x、z）；未命中回傳 Vector2.INF。
## 平面視角沿射線步進比對地形高度；星球視角與球面求交。
## 平面地圖的高度有消失點：畫面上落在同一點的地表，都在「眼點到該點地面位置」的連線上，所以改沿這條線步進。
func pick(origin: Vector3, dir: Vector3) -> Vector2:
	if globe_morph > 0.5:
		return MapView.ray_globe(globe_xform, MapView.globe_radius(map_size.x), map_size, origin, dir)
	if dir.y >= -1e-4:
		return Vector2.INF
	if _eye > 0.0:
		var seen := origin - dir * (origin.y / dir.y)
		origin = MapView.eye_position(_view_focus, _toward, _eye)
		dir = (Vector3(seen.x, 0.0, seen.z) - origin).normalized()
	var top := _top_height
	var t := maxf((top - origin.y) / dir.y, 0.0)
	var t_end := -origin.y / dir.y
	var step := maxf((t_end - t) / 96.0, 0.05)
	var prev_t := t
	while t <= t_end + step:
		var p := origin + dir * t
		if p.y <= surface_height(p.x, p.z):
			# 在前一步與這一步之間二分逼近。
			var lo := prev_t
			var hi := t
			for i in 8:
				var mid := (lo + hi) * 0.5
				var q := origin + dir * mid
				if q.y <= surface_height(q.x, q.z):
					hi = mid
				else:
					lo = mid
			var hit := origin + dir * hi
			return Vector2(hit.x, hit.z)
		prev_t = t
		t += step
	var ground := origin + dir * t_end
	return Vector2(ground.x, ground.z)


## 地圖座標所在的州郡 ID；水域或圖外為 -1。
## 可以鎖定的目標（地圖座標）：各州郡的治所。M2 起加入人物與建築。
func lock_targets() -> PackedVector2Array:
	var out := PackedVector2Array()
	if mode == Mode.KARMA:
		return _karma.glow_positions()
	for id in _world.province_count():
		out.append(_world.province_info(id)["seat"])
	return out


## 因果模式（GDD 5.4）的公開接點。

## 高亮一組人物與宿緣（事件卡的溯因視圖用）：只畫這些宿緣的線，這些人以外的魂光淡去；focus 給定時另標選取環與前世所在地點。
## 只在因果模式畫出；不在因果模式時先記著，切進來時生效。
func highlight_karma(people: PackedInt32Array, threads: PackedInt32Array, focus := -1) -> void:
	_karma.highlight(people, threads, focus)


## 選取一位的魂光：高亮他的魂牽著的全部未了宿緣與線另一端的人，並標出前世所在地點。
func select_karma_soul(person: int) -> void:
	_karma.select_soul(person)


func clear_karma_highlight() -> void:
	_karma.clear_highlight()


## 推進時間或換圖後重讀因果資料（只在因果模式有動作）。
func refresh_karma() -> void:
	if _karma != null:
		_karma.refresh()


## 因果模式下螢幕點 screen 最近的魂光所屬的人物編號；沒有為 -1。
func karma_soul_at(camera: MapCamera, screen: Vector2) -> int:
	return _karma.soul_at(camera, screen) if mode == Mode.KARMA else -1


func karma_layer() -> KarmaLayer:
	return _karma


## 介面單位改變時（Hud 重排）通知圖層，線寬與魂光大小跟著放大。
func set_ui_unit(unit: float) -> void:
	_ui_unit = unit
	if _karma != null:
		_karma.set_unit(unit)


## 目前轉墨的程度（0 原樣、1 全轉墨）。
func ink_wash() -> float:
	return _ink_wash


## 把顏色換算成因果模式的墨色調（GDD 5.3：亮度 × 0.35、彩度 × 0.2）。
static func wash_color(color: Color) -> Color:
	var luma := color.get_luminance()
	return Color(Color(luma, luma, luma).lerp(color, WASH_SATURATION) * WASH_LIGHT, color.a)


func province_at(map_pos: Vector2) -> int:
	if map_pos == Vector2.INF:
		return -1
	return _world.province_at(map_pos.x, map_pos.y)


func _add_copy(node_name: String, mesh: ArrayMesh, material: ShaderMaterial, bounds: AABB) -> MeshInstance3D:
	var instance := MeshInstance3D.new()
	instance.name = node_name
	instance.mesh = mesh
	instance.material_override = material
	instance.custom_aabb = bounds
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(instance)
	return instance


func _mesh_from(data: Dictionary, normal_key: String, uv_key: String, uv2_key: String) -> ArrayMesh:
	var mesh := ArrayMesh.new()
	if (data["positions"] as PackedVector3Array).is_empty():
		return mesh
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = data["positions"]
	arrays[Mesh.ARRAY_TEX_UV] = data[uv_key]
	if not normal_key.is_empty():
		arrays[Mesh.ARRAY_NORMAL] = data[normal_key]
	if not uv2_key.is_empty():
		arrays[Mesh.ARRAY_TEX_UV2] = data[uv2_key]
	arrays[Mesh.ARRAY_INDEX] = data["indices"]
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


func _make_terrain_material(world: SimWorld) -> ShaderMaterial:
	var grid := world.grid_size()
	var material := ShaderMaterial.new()
	material.shader = TERRAIN_SHADER
	material.set_shader_parameter("paper", _lin(PAPER))
	material.set_shader_parameter("ink", _lin(INK))
	material.set_shader_parameter("ink_light", _lin(INK_LIGHT))
	material.set_shader_parameter("gold", _lin(GOLD))
	material.set_shader_parameter("cyan", _lin(CYAN))
	material.set_shader_parameter("coast_line", _lin(COAST_LINE))
	material.set_shader_parameter("sea_colors", _lin_array(SEA_RAMP))
	material.set_shader_parameter("sand_color", _lin(SAND))
	material.set_shader_parameter("rock_colors", _lin_array(ROCK_RAMP))
	material.set_shader_parameter("map_size", Vector2(grid))
	material.set_shader_parameter("paper_tex", _texture(512, 512, Image.FORMAT_RGBA8, SimWorld.paper_texture(), true))
	material.set_shader_parameter("water_tex", water_texture())
	_set_relief_textures(material, world, world.relief_textures())
	_set_data_textures(material, world, world.data_textures())
	var blank := PackedByteArray()
	blank.resize(256 * 4)
	_blank_colors = _texture(256, 1, Image.FORMAT_RGBA8, blank, false)
	return material


## 跟著高度場換的參數：高度、湖面、植被底色、地表覆蓋。
func _set_relief_textures(material: ShaderMaterial, world: SimWorld, textures: Dictionary) -> void:
	var grid := world.grid_size()
	_fade_to = Vector3(textures["sea_level"], world.terrain_max_height() / (grid.x * 0.04), world.terrain_max_height())
	# 高度：16 位元半精度（可線性取樣），多級縮圖由橋接層在背景算好——頂點高度與水深取平滑後的級別。
	var elevation_size: Vector2i = textures["elevation_size"]
	var elevation := Image.create_from_data(
		elevation_size.x, elevation_size.y, true, Image.FORMAT_RH, textures["elevation_half"]
	)
	material.set_shader_parameter("elevation_tex", ImageTexture.create_from_image(elevation))
	material.set_shader_parameter(
		"lake_tex", _texture(elevation_size.x, elevation_size.y, Image.FORMAT_R8, textures["lake"], false)
	)
	var climate_size: Vector2i = textures["climate_size"]
	material.set_shader_parameter(
		"climate_tex", _texture(climate_size.x, climate_size.y, Image.FORMAT_RGB8, textures["climate"], false)
	)
	material.set_shader_parameter(
		"cover_tex", _texture(climate_size.x, climate_size.y, Image.FORMAT_RG8, textures["cover"], false)
	)


## 每世結算後換的參數：靈氣場、州郡 ID 與州郡的淡彩。
func _set_data_textures(material: ShaderMaterial, world: SimWorld, textures: Dictionary) -> void:
	var grid := world.grid_size()
	var aura_scale: int = textures["aura_scale"]
	material.set_shader_parameter(
		"field_tex", _texture(grid.x * aura_scale, grid.y * aura_scale, Image.FORMAT_R8, textures["aura"], false)
	)
	material.set_shader_parameter("pid_tex", _texture(grid.x, grid.y, Image.FORMAT_RG8, textures["province_id"], false))
	_province_colors = _texture(256, 1, Image.FORMAT_RGBA8, world.province_colors(), false)


func _make_line_material() -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = RIVER_SHADER
	material.set_shader_parameter("map_size", map_size)
	material.render_priority = 1
	return material


## 靈脈線（GDD 5.4 靈脈模式）：沿路徑建立與河流相同格式的帶狀網格，等級依強度（福地 1、洞天 2、大洞天 3）。
func _ley_arrays(world: SimWorld) -> Dictionary:
	var positions := PackedVector3Array()
	var tangents := PackedVector3Array()
	var side_level := PackedVector2Array()
	var taper := PackedVector2Array()
	var indices := PackedInt32Array()
	for line: Dictionary in world.ley_lines():
		var path: PackedVector2Array = line["path"]
		if path.size() < 2:
			continue
		var level := clampf(roundf((line["strength"] as float) * 4.0 - 1.0), 1.0, 3.0)
		var points := PackedVector3Array()
		var x := path[0].x
		for p in path:
			# 路徑跨過東西接縫時接到最近的副本，避免橫越整張地圖。
			x = MapView.nearest_copy_x(p.x, x, map_size.x)
			points.append(Vector3(x, world.terrain_height_at(p.x, p.y), p.y))
		var base := positions.size()
		for i in points.size():
			var tangent := (points[mini(i + 1, points.size() - 1)] - points[maxi(i - 1, 0)]).normalized()
			for side: float in [-1.0, 1.0]:
				positions.append(points[i])
				tangents.append(tangent)
				side_level.append(Vector2(side, level))
				taper.append(Vector2(1.0, 0.0))
		for i in points.size() - 1:
			var a := base + i * 2
			indices.append_array([a, a + 1, a + 2, a + 1, a + 3, a + 2])
	return {"positions": positions, "tangents": tangents, "side_level": side_level, "taper": taper, "indices": indices}


## 水面的雜訊貼圖（GDD 5.3「水」）：R 是波紋（脊狀雜訊的脊線），G 是切出泡沫形狀的雜訊；四邊接得上，可以平鋪。
## 與世界無關，整個程式只做一次。
static func water_texture() -> ImageTexture:
	if _water_texture != null:
		return _water_texture
	var wave := FastNoiseLite.new()
	wave.seed = 7
	wave.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	wave.fractal_type = FastNoiseLite.FRACTAL_RIDGED
	wave.fractal_octaves = 2
	wave.frequency = 4.0 / WATER_TEXTURE_SIZE
	var foam := FastNoiseLite.new()
	foam.seed = 11
	foam.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
	foam.fractal_type = FastNoiseLite.FRACTAL_FBM
	foam.fractal_octaves = 3
	foam.frequency = 10.0 / WATER_TEXTURE_SIZE
	var waves := wave.get_seamless_image(WATER_TEXTURE_SIZE, WATER_TEXTURE_SIZE).get_data()
	var foams := foam.get_seamless_image(WATER_TEXTURE_SIZE, WATER_TEXTURE_SIZE).get_data()
	var data := PackedByteArray()
	data.resize(waves.size() * 2)
	for i in waves.size():
		# 脊狀雜訊的脊線取三次方，收成細長的波線。
		data[i * 2] = int(255.0 * pow(waves[i] / 255.0, 3.0))
		data[i * 2 + 1] = foams[i]
	_water_texture = _texture(WATER_TEXTURE_SIZE, WATER_TEXTURE_SIZE, Image.FORMAT_RG8, data, true)
	return _water_texture


static func _texture(width: int, height: int, format: Image.Format, data: PackedByteArray, mipmaps: bool) -> ImageTexture:
	var image := Image.create_from_data(width, height, false, format, data)
	if mipmaps:
		image.generate_mipmaps()
	return ImageTexture.create_from_image(image)


static func _lin(color: Color) -> Vector3:
	var linear := color.srgb_to_linear()
	return Vector3(linear.r, linear.g, linear.b)


static func _lin_array(colors: Array[Color]) -> PackedVector3Array:
	var out := PackedVector3Array()
	for color in colors:
		out.append(_lin(color))
	return out
