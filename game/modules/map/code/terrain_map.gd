class_name TerrainMap
extends Node3D
## GDD 5.3 地圖：由 SimWorld 的資料建立地形網格、世界貼圖與河流製圖線，
## 並依鏡頭狀態更新著色參數。東西向以三份副本接成無縫。

enum Mode { SHANHE = 0, TIANXIA = 1, LINGMAI = 3 }

const TERRAIN_SHADER := preload("res://modules/map/code/terrain_ink.gdshader")
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
const TERRAIN_COLORS: Array[Color] = [
	Color("7DA4AD"), Color("A8C7C3"), Color("E8DDBC"), Color("C4A073"), Color("B5AE9C"),
	Color("5E8A5A"), Color("F3F1EA"), Color("DDBE7E"), Color("9AA682"),
]
const SEA_RAMP: Array[Color] = [
	Color("D3E0D6"), Color("A8C7C3"), Color("7DA4AD"), Color("5A8296"), Color("3E6379"),
]
const MOUNTAIN_RAMP: Array[Color] = [
	Color("BC9A68"), Color("9DAA80"), Color("7FA39A"), Color("8FA6BA"), Color("EEF0EC"),
]

var map_size := Vector2.ZERO
var mode: Mode = Mode.SHANHE
var globe_morph := 0.0
var globe_xform := Transform3D.IDENTITY

var _world: SimWorld
var _terrain_material: ShaderMaterial
var _river_material: ShaderMaterial
var _terrain_copies: Array[MeshInstance3D] = []
var _river_copies: Array[MeshInstance3D] = []
var _province_colors: ImageTexture
var _blank_colors: ImageTexture
var _height_scale := 1.0


## 以世界資料建立地圖。世界改變地形後再呼叫一次即可重建。
func build(world: SimWorld) -> void:
	_world = world
	for copy in _terrain_copies + _river_copies:
		copy.queue_free()
	_terrain_copies.clear()
	_river_copies.clear()

	var grid := world.grid_size()
	map_size = Vector2(grid)
	_terrain_material = _make_terrain_material(world)
	_river_material = _make_river_material()

	var terrain_mesh := _mesh_from(world.terrain_mesh(), "normals", "uvs", "curvature")
	var river_mesh := _mesh_from(world.river_mesh(), "tangents", "side_level", "taper")
	# 頂點著色器會把網格搬到球面上，包圍盒以整張地圖加星球為準。
	var reach := map_size.x * 2.0
	var bounds := AABB(Vector3(-reach, -reach, -reach), Vector3.ONE * reach * 2.0 + Vector3(map_size.x, 0.0, map_size.y))
	for i in COPIES:
		_terrain_copies.append(_add_copy("Terrain%d" % i, terrain_mesh, _terrain_material, bounds))
		_river_copies.append(_add_copy("River%d" % i, river_mesh, _river_material, bounds))
	recenter(map_size.x * 0.5)
	set_mode(mode)


## 把三份副本排在鏡頭焦點所在的那一份與其東西兩側。
func recenter(focus_x: float) -> void:
	var base := floorf(focus_x / map_size.x)
	for i in COPIES:
		var offset := (base + float(i - 1)) * map_size.x
		for copy: MeshInstance3D in [_terrain_copies[i], _river_copies[i]]:
			copy.position.x = offset
			copy.set_instance_shader_parameter("copy_offset", offset)


func set_mode(new_mode: Mode) -> void:
	mode = new_mode
	_terrain_material.set_shader_parameter("map_mode", int(mode))
	_terrain_material.set_shader_parameter("tint_opacity", 0.45 if mode == Mode.TIANXIA else 0.0)
	_terrain_material.set_shader_parameter("pcol_tex", _province_colors if mode == Mode.TIANXIA else _blank_colors)
	# GDD 5.3「河流」設色：山河、天下用 water 色；靈脈等以場為主的模式改用 ink-light。
	var line := WATER if mode != Mode.LINGMAI else INK_LIGHT
	_river_material.set_shader_parameter("line_color", _lin(line.darkened(0.25)))


func set_hover(province: int) -> void:
	_terrain_material.set_shader_parameter("hover_province", float(province))


func set_selected(province: int) -> void:
	_terrain_material.set_shader_parameter("selected_province", float(province))


