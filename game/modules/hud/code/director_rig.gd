class_name DirectorRig
extends Node
## GDD 5.13 導演模式的接線：把決策（Director）接到模擬的事件與速度、鏡頭的過場與操作偵測、字卡與設定。
## 每幀餵真實秒給 Director；沒有自己的決策。

## 玩家用快捷鍵切換導演模式，帶新的開關狀態。
signal toggled(on: bool)

## 到位的高度上限（地圖寬的比例）：棋子看得見的高度（5.6）；玩家原本更低則維持。
const LANDING_FRAC := 0.25
## 停留中換了要看的事，新事件的投影落在畫面中央這個比例的矩形外就平移過去（5.13：事件要在中央三分之一內）。
const REFOCUS_FRAC := 0.35

var director := Director.new()
var speed: DirectorSpeed

var _sim: SimHost
var _camera: MapCamera
var _card: DirectorCard
var _fade: ColorRect
## 鏡頭正前往（或剛到）的地圖座標。
var _dest := Vector2.ZERO
## 回傳 true 表示玩家正在用畫面（卡片、天下冊、選單等開著）。
var _busy: Callable
## 回傳 true 表示現在不該由導演掌鏡（開場運鏡、要玩家操作的引導步驟）。
var _suppressed: Callable


func setup(sim: SimHost, camera: MapCamera, busy: Callable, suppressed: Callable) -> void:
	_sim = sim
	_camera = camera
	_busy = busy
	_suppressed = suppressed
	speed = DirectorSpeed.new(sim)
	_fade = ColorRect.new()
	_fade.color = Ink.INK
	_fade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_fade.set_anchors_preset(Control.PRESET_FULL_RECT)
	_fade.modulate.a = 0.0
	add_child(_fade)
	sim.event_posted.connect(_on_event)
	sim.world_created.connect(director.reset)
	camera.player_acted.connect(director.player_acted)
	camera.directed_done.connect(director.arrive)
	director.go.connect(_on_go)
	director.refocus.connect(_on_refocus)
	director.yielded.connect(_on_yielded)
	director.caption_changed.connect(_on_caption)
	director.slow_changed.connect(_on_slow)


## 換上新的字卡（器物重建時）；目前的句子帶過去。
func attach_card(card: DirectorCard) -> void:
	_card = card
	card.person_requested.connect(func(_id: int) -> void: director.player_acted())
	card.province_requested.connect(func(_id: int) -> void: director.player_acted())
	card.sect_requested.connect(func(_id: int) -> void: director.player_acted())
	card.acted.connect(director.player_acted)
	_on_caption(director.caption())


func _process(delta: float) -> void:
	if _sim == null:
		return
	director.enabled = GameSettings.flag("director") and not _suppressed.call() and not _camera.is_flying()
	director.pace = int(GameSettings.value("director_pace"))
	director.idle_resume = float(GameSettings.value("director_idle"))
	speed.enabled = GameSettings.flag("director_slow")
	director.busy = _busy.call() or _camera.pointer_kind() != &"arrow"
	director.update(delta, not _sim.paused)
	_fade.modulate.a = _camera.directed_fade


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed(InputActions.DIRECTOR_TOGGLE):
		var on := not GameSettings.flag("director")
		GameSettings.set_value("director", on)
		toggled.emit(on)
		get_viewport().set_input_as_handled()


func _on_event(event: Dictionary) -> void:
	if event["kind"] == "WorldChanged":
		return
	var importance: int = event["importance"]
	if importance < RumorSlip.NOTE_IMPORTANCE:
		return
	director.post(int(event.get("province", -1)), importance, event)


func _on_go(province: int, _big: bool) -> void:
	var seat: Vector2 = _sim.world.province_info(province)["seat"]
	var spot: Variant = director.target_event().get("pos")
	if spot is Vector2:
		seat = spot
	_fly_to(seat)


func _fly_to(spot: Vector2) -> void:
	_dest = spot
	_camera.direct_to(spot, minf(_camera.z, _camera.map_width() * LANDING_FRAC), GameSettings.flag("director_reduce"))


## 要看的事換成同州的另一件：過場途中改去新事件格；停留中新事件的投影不在畫面中央就平移過去（不算換鏡頭，停留不重計）。
func _on_refocus(event: Dictionary) -> void:
	var spot: Variant = event.get("pos")
	if not spot is Vector2:
		return
	if director.state == Director.State.TRAVEL:
		if (spot as Vector2).distance_to(_dest) > 1.0:
			_fly_to(spot)
		return
	var view := Vector2(get_viewport().get_visible_rect().size)
	var screen := _camera.screen_of_cell(spot)
	if not screen.is_finite() or not Rect2(view * (0.5 - REFOCUS_FRAC * 0.5), view * REFOCUS_FRAC).has_point(screen):
		_fly_to(spot)


func _on_yielded() -> void:
	_camera.cancel_directed()
	if _card != null:
		_card.hide_card()
	speed.release()


func _on_caption(event: Dictionary) -> void:
	if _card == null:
		return
	if event.is_empty() or not GameSettings.flag("director_card"):
		_card.hide_card()
	else:
		_card.show_event(event)


func _on_slow(on: bool) -> void:
	if on:
		speed.engage()
	else:
		speed.release()
