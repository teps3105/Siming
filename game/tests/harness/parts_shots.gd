extends SceneTree
## 部件庫與頭像的畫面證據（GDD 8.12 驗收）：人物卡開著的截圖、30 人的佔位頭像並排、同一人少壯老入魔身故並排，
## 與 GPU（著色器）對 CPU（PartCompose）的逐像素核對。要開視窗才畫得出來。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy --resolution 1920x1080 --script res://tests/harness/parts_shots.gd -- --out=<資料夾>

const Suite := preload("res://tests/harness/suite.gd")

var _out := "user://parts_shots"
var _suite: Suite


## 一張並排圖：每格一個 PartStack，下面一行小字。
class Sheet:
	extends Control

	var labels: Array[String] = []
	var columns := 10
	var cell := Vector2(170.0, 232.0)

	func _draw() -> void:
		draw_rect(Rect2(Vector2.ZERO, size), Ink.PAPER, true)
		for i in labels.size():
			var at := Vector2(float(i % columns) * cell.x + 5.0, float(i / columns) * cell.y + 206.0)
			Ink.text(self, at, labels[i], 13, Ink.INK_LIGHT)


func _initialize() -> void:
	_main.call_deferred()


func _main() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
	DirAccess.make_dir_recursive_absolute(_out)
	PersonCard.show_placeholder = true
	_suite = Suite.new()
	_suite.tree = self
	_suite._begin_case()
	GameSettings.load_file()
	await _gpu_vs_cpu()
	await _cards()
	await _suite._finish_case()
	Suite.wipe_storage()
	quit(0)


func _save(image: Image, name: String) -> void:
	image.save_png(_out.path_join("%s.png" % name))
	print("截圖：%s" % name)


func _shot(name: String) -> void:
	await _suite.wait(0.6)
	_save(root.get_texture().get_image(), name)


## 一個 PartStack 在 SubViewport 裡以 1:1 畫出來，與 CPU 合成逐像素比。
func _gpu_vs_cpu() -> void:
	var catalog := PartCatalog.load_class("portrait")
	var report := []
	for id in [3, 12, 21]:
		var info := _person_like(id, {"realm": -1})
		var recipe := PortraitRecipe.from_person(info)
		var viewport := SubViewport.new()
		viewport.size = Vector2i(320, 400)
		viewport.transparent_bg = true
		viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
		var stack := PartStack.new()
		stack.size = Vector2(320.0, 400.0)
		viewport.add_child(stack)
		root.add_child(viewport)
		stack.set_recipe(recipe, catalog)
		await _suite.frames(4)
		var gpu := viewport.get_texture().get_image()
		gpu.convert(Image.FORMAT_RGBA8)
		var cpu := PartCompose.render(recipe, catalog, 1)
		var worst := 0
		var off := 0
		var total := 0
		for y in 400:
			for x in 320:
				var a := gpu.get_pixel(x, y)
				var b := cpu.get_pixel(x, y)
				# 半透明邊緣在透明底的 SubViewport 讀回時顏色已混過底，只比兩邊都完全不透明的像素。
				if a.a < 0.99 or b.a < 0.99:
					continue
				total += 1
				var d := maxi(maxi(absi(roundi(a.r * 255) - roundi(b.r * 255)), absi(roundi(a.g * 255) - roundi(b.g * 255))), maxi(absi(roundi(a.b * 255) - roundi(b.b * 255)), absi(roundi(a.a * 255) - roundi(b.a * 255))))
				worst = maxi(worst, d)
				if d > 3:
					off += 1
		report.append("id %d：完全不透明的像素 %d，差超過 3/255 的 %d，最大差 %d/255" % [id, total, off, worst])
		_save(gpu, "gpu_%d" % id)
		_save(cpu, "cpu_%d" % id)
		root.remove_child(viewport)
		viewport.queue_free()
	for line: String in report:
		print("GPU對CPU：", line)
	var file := FileAccess.open(_out.path_join("gpu_vs_cpu.txt"), FileAccess.WRITE)
	file.store_string("\n".join(report))


## 橋接 person() 沒有的人物，用來擺各種狀態：以真人資料為底再覆蓋。
func _person_like(id: int, overrides := {}) -> Dictionary:
	var info := {
		"id": id, "soul": id * 3 + 1, "gender": "Male" if id % 2 == 0 else "Female", "role": "Cultivator", "power_name": "",
		"is_leader": false, "age": 40, "lifespan": 100, "alive": true, "realm": 1, "fruit": -1,
	}
	info.merge(overrides, true)
	return info


