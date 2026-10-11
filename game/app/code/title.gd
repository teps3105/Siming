extends Node
## 標題畫面（GDD 9.1）：一顆慢轉的小星球，旁邊是主選單。選好或讀出世界後換到遊戲場景。

## 星球自轉一圈的秒數。
const SPIN_SECONDS := 180.0
## 鏡頭高度（地圖寬的比例）與焦點所在的列（地圖高的比例，略偏北）。
const VIEW_Z_FRAC := 0.95
const FOCUS_ROW_FRAC := 0.42
## 星球在畫面上往左讓開的距離（星球半徑的倍數），右邊留給選單。
const GLOBE_SHIFT := 1.3
## 選單紙面中心在畫面上的位置（比例）。
const PANEL_ANCHOR := Vector2(0.73, 0.5)

## 背景星球的種子與尺寸；測試會改成固定值。
@export var backdrop_seed := 0
@export var backdrop_size := "small"

var deck: MenuDeck

var _map: TerrainMap
var _camera: Camera3D
var _focus := Vector2.ZERO


func _ready() -> void:
	InputActions.register()
	GameSettings.apply_display(get_window())
	var world := SimWorld.new()
	var error := world.create(backdrop_seed if backdrop_seed != 0 else randi(), backdrop_size)
	if error.is_empty():
		_map = TerrainMap.new()
		add_child(_map)
		_map.build(world)
		_map.globe_morph = 1.0
		_camera = Camera3D.new()
		_camera.fov = MapView.FOV_DEG
		_camera.near = 0.5
		_camera.far = _map.map_size.x * 8.0
		add_child(_camera)
		_camera.current = true
		_focus = Vector2(_map.map_size.x * 0.5, _map.map_size.y * FOCUS_ROW_FRAC)
		_turn(0.0)
	else:
		push_error("背景星球生成失敗：%s" % error)
	var paper := Image.create_from_data(512, 512, false, Image.FORMAT_RGBA8, SimWorld.paper_texture())
	Ink.paper_texture = ImageTexture.create_from_image(paper)

	var layer := CanvasLayer.new()
	add_child(layer)
	deck = MenuDeck.new()
	deck.panel_anchor = PANEL_ANCHOR
	layer.add_child(deck)
	deck.world_ready.connect(func(ready_world: SimWorld, loaded: bool) -> void: AppFlow.enter_game(self, ready_world, loaded))
	var cursor := GameCursor.new()
	layer.add_child(cursor)
	cursor.setup(null, 1.0)


func _unhandled_key_input(event: InputEvent) -> void:
	AppFlow.screenshot_on_f12(event, get_viewport())


func _process(delta: float) -> void:
	if _map != null:
		_turn(delta)


## 星球往東轉 delta 秒，鏡頭擺在星球視角的位置（與 MapCamera 同一套算式）。
func _turn(delta: float) -> void:
	var width := _map.map_size.x
	_focus.x += width * delta / SPIN_SECONDS
	var z := width * VIEW_Z_FRAC
	var tilt := MapView.pitch(z, width)
	var target := Vector3(_focus.x, 0.0, _focus.y)
	var away := Vector3(0.0, z, z / tan(tilt)).normalized()
	_camera.global_position = target + away * MapView.camera_distance(z, width, 1.0)
	_camera.look_at(target, Vector3.UP)
	_camera.h_offset = MapView.globe_radius(width) * GLOBE_SHIFT
	_map.update_view(z, _focus, _camera.global_basis, delta, true)
