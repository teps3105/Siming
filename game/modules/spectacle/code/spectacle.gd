class_name Spectacle
extends Node
## GDD 5.6 地圖奇觀（第一批：天劫、飛升、宿緣了結）。
## 由 SimHost.event_posted 觸發，只讀事件字典，不讀遊戲規則；事件發生在鏡頭外時不放（傳聞簽已有），
## 傳聞簽點擊飛往時以 replay() 補放。奇觀放在 3D 世界裡：每個奇觀一個舞台（SpectacleStage），原點在事件所在格的
## 地表，隨鏡頭縮放，被山遮得住；位置每幀由地圖座標換成世界座標（含高度消失點與星球視角的位移）。
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
## 奇觀半徑在螢幕上的範圍（介面單位的像素）：一格的像素數乘 14，夾在這兩個值之間。
const RADIUS_PX_MIN := 110.0
const RADIUS_PX_MAX := 320.0
## 了結線的半寬（介面單位的像素）、孤高（弧頂離弦的比例）。
const LINE_HALF_PX := 3.4
const ARC_RISE := 0.22

var _sim: SimHost
var _map: TerrainMap
var _camera: MapCamera
var _queue := SpectacleQueue.new()
var _memory := {}
var _memory_order: Array[int] = []
var _holder: Node3D
var _stages := {}


func _init() -> void:
	add_to_group("spectacle")


func setup(sim: SimHost, map: TerrainMap, camera: MapCamera) -> void:
	_sim = sim
	_map = map
	_camera = camera
	_holder = Node3D.new()
	_holder.name = "SpectacleStages"
	add_child(_holder)
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
	_sync()


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
	_sync()
	return added


func clear() -> void:
	_queue.clear()
	_memory.clear()
	_memory_order.clear()
	_sync()


## 前進 seconds 秒（_process 在沒暫停時呼叫；測試也直接呼叫）。
func step(seconds: float) -> void:
	_queue.advance(seconds)
	_sync()


func queue() -> SpectacleQueue:
	return _queue


## 目前有舞台的奇觀數（測試與量測用）。
func stage_count() -> int:
	return _stages.size()


func _process(delta: float) -> void:
	if _sim == null or _holder == null:
		return
	if not _sim.paused:
		_queue.advance(delta)
	_sync()


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


# ── 舞台 ──────────────────────────────────────────────────────

## 讓舞台與佇列一致：新開始的奇觀建舞台，結束或被捨棄的拆掉，其餘依鏡頭擺放並推進到各自的 age。
func _sync() -> void:
	if _holder == null or _map == null or _camera == null:
		return
	var live := {}
	for list in [_queue.items(), _queue.spots()]:
		for item: Dictionary in list:
			if float(item["age"]) < 0.0:
				continue
			var order := int(item["order"])
			live[order] = true
			var stage: SpectacleStage = _stages.get(order)
			if stage == null:
				stage = _make_stage(item)
				_stages[order] = stage
				_holder.add_child(stage)
				stage.setup(item)
			_place(stage, item)
	for order: int in _stages.keys():
		if not live.has(order):
			(_stages[order] as Node).queue_free()
			_stages.erase(order)


func _make_stage(item: Dictionary) -> SpectacleStage:
	if str(item["kind"]) == "Spot":
		return SpotStage.new()
	match str(item["kind"]):
		"Tribulation":
			return TribulationStage.new()
		"Ascended":
			return AscendedStage.new()
	return ResolvedStage.new()


## 依鏡頭把舞台擺到事件所在格的地表（了結是兩端的世界座標），縮放成螢幕上該有的大小。
func _place(stage: SpectacleStage, item: Dictionary) -> void:
	var age := float(item["age"])
	var u := unit()
	var near_x := _camera.focus.x
	var kind := str(item["kind"])
	if item.has("ends") and kind != "Spot":
		_place_thread(stage, item, age, u, near_x)
		return
	var pos: Vector2 = item["pos"]
	var screen := screen_of(pos)
	stage.visible = screen.is_finite()
	if not screen.is_finite():
		return
	var world := _map.world_position(pos, near_x)
	var px := maxf(cell_px(pos), 0.01)
	var radius_px := clampf(px * 14.0, RADIUS_PX_MIN, RADIUS_PX_MAX) * u
	if kind == "Spot":
		radius_px = 40.0 * u
	var radius := radius_px / px
	stage.global_transform = Transform3D(_up_basis(world).scaled(Vector3.ONE * radius), world)
	# 光柱要貫到畫面頂端：正交鏡頭下鉛直 1 單位在螢幕上約 cos(俯角) 像素乘每單位像素數。
	var beam_height := maxf((screen.y + 40.0) / (px * cos(MapView.pitch())) / radius, 2.0)
	stage.tick(maxf(age, 0.0), {"scale": radius, "u": u, "px_world": 1.0 / px, "beam_height": beam_height})


func _place_thread(stage: SpectacleStage, item: Dictionary, age: float, u: float, near_x: float) -> void:
	var ends: Array = item["ends"]
	var a_screen := screen_of(ends[0])
	var b_screen := screen_of(ends[1])
	stage.visible = a_screen.is_finite() and b_screen.is_finite()
	if not stage.visible:
		return
	var a := _map.world_position(ends[0], near_x)
	var b := _map.world_position(ends[1], near_x)
	var mid_pos: Vector2 = (ends[0] + ends[1]) * 0.5
	var px := maxf(cell_px(mid_pos), 0.01)
	var up := _up_basis((a + b) * 0.5).y
	# 弧線二次貝茲：控制點在弦的中點之上，弧頂高度取弦長的一成多，至少幾格。
	var length := a.distance_to(b)
	var rise := maxf(length * ARC_RISE, 24.0 * u / px * 0.5)
	var control := (a + b) * 0.5 + up * rise * 2.0
	var radius_px := clampf(px * 14.0, RADIUS_PX_MIN, RADIUS_PX_MAX) * u
	stage.global_transform = Transform3D.IDENTITY
	stage.tick(maxf(age, 0.0), {"a": a, "b": b, "control": control, "scale": radius_px / px * 0.45,
		"line_half_width": LINE_HALF_PX * u / px, "u": u, "px_world": 1.0 / px})


## 平面視角地表朝上；星球視角朝球心向外。
func _up_basis(world: Vector3) -> Basis:
	if _map.globe_morph <= 0.5:
		return Basis.IDENTITY
	var up := (world - _map.globe_xform.origin).normalized()
	var side := up.cross(Vector3.BACK)
	if side.length_squared() < 1e-4:
		side = up.cross(Vector3.RIGHT)
	side = side.normalized()
	return Basis(side, up, side.cross(up).normalized())
