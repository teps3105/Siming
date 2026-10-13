class_name KarmaLayer
extends Node3D
## GDD 5.4 因果模式的圖層：魂光（每位在世有名之人一點）與因果線（未了的宿緣，二次貝茲弧）。
## 畫法：兩組 MultiMesh（線、魂光），各一個繪製呼叫、三份東西副本共用同一組資料；每個實例的資料寫在變換矩陣裡，
## 弧線與像素寬度全在頂點著色器算（karma_line.gdshader、karma_glow.gdshader），CPU 每幀只更新幾個參數。
## 資料由 SimWorld.karma_view() 一次取回，在推進之後與進入模式時重寫緩衝。

const LINE_SHADER := preload("res://modules/map/code/karma_line.gdshader")
const GLOW_SHADER := preload("res://modules/map/code/karma_glow.gdshader")
const COPIES := 3
## 線的網格段數：弧線以這麼多段折線近似。
const LINE_SEGMENTS := 24
## GDD 6.8：因果模式最多繪製的線數與魂光數。
const MAX_LINES := 1500
const MAX_GLOWS := 4000
## 魂光緩衝裡除了魂光之外，還留給選取環與前世所在地點的位置。
const MARK_SLOTS := 64
## GDD 5.1：五種宿緣的顏色，依 ThreadKind 的順序（殺業、恩義、情緣、師徒、怨）。
const KIND_COLORS: Array[Color] = [Color("B23A2E"), Color("C9A246"), Color("D48A9A"), Color("4F8F9A"), Color("5B3A6B")]
## 五種宿緣的字串鍵（kind_<key> 為名稱）。
const KIND_KEYS: Array[String] = ["feud", "grace", "love", "mentor", "grudge"]
## 點魂光時，滑鼠離魂光中心多近算點到（像素，乘介面單位）。
const PICK_RADIUS_PX := 16.0
const GOLD := Color("C9A246")
const RED := Color("B23A2E")
const PALE := Color("E8DFC6")
const CASING := Color("F3EBD8")

var _world: SimWorld
var _map: TerrainMap
var _line_material: ShaderMaterial
var _glow_material: ShaderMaterial
var _line_multimesh: MultiMesh
var _glow_multimesh: MultiMesh
var _line_nodes: Array[MultiMeshInstance3D] = []
var _glow_nodes: Array[MultiMeshInstance3D] = []
var _active := false
var _unit := 1.0
## 圖層選項「全部」：畫全部的線（預設只畫強度夠的線，GDD 5.4）。
var show_all_lines := false
## 目前畫著的內容（取自最近一次 karma_view）。
var _glow_person := PackedInt32Array()
var _glow_pos := PackedVector2Array()
var _glow_height := PackedFloat32Array()
var _glow_karma := PackedFloat32Array()
var _line_threads := PackedInt32Array()
var _line_debtor := PackedInt32Array()
var _line_creditor := PackedInt32Array()
var _line_count := 0
var _qualified_lines := 0
var _total_glows := 0
## 高亮：非空時只畫這些宿緣的線，這些人以外的魂光淡去。
var _hl_people := PackedInt32Array()
var _hl_threads := PackedInt32Array()
var _hl_focus := -1
var _hl_active := false
var _focus_past := PackedVector2Array()
var _focus_past_height := PackedFloat32Array()
var _mark_count := 0


func setup(map: TerrainMap, world: SimWorld) -> void:
	_map = map
	_world = world
	_line_multimesh = _make_multimesh(_make_line_mesh(), MAX_LINES)
	_glow_multimesh = _make_multimesh(_make_quad_mesh(), MAX_GLOWS + MARK_SLOTS)
	_line_material = ShaderMaterial.new()
	_line_material.shader = LINE_SHADER
	_line_material.render_priority = 3
	_line_material.set_shader_parameter("map_size", map.map_size)
	var colors := PackedVector3Array()
	for color in KIND_COLORS:
		var linear := color.srgb_to_linear()
		colors.append(Vector3(linear.r, linear.g, linear.b))
	_line_material.set_shader_parameter("kind_colors", colors)
	_line_material.set_shader_parameter("casing_color", _lin(CASING))
	_glow_material = ShaderMaterial.new()
	_glow_material.shader = GLOW_SHADER
	_glow_material.render_priority = 4
	_glow_material.set_shader_parameter("map_size", map.map_size)
	_glow_material.set_shader_parameter("gold", _lin(GOLD))
	_glow_material.set_shader_parameter("red", _lin(RED))
	_glow_material.set_shader_parameter("pale", _lin(PALE))
	var reach := map.map_size.x * 2.0
	var bounds := AABB(Vector3(-reach, -reach, -reach), Vector3.ONE * reach * 2.0 + Vector3(map.map_size.x, 0.0, map.map_size.y))
	for i in COPIES:
		_line_nodes.append(_add_node("KarmaLines%d" % i, _line_multimesh, _line_material, bounds))
		_glow_nodes.append(_add_node("KarmaGlows%d" % i, _glow_multimesh, _glow_material, bounds))
	set_active(false)


