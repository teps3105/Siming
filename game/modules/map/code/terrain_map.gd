class_name TerrainMap
extends Node3D
## GDD 5.3 地圖：由 SimWorld 的資料建立地形網格、世界貼圖與河流製圖線，
## 並依鏡頭狀態更新著色參數。東西向以三份副本接成無縫。

enum Mode { SHANHE = 0, TIANXIA = 1, LINGMAI = 3 }

const TERRAIN_SHADER := preload("res://modules/map/code/terrain_relief.gdshader")
const RIVER_SHADER := preload("res://modules/map/code/river_line.gdshader")
const COPIES := 3
## GDD 5.3 過渡時間：平面 ↔ 球面 0.6 秒。
const GLOBE_MORPH_SECONDS := 0.6

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

var map_size := Vector2.ZERO
var mode: Mode = Mode.SHANHE
var globe_morph := 0.0
var globe_xform := Transform3D.IDENTITY

var _world: SimWorld
var _terrain_material: ShaderMaterial
var _river_material: ShaderMaterial
var _terrain_copies: Array[MeshInstance3D] = []
var _river_copies: Array[MeshInstance3D] = []
var _ley_material: ShaderMaterial
var _ley_copies: Array[MeshInstance3D] = []
var _province_colors: ImageTexture
var _blank_colors: ImageTexture
## 高度消失點（MapView.lean）：鏡頭焦點、地面上由焦點指向鏡頭的方向與眼高；眼高為 0 時不位移。
var _view_focus := Vector2.ZERO
var _toward := Vector2(0.0, 1.0)
var _eye := 0.0
var _globe_relief := 1.0
## 全圖最高的地表高度（建圖時取一次；每次查詢要掃整張地圖）。
var _top_height := 1.0


## 以世界資料建立地圖。世界改變地形後再呼叫一次即可重建。
func build(world: SimWorld) -> void:
	_world = world
	_top_height = world.terrain_max_height()
	for copy in _terrain_copies + _river_copies + _ley_copies:
		copy.queue_free()
	_terrain_copies.clear()
	_river_copies.clear()
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
		_ley_copies.append(_add_copy("Ley%d" % i, ley_mesh, _ley_material, bounds))
	recenter(map_size.x * 0.5)
	set_mode(mode)


## 把三份副本排在鏡頭焦點所在的那一份與其東西兩側。
func recenter(focus_x: float) -> void:
	var base := floorf(focus_x / map_size.x)
	for i in COPIES:
		var offset := (base + float(i - 1)) * map_size.x
		for copy: MeshInstance3D in [_terrain_copies[i], _river_copies[i], _ley_copies[i]]:
			copy.position.x = offset
			copy.set_instance_shader_parameter("copy_offset", offset)


func set_mode(new_mode: Mode) -> void:
	mode = new_mode
	_terrain_material.set_shader_parameter("map_mode", int(mode))
	_terrain_material.set_shader_parameter("tint_opacity", 0.45 if mode == Mode.TIANXIA else 0.0)
	_terrain_material.set_shader_parameter("pcol_tex", _province_colors if mode == Mode.TIANXIA else _blank_colors)
	# GDD 5.3「河流」設色：山河、天下用 water 色；靈脈等以場為主的模式改用 ink-light。
	var line := WATER.darkened(0.25) if mode != Mode.LINGMAI else INK_LIGHT
	_river_material.set_shader_parameter("line_color", _lin(line))
	for copy in _ley_copies:
		copy.visible = mode == Mode.LINGMAI


## 當成佈景用（標題畫面）：不畫州界與水紋，整幅收成淡墨，低處起雲霧；veil 是畫面左側另加的霧。
func set_scenery(mist: float, veil: float) -> void:
	_terrain_material.set_shader_parameter("border_opacity", 0.0)
	for material: ShaderMaterial in [_terrain_material, _river_material]:
		material.set_shader_parameter("mist", mist)
		material.set_shader_parameter("mist_veil", veil)


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
	for material: ShaderMaterial in [_terrain_material, _river_material, _ley_material]:
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
func shown_position(x: float, pos: Vector2) -> Vector3:
	var h := surface_height(pos.x, pos.y)
	var shown := MapView.lean(Vector2(x, pos.y), h, _view_focus, _toward, _eye)
	return Vector3(shown.x, h, shown.y)


## shown_position 的反算：畫在世界座標 world 的點，地面位置（x、z）在哪裡。
func ground_position(world: Vector3) -> Vector2:
	return MapView.unlean(Vector2(world.x, world.z), world.y, _view_focus, _toward, _eye)


## 地圖座標 → 世界座標：平面取離 near_x 最近的東西副本並貼地；星球視角換算到球面。
func world_position(pos: Vector2, near_x: float) -> Vector3:
	if globe_morph > 0.5:
		var lon := pos.x / map_size.x * TAU
		var lat := (0.5 - pos.y / map_size.y) * PI
		var height := _world.terrain_height_at(pos.x, pos.y)
		var lift := MapView.globe_compress(height, _top_height) * _globe_relief
		return globe_xform * (MapView.sphere_dir(lon, lat) * (MapView.globe_radius(map_size.x) + lift))
	return shown_position(MapView.nearest_copy_x(pos.x, near_x, map_size.x), pos)


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
	for id in _world.province_count():
		out.append(_world.province_info(id)["seat"])
	return out


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
	var textures := world.map_textures()
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
	material.set_shader_parameter("sea_level", textures["sea_level"])
	material.set_shader_parameter("max_elevation", world.terrain_max_height() / (grid.x * 0.04))

	# 高度：32 位元浮點轉成 16 位元半精度（可線性取樣）。
	var elevation_size: Vector2i = textures["elevation_size"]
	var elevation := Image.create_from_data(
		elevation_size.x, elevation_size.y, false, Image.FORMAT_RF, (textures["elevation"] as PackedFloat32Array).to_byte_array()
	)
	elevation.convert(Image.FORMAT_RH)
	# 多級縮圖：頂點高度與水深取平滑後的級別。
	elevation.generate_mipmaps()
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
	material.set_shader_parameter("paper_tex", _texture(512, 512, Image.FORMAT_RGBA8, SimWorld.paper_texture(), true))
	var aura_scale: int = textures["aura_scale"]
	material.set_shader_parameter(
		"field_tex", _texture(grid.x * aura_scale, grid.y * aura_scale, Image.FORMAT_R8, textures["aura"], false)
	)
	material.set_shader_parameter("pid_tex", _texture(grid.x, grid.y, Image.FORMAT_RG8, textures["province_id"], false))
	_province_colors = _texture(256, 1, Image.FORMAT_RGBA8, world.province_colors(), false)
	var blank := PackedByteArray()
	blank.resize(256 * 4)
	_blank_colors = _texture(256, 1, Image.FORMAT_RGBA8, blank, false)
	return material


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
