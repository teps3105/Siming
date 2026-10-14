extends SceneTree
## 導播模式（GDD 5.13）的實機截圖：開視窗跑一段高速時間，每隔幾秒存一張，並把每次換鏡頭的時間表寫成日誌。
## 用法（要開視窗才畫得出來）：
##   Godot_v4.7.1-stable_win64_console.exe --audio-driver Dummy --path game --position 2600,40 --resolution 1920x1080
##     --script res://tests/harness/director_shots.gd -- --out=資料夾 [--mode=run|card] [--ui=1.0] [--speed=3] [--shots=40] [--every=2.0]
## run：連續截圖加換鏡頭日誌；card：等字卡出現後各存一張字卡的畫面。存檔與設定寫在測試用的資料夾。

const Suite := preload("res://tests/harness/suite.gd")

var _out := "user://director_shots"
var _mode := "run"
var _ui := 1.0
var _speed := 3
var _shots := 40
var _every := 2.0
## --replay=鍵,鍵：先把模擬推一段，挑出對照表鍵（EventScript.row_key，如 Died:Age、ThreadResolved:Feud）相符的真事件，
## 開拍後每隔 _replay_gap 遊戲秒把一件改成大事重新送進導播（為了在一輪裡拍到少見的事件）。
var _replay_keys := PackedStringArray()
var _replay := []
var _replay_gap := 45.0
## --polar=N：另挑南北各 N 件落在極區（焦點限制外，南北緯 75° 以外）的真事件一併重送；--gap=秒：重送間隔（遊戲秒）。
var _polar := 0
var _replay_next := 20.0
var _suite: Suite
var _log := PackedStringArray()
var _t0 := 0
## 引擎回報的處理秒數累計（遊戲裡的真實秒；幀太慢被夾住時比牆上時間短）。
var _game_t := 0.0


func _initialize() -> void:
	_main.call_deferred()


func _main() -> void:
	for arg in OS.get_cmdline_user_args():
		var pair := arg.trim_prefix("--").split("=", true, 1)
		match pair[0]:
			"out":
				_out = pair[1]
			"mode":
				_mode = pair[1]
			"ui":
				_ui = float(pair[1])
			"speed":
				_speed = int(pair[1])
			"shots":
				_shots = int(pair[1])
			"every":
				_every = float(pair[1])
			"replay":
				_replay_keys = pair[1].split(",")
			"polar":
				_polar = int(pair[1])
			"gap":
				_replay_gap = float(pair[1])
				_replay_next = minf(_replay_next, _replay_gap)
	DirAccess.make_dir_recursive_absolute(_out)
	SimWorld.set_default_prerun(1500, 100)
	_suite = Suite.new()
	_suite.tree = self
	_suite._begin_case()
	# 拍圖工具用自己的設定檔，不改玩家的設定。
	GameSettings.use_file("user://harness/director_shots.cfg", true)
	GameSettings.set_value("director", true)
	GameSettings.set_value("ui_scale", _ui)
	var view := Vector2i(root.get_visible_rect().size)
	var game: Node = await _suite.boot(false, view)
	var sim: SimHost = game.get_node("SimHost")
	var camera: MapCamera = game.get_node("MapCamera")
	if not _replay_keys.is_empty() or _polar > 0:
		_collect_replay(sim, camera._map.map_size)
	var hud: Hud = game.get_node("Hud")
	var director: Director = hud._director.director
	sim.set_speed(_speed)
	sim.set_paused(false)
	_t0 = Time.get_ticks_msec()
	director.go.connect(func(province: int, big: bool) -> void:
		_note("去 %d %s%s" % [province, sim.world.province_info(province)["name"], "（大事）" if big else ""]))
	var last_state := director.state
	var card: DirectorCard = hud._director_card
	if _mode == "card":
		while _secs() < 60.0 and not (director.state == Director.State.STAY and card.showing() and card.modulate.a > 0.99):
			await _suite.frames(1)
		await _suite.frames(10)
		await _shot("card_%dx%d_%d" % [view.x, view.y, int(_ui * 100.0)])
		_save_log()
		quit(0)
		return
	if _mode == "arrive":
		await _arrive_run(sim, camera, director, card)
		_save_log()
		quit(0)
		return
	var next_shot := 0.0
	var index := 0
	while index < _shots:
		await _suite.frames(1)
		_game_t += root.get_process_delta_time()
		if director.state != last_state:
			_note("狀態 %s → %s" % [Director.State.keys()[last_state], Director.State.keys()[director.state]])
			if director.state == Director.State.STAY:
				_note("到位 %s，字卡：%s" % [sim.world.province_info(director.current)["name"], card.text()])
			last_state = director.state
		if _secs() >= next_shot:
			next_shot += _every
			var name := "t%03d_s%d_p%d" % [int(_secs()), director.state, director.current]
			_note("截圖 %s 焦點 (%.1f, %.1f) 高 %.1f 速度 %d 字卡 %s" % [name, camera.focus.x, camera.focus.y, camera.z, sim.speed_index, card.text()])
			await _shot(name)
			index += 1
	_save_log()
	quit(0)


func _secs() -> float:
	return float(Time.get_ticks_msec() - _t0) / 1000.0


func _note(text: String) -> void:
	var line := "%7.2f（遊戲 %6.2f）  %s" % [_secs(), _game_t, text]
	print(line)
	_log.append(line)


