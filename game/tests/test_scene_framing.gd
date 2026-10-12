extends "res://tests/harness/suite.gd"
## GDD 9.1 標題畫面的取景（SceneFraming）：每次取的地方不同，構圖相近——山、平地、水各佔一份，山在遠處。

const ASPECT := 16.0 / 9.0

var _world: SimWorld
var _framing: SceneFraming
var _extent := Vector2.ZERO
var _rows := Vector2.ZERO


func suite_name() -> String:
	return "scene_framing"


func setup() -> void:
	if _world != null:
		return
	_world = SimWorld.new()
	assert_eq(_world.create(TEST_SEED, TEST_SIZE), "", "世界生成")
	_framing = SceneFraming.new(_world)
	var size := Vector2(_world.grid_size())
	var z := size.x * SceneFraming.VIEW_Z_FRAC
	_extent = SceneFraming.ground_extent(z, ASPECT)
	_rows = MapView.focus_row_limits(size, z)


func _pick(rng_seed: int) -> Vector2:
	var rng := RandomNumberGenerator.new()
	rng.seed = rng_seed
	return _framing.pick(_extent, _rows, rng)


func test_gdd_9_1_shares_of_a_frame_add_up_to_one() -> void:
	var shares := _framing.shares(Vector2(_world.grid_size()) * 0.5, _extent)
	assert_true(absf(shares.x + shares.y + shares.z - 1.0) < 1e-4, "山、平地、水合計為一：%s" % shares)


func test_gdd_9_1_every_pick_shows_mountain_plain_and_water() -> void:
	for rng_seed in 8:
		var focus := _pick(rng_seed + 1)
		var shares := _framing.shares(focus, _extent)
		assert_gt(shares.x, 0.12, "有山：%s" % shares)
		assert_gt(shares.y, 0.15, "有平地：%s" % shares)
		assert_gt(shares.z, 0.12, "有水：%s" % shares)
		assert_true(focus.y >= _rows.x and focus.y <= _rows.y, "焦點在鏡頭可到的範圍內")


func test_gdd_9_1_mountains_sit_in_the_far_half() -> void:
	for rng_seed in 8:
		var focus := _pick(rng_seed + 1)
		var bands := _framing.mountain_bands(focus, _extent)
		assert_gt(bands.x, bands.y, "遠處的山比近處多：遠 %s、近 %s" % [bands.x, bands.y])


func test_gdd_9_1_picks_vary_but_repeat_for_the_same_roll() -> void:
	assert_eq(_pick(7), _pick(7), "同一個亂數得到同一個取景")
	var seen := {}
	for rng_seed in 12:
		seen[_pick(rng_seed + 1)] = true
	assert_gt(seen.size(), 2, "不同的亂數取到不同的地方")
