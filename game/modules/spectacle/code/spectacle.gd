class_name Spectacle
extends CanvasLayer
## GDD 5.6 地圖奇觀（第一批：天劫、飛升、宿緣了結）。
## 由 SimHost.event_posted 觸發，只讀事件字典，不讀遊戲規則；事件發生在鏡頭外時不放（傳聞簽已有），
## 傳聞簽點擊飛往時以 replay() 補放。畫在地圖之上、介面之下的一層，位置每幀由地圖座標換成螢幕座標。
## 時長是實際秒，與遊戲速度無關；暫停（含開著選單）時不前進。

const KINDS: Array[String] = ["Tribulation", "Ascended", "ThreadResolved"]
## 記住最近多少件奇觀的資料（供重播；了結的兩端要用事件當下的位置）。
const MEMORY_MAX := 64
## 重播等鏡頭飛到的秒數。
const REPLAY_DELAY := 0.9
## 判定「在鏡頭內」時，螢幕邊緣外多寬的範圍也算。
const VIEW_MARGIN := 48.0
## 找不到了結另一端時，兩端各向兩側張開的距離（地圖座標）。
const FALLBACK_SPREAD := 2.5

var _sim: SimHost
var _map: TerrainMap
var _camera: MapCamera
var _queue := SpectacleQueue.new()
var _canvas: SpectacleCanvas
var _memory := {}
var _memory_order: Array[int] = []


func _init() -> void:
	layer = 0
	add_to_group("spectacle")


func setup(sim: SimHost, map: TerrainMap, camera: MapCamera) -> void:
	_sim = sim
	_map = map
	_camera = camera
	_canvas = SpectacleCanvas.new()
	_canvas.layer = self
	add_child(_canvas)
	sim.event_posted.connect(post)
	sim.world_created.connect(clear)


## 一筆新事件：天劫、飛升、了結才放；在鏡頭外的記下資料但不放。
func post(event: Dictionary) -> void:
	var spec := spec_of(event)
	if spec.is_empty():
		return
	_remember(spec)
	if not _in_view(spec):
		return
	_queue.add(spec, _sim.speed_multiplier() >= SpectacleQueue.FAST_SPEED)
	_redraw()


## 傳聞簽點擊飛往時重播：傳聞簽的記錄（或事件字典）。不是這三種事件、或這件已在放，回傳 false。
func replay(entry: Dictionary) -> bool:
	if not entry.has("id") or not KINDS.has(str(entry.get("kind", ""))):
		return false
	var id := int(entry["id"])
	if _queue.has_event(id):
		return false
	var spec: Dictionary = _memory.get(id, {})
	if spec.is_empty():
		spec = spec_of(entry)
	if spec.is_empty():
		return false
	var added := _queue.add(spec, false, REPLAY_DELAY)
	_redraw()
	return added


func clear() -> void:
	_queue.clear()
	_memory.clear()
	_memory_order.clear()
	_redraw()


## 前進 seconds 秒（_process 在沒暫停時呼叫；測試也直接呼叫）。
func step(seconds: float) -> void:
	_queue.advance(seconds)
	_redraw()


func queue() -> SpectacleQueue:
	return _queue


func _process(delta: float) -> void:
	if _sim == null or _canvas == null:
		return
	if not _sim.paused:
		_queue.advance(delta)
	if _queue.count() > 0 or _queue.spots().size() > 0:
		_canvas.queue_redraw()


func _redraw() -> void:
	if _canvas != null:
		_canvas.queue_redraw()


# ── 事件 → 奇觀資料 ───────────────────────────────────────────

