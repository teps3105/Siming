extends SceneTree
## 讀圖與量測工具（5.13 棋子演出）：開真正的遊戲，對照表每一列在鏡頭推近處拍動作中段，另拍多處同時演出；
## --perf 另跑十六倍速一分鐘量每幀耗時（--no-events 關掉棋子演出作對照）。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy --resolution 1920x1080 -s res://tests/harness/event_stage_view.gd -- --out=<資料夾> [--perf] [--no-events]
## 視窗要開（不能 --headless）。

const BOOTSTRAP := "res://app/scenes/bootstrap.tscn"

var _out := ""
var _perf := false
var _no_events := false
var _sim: SimHost
var _camera: MapCamera
var _map: TerrainMap
var _stage: EventStage
var _people: Array[int] = []
var _id := 70000


func _initialize() -> void:
	_main.call_deferred()


func _shot(name: String) -> void:
	for i in 6:
		await process_frame
	root.get_texture().get_image().save_png(_out.path_join("%s.png" % name))
	print("截圖：", name)


func _event(kind: String, pos: Vector2, extra := {}) -> Dictionary:
	_id += 1
	var event := {"id": _id, "tick": 100, "kind": kind, "importance": 3, "province": 0, "cell": -1, "pos": pos,
		"a_person": _people[0], "b_person": _people[1], "x": "", "n": 2, "q_province": -1}
	event.merge(extra, true)
	return event


func _main() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg == "--perf":
			_perf = true
		elif arg == "--no-events":
			_no_events = true
	DirAccess.make_dir_recursive_absolute(_out)
	GameSettings.file_path = _out.path_join("settings-event.cfg")
	GameSettings.load_file()
	GameSettings.set_value("ui_scale", 1.0)
	var game: Node = (load(BOOTSTRAP) as PackedScene).instantiate()
	game.start_seed = 20261011
	game.start_size = "medium"
	game.opening_flight = false
	game.auto_save = false
	game.show_guide = false
	root.add_child(game)
	await process_frame
	AudioServer.set_bus_mute(0, true)
	_sim = game.get_node("SimHost")
	_sim.set_paused(true)
	for i in 40:
		await process_frame
	_camera = game.find_children("*", "MapCamera", true, false)[0]
	_map = game.find_children("*", "TerrainMap", true, false)[0]
	_stage = get_first_node_in_group("event_stage")
	if _perf:
		await _run_perf()
		quit()
		return
	for id in 400:
		var info := _sim.world.person(id)
		if not info.is_empty() and int(info.get("age", 0)) >= 20 and _people.size() < 6:
			_people.append(id)
	var focus: Vector2 = _sim.world.province_info(0)["seat"]
	print("鏡頭焦點 ", focus, " 當事人 ", _people)
	var cases := [
		["Killed", {}, 0.8], ["FeudEscalated", {}, 0.8], ["ThreadResolved", {"x": "Feud"}, 0.8],
		["ThreadResolved", {"x": "Feud", "survivor_alive": true}, 0.8], ["ThreadResolved", {"x": "Grace"}, 0.6],
		["Died", {"x": "Age"}, 0.8], ["Breakthrough", {}, 0.4], ["Breakthrough", {}, 0.9], ["Ascended", {}, 0.5],
		["FellToDemon", {}, 0.4], ["FellToDemon", {}, 0.9], ["ThreadCreated", {"x": "Mentor"}, 0.6], ["Married", {}, 0.6],
		["Succession", {}, 0.6], ["PowerFounded", {}, 0.6], ["Traveled", {"x": "Out", "q_province": 3}, 0.9],
	]
	_camera.look_at_map(focus, 30.0, true)
	for i in 8:
		await process_frame
	var n := 0
	for entry: Array in cases:
		n += 1
		_stage.clear()
		var event := _event(entry[0], focus, entry[1])
		_stage.post(event)
		var plan := EventScript.plan(_stage.performances()[0]["event"])
		var t := EventScript.INTRO + float(plan["action_len"]) * float(entry[2])
		_stage.advance(t)
		await _shot("%02d_%s_%s_%d" % [n, entry[0], str(entry[1].get("x", "")), int(float(entry[2]) * 100)])
	for frac: float in [0.3, 0.6, 0.9]:
		_stage.clear()
		_stage.post(_event("Ascended", focus))
		var asc := EventScript.plan(_stage.performances()[0]["event"])
		_stage.advance(EventScript.INTRO + float(asc["action_len"]) * frac)
		await _shot("asc_%d" % int(frac * 100))
	# 定場與收場各一張
	_stage.clear()
	_stage.post(_event("Married", focus))
	_stage.advance(0.3)
	await _shot("intro_fade_in")
	_stage.advance(EventScript.INTRO + 1.0)
	_stage.advance(EventScript.HOLD)
	await _shot("outro_fade_out")
	# 同畫面多處同時演出
	_stage.clear()
	var kinds := [["Killed", {}], ["Married", {}], ["Succession", {}], ["Breakthrough", {}], ["FellToDemon", {}], ["Died", {"x": "Age"}],
		["PowerFounded", {}], ["Traveled", {"x": "Out", "q_province": 3}], ["Ascended", {}]]
	_camera.look_at_map(focus, 55.0, true)
	for i in 8:
		await process_frame
	for k in kinds.size():
		var pos := focus + Vector2(float(k % 3 - 1) * 5.5, float(k / 3 - 1) * 4.5)
		var extra: Dictionary = kinds[k][1].duplicate()
		extra["a_person"] = _people[k % 3 * 2]
		extra["b_person"] = _people[k % 3 * 2 + 1]
		_stage.post(_event(kinds[k][0], pos, extra))
	_stage.advance(EventScript.INTRO + 1.0)
	await _shot("crowd")
	print("同畫面棋子 ", _stage.figures().size())
	quit()


## 十六倍速跑一分鐘（真實時間），記每幀耗時。
func _run_perf() -> void:
	if _no_events:
		_sim.event_posted.disconnect(_stage.post)
	_camera.look_at_map(_camera.focus, _map.map_size.x * 0.12, true)
	_sim.set_speed(SimHost.SPEED_MULTIPLIERS.size() - 1)
	print("速度 ", _sim.speed_multiplier(), " 倍")
	_sim.set_paused(false)
	var frames: Array[float] = []
	var last := Time.get_ticks_usec()
	var begin := last
	var shown := 0
	var peak_figures := 0
	var posted := 0
	var pump := func(_e: Dictionary) -> void: posted += 1
	_sim.event_posted.connect(pump)
	while Time.get_ticks_usec() - begin < 60000000:
		await process_frame
		var now := Time.get_ticks_usec()
		frames.append(float(now - last) / 1000.0)
		last = now
		shown += 1
		peak_figures = maxi(peak_figures, _stage.figures().size())
	frames.sort()
	var sum := 0.0
	for f in frames:
		sum += f
	print("重畫 ", FigureActor.redraws, " 次共 ", FigureActor.redraw_usec / 1000, " ms")
	print("幀數 %d 平均 %.2f ms p95 %.2f ms p99 %.2f ms 最大 %.2f ms 棋子峰值 %d 事件 %d 登記中 %d" % [frames.size(), sum / frames.size(),
		frames[int(frames.size() * 0.95)], frames[int(frames.size() * 0.99)], frames[-1], peak_figures, posted, _stage.performances().size()])
