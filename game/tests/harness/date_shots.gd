extends SceneTree
## 日期雙表（決策 #91）的畫面檢查：四種視圖（1280×800、1920×1080，各 100%、150%）各存題簽、傳聞簽展開、
## 一張人物卡、一張有歷任掌門的宗門卡。要開視窗才畫得出來；開發時手動執行，不屬於測試套件。

const Suite := preload("res://tests/harness/suite.gd")
const VIEWS := [
	["deck100", Vector2i(1280, 800), 1.0],
	["deck150", Vector2i(1280, 800), 1.5],
	["full100", Vector2i(1920, 1080), 1.0],
	["full150", Vector2i(1920, 1080), 1.5],
]

var _out := "user://date_shots"
var _suite: Suite


func _initialize() -> void:
	_main.call_deferred()


func _main() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
	DirAccess.make_dir_recursive_absolute(_out)
	_suite = Suite.new()
	_suite.tree = self
	_suite._begin_case()
	GameSettings.load_file()
	for view: Array in VIEWS:
		await _view(view[0], view[1], view[2])
	await _suite._finish_case()
	Suite.wipe_storage()
	quit(0)


func _shot(name: String) -> void:
	await _suite.wait(0.5)
	root.get_texture().get_image().save_png(_out.path_join("%s-b.png" % name))
	print("截圖：%s" % name)


func _view(prefix: String, view: Vector2i, scale: float) -> void:
	# 拍圖工具用自己的設定檔，不改玩家的設定。
	GameSettings.use_file("user://harness/date_shots.cfg", true)
	GameSettings.set_value("ui_scale", scale)
	GameSettings.set_value("tutorial_done", true)
	var game: Node = await _suite.boot(false, view)
	root.size = view
	await _suite.frames(4)
	var hud: Hud = game.get_node("Hud")
	var sim: SimHost = game.get_node("SimHost")
	var world: SimWorld = sim.world
	world.drain_events()
	var people := {}
	for _round in 36:
		world.step(12 * 4)
		for event: Dictionary in world.drain_events():
			if int(event["importance"]) >= 3 and int(event["a_person"]) >= 0:
				people[int(event["a_person"])] = true
	# 最後一年的大事進傳聞簽。
	world.step(12)
	for event: Dictionary in world.drain_events():
		if int(event["importance"]) >= 3:
			sim.event_posted.emit(event)
	await _suite.frames(3)
	hud._dial.queue_redraw()
	await _shot("%s-1-dial" % prefix)
	# 句子長到與年月同一行放不下的簽：年月應移到第二行。
	sim.event_posted.emit({"id": 99001, "tick": 20000, "kind": "Killed", "importance": 3, "province": 0, "cell": -1,
		"souls": PackedInt32Array(), "a": "沈青霜青霜青霜青霜", "b": "韓烈韓烈韓烈韓烈韓烈", "f": "", "g": "", "p": "青州青州青州青州",
		"q": "", "x": "", "n": 0, "m": 0, "year": 99, "world_year": 12345})
	await _suite.frames(3)
	await _shot("%s-6-rumor-collapsed" % prefix)
	hud.rumor().expanded = true
	await _shot("%s-2-rumor" % prefix)
	hud.rumor().expanded = false
	var best := -1
	var most := -1
	for id: int in people:
		var count := world.person_history(id).size()
		if count > most:
			most = count
			best = id
	hud.open_person(best)
	await _shot("%s-3-person" % prefix)
	var sect := -1
	var runs := -1
	for row: Dictionary in world.sects():
		var succession := 0
		for entry: Dictionary in world.sect_history(row["id"]):
			if entry["kind"] == "Succession":
				succession += 1
		if succession > runs:
			runs = succession
			sect = row["id"]
	hud.open_sect(sect)
	await _suite.wait(0.4)
	await _shot("%s-4-sect" % prefix)
	var card: SectCard = hud.sect_card()
	for row: Dictionary in card._rows:
		if row["kind"] == "leaders":
			var list := card.list()
			list.scroll = clampf(float(row["top"]) - 4.0, 0.0, list.max_scroll())
			break
	await _shot("%s-5-sect-leaders" % prefix)
	root.remove_child(game)
	game.queue_free()
	await _suite.frames(2)
