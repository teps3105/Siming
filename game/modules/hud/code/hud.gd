class_name Hud
extends CanvasLayer
## GDD 5.7 介面佈局：把案頭器物放到畫面上並接到模擬、地圖與鏡頭。
## 常駐器物只有天時儀＋題簽（左上）與印匣（右下）；州郡卡與停留提示隨選取、停留出現。

const MARGIN := 16.0
## Steam Deck 版面（高度 ≤ 800）器物放大不改形（GDD 5.9）。
const DECK_HEIGHT := 800.0
const DECK_UNIT := 1.25

var _sim: SimHost
var _map: TerrainMap
var _camera: MapCamera
var _unit := 1.0
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
	sim.advanced.connect(func(_months: int) -> void: _card.refresh())
	get_viewport().size_changed.connect(_layout)
	_layout()


## 依畫面高度決定器物大小；大小改變時重建器物。
func _layout() -> void:
	var bounds := get_viewport().get_visible_rect().size
	var unit := DECK_UNIT if bounds.y <= DECK_HEIGHT else 1.0
	if _dial == null or unit != _unit:
		_unit = unit
		_build()
	_seals.position = bounds - _seals.size - Vector2(MARGIN, MARGIN)
	_card.position = Vector2(bounds.x - _card.size.x - MARGIN, MARGIN + 8.0)


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
	_card.setup(_sim.world, _unit)
	_card.center_requested.connect(func(pos: Vector2) -> void: _camera.look_at_map(pos))
	_card.closed.connect(func() -> void: _map.set_selected(-1))
	_card.show_province(selected)


func _set_mode(mode: TerrainMap.Mode) -> void:
	_map.set_mode(mode)
	_seals.set_mode(mode)


func _unhandled_input(event: InputEvent) -> void:
	if _sim == null:
		return
	if event.is_action_pressed(InputActions.PAUSE):
		_sim.toggle_paused()
	elif event.is_action_pressed(InputActions.SPEED_UP):
		_sim.set_speed(_sim.speed_index + 1)
	elif event.is_action_pressed(InputActions.SPEED_DOWN):
		_sim.set_speed(_sim.speed_index - 1)
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
	else:
		var speeds := [InputActions.SPEED_1, InputActions.SPEED_2, InputActions.SPEED_3, InputActions.SPEED_4]
		for i in speeds.size():
			if event.is_action_pressed(speeds[i]):
				_sim.set_speed(i)
				_sim.set_paused(false)
