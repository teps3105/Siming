extends SceneTree
## 導演模式（GDD 5.13）的實機截圖：開視窗跑一段高速時間，每隔幾秒存一張，並把每次換鏡頭的時間表寫成日誌。
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
	DirAccess.make_dir_recursive_absolute(_out)
	SimWorld.set_default_prerun(1500, 100)
	_suite = Suite.new()
	_suite.tree = self
	_suite._begin_case()
	GameSettings.set_value("director", true)
	GameSettings.set_value("ui_scale", _ui)
	var view := Vector2i(root.get_visible_rect().size)
	var game: Node = await _suite.boot(false, view)
	var sim: SimHost = game.get_node("SimHost")
	var camera: MapCamera = game.get_node("MapCamera")
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
			var name := "p%d_%s_t%03d_a%.1f" % [director.current, sim.world.province_info(director.current)["name"], int(_game_t), since]
			_note("%s 種類 %s 事件格 %s 治所 %s 螢幕 %s 焦點 (%.1f, %.1f) 高 %.1f/寬 %.0f 字卡 %s" % [name, event.get("kind", "?"), spot,
				sim.world.province_info(director.current)["seat"], screen, camera.focus.x, camera.focus.y, camera.z, camera.map_width(), card.text()])
			await _shot(name)
