extends Node
## 標題畫面（GDD 9.1）：整個畫面是一幅現場取景的地圖——有存檔時是最近那份存檔的世界，沒有時是隨機種子的世界；
## 每次開啟由 SceneFraming 挑一塊山、平地、水比例相近的地方，不畫地名與州界，雲霧在低處流動，
## 那是退在後面的淡墨山水；主體是崖頂松下論道對弈的幾名修士（TitleGathering），天上另有一人御劍飛過。
## 左上是標誌，底下接主選單。選好或讀出世界後換到遊戲場景。

## 雲霧的濃度，與畫面左側（題字與選單後面）另加的霧。
const MIST := 1.0
const MIST_VEIL := 0.9

## 背景世界的種子與尺寸：種子為 0 時依存檔或亂數決定；測試會改成固定值。
@export var backdrop_seed := 0
@export var backdrop_size := "small"
## 取景用的亂數種子；0 表示每次不同。測試與截圖會固定。
@export var framing_roll := 0

var deck: MenuDeck
var logo: TitleLogo
var flyer: SwordFlyer
var gathering: TitleGathering

var _world: SimWorld
var _map: TerrainMap
var _camera: Camera3D
var _framing: SceneFraming
var _focus := Vector2.ZERO


func _ready() -> void:
	InputActions.register()
	GameSettings.apply_display(get_window())
	_world = SimWorld.new()
	var error := _create_backdrop_world()
	if error.is_empty():
		_map = TerrainMap.new()
		add_child(_map)
		_map.build(_world)
		_map.set_scenery(MIST, MIST_VEIL)
		_camera = Camera3D.new()
		_camera.projection = Camera3D.PROJECTION_ORTHOGONAL
		_camera.keep_aspect = Camera3D.KEEP_HEIGHT
		_camera.near = 0.5
		_camera.far = _map.map_size.x * 8.0
		add_child(_camera)
		_camera.current = true
		_framing = SceneFraming.new(_world)
		_frame()
	else:
		push_error("標題畫面的背景世界生成失敗：%s" % error)
	var paper := Image.create_from_data(512, 512, false, Image.FORMAT_RGBA8, SimWorld.paper_texture())
	Ink.paper_texture = ImageTexture.create_from_image(paper)

	var layer := CanvasLayer.new()
	add_child(layer)
	flyer = SwordFlyer.new()
	layer.add_child(flyer)
	gathering = TitleGathering.new()
	layer.add_child(gathering)
	if framing_roll == 0:
		var seeds: Array[int] = []
		for i in TitleGathering.CAST.size():
			seeds.append(randi())
		gathering.seeds = seeds
		flyer.figure_seed = randi()
	logo = TitleLogo.new()
	layer.add_child(logo)
	deck = MenuDeck.new()
	deck.arrange = _arrange
	layer.add_child(deck)
	deck.world_ready.connect(func(ready_world: SimWorld, loaded: bool) -> void: AppFlow.enter_game(self, ready_world, loaded))
	var cursor := GameCursor.new()
	layer.add_child(cursor)
	cursor.setup(null, 1.0)


## 背景世界的開局選項（種子、尺寸等）。
func backdrop_options() -> Dictionary:
	return _world.world_options()


## 目前取景的焦點（地圖座標）。
func backdrop_focus() -> Vector2:
	return _focus


## 有存檔時用最近那份存檔的開局選項（同一個世界的地形），沒有時用亂數種子。
func _create_backdrop_world() -> String:
	if backdrop_seed != 0:
		return _world.create(backdrop_seed, backdrop_size)
	var options: Dictionary = SaveStore.latest().get("options", {})
	if not options.is_empty():
		return _world.create_with(options)
	return _world.create(randi(), backdrop_size)


## 挑一塊地方，把鏡頭擺上去（與 MapCamera 同一套算式）。
func _frame() -> void:
	var bounds := get_viewport().get_visible_rect().size
	var width := _map.map_size.x
	var z := width * SceneFraming.VIEW_Z_FRAC
	var rng := RandomNumberGenerator.new()
	if framing_roll != 0:
		rng.seed = framing_roll
	else:
		rng.randomize()
	_focus = _framing.pick(SceneFraming.ground_extent(z, bounds.x / bounds.y), MapView.focus_row_limits(_map.map_size, z), rng)
	var tilt := MapView.pitch()
	var target := Vector3(_focus.x, 0.0, _focus.y)
	_camera.size = MapView.view_height(z)
	_camera.global_position = target + Vector3(0.0, sin(tilt), cos(tilt)) * MapView.camera_distance(z, width, 0.0)
	_camera.look_at(target, Vector3.UP)
	_map.update_view(z, _focus, _camera.global_basis, 0.0, false)


## 選單排版前：標誌依畫面大小排好，選單接在它下面。
func _arrange(bounds: Vector2, unit: float) -> void:
	logo.arrange(bounds, unit)
	deck.panel_zoom = logo.menu_zoom()
	deck.panel_origin = logo.menu_origin()


func _unhandled_key_input(event: InputEvent) -> void:
	AppFlow.screenshot_on_f12(event, get_viewport())