func _sheet(items: Array, labels: Array[String], name: String, columns: int) -> void:
	var catalog := PartCatalog.load_class("portrait")
	var sheet := Sheet.new()
	sheet.labels = labels
	sheet.columns = columns
	var rows := ceili(float(items.size()) / columns)
	root.size = Vector2i(roundi(columns * sheet.cell.x), roundi(rows * sheet.cell.y))
	sheet.size = Vector2(root.size)
	# 蓋在整個遊戲畫面之上（遊戲留在樹上，只是被蓋住）。
	var layer := CanvasLayer.new()
	layer.layer = 100
	root.add_child(layer)
	layer.add_child(sheet)
	for i in items.size():
		var stack := PartStack.new()
		stack.size = Vector2(160.0, 200.0)
		stack.position = Vector2(float(i % columns) * sheet.cell.x + 5.0, float(i / columns) * sheet.cell.y + 4.0)
		sheet.add_child(stack)
		stack.set_recipe(PortraitRecipe.from_person(items[i]), catalog)
	await _shot(name)
	root.remove_child(layer)
	layer.queue_free()
	await _suite.frames(2)


func _cards() -> void:
	for layout: Array in [["1920x1080_100", Vector2i(1920, 1080), 1.0], ["1280x800_100", Vector2i(1280, 800), 1.0], ["1280x800_150", Vector2i(1280, 800), 1.5]]:
		GameSettings.set_value("ui_scale", layout[2])
		var game: Node = await _suite.boot(false, layout[1])
		root.size = layout[1]
		var sim: SimHost = game.get_node("SimHost")
		sim.world.step(12 * 40)
		sim.world.flush_terrain()
		var hud: Hud = game.get_node("Hud")
		var pick := _pick_cards(sim)
		if layout[0] == "1920x1080_100":
			# 30 位有名之人、同一人五種狀態：用真人資料。
			await _gallery(sim)
			root.size = layout[1]
			await _suite.frames(2)
		for key_name: String in ["alive", "dead", "demon"]:
			if pick[key_name] < 0:
				continue
			hud.open_person(pick[key_name])
			await _suite.wait(0.7)
			await _shot("card_%s_%s" % [layout[0], key_name])
		root.remove_child(game)
		game.queue_free()
		await _suite.frames(2)
	GameSettings.set_value("ui_scale", 1.0)


## 要截圖的人：歷程長的在世者、歷程長的已歿者、入魔者（沒有就略過）。
func _pick_cards(sim: SimHost) -> Dictionary:
	var out := {"alive": -1, "dead": -1, "demon": -1}
	var most := {"alive": -1, "dead": -1}
	var id := 0
	while not sim.world.person(id).is_empty():
		var info: Dictionary = sim.world.person(id)
		var key := "alive" if info["alive"] else "dead"
		var count: int = sim.world.person_history(id).size()
		if count > most[key]:
			most[key] = count
			out[key] = id
		if info["role"] == "Demonic" and info["alive"] and out["demon"] < 0:
			out["demon"] = id
		id += 1
	return out


func _gallery(sim: SimHost) -> void:
	var alive: Array = []
	var id := 0
	while not sim.world.person(id).is_empty():
		if sim.world.person(id)["alive"]:
			alive.append(id)
		id += 1
	var picked: Array = []
	var labels: Array[String] = []
	for i in mini(30, alive.size()):
		var person_id: int = alive[i * alive.size() / 30]
		var info: Dictionary = sim.world.person(person_id)
		picked.append(info)
		labels.append("%d %s %s" % [person_id, info["name"], info["role"]])
	await _sheet(picked, labels, "gallery_30", 10)
	# 同一人少、壯、老、入魔、身故：一位無門派的散修男子。
	var base := {}
	for pid: int in alive:
		var info: Dictionary = sim.world.person(pid)
		if info["role"] == "Cultivator" and info["gender"] == "Male" and String(info["power_name"]).is_empty() and info["realm"] >= 1:
			base = info
			break
	if base.is_empty():
		base = _person_like(10)
	var states: Array = []
	var names: Array[String] = ["少", "壯", "老", "入魔", "身故"]
	for spec: Array in [[0.12, "Cultivator", true], [0.4, "Cultivator", true], [0.85, "Cultivator", true], [0.4, "Demonic", true], [0.85, "Cultivator", false]]:
		var state := base.duplicate()
		state["age"] = int(float(base["lifespan"]) * spec[0])
		state["role"] = spec[1]
		state["alive"] = spec[2]
		states.append(state)
	await _sheet(states, names, "same_person_5", 5)
