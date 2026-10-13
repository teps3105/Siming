class_name SpectacleKit
extends RefCounted
## 奇觀特效的共用件：粒子網格與材質的組裝、貼圖與雜訊的快取、專案的紙墨朱金配色。
## 粒子是一個網格裡的一批四邊形，運動全在 fx_sprites.gdshader（依奇觀的 age 算位置），這裡只負責建網格、配參數。

const SPRITES := preload("res://modules/spectacle/code/fx/fx_sprites.gdshader")
const SPRITES_ADD := preload("res://modules/spectacle/code/fx/fx_sprites_add.gdshader")
const BEAM := preload("res://modules/spectacle/code/fx/fx_beam.gdshader")
const THREAD := preload("res://modules/spectacle/code/fx/fx_thread.gdshader")
const TEXTURE_PATH := "res://modules/spectacle/assets/textures/%s.png"
## 渲染優先序：因果層用 3、4，奇觀在其上。
const PRIORITY := 6
const THREAD_SEGMENTS := 40
const BEAM_ROWS := 10

## GDD 5.1 的紙、墨、朱、金。
const INK := Color("2B2620")
const INK_LIGHT := Color("6B6152")
const PALE := Color("F3EBD8")
const RED := Color("B23A2E")
const GOLD := Color("C9A246")
const GOLD_LIGHT := Color("E8CF86")
const AMBER := Color("A8741E")

static var _textures := {}
static var _meshes := {}
static var _noise: Texture2D


## 0..1 的偽隨機：同一個種子與序號永遠同一個值。
static func rand(seed_value: int, index: int) -> float:
	return fposmod(sin(float(seed_value) * 12.9898 + float(index) * 78.233) * 43758.5453, 1.0)


static func ease_out(t: float) -> float:
	var u := 1.0 - clampf(t, 0.0, 1.0)
	return 1.0 - u * u * u


static func with_alpha(color: Color, alpha: float) -> Color:
	return Color(color.r, color.g, color.b, color.a * clampf(alpha, 0.0, 1.0))


static func texture(name: String) -> Texture2D:
	if not _textures.has(name):
		_textures[name] = load(TEXTURE_PATH % name)
	return _textures[name]


## 可平鋪的雜訊（光柱的流動紋理），引擎內建的 FastNoiseLite 產生。
static func noise() -> Texture2D:
	if _noise == null:
		var generator := FastNoiseLite.new()
		generator.noise_type = FastNoiseLite.TYPE_SIMPLEX_SMOOTH
		generator.frequency = 0.02
		generator.fractal_octaves = 3
		var image := NoiseTexture2D.new()
		image.noise = generator
		image.width = 128
		image.height = 256
		image.seamless = true
		_noise = image
	return _noise


## count 個四邊形：VERTEX 是 -1..1 的角，UV 是貼圖座標。
static func sprite_mesh(count: int) -> ArrayMesh:
	if _meshes.has(count):
		return _meshes[count]
	var verts := PackedVector3Array()
	var uvs := PackedVector2Array()
	var idx := PackedInt32Array()
	for i in count:
		for corner: Vector2 in [Vector2(-1, -1), Vector2(1, -1), Vector2(1, 1), Vector2(-1, 1)]:
			verts.append(Vector3(corner.x, corner.y, 0.0))
			uvs.append(Vector2((corner.x + 1.0) * 0.5, (1.0 - corner.y) * 0.5))
		idx.append_array([i * 4, i * 4 + 1, i * 4 + 2, i * 4, i * 4 + 2, i * 4 + 3])
	var mesh := _surface(verts, uvs, idx)
	_meshes[count] = mesh
	return mesh


## 光柱的帶：VERTEX.x 是 -1..1、VERTEX.y 是 0..1。
static func beam_mesh() -> ArrayMesh:
	if _meshes.has("beam"):
		return _meshes["beam"]
	var verts := PackedVector3Array()
	var uvs := PackedVector2Array()
	var idx := PackedInt32Array()
	for row in BEAM_ROWS + 1:
		var y := float(row) / BEAM_ROWS
		for x in [-1.0, 1.0]:
			verts.append(Vector3(x, y, 0.0))
			uvs.append(Vector2((x + 1.0) * 0.5, y))
	for row in BEAM_ROWS:
		var k := row * 2
		idx.append_array([k, k + 1, k + 2, k + 1, k + 3, k + 2])
	var mesh := _surface(verts, uvs, idx)
	_meshes["beam"] = mesh
	return mesh


## 了結線的帶：UV.x 沿線 0..1、UV.y 橫向 -1..1，位置全在著色器算。
static func thread_mesh() -> ArrayMesh:
	if _meshes.has("thread"):
		return _meshes["thread"]
	var verts := PackedVector3Array()
	var uvs := PackedVector2Array()
	var idx := PackedInt32Array()
	for i in THREAD_SEGMENTS + 1:
		var f := float(i) / THREAD_SEGMENTS
		for side in [-1.0, 1.0]:
			verts.append(Vector3.ZERO)
			uvs.append(Vector2(f, side))
	for i in THREAD_SEGMENTS:
		var k := i * 2
		idx.append_array([k, k + 1, k + 2, k + 1, k + 3, k + 2])
	var mesh := _surface(verts, uvs, idx)
	_meshes["thread"] = mesh
	return mesh


static func _surface(verts: PackedVector3Array, uvs: PackedVector2Array, idx: PackedInt32Array) -> ArrayMesh:
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = idx
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


## 一批粒子：貼圖、數量與著色器參數（見 fx_sprites.gdshaderinc）。additive 為真用加法混合。
## curve 為真時粒子在世界座標出生（了結的火星），包圍盒放到最大。
static func sprites(parent: Node3D, tex_name: String, count: int, params: Dictionary, additive := false, priority := PRIORITY, curve := false) -> MeshInstance3D:
	var material := ShaderMaterial.new()
	material.shader = SPRITES_ADD if additive else SPRITES
	material.render_priority = priority
	material.set_shader_parameter("tex", texture(tex_name))
	for key: String in params:
		material.set_shader_parameter(key, params[key])
	var node := MeshInstance3D.new()
	node.mesh = sprite_mesh(count)
	node.material_override = material
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	if curve:
		node.custom_aabb = AABB(Vector3(-100000.0, -100000.0, -100000.0), Vector3.ONE * 200000.0)
	else:
		node.custom_aabb = AABB(Vector3(-8.0, -4.0, -8.0), Vector3(16.0, 24.0, 16.0))
	parent.add_child(node)
	return node


static func set_u(node: MeshInstance3D, key: String, value: Variant) -> void:
	(node.material_override as ShaderMaterial).set_shader_parameter(key, value)


static func set_all(nodes: Array, key: String, value: Variant) -> void:
	for node: MeshInstance3D in nodes:
		set_u(node, key, value)
