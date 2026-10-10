@tool
extends McpTestSuite
## GDD 6.4 橋接：SimWorld 各方法回傳的型別與尺寸，以及同種子的確定性。

const SEED := 20261011

var _world: SimWorld


func suite_name() -> String:
	return "sim_bridge"


func _small_world() -> SimWorld:
	if _world == null:
		_world = SimWorld.new()
		var error := _world.create(SEED, "small")
		assert_eq(error, "", "小尺寸世界應能生成")
	return _world


func test_gdd_6_4_unready_world_returns_empty_values() -> void:
	var world := SimWorld.new()
	assert_false(world.is_ready())
	assert_eq(world.grid_size(), Vector2i.ZERO)
	assert_eq(world.province_at(1.0, 1.0), -1)
	assert_true(world.terrain_mesh().is_empty())


func test_gdd_5_3_mesh_arrays_are_consistent() -> void:
	var mesh := _small_world().terrain_mesh()
	var count: int = (mesh["positions"] as PackedVector3Array).size()
	assert_gt(count, 0)
	assert_eq((mesh["normals"] as PackedVector3Array).size(), count)
	assert_eq((mesh["uvs"] as PackedVector2Array).size(), count)
	assert_eq((mesh["curvature"] as PackedVector2Array).size(), count)
	assert_eq((mesh["indices"] as PackedInt32Array).size() % 3, 0)


func test_gdd_5_3_texture_sizes_follow_the_grid() -> void:
	var world := _small_world()
	var grid := world.grid_size()
	var textures := world.map_textures()
	var texels: int = textures["scale"]
	assert_eq((textures["terrain"] as PackedByteArray).size(), grid.x * texels * grid.y * texels * 4)
	assert_eq((textures["aura"] as PackedByteArray).size(), grid.x * texels * grid.y * texels)
	assert_eq((textures["cls_a"] as PackedByteArray).size(), grid.x * grid.y * 4)
	assert_eq((textures["province_id"] as PackedByteArray).size(), grid.x * grid.y * 2)
	assert_eq(world.province_colors().size(), 256 * 4)
	assert_eq(SimWorld.paper_texture().size(), 512 * 512 * 4)
	assert_eq(SimWorld.stroke_texture().size(), 256 * 256)


func test_gdd_5_3_river_mesh_carries_side_and_level() -> void:
	var rivers := _small_world().river_mesh()
	var side_level: PackedVector2Array = rivers["side_level"]
	assert_gt(side_level.size(), 0, "世界應有河流")
	for entry in side_level:
		assert_true(absf(entry.x) == 1.0 and entry.y >= 1.0 and entry.y <= 3.0)


func test_gdd_2_3_province_seat_lies_in_its_own_province() -> void:
	var world := _small_world()
	assert_gt(world.province_count(), 0)
	for id in world.province_count():
		var seat: Vector2 = world.province_info(id)["seat"]
		assert_eq(world.province_at(seat.x, seat.y), id)


func test_gdd_5_3_labels_cover_every_province() -> void:
	var world := _small_world()
	var provinces := 0
	for label: Dictionary in world.map_labels():
		assert_false((label["text"] as String).is_empty())
		if label["kind"] == "province":
			provinces += 1
	assert_eq(provinces, world.province_count())


func test_gdd_6_5_same_seed_gives_same_hash() -> void:
	var other := SimWorld.new()
	other.create(SEED, "small")
	var world := _small_world()
	other.step(maxi(int(world.tick() - other.tick()), 0))
	world.step(maxi(int(other.tick() - world.tick()), 0))
	assert_eq(other.hash_hex(), world.hash_hex())
	world.step(12)
	other.step(12)
	assert_eq(other.hash_hex(), world.hash_hex(), "各推進 12 個月後仍相同")
