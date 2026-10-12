extends "res://tests/harness/suite.gd"
## GDD 5.2、5.3 鏡頭高度決定的畫面參數（MapView 純函式）。

const MAP_W := 384.0
const MAP_SIZE := Vector2(384.0, 192.0)


func suite_name() -> String:
	return "map_view"


func test_gdd_5_2_pitch_is_sixty_degrees_and_zoom_only_changes_scale() -> void:
	assert_true(is_equal_approx(rad_to_deg(MapView.pitch()), 60.0))
	assert_true(is_equal_approx(MapView.view_height(MAP_W * 0.6) / MapView.view_height(MAP_W * 0.2), 3.0), "可見範圍與鏡頭高度成正比")


func test_gdd_5_2_eye_height_follows_zoom_but_stays_above_the_peaks() -> void:
	var top := 4.0
	assert_true(is_equal_approx(MapView.eye_height(200.0, top) / MapView.eye_height(100.0, top), 2.0), "眼高與鏡頭高度成正比")
	assert_eq(MapView.eye_height(1.0, top), top * MapView.EYE_MIN_TOPS, "拉到最近時眼點不低於最高峰的固定倍數")


func test_gdd_5_2_heights_lean_toward_one_vanishing_point() -> void:
	var focus := Vector2(100.0, 80.0)
	var toward := Vector2(0.0, 1.0)
	var eye := 50.0
	var slope := 1.0 / tan(MapView.pitch())
	var nadir := MapView.eye_position(focus, toward, eye)
	assert_eq(MapView.lean(Vector2(130.0, 60.0), 0.0, focus, toward, eye), Vector2(130.0, 60.0), "地面不動")
	for ground: Vector2 in [Vector2(130.0, 60.0), Vector2(70.0, 95.0), focus]:
		for h: float in [2.0, 6.0, 12.0]:
			var shown := MapView.lean(ground, h, focus, toward, eye)
			# 畫面上等同於地面上的這一點：正交鏡頭把高度 h 畫成往遠處移 h／tan(俯角)。
			var seen := shown - toward * h * slope
			var from_nadir := seen - Vector2(nadir.x, nadir.z)
			var base := ground - Vector2(nadir.x, nadir.z)
			assert_true(absf(from_nadir.cross(base)) < 1e-2, "山頂落在消失點與山腳的連線上：%s" % from_nadir.cross(base))
			assert_true(from_nadir.length() > base.length(), "越高離消失點越遠")
			assert_true(MapView.unlean(shown, h, focus, toward, eye).distance_to(ground) < 1e-3, "可以換算回地面位置")


func test_gdd_5_3_globe_relief_grows_as_the_camera_pulls_back() -> void:
	assert_eq(MapView.globe_relief(MAP_W * 0.04, MAP_W), MapView.GLOBE_RELIEF_NEAR)
	assert_eq(MapView.globe_relief(MAP_W * 0.6, MAP_W), MapView.GLOBE_RELIEF_FAR)
	assert_eq(MapView.globe_relief(MAP_W * 1.1, MAP_W), MapView.GLOBE_RELIEF_FAR)
	assert_true(MapView.globe_relief(MAP_W * 0.2, MAP_W) < MapView.globe_relief(MAP_W * 0.4, MAP_W))
	assert_true(MapView.GLOBE_RELIEF_NEAR < MapView.GLOBE_RELIEF_FAR)


func test_gdd_5_3_globe_compress_keeps_lowlands_and_squeezes_peaks() -> void:
	var top := 40.0
	assert_eq(MapView.globe_compress(0.0, top), 0.0)
	assert_true(absf(MapView.globe_compress(0.4, top) - 0.4) < 0.01)
	assert_true(MapView.globe_compress(top, top) < top * 0.5)
	var prev := 0.0
	for i in range(1, 11):
		var h := MapView.globe_compress(top * i / 10.0, top)
		assert_true(h > prev)
		prev = h


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
	var rows := MapView.focus_row_limits(MAP_SIZE, MAP_W * 1.0, true)
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
