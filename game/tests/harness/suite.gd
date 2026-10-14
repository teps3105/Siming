extends RefCounted
## 測試套件的基類（GDD 6.7）。test_ 開頭的方法就是案例，可以是協程；執行程式（run.gd）一律等待它跑完。
## 規則測試只用斷言；流程測試另外用 boot() 開起遊戲，再以 key()、joy_button()、joy_axis()、
## click()、wheel()、drag()、tap()、touch()、touch_move()、two_fingers() 送輸入——全部走引擎的輸入分派，與玩家按下去的路徑相同。
## 判決在 run.gd：斷言失敗、案例期間出現沒有事先宣告的錯誤、或一個斷言都沒有，都算失敗。

const BOOTSTRAP := "res://app/scenes/bootstrap.tscn"
## 流程測試用的世界：固定種子、小尺寸（開得快）。
const TEST_SEED := 20261011
const TEST_SIZE := "small"
## 測試期間存檔與設定的位置（預設沒有任何檔案）；需要寫檔的套件在 setup 改到它底下自己的資料夾。
## 每個測試行程一個資料夾：幾個工作樹同時跑測試時不會讀到彼此寫的檔。
static var TEST_STORAGE := "user://test_run/%d" % OS.get_process_id()
## 流程測試的畫面大小。
const FLOW_VIEW := Vector2i(1920, 1080)

## 執行程式在跑每個套件前填入。
var tree: SceneTree

var _failed := false
var _message := ""
var _assertions := 0
var _expected_errors: Array[String] = []
var _tracked: Array[Object] = []


## 套件的名稱，報告與 --suite 篩選用。
func suite_name() -> String:
	return "unnamed"


## 每個案例之前、之後各呼叫一次；可以是協程。
func setup() -> void:
	pass


func teardown() -> void:
	pass


## 登記要在案例結束時釋放的物件（節點或一般 Object）；RefCounted 不必登記。
func track(obj: Object) -> Object:
	if obj != null and not obj is RefCounted:
		_tracked.append(obj)
	return obj


## 事先宣告這個案例會出現一則含有 text 的錯誤；沒宣告的錯誤會讓案例失敗。
func expect_error(text: String) -> void:
	_expected_errors.append(text)


func assert_true(condition: bool, msg: String = "") -> void:
	_check(condition, "預期為真" if msg.is_empty() else msg)


func assert_false(condition: bool, msg: String = "") -> void:
	_check(not condition, "預期為假" if msg.is_empty() else msg)


func assert_eq(actual: Variant, expected: Variant, msg: String = "") -> void:
	_check(_same(actual, expected), "預期 %s，實際 %s%s" % [expected, actual, _note(msg)])


func assert_ne(actual: Variant, not_expected: Variant, msg: String = "") -> void:
	_check(not _same(actual, not_expected), "不該等於 %s%s" % [not_expected, _note(msg)])


func assert_gt(actual: Variant, threshold: Variant, msg: String = "") -> void:
	_check(actual > threshold, "預期大於 %s，實際 %s%s" % [threshold, actual, _note(msg)])


func assert_lt(actual: Variant, threshold: Variant, msg: String = "") -> void:
	_check(actual < threshold, "預期小於 %s，實際 %s%s" % [threshold, actual, _note(msg)])


## 開起整個遊戲（入口場景，小尺寸世界），回傳入口節點；案例結束時自動關掉。時間先暫停，由案例自己放行。
## opening_flight 為真時保留開場運鏡，否則鏡頭直接落在開場位置。
func boot(opening_flight := false, view := FLOW_VIEW, show_guide := false) -> Node:
	# 無視窗模式的畫面預設只有 64×64，器物會蓋住整個畫面；預設固定成 1920×1080。
	tree.root.size = view
	var game: Node = (load(BOOTSTRAP) as PackedScene).instantiate()
	game.start_seed = TEST_SEED
	game.start_size = TEST_SIZE
	game.opening_flight = opening_flight
	game.auto_save = false
	game.show_guide = show_guide
	track(game)
	tree.root.add_child(game)
	await frames(2)
	AudioServer.set_bus_mute(0, true)
	(game.get_node("SimHost") as SimHost).set_paused(true)
	return game


## 等幾個畫面更新。
func frames(count: int = 1) -> void:
	for i in count:
		await tree.process_frame


## 等 seconds 秒。
func wait(seconds: float) -> void:
	await tree.create_timer(seconds).timeout


## 按一下鍵盤的鍵（按下與放開各一次）。
func key(keycode: Key) -> void:
	for pressed: bool in [true, false]:
		var event := InputEventKey.new()
		event.keycode = keycode
		event.physical_keycode = keycode
		event.pressed = pressed
		Input.parse_input_event(event)
		await frames(2)


## 按一下搖桿的鍵。
func joy_button(button: JoyButton) -> void:
	for pressed: bool in [true, false]:
		var event := InputEventJoypadButton.new()
		event.button_index = button
		event.pressed = pressed
		Input.parse_input_event(event)
		await frames(2)


## 按住搖桿的鍵 seconds 秒再放開。
func joy_hold(button: JoyButton, seconds: float) -> void:
	for pressed: bool in [true, false]:
		var event := InputEventJoypadButton.new()
		event.button_index = button
		event.pressed = pressed
		Input.parse_input_event(event)
		if pressed:
			await wait(seconds)
	await frames(2)


## 把搖桿的軸推到 value 並維持 seconds 秒，再放回中間。
func joy_axis(axis: JoyAxis, value: float, seconds: float) -> void:
	for target: float in [value, 0.0]:
		var event := InputEventJoypadMotion.new()
		event.axis = axis
		event.axis_value = target
		Input.parse_input_event(event)
		if target != 0.0:
			await wait(seconds)
	await frames(2)


