class_name InputHints
extends RefCounted
## GDD 5.9 按鍵提示：動作在目前輸入裝置上的實體鍵名稱，由 InputActions 的綁定表推得（不另列一份對照）。
## 介面拿這個名稱畫鍵帽或搖桿按鈕（Ink.key_glyph）；左右搖桿（LS、RS）、十字鍵（DPAD_V 上下、DPAD_H 左右）與鍵盤方向鍵（ARROWS_V、ARROWS_H）畫成圖標。

const JOY_BUTTON_NAMES := {
	JOY_BUTTON_A: "A",
	JOY_BUTTON_B: "B",
	JOY_BUTTON_X: "X",
	JOY_BUTTON_Y: "Y",
	JOY_BUTTON_LEFT_SHOULDER: "LB",
	JOY_BUTTON_RIGHT_SHOULDER: "RB",
	JOY_BUTTON_START: "Start",
	JOY_BUTTON_BACK: "Select",
	JOY_BUTTON_RIGHT_STICK: "R3",
	JOY_BUTTON_LEFT_STICK: "L3",
	JOY_BUTTON_DPAD_UP: "DPAD_V",
	JOY_BUTTON_DPAD_DOWN: "DPAD_V",
	JOY_BUTTON_DPAD_LEFT: "DPAD_H",
	JOY_BUTTON_DPAD_RIGHT: "DPAD_H",
}
const JOY_AXIS_NAMES := {
	JOY_AXIS_TRIGGER_LEFT: "LT",
	JOY_AXIS_TRIGGER_RIGHT: "RT",
	JOY_AXIS_RIGHT_X: "RS",
	JOY_AXIS_RIGHT_Y: "RS",
	JOY_AXIS_LEFT_X: "LS",
	JOY_AXIS_LEFT_Y: "LS",
}
const KEY_NAMES := {
	KEY_ESCAPE: "Esc",
	KEY_SPACE: "Space",
	KEY_ENTER: "Enter",
	KEY_DELETE: "Del",
	KEY_EQUAL: "=",
	KEY_MINUS: "-",
	KEY_UP: "ARROWS_V",
	KEY_DOWN: "ARROWS_V",
	KEY_LEFT: "ARROWS_H",
	KEY_RIGHT: "ARROWS_H",
}


## 動作在該裝置上的全部實體鍵名稱（依綁定表的順序）。
static func labels(action: StringName, gamepad: bool) -> Array[String]:
	var out: Array[String] = []
	for binding: Variant in InputActions.bindings().get(action, []):
		if binding is int:
			if not gamepad:
				out.append(KEY_NAMES.get(binding, OS.get_keycode_string(binding)))
		elif gamepad:
			var parts := (binding as String).split(":")
			var names: Dictionary = JOY_BUTTON_NAMES if parts[0] == "joy" else JOY_AXIS_NAMES
			if names.has(int(parts[1])):
				out.append(names[int(parts[1])])
	return out


## 動作的第一個實體鍵名稱；gamepad 為真取搖桿，否則取鍵盤。該裝置沒有綁定時回傳空字串。
static func label(action: StringName, gamepad: bool) -> String:
	var all := labels(action, gamepad)
	return "" if all.is_empty() else all[0]