func _shot(name: String) -> void:
	root.get_texture().get_image().save_png(_out.path_join("%s.png" % name))


func _save_log() -> void:
	var file := FileAccess.open(_out.path_join("log_%d.txt" % int(_ui * 100.0)), FileAccess.WRITE)
	file.store_string("\n".join(_log))


## 到位後 1.0 秒與 3.0 秒（遊戲秒）各拍一張，並記下字卡事件的格投影到螢幕的位置與鏡頭高度；跑 _shots 遊戲秒。
func _arrive_run(sim: SimHost, camera: MapCamera, director: Director, card: DirectorCard) -> void:
	var arrived := -1.0
	var taken := 0
	var last := director.state
	while _game_t < float(_shots):
		await _suite.frames(1)
		_game_t += root.get_process_delta_time()
		if not _replay.is_empty() and _game_t >= _replay_next:
			_replay_next = _game_t + _replay_gap
			var again: Dictionary = _replay.pop_front()
			again["importance"] = 5
			_note("重送 %s %s（州 %d）" % [again["kind"], again.get("x", ""), int(again["province"])])
			director.reset()
			camera.cancel_directed()
			arrived = -1.0
			sim.event_posted.emit(again)
		if director.state == Director.State.STAY and last != Director.State.STAY:
			arrived = _game_t
			taken = 0
		last = director.state
		if director.state != Director.State.STAY:
			arrived = -1.0
		if arrived < 0.0:
			continue
		var since := _game_t - arrived
		var due := 1.0 if taken == 0 else 3.0
		if taken < 2 and since >= due:
			taken += 1
			var event := director.target_event()
			var spot: Variant = event.get("pos")
			var screen := Vector2.INF
			if spot is Vector2:
				var world: Vector3 = camera._map.world_position(spot, camera.focus.x)
				screen = camera.unproject_position(world)
			var name := "p%d_%s_%s%s_t%03d_a%.1f" % [director.current, sim.world.province_info(director.current)["name"], event.get("kind", "?"), event.get("x", ""), int(_game_t), since]
			_note("%s 種類 %s 事件格 %s 治所 %s 螢幕 %s 焦點 (%.1f, %.1f) 高 %.1f/寬 %.0f 字卡 %s" % [name, event.get("kind", "?"), spot,
				sim.world.province_info(director.current)["seat"], screen, camera.focus.x, camera.focus.y, camera.z, camera.map_width(), card.text()])
			_note("DBG 狀態 " + _classify(event, camera))
			await _shot(name)


func _classify(event: Dictionary, camera: MapCamera) -> String:
	if event.is_empty():
		return "空事件"
	if not EventScript.accepts(event):
		return "無演出列"
	var id := int(event.get("id", -1))
	var stage: EventStage = get_nodes_in_group("event_stage")[0]
	for perf in stage.performances():
		if int(perf["id"]) == id:
			if float(perf["age"]) < 0.0:
				return "a排隊中"
			if not (perf["actors"] as Array).is_empty() or perf["light"] != null:
				return "ok播放中有棋子 age=%.1f/%.1f" % [perf["age"], perf["plan"]["total"]]
			var sp: Spectacle = stage._spectacle
			if sp.figure_hidden() or not sp.pos_in_view(perf["pos"]):
				return "d隱藏(figure_hidden=%s in_view=%s)" % [sp.figure_hidden(), sp.pos_in_view(perf["pos"])]
			return "b名額用完 age=%.1f" % perf["age"]
	return "c/b已不在（演完、被擠掉或沒登記）"


## 推一段模擬，每個鍵各挑一件有位置的真事件；--polar 另挑南北極區各 _polar 件（極區 = 焦點限制之外，南北緯 75° 以外）。
func _collect_replay(sim: SimHost, map_size: Vector2) -> void:
	var count := {"north": 0, "south": 0, "low": 1e9, "high": -1e9}
	var seen := {}
	var edge := (90.0 - MapView.FOCUS_MAX_LAT_DEG) / 180.0 * map_size.y
	var grab := func(event: Dictionary) -> void:
		if not event.has("pos") or int(event.get("province", -1)) < 0 or not EventScript.accepts(event):
			return
		var y := (event["pos"] as Vector2).y
		count["low"] = minf(float(count["low"]), y)
		count["high"] = maxf(float(count["high"]), y)
		var key := EventScript.row_key(event)
		if _replay_keys.has(key) and not seen.has(key):
			seen[key] = true
			_replay.append(event.duplicate())
		elif y < edge and int(count["north"]) < _polar:
			count["north"] = int(count["north"]) + 1
			_replay.append(event.duplicate())
		elif y > map_size.y - edge and int(count["south"]) < _polar:
			count["south"] = int(count["south"]) + 1
			_replay.append(event.duplicate())
	sim.event_posted.connect(grab)
	for round in 24:
		sim.advance(240)
		if seen.size() >= _replay_keys.size() and int(count["north"]) >= _polar and int(count["south"]) >= _polar:
			break
	sim.event_posted.disconnect(grab)
	_note("備重送 %d 件：%s；極區北 %d 南 %d；地圖 %s，事件列範圍 %.1f..%.1f，極區界 %.1f" % [_replay.size(), ",".join(seen.keys()), count["north"], count["south"], map_size, count["low"], count["high"], edge])
