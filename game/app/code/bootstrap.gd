extends Node
## 遊戲場景：裝配模擬、地圖、鏡頭與介面（GDD 6.4）。
## 平時由標題畫面交來一個已經開好或讀出來的世界（start_world）；直接執行這個場景時以固定種子開一個世界。

## 直接執行時的種子與星球尺寸（"small"、"medium"、"large"）；測試會在加入場景樹前改成小尺寸。
@export var start_seed := 20261011
@export var start_size := "medium"
## 選單端備妥的世界；有值就不另外開天。
var start_world: SimWorld
## 進場時是否先暫停（讀檔進來的世界）。
var start_paused := false
## 每世自動存檔一次（GDD 4.8）；測試會關掉。
var auto_save := true
## 是否顯示引導簽（GDD 5.7）；測試會關掉。
var show_guide := true
## 開場運鏡（GDD 5.2）：先看到星球，鏡頭再落到開場位置；測試預設關掉。
@export var opening_flight := true
## 開場鏡頭高度（地圖寬的比例）。
const START_Z_FRAC := 0.3

var _saved_tick := 0

@onready var _sim: SimHost = $SimHost
@onready var _map: TerrainMap = $TerrainMap
@onready var _camera: MapCamera = $MapCamera


func _ready() -> void:
	InputActions.register()
	if start_world != null:
		_sim.adopt_world(start_world)
	else:
		var error := _sim.create_world(start_seed, start_size)
		if not error.is_empty():
			push_error("世界生成失敗：%s" % error)
			return
	_sim.set_paused(start_paused)
	_map.build(_sim.world)
	var start_z := _map.map_size.x * START_Z_FRAC
	if opening_flight:
		_camera.attach(_map, _start_focus(), _map.map_size.x * MapView.Z_MAX_FRAC)
		_camera.descend_to(start_z, MapCamera.OPENING_SECONDS)
	else:
		_camera.attach(_map, _start_focus(), start_z)
	if has_node("Hud"):
		$Hud.guide_enabled = show_guide
		$Hud.setup(_sim, _map, _camera)
		$Hud.title_requested.connect(func() -> void: AppFlow.enter_title(self))
		$Hud.world_loaded.connect(func(world: SimWorld) -> void: AppFlow.enter_game(self, world, true))
	var spectacle := Spectacle.new()
	add_child(spectacle)
	spectacle.setup(_sim, _map, _camera)
	_saved_tick = _sim.world.tick()
	_sim.advanced.connect(_auto_save)
	_sim.fast_forward_finished.connect(func(_summary: Dictionary) -> void: _auto_save(0))


## 時間跨過一世的邊界就存一份自動存檔。
func _auto_save(_days: int) -> void:
	# 世界快進一口氣跨好幾世：不在途中存，停下時存一份。
	if _sim.fast_active():
		return
	var now: int = _sim.world.tick()
	if auto_save and SaveStore.auto_due(_saved_tick, now):
		var error := SaveStore.save_auto(_sim.world)
		if not error.is_empty():
			push_warning("自動存檔失敗：%s" % error)
	_saved_tick = now


func _unhandled_key_input(event: InputEvent) -> void:
	AppFlow.screenshot_on_f12(event, get_viewport())


## 開場焦點：形心在南北緯 60° 內、面積最大的大洲（GDD 5.3）。
func _start_focus() -> Vector2:
	var best := _map.map_size * 0.5
	var best_rank := -1.0
	for label: Dictionary in _sim.world.map_labels():
		if label["kind"] != "continent":
			continue
		var pos: Vector2 = label["pos"]
		var latitude := absf(90.0 - 180.0 * pos.y / _map.map_size.y)
		if latitude <= 60.0 and label["rank"] > best_rank:
			best_rank = label["rank"]
			best = pos
	return best
