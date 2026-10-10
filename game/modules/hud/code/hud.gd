class_name Hud
extends CanvasLayer
## GDD 5.7 介面佈局：把案頭器物放到畫面上並接到模擬、地圖與鏡頭。
## 常駐器物只有天時儀＋題簽（左上）與印匣（右下）；州郡卡與停留提示隨選取、停留出現。

const MARGIN := 12.0
## Steam Deck 版面（高度 ≤ 800）器物放大不改形（GDD 5.9）。
const DECK_HEIGHT := 800.0
const DECK_UNIT := 8.0 / 7.0
## Steam Deck 版面的命簿是右側抽屜，寬度佔畫面的比例（GDD 5.8）。
const DECK_CARD_WIDTH_FRAC := 0.45

var _sim: SimHost
var _map: TerrainMap
var _camera: MapCamera
var _unit := 1.0
var _card_width := -1.0
var _overlay: MapOverlay
var _dial: TimeDial
var _seals: SealBox
var _card: ProvinceCard


func setup(sim: SimHost, map: TerrainMap, camera: MapCamera) -> void:
	_sim = sim
	_map = map
	_camera = camera
	camera.province_hovered.connect(func(id: int) -> void: _overlay.set_hover(id))
	camera.province_selected.connect(func(id: int) -> void: _card.show_province(id))
	sim.advanced.connect(func(_months: int) -> void:
		_card.refresh()
		_overlay.refresh_seats())
	var paper := Image.create_from_data(512, 512, false, Image.FORMAT_RGBA8, SimWorld.paper_texture())
	Ink.paper_texture = ImageTexture.create_from_image(paper)
	get_viewport().size_changed.connect(_layout)
	_layout()


## 依畫面高度決定器物大小；大小改變時重建器物。
func _layout() -> void:
	var bounds := get_viewport().get_visible_rect().size
	var unit := DECK_UNIT if bounds.y <= DECK_HEIGHT else 1.0
	var card_width := bounds.x * DECK_CARD_WIDTH_FRAC if unit > 1.0 else -1.0
	if _dial == null or unit != _unit or card_width != _card_width:
		_unit = unit
		_card_width = card_width
		_build()
	_seals.position = bounds - _seals.size - Vector2(MARGIN, MARGIN)
	_card.position = Vector2(bounds.x - _card.size.x - MARGIN, MARGIN)


func _build() -> void:
	var selected := _card.province if _card != null else -1
	for child in get_children():
		child.queue_free()

	_overlay = MapOverlay.new()
	add_child(_overlay)
	_overlay.setup(_sim.world, _map, _camera, _unit)

	_dial = TimeDial.new()
	add_child(_dial)
	_dial.setup(_sim, _unit)
	_dial.position = Vector2(MARGIN, MARGIN)

	_seals = SealBox.new()
	add_child(_seals)
	_seals.setup(_unit)
	_seals.set_mode(_map.mode)
	_seals.mode_chosen.connect(_set_mode)

	_card = ProvinceCard.new()
	add_child(_card)
	_card.setup(_sim.world, _unit, _card_width)
	_card.center_requested.connect(func(pos: Vector2) -> void: _camera.look_at_map(pos))
	_card.closed.connect(func() -> void: _map.set_selected(-1))
	_card.show_province(selected)

	# 搖桿游標畫在所有器物之上。
	var cursor := PadCursor.new()
	add_child(cursor)
	cursor.setup(_camera, _unit)
	_apply_device()


## 輸入裝置換了（鍵鼠↔搖桿），器物上的實體鍵圖像跟著換。
func _process(_delta: float) -> void:
	if _camera != null and _dial != null and _camera.use_crosshair != _dial.gamepad:
		_apply_device()


func _apply_device() -> void:
	_dial.gamepad = _camera.use_crosshair
	_seals.gamepad = _camera.use_crosshair
	_card.gamepad = _camera.use_crosshair


func _set_mode(mode: TerrainMap.Mode) -> void:
	_map.set_mode(mode)
	_seals.set_mode(mode)


func _unhandled_input(event: InputEvent) -> void:
	if _sim == null:
		return
	if event.is_action_pressed(InputActions.PAUSE):
		_sim.toggle_paused()
	elif event.is_action_pressed(InputActions.SPEED_CYCLE):
		_sim.set_speed((_sim.speed_index + 1) % SimHost.SPEED_MULTIPLIERS.size())
		_sim.set_paused(false)
	elif event.is_action_pressed(InputActions.MODE_NEXT):
		_seals.cycle(1)
	elif event.is_action_pressed(InputActions.MODE_PREV):
		_seals.cycle(-1)
	elif event.is_action_pressed(InputActions.MODE_SHANHE):
		_set_mode(TerrainMap.Mode.SHANHE)
	elif event.is_action_pressed(InputActions.MODE_TIANXIA):
		_set_mode(TerrainMap.Mode.TIANXIA)
	elif event.is_action_pressed(InputActions.MODE_LINGMAI):
		_set_mode(TerrainMap.Mode.LINGMAI)
	elif event.is_action_pressed(InputActions.CANCEL):
		_card.close()
	elif event.is_action_pressed(InputActions.CARD_CENTER):
		_card.request_center()
	else:
		var speeds := [InputActions.SPEED_1, InputActions.SPEED_2, InputActions.SPEED_3, InputActions.SPEED_4]
		for i in speeds.size():
			if event.is_action_pressed(speeds[i]):
				_sim.set_speed(i)
				_sim.set_paused(false)
