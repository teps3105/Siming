class_name EventStage
extends Node
## 5.13 棋子演出：事件發生時，當事人以 2D 棋子立在事件所在格上演出一段小影片（定場、動作、餘韻、收場）。
## 事件 → 腳本由 EventScript 的資料表決定；這裡管排隊、等鏡頭、計時與棋子的建立擺放。
## 擺位、深度、鏡頭高度門檻與樣子的決定方式沿用天劫：投影與門檻問 Spectacle，棋子是 FigureActor。
## 演出以真實時間播放，不隨遊戲速度；遊戲暫停時不前進。演出（時間線）與棋子分開：每件事件一登記就排隊計時，
## 棋子只在演出播放中、事件格在畫面內、鏡頭夠近時才建立，且全場最多 MAX_FIGURES 枚。

## 同時存在的棋子上限，超過的丟重要度最低的演出的棋子。
const MAX_FIGURES := 12
## 同一格排隊（尚未開演）最多幾件，超過丟重要度最低的。
const MAX_WAITING := 3
## 全場登記的演出總數上限（高倍速下事件很多）。
const MAX_PERFORMANCES := 96
## 等鏡頭最多幾秒（真實時間）。
const HOLD_LIMIT := 5.0
## 導演要看的那件事所在州郡等鏡頭最多幾秒：比導演過場上限（Director.TRAVEL_LIMIT）長一點，到位前不會自己開演。
const FOCUS_HOLD_LIMIT := 8.0
## 兩人演出各站事件格一側，離中心幾個身高。
const PAIR_GAP := 0.32
## 事件格在不在畫面內，每幾幀重算一次。
const VISIBLE_EVERY := 6
const POOL_MAX := 16
## 高倍速（≥ 20×）下動作每秒畫幾格：重畫是棋子最大的耗時，減半。
const FAST_FPS := 12.0

## 事件編號 → 演出長度（秒）：登記時依當事人現況（對方死了沒）算出的實際長度，duration_for 優先採用。
static var _known := {}

var _spectacle: Spectacle
var _sim: SimHost
var _holder: Node3D
## 演出：{id, event, plan, importance, province, pos, cell, serial, age（−1 尚未開演）, actors: Array[FigureActor], light（轉世的光效，沒有為 null）, to_pos,
## focus（導演字卡那件事：不被排隊上限丟掉、在棋子名額與同格排隊上優先、到位才開演）}
var _perfs: Array[Dictionary] = []
var _holds := {}
var _serial := 0
var _floor := -1
## 空出來的棋子留著重用（建一枚要配一個 SubViewport）。
var _pool: Array[FigureActor] = []
var _throttle := false


func _init() -> void:
	add_to_group("event_stage")


func setup(spectacle: Spectacle, sim: SimHost) -> void:
	_spectacle = spectacle
	_sim = sim
	_holder = Node3D.new()
	_holder.name = "EventFigures"
	add_child(_holder)
	sim.event_posted.connect(post)
	sim.world_created.connect(clear)


## 這件事的演出長度（秒，四段合計）；沒有對應動作回 0。
static func duration_for(event: Dictionary) -> float:
	var id := int(event.get("id", -1))
	if id >= 0 and _known.has(id):
		return float(_known[id])
	return EventScript.duration_for(event)


## 一筆新事件：表裡有對應動作才登記，立刻排隊（沒被 hold 的州郡就從定場開播）。
func post(event: Dictionary) -> void:
	# 最前面的快速跳過：表裡沒有的種類、登記滿了而重要度不高於排隊裡最低的，不建字典、不查位置。
	if not EventScript.accepts(event):
		return
	if _perfs.size() >= MAX_PERFORMANCES and int(event.get("importance", 1)) <= _waiting_floor():
		return
	var perf := _register(event, false)
	if perf.is_empty():
		return
	_trim(perf["cell"])
	_start_ready()
	# 棋子由 _process 每幀對齊；這裡只在這件馬上開演時立刻建（高倍速下每秒上千件事件，不能每件都全場對齊一次）。
	if float(perf["age"]) >= 0.0 and not _spectacle.figure_hidden() and _is_visible(perf):
		_sync()


