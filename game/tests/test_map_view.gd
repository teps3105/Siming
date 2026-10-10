@tool
extends McpTestSuite
## GDD 5.2、5.3 鏡頭高度決定的畫面參數（MapView 純函式）。

const MAP_W := 384.0
const MAP_SIZE := Vector2(384.0, 192.0)


func suite_name() -> String:
	return "map_view"


func test_gdd_5_2_pitch_runs_from_50_to_85_degrees() -> void:
	var limits := MapView.z_range(MAP_W)
	assert_true(is_equal_approx(rad_to_deg(MapView.pitch(limits.x, MAP_W)), 50.0))
	assert_true(is_equal_approx(rad_to_deg(MapView.pitch(limits.y, MAP_W)), 85.0))


func test_gdd_5_2_flatten_starts_above_sixty_percent_of_map_width() -> void:
	assert_eq(MapView.flatten_weight(MAP_W * 0.6, MAP_W), 0.0)
	assert_eq(MapView.height_scale(MAP_W * 0.3, MAP_W), 1.0)
	assert_true(is_equal_approx(MapView.height_scale(MAP_W * 1.1, MAP_W), 0.05))


func test_gdd_5_3_far_view_shows_only_the_largest_rivers() -> void:
	var alpha := MapView.river_alpha(MAP_W * 0.75, MAP_W, 0.0)
	assert_eq(alpha, Vector3(0.0, 0.0, 1.0), "遠景只畫江")


func test_gdd_5_3_mid_view_adds_big_rivers() -> void:
	var alpha := MapView.river_alpha(MAP_W * 0.35, MAP_W, 0.0)
	assert_eq(alpha, Vector3(0.0, 1.0, 1.0), "中景加入大河")


func test_gdd_5_3_near_view_shows_every_river() -> void:
	var alpha := MapView.river_alpha(MAP_W * 0.1, MAP_W, 0.0)
	assert_eq(alpha, Vector3(1.0, 1.0, 1.0), "近景三級都畫")


func test_gdd_5_3_rivers_fade_linearly_around_threshold() -> void:
	var at_threshold := MapView.river_alpha(MAP_W * MapView.RIVER_BIG_Z_FRAC, MAP_W, 0.0)
	assert_true(is_equal_approx(at_threshold.y, 0.5), "門檻處恰為半透明")


func test_gdd_5_3_rivers_hidden_in_globe_view() -> void:
	assert_eq(MapView.river_alpha(MAP_W * 1.05, MAP_W, 1.0).z, 0.0)


func test_gdd_5_2_focus_stays_within_75_degrees_latitude() -> void:
	var rows := MapView.focus_row_limits(MAP_SIZE, MAP_W * 1.0)
	assert_true(is_equal_approx(rows.x, MAP_SIZE.y * 15.0 / 180.0))
	assert_true(is_equal_approx(rows.y, MAP_SIZE.y * 165.0 / 180.0))


func test_gdd_5_3_globe_pick_returns_the_focus_under_the_camera() -> void:
	var focus := Vector2(188.0, 90.0)
	# 俯視鏡頭：看向 -y，畫面上方為北（-z）。
	var camera_basis := Basis(Vector3.RIGHT, -PI * 0.5)
	var frame := MapView.globe_frame(focus, MAP_SIZE, camera_basis)
	var radius := MapView.globe_radius(MAP_W)
	var origin := Vector3(focus.x, 300.0, focus.y)
	var hit := MapView.ray_globe(frame, radius, MAP_SIZE, origin, Vector3.DOWN)
	assert_true(hit.distance_to(focus) < 0.5, "焦點正對鏡頭：%s" % hit)
	var north := MapView.ray_globe(frame, radius, MAP_SIZE, origin + Vector3(0.0, 0.0, -radius * 0.5), Vector3.DOWN)
	assert_true(north.y < hit.y, "畫面上方是北")
	var east := MapView.ray_globe(frame, radius, MAP_SIZE, origin + Vector3(radius * 0.3, 0.0, 0.0), Vector3.DOWN)
	assert_true(east.x > hit.x, "畫面右方是東")


func test_gdd_5_3_nearest_copy_wraps_across_the_seam() -> void:
	assert_eq(MapView.nearest_copy_x(380.0, 2.0, MAP_W), -4.0)
	assert_eq(MapView.nearest_copy_x(10.0, 700.0, MAP_W), 778.0)