## 每幀由鏡頭呼叫：依鏡頭高度、焦點與朝向更新高度縮放、古地圖化、河流分級與星球過渡。
func update_view(z: float, focus: Vector2, camera_basis: Basis, delta: float) -> void:
	var target := MapView.globe_target(z, map_size.x)
	globe_morph = move_toward(globe_morph, target, delta / GLOBE_MORPH_SECONDS)
	var eased := smoothstep(0.0, 1.0, globe_morph)
	globe_xform = MapView.globe_frame(focus, map_size, camera_basis)
	_height_scale = MapView.height_scale(z, map_size.x)
	var radius := MapView.globe_radius(map_size.x)
	for material: ShaderMaterial in [_terrain_material, _river_material]:
		material.set_shader_parameter("height_scale", _height_scale)
		material.set_shader_parameter("globe_morph", eased)
		material.set_shader_parameter("globe_radius", radius)
		material.set_shader_parameter("globe_xform", globe_xform)
	_terrain_material.set_shader_parameter("flatten", MapView.flatten_weight(z, map_size.x))
	_terrain_material.set_shader_parameter("fog_start", z * 3.0)
	_terrain_material.set_shader_parameter("fog_end", z * 7.0)
	_river_material.set_shader_parameter("level_alpha", MapView.river_alpha(z, map_size.x, eased))
	recenter(focus.x)


## 目前的地形高度縮放（選取與標籤取高度時使用）。
func height_scale() -> float:
	return _height_scale


## 世界座標 (x, z) 的地表高度（已含高度縮放）。
func surface_height(x: float, z: float) -> float:
	return _world.terrain_height_at(x, z) * _height_scale


## 射線與地圖求交，回傳地圖座標（x、z）；未命中回傳 Vector2.INF。
## 平面視角沿射線步進比對地形高度；星球視角與球面求交。
func pick(origin: Vector3, dir: Vector3) -> Vector2:
	if globe_morph > 0.5:
		return MapView.ray_globe(globe_xform, MapView.globe_radius(map_size.x), map_size, origin, dir)
	if dir.y >= -1e-4:
		return Vector2.INF
	var top := _world.terrain_max_height() * _height_scale
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
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = data["positions"]
	arrays[Mesh.ARRAY_NORMAL] = data[normal_key]
	arrays[Mesh.ARRAY_TEX_UV] = data[uv_key]
	arrays[Mesh.ARRAY_TEX_UV2] = data[uv2_key]
	arrays[Mesh.ARRAY_INDEX] = data["indices"]
	var mesh := ArrayMesh.new()
	if (data["positions"] as PackedVector3Array).is_empty():
		return mesh
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


func _make_terrain_material(world: SimWorld) -> ShaderMaterial:
	var grid := world.grid_size()
	var textures := world.map_textures()
	var scale: int = textures["scale"]
	var material := ShaderMaterial.new()
	material.shader = TERRAIN_SHADER
	material.set_shader_parameter("paper", _lin(PAPER))
	material.set_shader_parameter("ink", _lin(INK))
	material.set_shader_parameter("ink_light", _lin(INK_LIGHT))
	material.set_shader_parameter("water", _lin(WATER))
	material.set_shader_parameter("gold", _lin(GOLD))
	material.set_shader_parameter("cyan", _lin(CYAN))
	material.set_shader_parameter("terrain_colors", _lin_array(TERRAIN_COLORS))
	material.set_shader_parameter("sea_colors", _lin_array(SEA_RAMP))
	material.set_shader_parameter("mountain_colors", _lin_array(MOUNTAIN_RAMP))
	material.set_shader_parameter("map_size", Vector2(grid))
	material.set_shader_parameter("max_height", world.terrain_max_height())
	material.set_shader_parameter("paper_tex", _texture(512, 512, Image.FORMAT_RGBA8, SimWorld.paper_texture(), true))
	material.set_shader_parameter("stroke_tex", _texture(256, 256, Image.FORMAT_R8, SimWorld.stroke_texture(), true))
	material.set_shader_parameter(
		"terrain_tex", _texture(grid.x * scale, grid.y * scale, Image.FORMAT_RGBA8, textures["terrain"], false)
	)
	material.set_shader_parameter(
		"field_tex", _texture(grid.x * scale, grid.y * scale, Image.FORMAT_R8, textures["aura"], false)
	)
	material.set_shader_parameter("cls_a_tex", _texture(grid.x, grid.y, Image.FORMAT_RGBA8, textures["cls_a"], false))
	material.set_shader_parameter("cls_b_tex", _texture(grid.x, grid.y, Image.FORMAT_RGBA8, textures["cls_b"], false))
	material.set_shader_parameter("pid_tex", _texture(grid.x, grid.y, Image.FORMAT_RG8, textures["province_id"], false))
	_province_colors = _texture(256, 1, Image.FORMAT_RGBA8, world.province_colors(), false)
	var blank := PackedByteArray()
	blank.resize(256 * 4)
	_blank_colors = _texture(256, 1, Image.FORMAT_RGBA8, blank, false)
	return material


func _make_river_material() -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = RIVER_SHADER
	material.set_shader_parameter("map_size", map_size)
	material.render_priority = 1
	return material


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
