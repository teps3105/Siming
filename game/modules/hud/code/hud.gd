class_name Hud
extends CanvasLayer
## GDD 5.7 介面佈局：把案頭器物放到畫面上並接到模擬、地圖與鏡頭。
## 常駐器物只有天時儀＋題簽與視角鈕（左上）、印匣（右下）；州郡卡與停留提示隨選取、停留出現。
## 遊戲中選單（GDD 9.1）開著時時間暫停、地圖不收輸入，關掉後恢復原本的行止。

## 在遊戲中選單裡選了回主選單。
signal title_requested
## 在遊戲中選單裡讀出了另一個世界。
signal world_loaded(world: SimWorld)

const MARGIN := 12.0
const GUIDE_GAP := 24.0
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
var _view: ViewToggle
var _cursor: GameCursor
var _guide: GuideSlip
var _notice: NoticeSlip
var _menu: MenuDeck
## 是否顯示引導簽（測試會關掉）；在 setup 之前設定。
var guide_enabled := true
## 開選單前時間是否已經暫停。
var _paused_before_menu := false
## 開關選單造成的暫停與續行不算玩家動了時間。
var _menu_switching := false


func setup(sim: SimHost, map: TerrainMap, camera: MapCamera) -> void:
	_sim = sim
	_map = map
	_camera = camera
	camera.province_hovered.connect(func(id: int) -> void: _overlay.set_hover(id))
	camera.province_selected.connect(func(id: int) -> void:
		_card.show_province(id)
		if id >= 0:
			_guide.notify(&"select"))
	camera.globe_changed.connect(func(on: bool) -> void:
		_view.globe = on
		_guide.notify(&"globe"))
	sim.time_state_changed.connect(func() -> void:
		if not _menu_switching:
			_guide.notify(&"time"))
	sim.world_changed.connect(func(change: String, era: String) -> void: _notice.post(_world_notice(change, era)))
	sim.world_created.connect(func() -> void:
		if _notice != null:
			_notice.clear())
	sim.advanced.connect(func(_months: int) -> void:
		_card.refresh()
		_overlay.refresh_seats()
		_overlay.refresh_sects()
		_map.refresh_faith())
	# 地圖換貼圖的那一幀已經有上傳的工作，標籤留到下一幀再重讀。
	map.refreshed.connect(func(_terrain: bool) -> void:
		await get_tree().process_frame
		_overlay.reload()
		_card.refresh())
	var paper := Image.create_from_data(512, 512, false, Image.FORMAT_RGBA8, SimWorld.paper_texture())
	Ink.paper_texture = ImageTexture.create_from_image(paper)
	get_viewport().size_changed.connect(_layout)
	_layout()


## 依畫面高度決定器物大小；大小改變時重建器物。
## GDD 7.2 世界大事的那句話；開啟紀元的加上改元。
static func _world_notice(change: String, era: String) -> String:
	var event := Locale.t("world_" + change)
	return event if era.is_empty() else Locale.t("world_new_era").format({"event": event, "era": era})


func _layout() -> void:
	var bounds := get_viewport().get_visible_rect().size
	var unit := Ink.layout_unit(bounds, GameSettings.ui_scale())
	var card_width := bounds.x * DECK_CARD_WIDTH_FRAC if bounds.y <= Ink.DECK_HEIGHT else -1.0
	if _dial == null or unit != _unit or card_width != _card_width:
		_unit = unit
		_card_width = card_width
		_build()
	_seals.position = bounds - _seals.size - Vector2(MARGIN, MARGIN)
	_place_guide()
	_card.position = Vector2(bounds.x - _card.size.x - MARGIN, MARGIN)
	_notice.top_center = Vector2(bounds.x * 0.5, MARGIN)


func _build() -> void:
	var selected := _card.province if _card != null else -1
	var notices: Array[String] = []
	if _notice != null:
		notices = _notice.pending()
	for child in get_children():
		if child != _menu:
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

	_view = ViewToggle.new()
	add_child(_view)
	_view.setup(_unit)
	_view.globe = _camera.globe
	_view.toggled.connect(func() -> void: _camera.toggle_globe())
	var under_dial := maxf(_dial.outer_radius() * 2.0, _dial._speed_rect().end.y + Ink.SLIP_BAND)
	_view.position = _dial.position + Vector2(0.0, under_dial + 8.0 * _unit)

	_guide = GuideSlip.new()
	add_child(_guide)
	_guide.resized.connect(_place_guide)
	_guide.setup(_camera, _map.map_size.x, _unit, guide_enabled)

	_notice = NoticeSlip.new()
	add_child(_notice)
	_notice.setup(_unit)
	for line in notices:
		_notice.post(line)

	# 遊戲內游標畫在所有器物之上。
	_cursor = GameCursor.new()
	add_child(_cursor)
	_cursor.setup(_camera, _unit)
	if _menu != null:
		# 選單蓋在器物之上、游標之下。
		move_child(_menu, -1)
		move_child(_cursor, -1)
	_apply_device()


