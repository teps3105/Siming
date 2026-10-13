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
## 命簿「返回」最多記幾層（GDD 5.8）。
const BACK_DEPTH := 8

var _sim: SimHost
var _map: TerrainMap
var _camera: MapCamera
var _unit := 1.0
var _card_width := -1.0
var _overlay: MapOverlay
var _dial: TimeDial
var _seals: SealBox
var _card: ProvinceCard
var _person_card: PersonCard
var _event_card: EventCard
var _sect_card: SectCard
var _trace_key := ""
var _trace_on := false
## 每月推進換一次，讓溯因標示跟著重畫。
var _trace_stamp := 0
## 上一次排命簿時的高度上限。
var _last_card_limit := 0.0
## 從別張卡點進來時，回得去的上一張卡：{kind: "province"|"person"|"event"|"sect", id}，最近的在最後。
var _back_stack: Array[Dictionary] = []
var _view: ViewToggle
var _cursor: GameCursor
var _guide: GuideSlip
var _notice: NoticeSlip
var _rumor: RumorSlip
var _almanac: Almanac
var _menu: MenuDeck
## 是否顯示引導簽（測試會關掉）；在 setup 之前設定。
var guide_enabled := true
## 開選單前時間是否已經暫停。
var _paused_before_menu := false
## 開關選單造成的暫停與續行不算玩家動了時間。
var _menu_switching := false
## 傳聞簽全螢幕展開時是否已替它暫停了時間，與展開前時間是否已經暫停（器物重建時展開狀態會帶到新的簽上，不可重記）。
var _rumor_pause_held := false
var _paused_before_rumor := false
## 天下冊全螢幕展開時同理；頁籤停在上次的一頁（器物重建時帶到新的天下冊上）。
var _almanac_pause_held := false
var _paused_before_almanac := false
var _almanac_page := 0


func setup(sim: SimHost, map: TerrainMap, camera: MapCamera) -> void:
	_sim = sim
	_map = map
	_camera = camera
	camera.province_hovered.connect(func(id: int) -> void: _overlay.set_hover(id))
	camera.province_selected.connect(func(id: int) -> void:
		# 在地圖上另選一處：換成那州郡的卡，從卡點進來的返回記錄作廢。
		_back_stack.clear()
		_person_card.suspend()
		_event_card.suspend()
		_sect_card.suspend()
		_card.show_province(id)
		if id >= 0:
			_guide.notify(&"select"))
	camera.soul_selected.connect(_on_soul_selected)
	camera.globe_changed.connect(func(on: bool) -> void:
		_view.globe = on
		_guide.notify(&"globe"))
	sim.time_state_changed.connect(func() -> void:
		if not _menu_switching:
			_guide.notify(&"time"))
	sim.world_changed.connect(func(change: String, era: String) -> void: _notice.post(EventText.world_line(change, era)))
	sim.event_posted.connect(func(event: Dictionary) -> void: _rumor.post(event))
	sim.world_created.connect(func() -> void:
		if _notice != null:
			_notice.clear()
		if _rumor != null:
			_rumor.clear())
	sim.advanced.connect(func(_months: int) -> void:
		_card.refresh()
		_person_card.refresh()
		_event_card.refresh()
		_sect_card.refresh()
		_trace_stamp += 1
		_overlay.refresh_seats()
		_overlay.refresh_sects()
		_map.refresh_faith()
		_map.refresh_karma())
	# 地圖換貼圖的那一幀已經有上傳的工作，標籤留到下一幀再重讀。
	map.refreshed.connect(func(_terrain: bool) -> void:
		await get_tree().process_frame
		_overlay.reload()
		_card.refresh()
		_person_card.refresh()
		_event_card.refresh()
		_sect_card.refresh())
	var paper := Image.create_from_data(512, 512, false, Image.FORMAT_RGBA8, SimWorld.paper_texture())
	Ink.paper_texture = ImageTexture.create_from_image(paper)
	get_viewport().size_changed.connect(_layout)
	_layout()


## GDD 7.2 世界大事的那句話；開啟紀元的加上改元。
static func _world_notice(change: String, era: String) -> String:
	return EventText.world_line(change, era)


## 依畫面高度決定器物大小；大小改變時重建器物。
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
	_place_cards(bounds)
	_place_rumor()
	_rumor.fit_screen()
	_notice.top_center = Vector2(bounds.x * 0.5, MARGIN)