func materials() -> Array[ShaderMaterial]:
	return [_line_material, _glow_material]


## 介面單位（Ink.layout_unit）：線寬、魂光半徑與流動速度乘它。
func set_unit(unit: float) -> void:
	_unit = unit
	for material in materials():
		material.set_shader_parameter("px_scale", unit)


func set_copy_offset(index: int, offset: float) -> void:
	_line_nodes[index].set_instance_shader_parameter("copy_offset", offset)
	_glow_nodes[index].set_instance_shader_parameter("copy_offset", offset)


func is_active() -> bool:
	return _active


## 進入或離開因果模式：離開後三份副本全隱藏，不留任何東西。
func set_active(on: bool) -> void:
	_active = on
	for node in _line_nodes + _glow_nodes:
		node.visible = on
	if on:
		refresh()


## 重新讀取魂光與因果線並寫進緩衝（推進時間之後、進入模式時）。
func refresh() -> void:
	if not _active or _world == null:
		return
	if _hl_active:
		_refresh_highlight()
		return
	var view := _world.karma_view({"all": show_all_lines, "max_lines": MAX_LINES, "max_souls": MAX_GLOWS})
	if view.is_empty():
		_line_count = 0
		_glow_person = PackedInt32Array()
		_write_glows(1)
		_write_lines({}, 0)
		return
	_total_glows = view["total_glows"]
	_qualified_lines = view["qualified_lines"]
	_set_glows(view)
	_write_lines(view, 0)
	_write_glows(0)


func _refresh_highlight() -> void:
	var lines := _world.karma_threads(_hl_threads)
	var view := _world.karma_view({"max_lines": 0, "max_souls": MAX_GLOWS})
	if not view.is_empty():
		_total_glows = view["total_glows"]
		_set_glows(view)
	_write_lines(lines, 1)
	_write_glows(0)


func _set_glows(view: Dictionary) -> void:
	_glow_person = view["glow_person"]
	_glow_pos = view["glow_pos"]
	_glow_height = view["glow_height"]
	_glow_karma = view["glow_karma"]


## 魂光的數目與因果線的數目（實際畫出的）。
func glow_count() -> int:
	return _glow_person.size()


func line_count() -> int:
	return _line_count


## 取捨前符合條件的線數與在世有名之人數（超過上限時兩者比畫出的多）。
func qualified_lines() -> int:
	return _qualified_lines


func total_glows() -> int:
	return _total_glows


## 畫出的魂光：人物編號與地圖座標。
func glow_people() -> PackedInt32Array:
	return _glow_person


func glow_positions() -> PackedVector2Array:
	return _glow_pos


## 畫出的線兩端的人物編號（不在世為 -1）。
func line_ends() -> Array[PackedInt32Array]:
	return [_line_debtor, _line_creditor]


func line_threads() -> PackedInt32Array:
	return _line_threads


