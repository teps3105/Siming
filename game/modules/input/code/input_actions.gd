class_name InputActions
extends RefCounted
## GDD 5.9 動作層：鍵鼠與搖桿綁到同一組 InputMap 動作，其他模組只讀動作。
## 綁定表是唯一來源，啟動時由 register() 寫入 InputMap。

const PAN_LEFT := &"map_pan_left"
const PAN_RIGHT := &"map_pan_right"
const PAN_UP := &"map_pan_up"
const PAN_DOWN := &"map_pan_down"
const ZOOM_IN := &"map_zoom_in"
const ZOOM_OUT := &"map_zoom_out"
const ROTATE_LEFT := &"map_rotate_left"
const ROTATE_RIGHT := &"map_rotate_right"
const SELECT := &"map_select"
const CANCEL := &"ui_back"
const CARD_CENTER := &"card_center"
const PAUSE := &"time_pause"
const SPEED_UP := &"time_faster"
const SPEED_DOWN := &"time_slower"
const SPEED_1 := &"time_speed_1"
const SPEED_2 := &"time_speed_2"
const SPEED_3 := &"time_speed_3"
const SPEED_4 := &"time_speed_4"
const MODE_NEXT := &"map_mode_next"
const MODE_PREV := &"map_mode_prev"
const MODE_SHANHE := &"map_mode_shanhe"
const MODE_TIANXIA := &"map_mode_tianxia"
const MODE_LINGMAI := &"map_mode_lingmai"

const STICK_DEADZONE := 0.2


## 動作 → 綁定。鍵用 Key 常數；搖桿鍵用 "joy:<JoyButton>"；搖桿軸用 "axis:<JoyAxis>:<方向>"。
static func bindings() -> Dictionary:
	return {
		PAN_LEFT: [KEY_A, KEY_LEFT, "axis:%d:-1" % JOY_AXIS_LEFT_X],
		PAN_RIGHT: [KEY_D, KEY_RIGHT, "axis:%d:1" % JOY_AXIS_LEFT_X],
		PAN_UP: [KEY_W, KEY_UP, "axis:%d:-1" % JOY_AXIS_LEFT_Y],
		PAN_DOWN: [KEY_S, KEY_DOWN, "axis:%d:1" % JOY_AXIS_LEFT_Y],
		ZOOM_IN: [KEY_EQUAL, "axis:%d:-1" % JOY_AXIS_RIGHT_Y],
		ZOOM_OUT: [KEY_MINUS, "axis:%d:1" % JOY_AXIS_RIGHT_Y],
		ROTATE_LEFT: [KEY_Q, "axis:%d:-1" % JOY_AXIS_RIGHT_X],
		ROTATE_RIGHT: [KEY_E, "axis:%d:1" % JOY_AXIS_RIGHT_X],
		SELECT: ["joy:%d" % JOY_BUTTON_A],
		CANCEL: [KEY_ESCAPE, "joy:%d" % JOY_BUTTON_B],
		CARD_CENTER: [KEY_C, "joy:%d" % JOY_BUTTON_X],
		PAUSE: [KEY_SPACE, "joy:%d" % JOY_BUTTON_START],
		SPEED_UP: ["axis:%d:1" % JOY_AXIS_TRIGGER_RIGHT],
		SPEED_DOWN: ["axis:%d:1" % JOY_AXIS_TRIGGER_LEFT],
		SPEED_1: [KEY_1],
		SPEED_2: [KEY_2],
		SPEED_3: [KEY_3],
		SPEED_4: [KEY_4],
		MODE_NEXT: ["joy:%d" % JOY_BUTTON_RIGHT_SHOULDER],
		MODE_PREV: ["joy:%d" % JOY_BUTTON_LEFT_SHOULDER],
		MODE_SHANHE: [KEY_F1],
		MODE_TIANXIA: [KEY_F2],
		MODE_LINGMAI: [KEY_F4],
	}


static func register() -> void:
	var table := bindings()
	for action: StringName in table:
		if InputMap.has_action(action):
			InputMap.action_erase_events(action)
		else:
			InputMap.add_action(action, STICK_DEADZONE)
		for binding in table[action]:
			InputMap.action_add_event(action, _event(binding))


static func _event(binding: Variant) -> InputEvent:
	if binding is int:
		var key := InputEventKey.new()
		key.physical_keycode = binding
		return key
	var parts := (binding as String).split(":")
	if parts[0] == "joy":
		var button := InputEventJoypadButton.new()
		button.button_index = int(parts[1]) as JoyButton
		return button
	var motion := InputEventJoypadMotion.new()
	motion.axis = int(parts[1]) as JoyAxis
	motion.axis_value = float(parts[2])
	return motion