## 兩張命簿都靠右上角（一次只開一張）。
func _place_cards(bounds: Vector2) -> void:
	# 命簿放右上、印匣在右下，卡的下緣停在印匣（連同當下顯示著的例簽）上緣之上留一個間距。
	var limit := _card_limit(bounds)
	_last_card_limit = limit
	_card.max_height = limit
	_person_card.max_height = limit
	_event_card.max_height = limit
	_sect_card.max_height = limit
	_card.position = Vector2(bounds.x - _card.size.x - MARGIN, MARGIN)
	_person_card.position = Vector2(bounds.x - _person_card.size.x - MARGIN, MARGIN)
	_event_card.position = Vector2(bounds.x - _event_card.size.x - MARGIN, MARGIN)
	_sect_card.position = Vector2(bounds.x - _sect_card.size.x - MARGIN, MARGIN)
	# 題簽（日期最寬）不蓋到攤開的命簿：命簿最寬的那張決定題簽右端的上限。
	var card_width := maxf(maxf(_card.size.x, _person_card.size.x), maxf(_event_card.size.x, _sect_card.size.x))
	_dial.max_right = bounds.x - card_width - MARGIN * 2.0


## 命簿（含軸頭）最高能到哪裡：印匣與例簽上緣之上一個間距。
func _card_limit(bounds: Vector2) -> float:
	return (_seals.top_edge() if _seals != null else bounds.y) - MARGIN - MARGIN


func _build() -> void:
	var selected := _card.province if _card != null else -1
	var shown_person := _person_card.person if _person_card != null else -1
	var shown_event := _event_card.event if _event_card != null else -1
	var shown_sect := _sect_card.sect if _sect_card != null else -1
	var stack := _back_stack.duplicate()
	var rumors: Array[Dictionary] = []
	if _rumor != null:
		rumors = _rumor.snapshot()
	var rumors_open := _rumor != null and _rumor.expanded
	var almanac_open := _almanac != null and _almanac.expanded
	if _almanac != null:
		_almanac_page = _almanac.page
	var notices: Array[String] = []
	if _notice != null:
		notices = _notice.pending()
	for child in get_children():
		if child != _menu:
			child.queue_free()

	_overlay = MapOverlay.new()
	add_child(_overlay)
	_overlay.setup(_sim.world, _map, _camera, _unit)
	_overlay.sect_requested.connect(func(id: int) -> void: open_sect(id, false))
	_camera.over_gate = _overlay.gate_at
	_map.set_ui_unit(_unit)

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
	_card.closed.connect(func() -> void:
		_back_stack.clear()
		_map.set_selected(-1))
	_card.person_requested.connect(func(id: int) -> void: open_person(id))
	_card.show_province(selected)

	_person_card = PersonCard.new()
	add_child(_person_card)
	_person_card.setup(_sim.world, _unit, _card_width)
	_person_card.center_requested.connect(func(pos: Vector2) -> void: _camera.look_at_map(pos))
	_person_card.person_requested.connect(func(id: int) -> void: open_person(id))
	_person_card.province_requested.connect(func(id: int) -> void: open_province(id))
	_person_card.event_requested.connect(func(id: int) -> void: open_event(id))
	_person_card.sect_requested.connect(func(id: int) -> void: open_sect(id))
	_person_card.back_requested.connect(go_back)
	_person_card.closed.connect(func() -> void:
		_back_stack.clear()
		_map.clear_karma_highlight()
		_map.set_selected(-1))

	_event_card = EventCard.new()
	add_child(_event_card)
	_event_card.setup(_sim.world, _unit, _card_width)
	_event_card.center_requested.connect(func(pos: Vector2) -> void: _camera.look_at_map(pos))
	_event_card.person_requested.connect(func(id: int) -> void: open_person(id))
	_event_card.province_requested.connect(func(id: int) -> void: open_province(id))
	_event_card.event_requested.connect(func(id: int) -> void: open_event(id))
	_event_card.sect_requested.connect(func(id: int) -> void: open_sect(id))
	_event_card.back_requested.connect(go_back)
	_event_card.closed.connect(func() -> void:
		_back_stack.clear()
		_map.set_selected(-1))

	_sect_card = SectCard.new()
	add_child(_sect_card)
	_sect_card.setup(_sim.world, _unit, _card_width)
	_sect_card.center_requested.connect(func(pos: Vector2) -> void: _camera.look_at_map(pos))
	_sect_card.person_requested.connect(func(id: int) -> void: open_person(id))
	_sect_card.province_requested.connect(func(id: int) -> void: open_province(id))
	_sect_card.event_requested.connect(func(id: int) -> void: open_event(id))
	_sect_card.sect_requested.connect(func(id: int) -> void: open_sect(id))
	_sect_card.back_requested.connect(go_back)
	_sect_card.closed.connect(func() -> void:
		_back_stack.clear()
		_map.set_selected(-1))
	_back_stack = stack
	if shown_person >= 0:
		_person_card.can_back = not _back_stack.is_empty()
		_person_card.show_person(shown_person)
	if shown_event >= 0:
		_event_card.can_back = not _back_stack.is_empty()
		_event_card.show_event(shown_event)
	if shown_sect >= 0:
		_sect_card.can_back = not _back_stack.is_empty()
		_sect_card.show_sect(shown_sect)

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

	_rumor = RumorSlip.new()
	add_child(_rumor)
	_rumor.setup(_unit)
	_rumor.compact = get_viewport().get_visible_rect().size.y <= Ink.DECK_HEIGHT
	_rumor.restore(rumors)
	_rumor.fullscreen_changed.connect(_on_rumor_fullscreen)
	_rumor.expanded = rumors_open
	_rumor.fly_requested.connect(func(map_pos: Vector2) -> void: _camera.look_at_map(map_pos))
	_rumor.person_requested.connect(func(id: int) -> void: open_person(id))
	_rumor.sect_requested.connect(func(id: int) -> void: open_sect(id))
	_rumor.trace_requested.connect(func(id: int) -> void: open_event(id))
	_place_rumor()

	_notice = NoticeSlip.new()
	add_child(_notice)
	_notice.setup(_unit)
	for line in notices:
		_notice.post(line)

	_almanac = Almanac.new()
	add_child(_almanac)
	_almanac.setup(_sim, _unit)
	_almanac.page = _almanac_page
	_almanac.fullscreen_changed.connect(_on_almanac_fullscreen)
	_almanac.expanded = almanac_open
	_almanac.province_requested.connect(func(id: int) -> void: open_province(id))
	_almanac.person_requested.connect(func(id: int) -> void: open_person(id))
	_almanac.event_requested.connect(func(id: int) -> void: open_event(id))
	_almanac.sect_requested.connect(func(id: int) -> void: open_sect(id))
	_dial.almanac_requested.connect(toggle_almanac)

	# 遊戲內游標畫在所有器物之上。
	_cursor = GameCursor.new()
	add_child(_cursor)
	_cursor.setup(_camera, _unit)
	_cursor.extra_hot = _overlay.gate_at
	if _menu != null:
		# 選單蓋在器物之上、游標之下。
		move_child(_menu, -1)
		move_child(_cursor, -1)
	_apply_device()


