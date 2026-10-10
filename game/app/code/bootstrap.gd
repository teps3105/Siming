extends Node
## 遊戲入口：裝配模擬、地圖、鏡頭與介面（GDD 6.4）。
## 開局設定畫面完成前，以固定種子開一個中尺寸世界。

const START_SEED := 20261011
const START_SIZE := "medium"
## 開場鏡頭高度（地圖寬的比例）。
const START_Z_FRAC := 0.3

@onready var _sim: SimHost = $SimHost
@onready var _map: TerrainMap = $TerrainMap
@onready var _camera: MapCamera = $MapCamera


func _ready() -> void:
	InputActions.register()
	var error := _sim.create_world(START_SEED, START_SIZE)
	if not error.is_empty():
		push_error("世界生成失敗：%s" % error)
		return
	_map.build(_sim.world)
	_camera.attach(_map, _start_focus(), _map.map_size.x * START_Z_FRAC)
	if has_node("Hud"):
		$Hud.setup(_sim, _map, _camera)


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