## 滑鼠左鍵在螢幕位置 pos 點一下。
func click(pos: Vector2) -> void:
	for pressed: bool in [true, false]:
		Input.parse_input_event(_mouse_button(MOUSE_BUTTON_LEFT, pressed, pos))
		await frames(2)


## 滑鼠移到螢幕位置 pos（不按鍵）。
func move_mouse(pos: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = pos
	event.global_position = pos
	Input.parse_input_event(event)
	await frames(2)


## 滾輪在螢幕位置 pos 滾一格；up 為真是往上（拉近）。
func wheel(pos: Vector2, up: bool) -> void:
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_WHEEL_UP if up else MOUSE_BUTTON_WHEEL_DOWN, true, pos))
	await frames(2)


## 按住左鍵從 from 拖到 to。
func drag(from: Vector2, to: Vector2) -> void:
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_LEFT, true, from))
	await frames(1)
	var steps := 8
	for i in steps:
		var motion := InputEventMouseMotion.new()
		motion.position = from.lerp(to, float(i + 1) / steps)
		motion.global_position = motion.position
		motion.relative = (to - from) / steps
		motion.button_mask = MOUSE_BUTTON_MASK_LEFT
		Input.parse_input_event(motion)
		await frames(1)
	Input.parse_input_event(_mouse_button(MOUSE_BUTTON_LEFT, false, to))
	await frames(2)


## 手指 index 在螢幕位置 pos 按下或放開。
func touch(index: int, pos: Vector2, pressed: bool) -> void:
	var event := InputEventScreenTouch.new()
	event.index = index
	event.position = pos
	event.pressed = pressed
	Input.parse_input_event(event)
	await frames(1)


## 已按下的手指 index 從 from 滑到 to。
func touch_move(index: int, from: Vector2, to: Vector2) -> void:
	var steps := 8
	for i in steps:
		_touch_step(index, from, to, i, steps)
		await frames(1)


## 手指在螢幕位置 pos 點一下。
func tap(pos: Vector2) -> void:
	await touch(0, pos, true)
	await frames(1)
	await touch(0, pos, false)
	await frames(1)


## 兩指同時按下，各自從起點滑到終點，再一起放開。
func two_fingers(a_from: Vector2, a_to: Vector2, b_from: Vector2, b_to: Vector2) -> void:
	await touch(0, a_from, true)
	await touch(1, b_from, true)
	var steps := 8
	for i in steps:
		_touch_step(0, a_from, a_to, i, steps)
		_touch_step(1, b_from, b_to, i, steps)
		await frames(1)
	await touch(1, b_to, false)
	await touch(0, a_to, false)
	await frames(1)


func _touch_step(index: int, from: Vector2, to: Vector2, step: int, steps: int) -> void:
	var event := InputEventScreenDrag.new()
	event.index = index
	event.position = from.lerp(to, float(step + 1) / steps)
	event.relative = (to - from) / steps
	Input.parse_input_event(event)


func _mouse_button(button: MouseButton, pressed: bool, pos: Vector2) -> InputEventMouseButton:
	var event := InputEventMouseButton.new()
	event.button_index = button
	event.pressed = pressed
	event.position = pos
	event.global_position = pos
	return event


func _check(passed: bool, message: String) -> void:
	_assertions += 1
	if not passed and not _failed:
		_failed = true
		_message = message


func _same(a: Variant, b: Variant) -> bool:
	if typeof(a) != typeof(b) and not (a is int and b is float) and not (a is float and b is int):
		# 陣列與字典的型別標記不同（Array 與 Array[int]）時仍比內容。
		if not ((a is Array and b is Array) or (a is Dictionary and b is Dictionary)):
			return false
	return a == b


func _note(msg: String) -> String:
	return "" if msg.is_empty() else "（%s）" % msg


## 刪掉這次測試寫下的存檔與設定（執行程式結束前呼叫）。
static func wipe_storage(dir: String = TEST_STORAGE) -> void:
	var path := ProjectSettings.globalize_path(dir)
	if not DirAccess.dir_exists_absolute(path):
		return
	for sub in DirAccess.get_directories_at(path):
		wipe_storage(dir.path_join(sub))
	for file in DirAccess.get_files_at(path):
		DirAccess.remove_absolute(path.path_join(file))
	DirAccess.remove_absolute(path)


## 執行程式在每個案例前後呼叫。
func _begin_case() -> void:
	_failed = false
	_message = ""
	_assertions = 0
	_expected_errors.clear()
	# 存檔與設定指到測試用的位置：測試不讀也不寫玩家的檔案；設定每個案例從預設值開始，前一案例寫的值不帶過來。
	SaveStore.root = TEST_STORAGE.path_join("saves")
	GameSettings.use_file(TEST_STORAGE.path_join("settings.cfg"), true)


func _finish_case() -> void:
	for obj in _tracked:
		if not is_instance_valid(obj):
			continue
		if obj is Node:
			var node := obj as Node
			if node.get_parent() != null:
				node.get_parent().remove_child(node)
			node.queue_free()
		else:
			obj.free()
	_tracked.clear()
	# 放開所有可能還按著的動作，免得影響下一個案例。
	for action in InputMap.get_actions():
		Input.action_release(action)
	tree.paused = false
	Engine.time_scale = 1.0
	SaveStore.root = SaveStore.DEFAULT_ROOT
	# 案例之間不指回玩家的設定檔，免得收尾時的寫入落到玩家檔上。
	GameSettings.file_path = TEST_STORAGE.path_join("idle.cfg")
	GameSettings.load_file()
	await tree.process_frame