## 傳聞簽全螢幕展開：模擬暫停（記下展開前的行止）、鏡頭讓出畫面；收合時行止與速度回到展開前。
func _on_rumor_fullscreen(on: bool) -> void:
	# 兩張全螢幕互斥：先收另一張（它的行止先還原），這一張才記下展開前的行止。
	if on and _almanac != null:
		_almanac.expanded = false
	_camera.modal = on
	if _sim == null:
		return
	if on and not _rumor_pause_held:
		_rumor_pause_held = true
		_paused_before_rumor = _sim.paused
		_menu_switching = true
		_sim.set_paused(true)
		_menu_switching = false
	elif not on and _rumor_pause_held:
		_rumor_pause_held = false
		_menu_switching = true
		_sim.set_paused(_paused_before_rumor)
		_menu_switching = false


## 天下冊全螢幕展開：與傳聞簽相同——模擬暫停、鏡頭讓出畫面，收合時行止與速度回到展開前。
func _on_almanac_fullscreen(on: bool) -> void:
	if on and _rumor != null:
		_rumor.expanded = false
	_camera.modal = on
	if _sim == null:
		return
	if on and not _almanac_pause_held:
		_almanac_pause_held = true
		_paused_before_almanac = _sim.paused
		_menu_switching = true
		_sim.set_paused(true)
		_menu_switching = false
	elif not on and _almanac_pause_held:
		_almanac_pause_held = false
		_menu_switching = true
		_sim.set_paused(_paused_before_almanac)
		_menu_switching = false


