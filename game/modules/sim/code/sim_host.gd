class_name SimHost
extends Node
## GDD 6.4 模擬驅動：Godot 端唯一建立與推進 SimWorld 的節點。
## 平常按細步檔位逐日推進；跨長時間走世界快進（粗步，每幀最多花 8 ms）。
## 其他模組由此取得世界後只做查詢。

signal world_created
## 世界往前走了 days 日（細步與快進每幀各發一次）。
signal advanced(days: int)
signal time_state_changed
## GDD 2.8 世界大事：change 是大事代號（ColdBegan…），era 是它開啟的紀元名（沒改元為空字串）。
signal world_changed(change: String, era: String)
## GDD 6.4 每一筆新事件（SimWorld.drain_events 的字典）；傳聞簽等器物由此取得，事件佇列只在這裡取走。
## 世界快進期間的事件不發這個訊號（照樣寫進帳，結束時進摘要）。
signal event_posted(event: Dictionary)
## 世界快進開始與結束；結束帶摘要（見 _summary）。
signal fast_forward_started
signal fast_forward_finished(summary: Dictionary)

## GDD 2.6 細步四檔：每秒走的日數。
const DAYS_PER_SECOND: Array[int] = [1, 2, 5, 10]
## 世界快進每幀花的時間預算（毫秒）。
const FAST_BUDGET_MSEC := 8
## 一幀最多補多少日（視窗卡頓後不一口氣追趕）。
const MAX_CATCH_UP_DAYS := 30
## 每月日數、每年日數（GDD 2.6）。
const DAYS_PER_MONTH := 30
const DAYS_PER_YEAR := 360

## 世界快進的終點。
enum Stop {
	## 下一件世界大事（WorldChanged）。
	WORLD_EVENT,
	## 指定年數。
	YEARS,
	## 按住才快進：沒有終點，放開（或任何一鍵）就停。
	HOLD,
	## 關注者的下一件事。關注功能還沒做，目前不可用。
	FOLLOWED,
}

var world: SimWorld = SimWorld.new()
var paused := false
var speed_index := 0

var _backlog := 0.0
var _fast := false
var _fast_stop := Stop.YEARS
var _fast_from: Dictionary = {}


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
	_fast = false
	# 開局前的歷史與讀檔重播留下的事件不是新消息。
	world.drain_events()
	world_created.emit()


## 立即推進 months 個月（測試與工具用；平時由 _process 依速度推進）。
func advance(months: int) -> void:
	if not world.is_ready() or months <= 0:
		return
	world.step(months)
	_announce()
	advanced.emit(months * DAYS_PER_MONTH)


## 立即推進 days 日（測試與工具用）。
func advance_days(days: int) -> void:
	if not world.is_ready() or days <= 0:
		return
	world.step_days(days)
	_announce()
	advanced.emit(days)


func set_paused(value: bool) -> void:
	if paused == value:
		return
	paused = value
	time_state_changed.emit()


func toggle_paused() -> void:
	set_paused(not paused)


func set_speed(index: int) -> void:
	index = clampi(index, 0, DAYS_PER_SECOND.size() - 1)
	if speed_index == index:
		return
	speed_index = index
	_backlog = 0.0
	time_state_changed.emit()


## 目前檔位每秒走幾日。
func days_per_second() -> int:
	return DAYS_PER_SECOND[speed_index]


func _process(delta: float) -> void:
	if not world.is_ready():
		return
	if _fast:
		_fast_tick()
		return
	if paused:
		return
	_backlog = minf(_backlog + days_per_second() * delta, float(MAX_CATCH_UP_DAYS))
	var days := int(_backlog)
	if days > 0:
		_backlog -= days
		world.step_days(days)
		_announce()
		advanced.emit(days)


# ── 世界快進（GDD 2.6）──────────────────────────────────────

## 世界快進是否進行中。
func fast_active() -> bool:
	return _fast


## 這個終點現在能不能用。「關注者的下一件事」要等關注功能做出來才開。
func can_fast_to(stop: Stop) -> bool:
	return world.is_ready() and stop != Stop.FOLLOWED


## 開始世界快進。years 只在 Stop.YEARS 用。失敗（沒有世界、已在快進、終點不可用）回傳 false。
## 快進不依暫停狀態：暫停中也能快進，結束後維持暫停。
func fast_forward(stop: Stop, years := 0) -> bool:
	if _fast or not can_fast_to(stop):
		return false
	if not world.fast_begin(int(stop), years):
		return false
	_fast = true
	_fast_stop = stop
	_fast_from = world.date_parts()
	_backlog = 0.0
	fast_forward_started.emit()
	time_state_changed.emit()
	return true


## 中止世界快進，停在現在這一天並出摘要。沒有快進時什麼也不做。
func fast_stop() -> void:
	if _fast:
		_finish_fast()


func _fast_tick() -> void:
	var before: int = world.tick()
	var step: Dictionary = world.fast_step(FAST_BUDGET_MSEC)
	var moved: int = int(step["tick"]) - before
	if moved > 0:
		advanced.emit(moved)
	if step["done"]:
		_finish_fast()


func _finish_fast() -> void:
	_fast = false
	var raw: Dictionary = world.fast_finish()
	# 快進沒有推進的日也要讓畫面跟上（地圖換圖、命簿重讀）。
	var summary := _summary(raw)
	time_state_changed.emit()
	fast_forward_finished.emit(summary)


## 摘要：跨過的日數與年月日、終點，期間各類事件件數、世界大事（事件字典，依時間），人口與修士數前後。
func _summary(raw: Dictionary) -> Dictionary:
	var start: int = int(raw.get("start_tick", 0))
	var end: int = int(raw.get("end_tick", start))
	var days := end - start
	var events: Array[Dictionary] = []
	for id: int in raw.get("world_events", PackedInt64Array()):
		var event: Dictionary = world.event(id)
		if not event.is_empty():
			events.append(event)
	return {
		"stop": _fast_stop,
		"from_parts": _fast_from,
		"to_parts": world.date_parts(),
		"start_tick": start,
		"end_tick": end,
		"days": days,
		"years": days / DAYS_PER_YEAR,
		"months": days % DAYS_PER_YEAR / DAYS_PER_MONTH,
		"rest_days": days % DAYS_PER_MONTH,
		"kinds": raw.get("kinds", {}),
		"world_events": events,
		"hit": int(raw.get("hit", -1)),
		"population0": float(raw.get("population0", 0.0)),
		"population1": float(raw.get("population1", 0.0)),
		"people0": int(raw.get("people0", 0)),
		"people1": int(raw.get("people1", 0)),
	}


## 取走這次推進新增的事件，逐筆發出去，世界大事另發 world_changed。
func _announce() -> void:
	for event: Dictionary in world.drain_events():
		if event["kind"] == "WorldChanged":
			world_changed.emit(event["change"], event["era"])
		event_posted.emit(event)