func _write_lines(data: Dictionary, state: int) -> void:
	var count := 0
	var buffer := PackedFloat32Array()
	buffer.resize(MAX_LINES * 12)
	if not data.is_empty() and data.has("line_thread"):
		count = mini((data["line_thread"] as PackedInt32Array).size(), MAX_LINES)
		var from: PackedVector2Array = data["line_from"]
		var to: PackedVector2Array = data["line_to"]
		var from_h: PackedFloat32Array = data["line_from_height"]
		var to_h: PackedFloat32Array = data["line_to_height"]
		var strength: PackedFloat32Array = data["line_strength"]
		var kind: PackedInt32Array = data["line_kind"]
		var dead: PackedInt32Array = data["line_dead"]
		_line_threads = data["line_thread"]
		_line_debtor = data["line_debtor"]
		_line_creditor = data["line_creditor"]
		for i in count:
			var o := i * 12
			# 第 0 欄 (ax, az, bx)、第 1 欄 (bz, 欠債方高, 債主高)、第 2 欄 (強度, 種類 + 8 × 不在世端, 狀態)；
			# 變換矩陣以列為序：[xx yx zx ox, xy yy zy oy, xz yz zz oz]。
			buffer[o] = from[i].x
			buffer[o + 1] = to[i].y
			buffer[o + 2] = strength[i]
			buffer[o + 4] = from[i].y
			buffer[o + 5] = from_h[i]
			buffer[o + 6] = float(kind[i] + 8 * dead[i])
			buffer[o + 8] = to[i].x
			buffer[o + 9] = to_h[i]
			buffer[o + 10] = float(state)
	else:
		_line_threads = PackedInt32Array()
		_line_debtor = PackedInt32Array()
		_line_creditor = PackedInt32Array()
	_line_count = count
	_line_multimesh.buffer = buffer
	_line_multimesh.visible_instance_count = count


func _write_glows(_unused: int) -> void:
	var buffer := PackedFloat32Array()
	buffer.resize((MAX_GLOWS + MARK_SLOTS) * 12)
	var count := mini(_glow_person.size(), MAX_GLOWS)
	var lit := {}
	for id in _hl_people:
		lit[id] = true
	for i in count:
		var state := 0
		if _hl_active:
			state = 1 if lit.has(_glow_person[i]) else 2
		_put_glow(buffer, i, _glow_pos[i], _glow_height[i], _glow_karma[i], 0, state)
	var used := count
	_mark_count = 0
	if _hl_active and _hl_focus >= 0:
		var at := _glow_person.find(_hl_focus)
		if at >= 0:
			_put_glow(buffer, used, _glow_pos[at], _glow_height[at], 0.0, 1, 1)
			used += 1
			_mark_count += 1
		for i in mini(_focus_past.size(), MARK_SLOTS - 1):
			_put_glow(buffer, used, _focus_past[i], _focus_past_height[i], 0.0, 2, 1)
			used += 1
			_mark_count += 1
	_glow_multimesh.buffer = buffer
	_glow_multimesh.visible_instance_count = used


## 一個魂光（或標記）寫進緩衝：第 0 欄 (x, z, 地表高)、第 1 欄 (淨業力, 種類, 狀態)。
static func _put_glow(buffer: PackedFloat32Array, index: int, pos: Vector2, height: float, karma: float, kind: int, state: int) -> void:
	var o := index * 12
	buffer[o] = pos.x
	buffer[o + 1] = karma
	buffer[o + 4] = pos.y
	buffer[o + 5] = float(kind)
	buffer[o + 8] = height
	buffer[o + 9] = float(state)


# ── 高亮（選取魂光、事件卡的溯因視圖）──────────────────────────────

## 高亮一組人物與宿緣：只畫這些宿緣的線，這些人以外的魂光淡去。focus 給定時另畫選取環並以虛線圈標出前世所在地點。
func highlight(people: PackedInt32Array, threads: PackedInt32Array, focus := -1) -> void:
	_hl_people = people
	_hl_threads = threads
	_hl_focus = focus
	_hl_active = true
	_focus_past = PackedVector2Array()
	_focus_past_height = PackedFloat32Array()
	if focus >= 0:
		var info := _world.karma_focus(focus)
		if not info.is_empty():
			_focus_past = info["past"]
			_focus_past_height = info["past_height"]
	refresh()


## 選取一位的魂光：高亮他的魂牽著的全部未了宿緣與線另一端的人。
func select_soul(person: int) -> void:
	var info := _world.karma_focus(person)
	if info.is_empty():
		return
	var threads: PackedInt32Array = info["threads"]
	var people := PackedInt32Array([person])
	var lines := _world.karma_threads(threads)
	if not lines.is_empty():
		for id: int in lines["line_debtor"]:
			if id >= 0 and not people.has(id):
				people.append(id)
		for id: int in lines["line_creditor"]:
			if id >= 0 and not people.has(id):
				people.append(id)
	highlight(people, threads, person)