## 登記一件演出（不排隊、不開演）；表裡沒有對應動作回空字典。
func _register(event: Dictionary, focus: bool) -> Dictionary:
	var prepared := event
	if str(event.get("kind", "")) == "ThreadResolved" and not event.has("survivor_alive"):
		prepared = event.duplicate()
		prepared["survivor_alive"] = _debtor_alive(event)
	var plan := EventScript.plan(prepared)
	if plan.is_empty():
		return {}
	var pos: Vector2 = prepared["pos"]
	_serial += 1
	var perf := {"id": int(prepared.get("id", -1)), "event": prepared, "plan": plan, "importance": int(prepared.get("importance", 1)),
		"province": int(prepared.get("province", -1)), "pos": pos, "cell": Vector2i(floori(pos.x), floori(pos.y)),
		"serial": _serial, "age": -1.0, "actors": [], "light": null, "to_pos": _destination(plan), "focus": focus}
	if int(perf["id"]) >= 0:
		_known[perf["id"]] = float(plan["total"])
	_perfs.append(perf)
	return perf


## 導演選定要看的那件事（字卡事件）：它的演出排在最前，不被排隊上限丟掉，棋子名額與同格排隊都優先；
## 州郡還在等鏡頭時（過場中）停在開演前，到位 release 才從定場播；已在播或已演完、被擠掉的，重新登記一份從頭播。
## 表裡沒有對應動作的事件（例如饑荒）不處理。
func focus(event: Dictionary) -> void:
	if event.is_empty() or not EventScript.accepts(event) or not event.has("pos"):
		return
	for perf in _perfs:
		perf["focus"] = false
	var id := int(event.get("id", -1))
	var province := int(event.get("province", -1))
	var waiting := _holds.has(province)
	var perf := {}
	for each in _perfs:
		if id >= 0 and int(each["id"]) == id:
			perf = each
	if not perf.is_empty() and (float(perf["age"]) < 0.0 or not waiting):
		perf["focus"] = true
	else:
		if not perf.is_empty():
			_free_actors(perf)
			_perfs.erase(perf)
		perf = _register(event, true)
		if perf.is_empty():
			return
	if waiting:
		_holds[province] = maxf(float(_holds[province]), FOCUS_HOLD_LIMIT)
	_start_ready()
	_sync()


## 該州郡排隊中與播放中的演出還要幾秒（各格並行，取最久的一格）；沒有為 0。
## 州郡裡有導演字卡那件事的演出時，只算它自己這件（導演停留以它的長度為準）。
func remaining_seconds(province: int) -> float:
	var per_cell := {}
	for perf in _perfs:
		if int(perf["province"]) == province and bool(perf["focus"]):
			return float((perf["plan"] as Dictionary)["total"]) - maxf(float(perf["age"]), 0.0)
	for perf in _perfs:
		if int(perf["province"]) != province:
			continue
		var left := float((perf["plan"] as Dictionary)["total"]) - maxf(float(perf["age"]), 0.0)
		per_cell[perf["cell"]] = float(per_cell.get(perf["cell"], 0.0)) + left
	var best := 0.0
	for cell: Vector2i in per_cell:
		best = maxf(best, float(per_cell[cell]))
	return best


## 這個州郡尚未開演的演出停在開演前等鏡頭（最多 HOLD_LIMIT 秒）；正在播的不受影響。
func hold(province: int) -> void:
	_holds[province] = HOLD_LIMIT


## 解除等待，從定場開始播。
func release(province: int) -> void:
	_holds.erase(province)
	_start_ready()
	_sync()


## 前進 seconds 秒（_process 在沒暫停時呼叫；測試也直接呼叫）。
func advance(seconds: float) -> void:
	_floor = -1
	for province: int in _holds.keys():
		_holds[province] = float(_holds[province]) - seconds
		if float(_holds[province]) <= 0.0:
			_holds.erase(province)
	var keep: Array[Dictionary] = []
	for perf in _perfs:
		if float(perf["age"]) >= 0.0:
			perf["age"] = float(perf["age"]) + seconds
			if float(perf["age"]) >= float((perf["plan"] as Dictionary)["total"]):
				_free_actors(perf)
				continue
		keep.append(perf)
	_perfs = keep
	_start_ready()
	_sync()