## 事件（或傳聞簽記錄）轉成奇觀資料；不是這三種或沒有地點回空字典。
## 了結的 ends 是 [欠債方所在, 債主所在]（地圖座標）：債主端取事件地點，欠債端取人物目前所在（見 _debtor_end）。
func spec_of(event: Dictionary) -> Dictionary:
	var kind := str(event.get("kind", ""))
	if not KINDS.has(kind) or not event.has("pos"):
		return {}
	var pos: Vector2 = event["pos"]
	var spec := {"id": int(event.get("id", -1)), "kind": kind, "importance": int(event.get("importance", 3)),
		"pos": pos, "seed": int(event.get("id", 0)) * 7 + 13}
	match kind:
		"Tribulation":
			spec["outcome"] = str(event.get("x", "Passed"))
			spec["bolts"] = clampi(int(event.get("n", 2)) + 1, 3, 9)
		"ThreadResolved":
			spec["thread_kind"] = maxi(KarmaLayer.KIND_KEYS.find(str(event.get("x", "")).to_lower()), 0)
			var near := pos
			var far := _debtor_end(event)
			spec["ends_known"] = far.is_finite() and far.distance_to(near) > 0.5
			if not spec["ends_known"]:
				far = near + Vector2(-FALLBACK_SPREAD, 0.0)
				near = near + Vector2(FALLBACK_SPREAD, 0.0)
			spec["ends"] = [far, near]
	return spec


## 了結欠債方一端：事件字典若給了 a_pos 就用它（橋接補欄位後的正途），否則取人物目前的所在格。
func _debtor_end(event: Dictionary) -> Vector2:
	if event.has("a_pos"):
		return event["a_pos"]
	var who := int(event.get("a_person", -1))
	if who < 0 and event.has("persons") and (event["persons"] as PackedInt32Array).size() > 0:
		who = (event["persons"] as PackedInt32Array)[0]
	if who >= 0 and _sim != null:
		var info := _sim.world.person(who)
		if info.has("pos"):
			return info["pos"]
	return Vector2.INF


func _remember(spec: Dictionary) -> void:
	var id := int(spec["id"])
	if id < 0:
		return
	if not _memory.has(id):
		_memory_order.append(id)
		if _memory_order.size() > MEMORY_MAX:
			_memory.erase(_memory_order.pop_front())
	_memory[id] = spec


# ── 投影 ──────────────────────────────────────────────────────

## 地圖座標在螢幕上的位置；在鏡頭後方或星球背面為 Vector2.INF。
func screen_of(pos: Vector2) -> Vector2:
	if _map == null or _camera == null:
		return Vector2.INF
	var world := _map.world_position(pos, _camera.focus.x)
	if _camera.is_position_behind(world) or not _map.faces_camera(pos, _camera.global_position):
		return Vector2.INF
	return _camera.unproject_position(world)


## 一個地圖格在螢幕上約幾像素寬（取不到時 4）。
func cell_px(pos: Vector2) -> float:
	var a := screen_of(pos)
	var b := screen_of(pos + Vector2(1.0, 0.0))
	if not a.is_finite() or not b.is_finite():
		return 4.0
	return a.distance_to(b)


func _in_view(spec: Dictionary) -> bool:
	var rect := Rect2(Vector2.ZERO, get_viewport().get_visible_rect().size).grow(VIEW_MARGIN)
	var points: Array[Vector2] = [spec["pos"]]
	if spec.has("ends"):
		var ends: Array = spec["ends"]
		points = [ends[0], ends[1], (ends[0] + ends[1]) * 0.5]
	for pos in points:
		var s := screen_of(pos)
		if s.is_finite() and rect.has_point(s):
			return true
	return false


## 介面單位（線寬、半徑乘它）。
func unit() -> float:
	return Ink.layout_unit(get_viewport().get_visible_rect().size, GameSettings.ui_scale())


## 畫布呼叫：把佇列裡的奇觀畫出來。
func paint(canvas: CanvasItem) -> void:
	var u := unit()
	for list in [_queue.items(), _queue.spots()]:
		for item: Dictionary in list:
			var age := float(item["age"])
			if age < 0.0:
				continue
			var t := age / float(item["duration"])
			if item.has("ends") and item["kind"] != "Spot":
				var a := screen_of(item["ends"][0])
				var b := screen_of(item["ends"][1])
				if a.is_finite() and b.is_finite():
					SpectacleBrush.resolved(canvas, item, a, b, u, age, t)
				continue
			var p := screen_of(item["pos"])
			if not p.is_finite():
				continue
			var radius := clampf(cell_px(item["pos"]) * 14.0, 110.0, 320.0) * u
			match str(item["kind"]):
				"Tribulation":
					SpectacleBrush.tribulation(canvas, item, p, radius, u, age, t)
				"Ascended":
					SpectacleBrush.ascended(canvas, item, p, radius, u, age, t)
				"Spot":
					SpectacleBrush.spot(canvas, item, p, u, t)
