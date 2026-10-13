extends Node
## 標題畫面（GDD 9.1）：背景是一幅淡墨山水（TitleBackdrop），山形取自一個世界的地形——有存檔時是最近那份存檔的世界，
## 沒有時是隨機種子的世界；每次開啟由 SceneFraming 挑一塊山、平地、水比例相近、主山在中間偏左的地方，
## 取由近到遠幾道山稜畫成一層層的山影。左下角崖頂松下三人對弈論道，其餘的人散在山水裡各做各的事（TitleGathering），
## 天上另有一人御劍飛過。右上是標誌，底下接主選單。選好或讀出世界後換到遊戲場景。

## 背景世界的種子與尺寸：種子為 0 時依存檔或亂數決定；測試會改成固定值。
@export var backdrop_seed := 0
@export var backdrop_size := "small"
## 取景用的亂數種子；0 表示每次不同。測試與截圖會固定。
@export var framing_roll := 0

var deck: MenuDeck
var logo: TitleLogo
var flyer: SwordFlyer
var gathering: TitleGathering
var backdrop: TitleBackdrop

var _world: SimWorld
var _focus := Vector2.ZERO


func _ready() -> void:
	InputActions.register()
	GameSettings.apply_display(get_window())
	var paper := Image.create_from_data(512, 512, false, Image.FORMAT_RGBA8, SimWorld.paper_texture())
	Ink.paper_texture = ImageTexture.create_from_image(paper)

	var layer := CanvasLayer.new()
	add_child(layer)
	backdrop = TitleBackdrop.new()
	layer.add_child(backdrop)
	_world = SimWorld.new()
	var error := _create_backdrop_world()
	if error.is_empty():
		_frame()
	else:
		push_error("標題畫面的背景世界生成失敗：%s" % error)
	flyer = SwordFlyer.new()
	layer.add_child(flyer)
	gathering = TitleGathering.new()
	layer.add_child(gathering)
	if framing_roll == 0:
		var seeds: Array[int] = []
		for i in TitleGathering.roster().size():
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


## 挑一塊地方，取它由近到遠的幾道山稜交給背景；取的地比畫面涵蓋的寬，山頭才多。
func _frame() -> void:
	var bounds := get_viewport().get_visible_rect().size
	var size := Vector2(_world.grid_size())
	var z := size.x * SceneFraming.VIEW_Z_FRAC
	var rng := RandomNumberGenerator.new()
	if framing_roll != 0:
		rng.seed = framing_roll
	else:
		rng.randomize()
	var framing := SceneFraming.new(_world)
	var extent := SceneFraming.ground_extent(z, bounds.x / bounds.y)
	_focus = framing.pick(extent, MapView.focus_row_limits(size, z), rng)
	backdrop.ridges = framing.ridges(_focus, extent * TitleBackdrop.WIDER, TitleBackdrop.LAYERS, TitleBackdrop.COLUMNS)


## 選單排版前：標誌依畫面大小排好，選單接在它下面。
func _arrange(bounds: Vector2, unit: float) -> void:
	logo.arrange(bounds, unit)
	deck.panel_zoom = logo.menu_zoom()
	deck.panel_origin = logo.menu_origin()


func _unhandled_key_input(event: InputEvent) -> void:
	AppFlow.screenshot_on_f12(event, get_viewport())