## 展開或收回天下冊。
func toggle_almanac() -> void:
	if _menu != null or _sim == null:
		return
	_almanac.expanded = not _almanac.expanded


## 引導簽在底部中央、印鈕列上方 24 px（GDD 5.7）；畫面窄到會壓住印匣的例簽時往左讓開。
func _place_guide() -> void:
	if _guide == null or _seals == null:
		return
	var bounds := get_viewport().get_visible_rect().size
	var seals_top := _seals.position.y + _seals._badge_position(0).y
	var x := minf((bounds.x - _guide.size.x) * 0.5, _seals.position.x - GUIDE_GAP - _guide.size.x)
	_guide.position = Vector2(maxf(x, MARGIN), seals_top - GUIDE_GAP - _guide.size.y).round()


## 傳聞簽在右上角；命簿開著時讓到它左邊，左邊被天時儀與視角鈕佔住時改放在它們下方（GDD 5.7）。
func _place_rumor() -> void:
	if _rumor == null or _seals == null or _view == null or _rumor.expanded:
		return
	var bounds := get_viewport().get_visible_rect().size
	var open_card := _open_card()
	var right := open_card.position.x - MARGIN if open_card != null else bounds.x - MARGIN
	_rumor.max_width = maxf(right - MARGIN, 0.0)
	var width := _rumor.size.x - RumorSlip.SHADOW * 2.0
	var dial_right := _dial.position.x + maxf(_dial._title_rect().end.x, _dial._speed_rect().end.x) + Ink.SLIP_BAND
	var y := MARGIN
	if right - width < dial_right + MARGIN:
		y = _view.position.y + _view.size.y + MARGIN
	_rumor.max_height = maxf(_seals.position.y - y - MARGIN, 0.0)
	_rumor.right_edge = right
	_rumor.top_edge = y


## 目前看得到的命簿（沒有為 null）。
func _open_card() -> LedgerCard:
	if _sect_card.visible:
		return _sect_card
	if _event_card.visible:
		return _event_card
	if _person_card.visible:
		return _person_card
	if _card.visible:
		return _card
	return null


func person_card() -> PersonCard:
	return _person_card


func province_card() -> ProvinceCard:
	return _card


func event_card() -> EventCard:
	return _event_card


func sect_card() -> SectCard:
	return _sect_card


## 開某人的人物卡並飛往他所在（GDD 5.8「點了就到」）；已身故的人也開得了。
## 目前開著的卡記進返回記錄，Esc／B 回到它。傳聞簽日後的「看人物」接這裡。
func open_person(id: int) -> void:
	if _sim == null or id < 0 or _sim.world.person(id).is_empty():
		return
	_rumor.expanded = false
	_almanac.expanded = false
	_remember_open_card()
	_card.suspend()
	_event_card.suspend()
	_sect_card.suspend()
	_person_card.can_back = not _back_stack.is_empty()
	_person_card.show_person(id)
	_camera.look_at_map(_sim.world.person(id)["pos"])
	_place_cards(get_viewport().get_visible_rect().size)


## 飛往某州郡並開它的州郡卡（記下目前開著的卡，可返回）。
func open_province(id: int) -> void:
	if _sim == null or id < 0 or id >= _sim.world.province_count():
		return
	_rumor.expanded = false
	_almanac.expanded = false
	_remember_open_card()
	_person_card.suspend()
	_event_card.suspend()
	_sect_card.suspend()
	_map.set_selected(id)
	_card.show_province(id)
	_camera.look_at_map(_sim.world.province_info(id)["seat"])
	_place_cards(get_viewport().get_visible_rect().size)


## 開某則事件的事件卡（溯因視圖，GDD 5.8）；事件已修剪或編號無效時卡仍開，寫明「此事已不可考」。
## 目前開著的卡記進返回記錄，Esc／B 回到它。有地點的事件不自動飛往（飛往是卡上的動作）。
func open_event(id: int) -> void:
	if _sim == null or id < 0:
		return
	_rumor.expanded = false
	_almanac.expanded = false
	_remember_open_card()
	_card.suspend()
	_person_card.suspend()
	_sect_card.suspend()
	_event_card.can_back = not _back_stack.is_empty()
	_event_card.show_event(id)
	_place_cards(get_viewport().get_visible_rect().size)


