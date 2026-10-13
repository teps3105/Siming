class_name SimHost
extends Node
## GDD 6.4 模擬驅動：Godot 端唯一建立與推進 SimWorld 的節點。
## 依速度每幀推進若干個月，每幀最多花 8 ms；其他模組由此取得世界後只做查詢。

signal world_created
signal advanced(months: int)
signal time_state_changed
signal slowed_down
## GDD 2.8 世界大事：change 是大事代號（ColdBegan…），era 是它開啟的紀元名（沒改元為空字串）。
signal world_changed(change: String, era: String)
## GDD 6.4 每一筆新事件（SimWorld.drain_events 的字典）；傳聞簽等器物由此取得，事件佇列只在這裡取走。
signal event_posted(event: Dictionary)

## GDD 2.6 速度：1×、5×、10×、20×；1× 每秒 2 個月。
const SPEED_MULTIPLIERS: Array[int] = [1, 5, 10, 20]
const MONTHS_PER_SECOND := 2.0
## 每幀推進的時間預算（毫秒）與累積上限（月）。
const FRAME_BUDGET_MSEC := 8
const BACKLOG_LIMIT := 60.0

var world: SimWorld = SimWorld.new()
var paused := false
var speed_index := 0

var _backlog := 0.0


## GDD 4.6 開天。size 為 "small"、"medium"、"large"。失敗時回傳原因，成功回傳空字串。
func create_world(world_seed: int, size: String = "medium") -> String:
	var error := world.create(world_seed, size)
	if error.is_empty():
		world.drain_events()
		world_created.emit()
	return error


## 接手一個已經開好或讀出來的世界（選單端建立，GDD 9.1）。
func adopt_world(ready_world: SimWorld) -> void:
	world = ready_world
	_backlog = 0.0
	# 開局前的歷史與讀檔重播留下的事件不是新消息。
	world.drain_events()
	world_created.emit()


## 立即推進 months 個月（測試與工具用；平時由 _process 依速度推進）。
func advance(months: int) -> void:
	if not world.is_ready() or months <= 0:
		return
	world.step(months)
	_announce()
	advanced.emit(months)


func set_paused(value: bool) -> void:
	if paused == value:
		return
	paused = value
	time_state_changed.emit()


func toggle_paused() -> void:
	set_paused(not paused)


func set_speed(index: int) -> void:
	index = clampi(index, 0, SPEED_MULTIPLIERS.size() - 1)
	if speed_index == index:
		return
	speed_index = index
	_backlog = 0.0
	time_state_changed.emit()


func speed_multiplier() -> int:
	return SPEED_MULTIPLIERS[speed_index]


func _process(delta: float) -> void:
	if paused or not world.is_ready():
		return
	_backlog += MONTHS_PER_SECOND * speed_multiplier() * delta
	var deadline := Time.get_ticks_msec() + FRAME_BUDGET_MSEC
	var done := 0
	while _backlog >= 1.0 and Time.get_ticks_msec() < deadline:
		world.step(1)
		_backlog -= 1.0
		done += 1
	if done > 0:
		_announce()
		advanced.emit(done)
	if _backlog > BACKLOG_LIMIT and speed_index > 0:
		# 推演趕不上：自動降一級並提示（GDD 6.4）。
		set_speed(speed_index - 1)
		slowed_down.emit()


## 取走這次推進新增的事件，逐筆發出去，世界大事另發 world_changed。
func _announce() -> void:
	for event: Dictionary in world.drain_events():
		if event["kind"] == "WorldChanged":
			world_changed.emit(event["change"], event["era"])
		event_posted.emit(event)
