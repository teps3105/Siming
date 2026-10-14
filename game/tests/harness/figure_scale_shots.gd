extends SceneTree
## 讀圖工具（棋子大小綁地面）：同一個種子、同一件兩人演出的事件（Married），
## 對 --cells 列出的每個棋子高度（格）各拍三個鏡頭高度：導播到位高度、地圖寬 0.25 倍、玩家可拉到的最近。
## 檔名 cells<格數>_<鏡頭>_z<高度>.png，並印出每張棋子的畫面像素高度（公式值）。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy --resolution 1920x1080 -s res://tests/harness/figure_scale_shots.gd -- --out=<資料夾> [--cells=1,2,3]
## 視窗要開（不能 --headless）。

const BOOTSTRAP := "res://app/scenes/bootstrap.tscn"

var _out := ""
var _cells: Array[float] = [1.0, 2.0, 3.0]
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
		elif arg.begins_with("--cells="):
			_cells.clear()
			for part in arg.trim_prefix("--cells=").split(","):
				_cells.append(float(part))
	DirAccess.make_dir_recursive_absolute(_out)
	GameSettings.file_path = _out.path_join("settings-scale.cfg")
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
	var rig: DirectorRig = game.get_node("Hud")._director
	for id in 400:
		var info := _sim.world.person(id)
		if not info.is_empty() and int(info.get("age", 0)) >= 20 and _people.size() < 2:
			_people.append(id)
	var focus: Vector2 = _sim.world.province_info(0)["seat"]
	var width := _map.map_size.x
	var view := Vector2(root.get_visible_rect().size)
	print("鏡頭焦點 ", focus, " 地圖寬 ", width, " 視窗 ", view, " 當事人 ", _people)
	for cells in _cells:
		FigureActor.cells_override = cells
		var heights := [["landing", rig.landing_height()], ["quarter", width * 0.25], ["nearest", MapView.z_range(width).x]]
		for entry: Array in heights:
			var z: float = entry[1]
			_camera.look_at_map(focus, z, true)
			for i in 8:
				await process_frame
			_stage.clear()
			_stage.post(_event("Married", focus))
			var plan := EventScript.plan(_stage.performances()[0]["event"])
			_stage.advance(EventScript.INTRO + float(plan["action_len"]) * 0.6)
			var px := FigureActor.screen_height(view.y / MapView.view_height(z))
			print("格數 %.0f 鏡頭 %s z=%.2f（地圖寬的 %.3f 倍）棋子畫面高 %.1f px" % [cells, entry[0], z, z / width, px])
			await _shot("cells%d_%s_z%.1f" % [int(cells), entry[0], z])
	quit()