func clear() -> void:
	for perf in _perfs:
		_free_actors(perf)
	for actor in _pool:
		actor.queue_free()
	_pool.clear()
	_perfs.clear()
	_holds.clear()
	_known.clear()


## 登記中的演出（測試與量測用）。
func performances() -> Array[Dictionary]:
	return _perfs


## 目前存在的轉世光效（沒有人物棋子，與棋子共用全場上限）。
func lights() -> Array[Node3D]:
	var out: Array[Node3D] = []
	for perf in _perfs:
		if perf["light"] != null:
			out.append(perf["light"])
	return out


## 目前存在的棋子。
func figures() -> Array[FigureActor]:
	var out: Array[FigureActor] = []
	for perf in _perfs:
		for actor: FigureActor in perf["actors"]:
			out.append(actor)
	return out


func _process(delta: float) -> void:
	if _sim == null or _holder == null:
		return
	_throttle = true
	if not _sim.paused:
		advance(delta)
	else:
		_sync()
	_throttle = false


# ── 排隊 ──────────────────────────────────────────────────────

## 同一格排隊超過 MAX_WAITING 件、或全場登記太多時，丟重要度最低的（同重要度丟先到的）。
func _trim(cell: Vector2i) -> void:
	while _waiting_in(cell).size() > MAX_WAITING:
		_drop(_lowest(_waiting_in(cell)))
	while _perfs.size() > MAX_PERFORMANCES:
		var waiting: Array[Dictionary] = []
		for perf in _perfs:
			if float(perf["age"]) < 0.0 and not bool(perf["focus"]):
				waiting.append(perf)
		if waiting.is_empty():
			break
		_drop(_lowest(waiting))


## 排隊中最低的重要度（一個畫面算一次）；沒有排隊的（全在播）為 99：登記滿了又沒有可丟的，新來的一律不收。
func _waiting_floor() -> int:
	if _floor < 0:
		_floor = 99
		for perf in _perfs:
			if float(perf["age"]) < 0.0:
				_floor = mini(_floor, int(perf["importance"]))
	return _floor


func _waiting_in(cell: Vector2i) -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	for perf in _perfs:
		if perf["cell"] == cell and float(perf["age"]) < 0.0 and not bool(perf["focus"]):
			out.append(perf)
	return out


static func _lowest(list: Array[Dictionary]) -> Dictionary:
	var victim: Dictionary = list[0]
	for perf in list:
		if int(perf["importance"]) < int(victim["importance"]) \
				or (int(perf["importance"]) == int(victim["importance"]) and int(perf["serial"]) < int(victim["serial"])):
			victim = perf
	return victim


func _drop(perf: Dictionary) -> void:
	_free_actors(perf)
	_perfs.erase(perf)


## 每一格沒有在播的演出時，讓排隊裡重要度最高（同重要度先到）且州郡沒被 hold 的開演。
func _start_ready() -> void:
	# 導演字卡那件事輪到開演時，同格正在播的別件讓位（它要在畫面中央演）。
	for perf in _perfs.duplicate():
		if bool(perf["focus"]) and float(perf["age"]) < 0.0 and not _holds.has(perf["province"]):
			for other in _perfs.duplicate():
				if other != perf and other["cell"] == perf["cell"] and float(other["age"]) >= 0.0:
					_free_actors(other)
					_perfs.erase(other)
	var busy := {}
	for perf in _perfs:
		if float(perf["age"]) >= 0.0:
			busy[perf["cell"]] = true
	var best := {}
	for perf in _perfs:
		if float(perf["age"]) >= 0.0 or busy.has(perf["cell"]) or _holds.has(perf["province"]):
			continue
		var cell: Vector2i = perf["cell"]
		if not best.has(cell) or _start_rank(perf) > _start_rank(best[cell]):
			best[cell] = perf
	for cell: Vector2i in best:
		best[cell]["age"] = 0.0


## 開演順位：導演字卡那件事最優先，其餘依重要度。
static func _start_rank(perf: Dictionary) -> int:
	return int(perf["importance"]) + (1000 if bool(perf["focus"]) else 0)


# ── 事件資料 ──────────────────────────────────────────────────

