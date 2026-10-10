extends "res://tests/harness/suite.gd"
## 流程：開場運鏡——開局先看到整顆星球，鏡頭再落到面積最大的大洲上方（GDD 5.2「開場運鏡」）。

var _game: Node
var _map: TerrainMap
var _camera: MapCamera


func suite_name() -> String:
	return "opening_journey"


func setup() -> void:
	_game = await boot(true)
	_map = _game.get_node("TerrainMap")
	_camera = _game.get_node("MapCamera")


func test_opening_starts_on_the_globe_and_descends_to_the_continent() -> void:
	var width := _map.map_size.x
	assert_gt(_camera.z, width * MapView.GLOBE_Z_FRAC, "第一個畫面是星球")
	assert_true(_camera.is_flying(), "開場運鏡進行中")
	var focus := _camera.focus
	await wait(MapCamera.OPENING_SECONDS * 0.5)
	assert_true(_camera.is_flying(), "運鏡到一半還沒結束")
	assert_lt(_camera.z, width * MapView.Z_MAX_FRAC * 0.98, "鏡頭正在下降")
	await wait(MapCamera.OPENING_SECONDS * 0.5 + 0.8)
	assert_false(_camera.is_flying(), "運鏡結束")
	assert_lt(absf(_camera.z - width * 0.3), width * 0.01, "停在地圖寬 × 0.3 的高度")
	assert_eq(_map.globe_morph, 0.0, "落地後是平面地圖")
	assert_lt(_camera.focus.distance_to(focus), 0.5, "運鏡期間焦點不動")


func test_any_press_skips_the_opening_flight() -> void:
	await wait(0.3)
	assert_true(_camera.is_flying())
	await key(KEY_SHIFT)
	assert_false(_camera.is_flying(), "按任何鍵就結束運鏡")
	await wait(0.8)
	assert_lt(absf(_camera.z - _map.map_size.x * 0.3), _map.map_size.x * 0.01, "直接到開場位置")
