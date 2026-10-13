extends SceneTree
## 讀圖工具：開真正的遊戲，在鏡頭中心放一場天劫奇觀，依方位角與鏡頭高度各拍幾張（人物的迎雷、受雷、站立）。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy --resolution 1920x1080 -s res://tests/harness/figure_game_view.gd -- --out=<資料夾>
## 視窗要開（不能 --headless）。

const BOOTSTRAP := "res://app/scenes/bootstrap.tscn"

var _out := ""
var _tag := ""


func _initialize() -> void:
	_main.call_deferred()


func _shot(name: String) -> void:
	for i in 8:
		await process_frame
	root.get_texture().get_image().save_png(_out.path_join("%s.png" % name))
	print("截圖：", name)


func _main() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--tag="):
			_tag = arg.trim_prefix("--tag=")
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
	var map: TerrainMap = game.find_children("*", "TerrainMap", true, false)[0]
	var who := -1
	for id in 400:
		var info := sim.world.person(id)
		if not info.is_empty() and int(info.get("age", 0)) >= 20:
			who = id
			print("當事人 ", id, " ", info.get("gender"), " 年", info.get("age"))
			break
	var focus := camera.focus
	print("鏡頭焦點 ", focus, " z ", camera.z, " 地圖寬 ", map.map_size.x, " 俯角 ", rad_to_deg(MapView.pitch()))
	var cases := [
		{"name": "az0_near", "yaw": 0.0, "z": 30.0},
		{"name": "az0_far03", "yaw": 0.0, "z": 0.3 * map.map_size.x},
		{"name": "azp45_near", "yaw": 45.0, "z": 30.0},
		{"name": "azm45_near", "yaw": -45.0, "z": 30.0},
		{"name": "azp45_far03", "yaw": 45.0, "z": 0.3 * map.map_size.x},
		{"name": "az0_toofar", "yaw": 0.0, "z": 0.5 * map.map_size.x},
	]
	# 山前：找一格「正北方三格比它高很多」的陸地，人物後仰時頭部會疊到山上。
	var best := Vector2.ZERO
	var best_score := 0.0
	for gy in range(8, int(map.map_size.y) - 8, 2):
		for gx in range(8, int(map.map_size.x) - 8, 2):
			var here := map.surface_height(float(gx), float(gy))
			var behind := map.surface_height(float(gx), float(gy - 3))
			if here > 0.0 and behind - here > best_score:
				best_score = behind - here
				best = Vector2(gx, gy)
	print("山前格 ", best, " 北三格高差 ", best_score)
	var mount := [
		{"name": "mount_az0_near", "yaw": 0.0, "z": 30.0, "pull": 2.0},
		{"name": "mount_az0_near_nopull", "yaw": 0.0, "z": 30.0, "pull": 0.0},
		{"name": "mount_azp45_near", "yaw": 45.0, "z": 30.0, "pull": 2.0},
		{"name": "mount_azp45_near_nopull", "yaw": 45.0, "z": 30.0, "pull": 0.0},
	]
	for entry: Dictionary in mount:
		FigureActor.depth_pull_factor = float(entry["pull"])
		camera._target_yaw = deg_to_rad(float(entry["yaw"]))
		camera.look_at_map(best, float(entry["z"]), true)
		for i in 6:
			await process_frame
		spectacle.clear()
		spectacle.post({"id": 90002, "kind": "Tribulation", "pos": best, "n": 3, "x": "Passed", "importance": 3, "a_person": who})
		spectacle.step(0.9)
		await _shot(str(entry["name"]))
	FigureActor.depth_pull_factor = 2.0
	if _tag == "globe":
		camera.set_globe(true)
		for entry: Dictionary in [{"name": "globe_near", "z": 40.0}, {"name": "globe_far03", "z": 0.3 * map.map_size.x}]:
			camera.look_at_map(best, float(entry["z"]), true)
			for i in 40:
				await process_frame
			spectacle.clear()
			spectacle.post({"id": 90003, "kind": "Tribulation", "pos": best, "n": 3, "x": "Passed", "importance": 3, "a_person": who})
			spectacle.step(0.9)
			await _shot(str(entry["name"]))
		quit()
		return
	if _tag == "mount":
		quit()
		return
	for entry: Dictionary in cases:
		camera._target_yaw = deg_to_rad(float(entry["yaw"]))
		camera.look_at_map(focus, float(entry["z"]), true)
		for i in 6:
			await process_frame
		for moment: Array in [["brace", 0.9], ["struck", 1.55], ["stand", 3.6]]:
			spectacle.clear()
			spectacle.post({"id": 90001, "kind": "Tribulation", "pos": focus, "n": 3, "x": "Passed", "importance": 3, "a_person": who})
			spectacle.step(float(moment[1]))
			await _shot("%s_%s" % [entry["name"], moment[0]])
	quit()
