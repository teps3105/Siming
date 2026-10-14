extends SceneTree
## 讀圖工具（5.13 棋子演出）：對照表指定的列，各拍定場、動作中（多步驟的列再拍後段）、停格三張。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy --resolution 1920x1080 -s res://tests/harness/event_stage_rows.gd -- --out=<資料夾> [--rows=Married,ThreadResolved:Love ...；預設全部]
## 視窗要開（不能 --headless）。

const BOOTSTRAP := "res://app/scenes/bootstrap.tscn"

var _out := ""
var _rows: PackedStringArray = []
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
		elif arg.begins_with("--rows="):
			_rows = arg.trim_prefix("--rows=").split(",")
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
	var waited := 0
	while _stage == null and waited < 3000:
		await process_frame
		waited += 1
		_stage = get_first_node_in_group("event_stage")
	if _stage == null:
		print("找不到事件舞台")
		quit()
		return
	for id in 400:
		var info := _sim.world.person(id)
		if not info.is_empty() and int(info.get("age", 0)) >= 20 and _people.size() < 6:
			_people.append(id)
	var focus: Vector2 = _sim.world.province_info(0)["seat"]
	print("鏡頭焦點 ", focus, " 當事人 ", _people)
	var cases := [
		["ThreadCreated", {"x": "Mentor"}], ["ThreadResolved", {"x": "Mentor"}], ["Married", {}], ["ThreadResolved", {"x": "Love"}],
		["ThreadResolved", {"x": "Grace"}], ["Died", {"x": "Age"}], ["Breakthrough", {}], ["SectSplit", {}],
		["Killed", {"a_path": 0, "tag": "劍訣"}], ["Killed", {"a_path": 1, "tag": "劈"}], ["Killed", {"a_path": 2, "tag": "掌"}],
		["ThreadResolved", {"x": "Feud", "b_path": 1, "survivor_alive": false, "tag": "倒"}], ["ThreadResolved", {"x": "Feud", "b_path": 2, "survivor_alive": true, "tag": "擋"}],
		["ThreadResolved", {"x": "Grudge"}], ["Succession", {}], ["PowerFounded", {}], ["FellToDemon", {}], ["Died", {"x": "Plague"}],
	]
	_camera.look_at_map(focus, 30.0, true)
	for i in 8:
		await process_frame
	for entry: Array in cases:
		var label := "%s_%s%s" % [entry[0], str(entry[1].get("x", "")), str(entry[1].get("tag", ""))]
		if not _rows.is_empty() and not _rows.has(label):
			continue
		_stage.clear()
		var event := _event(entry[0], focus, entry[1])
		_stage.post(event)
		var plan := EventScript.plan(_stage.performances()[0]["event"])
		var span := float(plan["action_len"])
		_stage.advance(0.5)
		await _shot("%s_1定場" % label)
		_stage.advance(EventScript.INTRO - 0.5 + span * 0.5)
		await _shot("%s_2動作中" % label)
		if (plan["actors"][0]["steps"] as Array).size() > 1:
			_stage.advance(span * 0.3)
			await _shot("%s_3動作後段" % label)
			_stage.advance(span * 0.2 + 0.4)
		else:
			_stage.advance(span * 0.5 + 0.4)
		await _shot("%s_4停格" % label)
	quit()