## 開某個宗門的宗門卡（GDD 5.8）；fly 為真時鏡頭飛往山門（天下冊與各卡的宗門名都飛往，地圖上點山門本就在眼前所以不飛）。
## 已覆滅的宗門也開，飛往它原本的山門。目前開著的卡記進返回記錄，Esc／B 回到它。
func open_sect(id: int, fly := true) -> void:
	if _sim == null or id < 0:
		return
	var info := _sim.world.sect(id)
	if info.is_empty():
		return
	_rumor.expanded = false
	_almanac.expanded = false
	_remember_open_card()
	_card.suspend()
	_person_card.suspend()
	_event_card.suspend()
	_sect_card.can_back = not _back_stack.is_empty()
	_sect_card.show_sect(id)
	if fly:
		_camera.look_at_map(info["pos"])
	_place_cards(get_viewport().get_visible_rect().size)


## 開目前選取人物（人物卡開著的那位）最近一則事件的事件卡（GDD 5.9 的 X）；沒有事件回傳 false。
func trace_selected_person() -> bool:
	if _sim == null or not _person_card.is_open():
		return false
	var last: int = _sim.world.person(_person_card.person).get("last_event", -1)
	if last < 0:
		return false
	open_event(last)
	return true


## 回到上一張卡；沒有上一張就收起目前這張。
func go_back() -> void:
	if _back_stack.is_empty():
		_close_cards()
		return
	var previous: Dictionary = _back_stack.pop_back()
	_person_card.suspend()
	_card.suspend()
	_event_card.suspend()
	_sect_card.suspend()
	if previous["kind"] == "sect":
		_sect_card.can_back = not _back_stack.is_empty()
		_sect_card.show_sect(previous["id"])
	elif previous["kind"] == "person":
		_person_card.can_back = not _back_stack.is_empty()
		_person_card.show_person(previous["id"])
	elif previous["kind"] == "event":
		_event_card.can_back = not _back_stack.is_empty()
		_event_card.show_event(previous["id"])
	else:
		_map.set_selected(previous["id"])
		_card.show_province(previous["id"])
	_place_cards(get_viewport().get_visible_rect().size)


## 目前開著的卡先記進返回記錄。
func _remember_open_card() -> void:
	var entry := {}
	if _sect_card.is_open():
		entry = {"kind": "sect", "id": _sect_card.sect}
	elif _event_card.is_open():
		entry = {"kind": "event", "id": _event_card.event}
	elif _person_card.is_open():
		entry = {"kind": "person", "id": _person_card.person}
	elif _card.is_open():
		entry = {"kind": "province", "id": _card.province}
	if entry.is_empty():
		return
	if not _back_stack.is_empty() and _back_stack[-1] == entry:
		return
	_back_stack.append(entry)
	if _back_stack.size() > BACK_DEPTH:
		_back_stack.pop_front()


## 關掉所有命簿（返回記錄一併清掉）。
func _close_cards() -> void:
	_back_stack.clear()
	_person_card.close()
	_event_card.close()
	_sect_card.close()
	_card.close()


func rumor() -> RumorSlip:
	return _rumor


func almanac() -> Almanac:
	return _almanac


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
	# 全螢幕傳聞先收合，時間回到展開前的行止，選單才記得住開選單前的暫停狀態。
	_rumor.expanded = false
	_almanac.expanded = false
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
	_sync_trace()
	if _menu != null:
		_camera.use_crosshair = _menu.panel().gamepad
	if _camera.use_crosshair != _dial.gamepad:
		_apply_device()
	# 例簽出現或收起時，命簿的高度上限跟著變。
	var bounds := get_viewport().get_visible_rect().size
	if not is_equal_approx(_card_limit(bounds), _last_card_limit):
		_place_cards(bounds)
	_place_rumor()
	_overlay.reserved = _occupied()


## 器物目前佔住的畫面範圍（地圖標籤要避開）。
func _occupied() -> Array[Rect2]:
	var disc := Rect2(_dial.position, Vector2.ONE * _dial.outer_radius() * 2.0)
	var slips := _dial._title_rect().merge(_dial._speed_rect())
	slips.position += _dial.position
	var seals := _seals._seal_rect(0).merge(_seals._seal_rect(SealBox.SEALS.size() - 1))
	seals.position += _seals.position
	var out: Array[Rect2] = [disc.grow(6.0), slips.grow(6.0), seals.grow(6.0)]
	var open_card := _open_card()
	if open_card != null:
		out.append(Rect2(open_card.position, open_card.size).grow(6.0))
	if _guide.visible:
		out.append(Rect2(_guide.position, _guide.size).grow(6.0))
	out.append(Rect2(_view.position, _view.size).grow(6.0))
	out.append(_rumor.occupied_rect().grow(6.0))
	return out


