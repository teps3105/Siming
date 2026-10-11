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
## 搖桿當滑鼠用：左鍵（LT）與右鍵（RT），作用在游標的位置；右搖桿移動游標。
const PAD_LEFT := &"pad_mouse_left"
const PAD_RIGHT := &"pad_mouse_right"
## 搖桿的 A：自動鎖定離游標最近的目標並選取；連按換下一個。
const LOCK_ON := &"pad_lock_on"
const CURSOR_LEFT := &"pad_cursor_left"
const CURSOR_RIGHT := &"pad_cursor_right"
const CURSOR_UP := &"pad_cursor_up"
const CURSOR_DOWN := &"pad_cursor_down"
const CANCEL := &"ui_back"
const CARD_CENTER := &"card_center"
const PAUSE := &"time_pause"
## 鍵盤的 Tab、搖桿的 Y：切到下一檔速度，到最快再回到最慢。
const SPEED_CYCLE := &"time_speed_cycle"
## 搖桿的 Start：開總覽選單。
const MENU := &"menu_open"
const MODE_NEXT := &"map_mode_next"
const MODE_PREV := &"map_mode_prev"
const MODE_SHANHE := &"map_mode_shanhe"
const MODE_TIANXIA := &"map_mode_tianxia"
const MODE_LINGMAI := &"map_mode_lingmai"
## 星球視角與平面地圖互換：鍵盤 G、搖桿按下右搖桿。
const VIEW_TOGGLE := &"map_view_toggle"

const STICK_DEADZONE := 0.2


## 動作 → 綁定。鍵用 Key 常數；搖桿鍵用 "joy:<JoyButton>"；搖桿軸用 "axis:<JoyAxis>:<方向>"。
static func bindings() -> Dictionary:
	return {
		PAN_LEFT: [KEY_A, KEY_LEFT, "axis:%d:-1" % JOY_AXIS_LEFT_X],
		PAN_RIGHT: [KEY_D, KEY_RIGHT, "axis:%d:1" % JOY_AXIS_LEFT_X],
		PAN_UP: [KEY_W, KEY_UP, "axis:%d:-1" % JOY_AXIS_LEFT_Y],
		PAN_DOWN: [KEY_S, KEY_DOWN, "axis:%d:1" % JOY_AXIS_LEFT_Y],
		ZOOM_IN: [KEY_EQUAL, "joy:%d" % JOY_BUTTON_DPAD_UP],
		ZOOM_OUT: [KEY_MINUS, "joy:%d" % JOY_BUTTON_DPAD_DOWN],
		ROTATE_LEFT: [KEY_Q, "joy:%d" % JOY_BUTTON_DPAD_LEFT],
		ROTATE_RIGHT: [KEY_E, "joy:%d" % JOY_BUTTON_DPAD_RIGHT],
		CURSOR_LEFT: ["axis:%d:-1" % JOY_AXIS_RIGHT_X],
		CURSOR_RIGHT: ["axis:%d:1" % JOY_AXIS_RIGHT_X],
		CURSOR_UP: ["axis:%d:-1" % JOY_AXIS_RIGHT_Y],
		CURSOR_DOWN: ["axis:%d:1" % JOY_AXIS_RIGHT_Y],
		PAD_LEFT: ["axis:%d:1" % JOY_AXIS_TRIGGER_LEFT],
		LOCK_ON: ["joy:%d" % JOY_BUTTON_A],
		PAD_RIGHT: ["axis:%d:1" % JOY_AXIS_TRIGGER_RIGHT],
		CANCEL: [KEY_ESCAPE, "joy:%d" % JOY_BUTTON_B],
		CARD_CENTER: [KEY_C, "joy:%d" % JOY_BUTTON_X],
		PAUSE: [KEY_SPACE, "joy:%d" % JOY_BUTTON_BACK],
		SPEED_CYCLE: [KEY_TAB, "joy:%d" % JOY_BUTTON_Y],
		# 總覽選單（M2 才有內容）：先把鍵留給它。
		MENU: ["joy:%d" % JOY_BUTTON_START],
		MODE_NEXT: ["joy:%d" % JOY_BUTTON_RIGHT_SHOULDER],
		MODE_PREV: ["joy:%d" % JOY_BUTTON_LEFT_SHOULDER],
		MODE_SHANHE: [KEY_F1],
		MODE_TIANXIA: [KEY_F2],
		MODE_LINGMAI: [KEY_F4],
		VIEW_TOGGLE: [KEY_G, "joy:%d" % JOY_BUTTON_RIGHT_STICK],
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