## 了結的欠債方（a）此刻是否還在世：在世的對方只迎擊、不倒下。
func _debtor_alive(event: Dictionary) -> bool:
	var who := int(event.get("a_person", -1))
	if who < 0 or _sim == null or _sim.world == null:
		return false
	return bool(_sim.world.person(who).get("alive", false))


## 行走的終點（地圖座標）：目的州郡的治所；其他演出沒有。
func _destination(plan: Dictionary) -> Vector2:
	if not bool(plan["travel"]) or _sim == null or _sim.world == null:
		return Vector2.INF
	var to := int(plan["to"])
	if to < 0 or to >= _sim.world.province_count():
		return Vector2.INF
	return _sim.world.province_info(to).get("seat", Vector2.INF)


# ── 棋子 ──────────────────────────────────────────────────────

func _free_actors(perf: Dictionary) -> void:
	if perf["light"] != null:
		if is_instance_valid(perf["light"]):
			(perf["light"] as Node).queue_free()
		perf["light"] = null
	for actor: FigureActor in perf["actors"]:
		if is_instance_valid(actor):
			if _pool.size() < POOL_MAX:
				actor.visible = false
				_pool.append(actor)
			else:
				actor.queue_free()
	perf["actors"] = []


## 讓棋子與演出一致：播放中、事件格在畫面內、鏡頭夠近的建棋子並擺放（全場最多 MAX_FIGURES 枚，先給重要度高的），其餘拆掉。
func _sync() -> void:
	if _holder == null or _spectacle == null or _spectacle.camera() == null:
		return
	var hidden := _spectacle.figure_hidden()
	var shown: Array[Dictionary] = []
	for perf in _perfs:
		if float(perf["age"]) < 0.0 or hidden or not _is_visible(perf):
			if not (perf["actors"] as Array).is_empty() or perf["light"] != null:
				_free_actors(perf)
			continue
		shown.append(perf)
	if shown.size() > 1:
		shown.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
			return _start_rank(a) > _start_rank(b) or (_start_rank(a) == _start_rank(b) and int(a["serial"]) < int(b["serial"])))
	var budget := MAX_FIGURES
	for perf in shown:
		var need: int = maxi((perf["plan"] as Dictionary)["actors"].size(), 1)
		if need > budget:
			_free_actors(perf)
			continue
		budget -= need
		_stage(perf)


## 事件格在不在畫面內。每幀的 _process 裡每件演出錯開，每 VISIBLE_EVERY 幀才重算一次（高倍速下登記的演出很多）；
## 直接呼叫 advance（測試、導演）時一律重算。
func _is_visible(perf: Dictionary) -> bool:
	if _throttle and perf.has("seen") and (Engine.get_process_frames() + int(perf["serial"])) % VISIBLE_EVERY != 0:
		return bool(perf["seen"])
	perf["seen"] = _spectacle.pos_in_view(perf["pos"])
	return perf["seen"]


func _stage(perf: Dictionary) -> void:
	var plan: Dictionary = perf["plan"]
	if bool(plan["light"]):
		_stage_light(perf)
		return
	var actors_plan: Array = plan["actors"]
	if (perf["actors"] as Array).size() != actors_plan.size():
		_free_actors(perf)
		for entry: Dictionary in actors_plan:
			var actor: FigureActor = _pool.pop_back() if not _pool.is_empty() else FigureActor.new()
			if actor.get_parent() == null:
				_holder.add_child(actor)
			actor.setup(_look(int(entry["person"]), int(perf["id"])))
			(perf["actors"] as Array).append(actor)
	var camera := _spectacle.camera()
	var pos: Vector2 = perf["pos"]
	var base := _spectacle.world_of(pos)
	var up := _spectacle.up_basis(base).y
	var u := _spectacle.unit()
	var height := FigureActor.world_height()
	var age := float(perf["age"])
	var total := float(plan["total"])
	var fade := EventScript.opacity_at(total, age)
	var count := actors_plan.size()
	var fast := _sim != null and _sim.speed_multiplier() >= SpectacleQueue.FAST_SPEED
	var right := _ground_dir_of_world(pos, base, camera.global_basis.x, up)
	for i in count:
		var actor: FigureActor = perf["actors"][i]
		var doing := EventScript.pose_at(actors_plan[i], age, bool(plan["travel"]))
		var at := pos
		if count == 2:
			at = _moved(pos, base, right, (-1.0 if i == 0 else 1.0) * PAIR_GAP * height)
			actor.face_right = i == 0
		elif bool(plan["travel"]):
			var dir := _travel_dir(pos, perf["to_pos"])
			var a := _spectacle.screen_of(pos)
			var b := _spectacle.screen_of(pos + dir)
			actor.face_right = a.is_finite() and b.is_finite() and b.x > a.x
			at = _moved(pos, base, dir, EventScript.WALK_SPEED * maxf(age - EventScript.INTRO, 0.0) * height)
		else:
			actor.face_right = false
		var moment := float(doing["time"])
		if fast:
			moment = floorf(moment * FAST_FPS) / FAST_FPS
		actor.play(str(doing["action"]), moment)
		var origin := _spectacle.world_of(at)
		origin += _spectacle.up_basis(origin).y * EventScript.lift_at(plan, age) * height
		actor.place(origin, _spectacle.up_basis(origin).y, camera, u, false)
		actor.set_fade(fade)