func clear_highlight() -> void:
	if not _hl_active:
		return
	_hl_active = false
	_hl_people = PackedInt32Array()
	_hl_threads = PackedInt32Array()
	_hl_focus = -1
	_focus_past = PackedVector2Array()
	_focus_past_height = PackedFloat32Array()
	refresh()


func is_highlighting() -> bool:
	return _hl_active


func highlighted_people() -> PackedInt32Array:
	return _hl_people


func highlighted_threads() -> PackedInt32Array:
	return _hl_threads


## 選取環與前世所在地點的標記數（選取了一位時 1 + 前世地點數）。
func mark_count() -> int:
	return _mark_count


# ── 點選 ────────────────────────────────────────────────────────

## 螢幕上離 screen 最近、且在 PICK_RADIUS_PX 之內的魂光所屬的人物編號；沒有為 -1。魂光密集處取最近的一個。
func soul_at(camera: MapCamera, screen: Vector2) -> int:
	var best := -1
	var best_d := INF
	var limit := PICK_RADIUS_PX * _unit
	for i in _glow_person.size():
		var p := _glow_pos[i]
		var world := _map.world_position(p, camera.focus.x, _glow_height[i])
		if camera.is_position_behind(world) or not _map.faces_camera(p, camera.global_position):
			continue
		var d := camera.unproject_position(world).distance_to(screen)
		if d <= limit and d < best_d:
			best_d = d
			best = _glow_person[i]
	return best


## 某人魂光的地圖座標；沒有畫出為 Vector2.INF。
func glow_position_of(person: int) -> Vector2:
	var at := _glow_person.find(person)
	return _glow_pos[at] if at >= 0 else Vector2.INF


# ── 建立 ────────────────────────────────────────────────────────

func _add_node(node_name: String, multimesh: MultiMesh, material: ShaderMaterial, bounds: AABB) -> MultiMeshInstance3D:
	var node := MultiMeshInstance3D.new()
	node.name = node_name
	node.multimesh = multimesh
	node.material_override = material
	node.custom_aabb = bounds
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(node)
	return node


func _make_multimesh(mesh: ArrayMesh, count: int) -> MultiMesh:
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.mesh = mesh
	multimesh.instance_count = count
	multimesh.visible_instance_count = 0
	multimesh.custom_aabb = AABB(Vector3(-1.0e5, -1.0e5, -1.0e5), Vector3.ONE * 2.0e5)
	return multimesh


## 線的網格：LINE_SEGMENTS 段的帶，頂點的 UV = (沿線比例, 左右 -1／+1)，位置由著色器算。
func _make_line_mesh() -> ArrayMesh:
	var vertices := PackedVector3Array()
	var uvs := PackedVector2Array()
	var indices := PackedInt32Array()
	for i in LINE_SEGMENTS + 1:
		var t := float(i) / LINE_SEGMENTS
		for side: float in [-1.0, 1.0]:
			vertices.append(Vector3.ZERO)
			uvs.append(Vector2(t, side))
	for i in LINE_SEGMENTS:
		var a := i * 2
		indices.append_array([a, a + 1, a + 2, a + 1, a + 3, a + 2])
	return _mesh(vertices, uvs, indices)


func _make_quad_mesh() -> ArrayMesh:
	var vertices := PackedVector3Array([Vector3.ZERO, Vector3.ZERO, Vector3.ZERO, Vector3.ZERO])
	var uvs := PackedVector2Array([Vector2(0, 0), Vector2(1, 0), Vector2(1, 1), Vector2(0, 1)])
	return _mesh(vertices, uvs, PackedInt32Array([0, 1, 2, 0, 2, 3]))


static func _mesh(vertices: PackedVector3Array, uvs: PackedVector2Array, indices: PackedInt32Array) -> ArrayMesh:
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	mesh.custom_aabb = AABB(Vector3(-1.0e5, -1.0e5, -1.0e5), Vector3.ONE * 2.0e5)
	return mesh


static func _lin(color: Color) -> Vector3:
	var linear := color.srgb_to_linear()
	return Vector3(linear.r, linear.g, linear.b)