func _apply_device() -> void:
	_dial.gamepad = _camera.use_crosshair
	_seals.gamepad = _camera.use_crosshair
	_card.gamepad = _camera.use_crosshair
	_person_card.gamepad = _camera.use_crosshair
	_event_card.gamepad = _camera.use_crosshair
	_sect_card.gamepad = _camera.use_crosshair
	_view.gamepad = _camera.use_crosshair
	_guide.gamepad = _camera.use_crosshair
	_rumor.gamepad = _camera.use_crosshair
	_almanac.gamepad = _camera.use_crosshair


## 因果模式下點了魂光（GDD 5.4）：開那位的人物卡，地圖只留他的線並標出前世所在，鏡頭落在他的魂光上。
func _on_soul_selected(person: int) -> void:
	_map.set_selected(-1)
	open_person(person)
	_map.select_karma_soul(person)
	var spot := _map.karma_layer().glow_position_of(person)
	if spot != Vector2.INF:
		_camera.look_at_map(spot)


## 事件卡開著且在因果模式時，把這則事件與因樹的人物與其間的宿緣線標在地圖上，其餘淡去；
## 換事件跟著換，卡關掉或離開因果模式就還原。不切模式。狀態變了才重畫。
func _sync_trace() -> void:
	var lit := _event_card != null and _event_card.is_open() and _map.mode == TerrainMap.Mode.KARMA and not _event_card.is_gone()
	var key := "%d:%s" % [_event_card.event, _trace_stamp] if lit else ""
	if key == _trace_key:
		return
	_trace_key = key
	if lit:
		var people := _event_card.trace_people()
		_map.highlight_karma(people, _sim.world.karma_between(people))
		_trace_on = true
	elif _trace_on:
		_trace_on = false
		# 換成人物卡時，人物卡自己的強調不能被清掉。
		if not _person_card.is_open():
			_map.clear_karma_highlight()


func _set_mode(mode: TerrainMap.Mode) -> void:
	if mode != _map.mode:
		_guide.notify(&"mode")
	_map.set_mode(mode)
	_seals.set_mode(mode)


func _unhandled_input(event: InputEvent) -> void:
	if _sim == null or _menu != null:
		return
	if event.is_action_pressed(InputActions.ALMANAC):
		_almanac.expanded = not _almanac.expanded
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed(InputActions.RUMOR_LIST):
		_rumor.expanded = not _rumor.expanded
	elif _almanac.expanded:
		# 天下冊開著：同傳聞簽，只收 Esc／B 收合與 Start 開選單，其餘時間與地圖的快捷鍵都不動。
		if event.is_action_pressed(InputActions.CANCEL):
			_almanac.expanded = false
			get_viewport().set_input_as_handled()
		elif event.is_action_pressed(InputActions.MENU):
			open_menu()
			get_viewport().set_input_as_handled()
	elif _rumor.expanded:
		# 全螢幕傳聞開著：只收 Esc／B 收合與 Start 開選單，其餘時間與地圖的快捷鍵都不動。
		if event.is_action_pressed(InputActions.CANCEL):
			_rumor.expanded = false
			get_viewport().set_input_as_handled()
		elif event.is_action_pressed(InputActions.MENU):
			open_menu()
			get_viewport().set_input_as_handled()
	elif event.is_action_pressed(InputActions.MENU) or (event is InputEventKey and event.is_action_pressed(InputActions.CANCEL) and _open_card() == null):
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
	elif event.is_action_pressed(InputActions.MODE_YINGUO):
		_set_mode(TerrainMap.Mode.KARMA)
	elif event.is_action_pressed(InputActions.CANCEL):
		go_back()
	elif event.is_action_pressed(InputActions.CARD_TRACE):
		trace_selected_person()
	elif event.is_action_pressed(InputActions.CARD_CENTER):
		if _sect_card.is_open():
			_sect_card.request_center()
		elif _event_card.is_open():
			_event_card.request_center()
		elif _person_card.is_open():
			_person_card.request_center()
		else:
			_card.request_center()