## 轉世的光效：立在事件格地表，縮放等於身高，時間用演出的 age（RebirthStage 自己淡入淡出）。
func _stage_light(perf: Dictionary) -> void:
	if perf["light"] == null:
		var made := RebirthStage.new()
		_holder.add_child(made)
		made.setup({"seed": int(perf["id"]) * 7 + 13, "total": float((perf["plan"] as Dictionary)["total"])})
		perf["light"] = made
	var light: RebirthStage = perf["light"]
	var base := _spectacle.world_of(perf["pos"])
	var basis := _spectacle.up_basis(base)
	var height := FigureActor.world_height()
	light.global_transform = Transform3D(basis.scaled(Vector3.ONE * height), base)
	light.tick(maxf(float(perf["age"]), 0.0), {})


## 樣子：人物編號決定（沒有就用事件編號），橋接給得出的欄位（性別、年紀）照用，同天劫。
func _look(person: int, fallback: int) -> Dictionary:
	var given := {}
	if person >= 0 and _sim != null and _sim.world != null:
		given = FigureActor.given_from_person(_sim.world.person(person))
	return FigureActor.look_of(person if person >= 0 else fallback * 7 + 13, given)


## 地圖座標 pos 沿地圖方向 dir（單位向量）走世界距離 distance 之後的地圖座標。
func _moved(pos: Vector2, base: Vector3, dir: Vector2, distance: float) -> Vector2:
	if dir == Vector2.ZERO:
		return pos
	var per_unit := _spectacle.world_of(pos + dir).distance_to(base)
	return pos + dir * (distance / maxf(per_unit, 1e-4))


## 世界裡的水平方向換成地圖座標的單位方向（最小平方：用地圖兩軸的世界向量去逼近）。
func _ground_dir_of_world(pos: Vector2, base: Vector3, world_dir: Vector3, up: Vector3) -> Vector2:
	var flat := (world_dir - up * world_dir.dot(up)).normalized()
	var e1 := _spectacle.world_of(pos + Vector2(1.0, 0.0)) - base
	var e2 := _spectacle.world_of(pos + Vector2(0.0, 1.0)) - base
	var a := e1.dot(e1)
	var b := e1.dot(e2)
	var d := e2.dot(e2)
	var det := a * d - b * b
	if absf(det) < 1e-9:
		return Vector2.RIGHT
	var r1 := e1.dot(flat)
	var r2 := e2.dot(flat)
	var v := Vector2((d * r1 - b * r2) / det, (a * r2 - b * r1) / det)
	return v.normalized() if v.length() > 1e-6 else Vector2.RIGHT


## 從 pos 朝目的地的地圖方向（東西取較短的一邊）；沒有目的地時朝東。
func _travel_dir(pos: Vector2, to: Vector2) -> Vector2:
	if not to.is_finite():
		return Vector2.RIGHT
	var width := _spectacle.map_width()
	var dx := fposmod(to.x - pos.x + width * 0.5, width) - width * 0.5
	var dir := Vector2(dx, to.y - pos.y)
	return dir.normalized() if dir.length() > 1e-4 else Vector2.RIGHT
