extends SceneTree
## 讀圖工具：開真正的遊戲，在鏡頭中心擺一枚棋子，依序擺出每個動作的關鍵格各拍一張（裁成人物周圍）。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy --resolution 1920x1080 -s res://tests/harness/figure_action_view.gd -- --out=<資料夾> [--actions=walk,fall,...]
## 視窗要開（不能 --headless）。關鍵格取動作資料的事件格，加上第 0 格與最後一格。

const BOOTSTRAP := "res://app/scenes/bootstrap.tscn"
const CROP := Rect2i(560, 250, 800, 640)

var _out := ""
var _only: PackedStringArray = []


func _initialize() -> void:
	_main.call_deferred()


func _shot(name: String) -> void:
	for i in 8:
		await process_frame
	root.get_texture().get_image().get_region(CROP).save_png(_out.path_join("%s.png" % name))
	print("截圖：", name)


func _main() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--actions="):
			_only = arg.trim_prefix("--actions=").split(",")
	DirAccess.make_dir_recursive_absolute(_out)
	GameSettings.file_path = _out.path_join("settings-figure.cfg")
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
	var sim: SimHost = game.get_node("SimHost")
	sim.set_paused(true)
	for i in 40:
		await process_frame
	var spectacle: Spectacle = get_first_node_in_group("spectacle")
	var camera: MapCamera = game.find_children("*", "MapCamera", true, false)[0]
	var focus := camera.focus
	camera._target_yaw = 0.0
	camera.look_at_map(focus, 30.0, true)
	for i in 6:
		await process_frame
	spectacle.clear()
	spectacle.post({"id": 90001, "kind": "Tribulation", "pos": focus, "n": 3, "x": "Passed", "importance": 3, "a_person": -1,
		"given": {"sex": 0, "age": 1, "school": 1, "prop": 0}})
	spectacle.step(3.6)
	var actor: FigureActor = (game.find_children("*", "FigureActor", true, false) as Array)[0]
	# 奇觀每幀會把棋子重設成天劫的動作；停掉奇觀，只留棋子自己更新，換成指定的樣子（與排片同一人）。
	spectacle.process_mode = Node.PROCESS_MODE_DISABLED
	actor.process_mode = Node.PROCESS_MODE_ALWAYS
	actor.setup(FigureActor.look_of(25, {"sex": 0, "age": 1, "school": 1, "prop": 0}))
	for action: String in FigureMotion.names():
		if not _only.is_empty() and not _only.has(action):
			continue
		var marks := {0: "start", FigureMotion.frame_count(action): "end"}
		var events := FigureMotion.events(action)
		for key: String in events:
			marks[int(events[key])] = key
		var frames := marks.keys()
		frames.sort()
		for frame: int in frames:
			actor.play(action, frame / 24.0)
			await _shot("%s_f%02d_%s" % [action, frame, marks[frame]])
	quit()