## 引導簽在底部中央、印鈕列上方 24 px（GDD 5.7）；畫面窄到會壓住印匣的例簽時往左讓開。
func _place_guide() -> void:
	if _guide == null or _seals == null:
		return
	var bounds := get_viewport().get_visible_rect().size
	var seals_top := _seals.position.y + _seals._badge_position(0).y
	var x := minf((bounds.x - _guide.size.x) * 0.5, _seals.position.x - GUIDE_GAP - _guide.size.x)
	_guide.position = Vector2(maxf(x, MARGIN), seals_top - GUIDE_GAP - _guide.size.y).round()


func guide() -> GuideSlip:
	return _guide


## 選單是否開著。
func menu_open() -> bool:
	return _menu != null


func menu() -> MenuDeck:
	return _menu


## 開遊戲中選單：時間暫停，地圖不收輸入。
func open_menu() -> void:
	if _menu != null or _sim == null:
		return
	_paused_before_menu = _sim.paused
	_menu_switching = true
	_sim.set_paused(true)
	_menu_switching = false
	_camera.input_enabled = false
	_menu = MenuDeck.new()
	_menu.in_game = true
	_menu.world = _sim.world
	add_child(_menu)
	move_child(_cursor, -1)
	_menu.panel().gamepad = _camera.use_crosshair
	_menu.resumed.connect(close_menu)
	_menu.layout_changed.connect(_layout)
	_menu.guide_reset.connect(func() -> void: _guide.setup(_camera, _map.map_size.x, _unit, guide_enabled))
	_menu.title_requested.connect(func() -> void: title_requested.emit())
	_menu.world_ready.connect(func(world: SimWorld, _loaded: bool) -> void: world_loaded.emit(world))


## 關選單：時間回到開選單前的行止。
func close_menu() -> void:
	if _menu == null:
		return
	_camera.use_crosshair = _menu.panel().gamepad
	_menu.queue_free()
	_menu = null
	_camera.input_enabled = true
	_menu_switching = true
	_sim.set_paused(_paused_before_menu)
	_menu_switching = false


## 輸入裝置換了（鍵鼠↔搖桿），器物上的實體鍵圖像跟著換。
func _process(_delta: float) -> void:
	if _camera == null or _dial == null:
		return
	if _menu != null:
		_camera.use_crosshair = _menu.panel().gamepad
	if _camera.use_crosshair != _dial.gamepad:
		_apply_device()
	_overlay.reserved = _occupied()


## 器物目前佔住的畫面範圍（地圖標籤要避開）。
func _occupied() -> Array[Rect2]:
	var disc := Rect2(_dial.position, Vector2.ONE * _dial.outer_radius() * 2.0)
	var slips := _dial._title_rect().merge(_dial._speed_rect())
	slips.position += _dial.position
	var seals := _seals._seal_rect(0).merge(_seals._seal_rect(SealBox.SEALS.size() - 1))
	seals.position += _seals.position
	var out: Array[Rect2] = [disc.grow(6.0), slips.grow(6.0), seals.grow(6.0)]
	if _card.visible:
		out.append(Rect2(_card.position, _card.size).grow(6.0))
	if _guide.visible:
		out.append(Rect2(_guide.position, _guide.size).grow(6.0))
	out.append(Rect2(_view.position, _view.size).grow(6.0))
	return out


func _apply_device() -> void:
	_dial.gamepad = _camera.use_crosshair
	_seals.gamepad = _camera.use_crosshair
	_card.gamepad = _camera.use_crosshair
	_view.gamepad = _camera.use_crosshair
	_guide.gamepad = _camera.use_crosshair


func _set_mode(mode: TerrainMap.Mode) -> void:
	if mode != _map.mode:
		_guide.notify(&"mode")
	_map.set_mode(mode)
	_seals.set_mode(mode)


func _unhandled_input(event: InputEvent) -> void:
	if _sim == null or _menu != null:
		return
	if event.is_action_pressed(InputActions.MENU) or (event is InputEventKey and event.is_action_pressed(InputActions.CANCEL) and not _card.visible):
		# Esc：有卡片開著先關卡片，否則開選單；Start 一律開選單。搖桿的 B 只取消與關卡片（GDD 5.9）。
		open_menu()
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed(InputActions.PAUSE):
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
	elif event.is_action_pressed(InputActions.MODE_SANJIAO):
		_set_mode(TerrainMap.Mode.SANJIAO)
	elif event.is_action_pressed(InputActions.MODE_LINGMAI):
		_set_mode(TerrainMap.Mode.LINGMAI)
	elif event.is_action_pressed(InputActions.CANCEL):
		_card.close()
	elif event.is_action_pressed(InputActions.CARD_CENTER):
		_card.request_center()
