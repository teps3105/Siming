class_name FigureMotion
extends RefCounted
## 地圖人物的動作資料（8.11）：由 Blender 骨架逐格取樣、換算成 CultivatorFigure 的關節姿勢（tools/figure/model/export_poses.py），
## 存成 resources/actions.json。這裡讀進來並依動作內的時間取姿勢——不用 AnimationPlayer、不讀引擎時間：
## 同一個動作與時間永遠得到同一個姿勢，暫停時停住。
##
## 檔案格式：{"version", "fps", "actions": {名稱: {"loop", "loop_start", "length"（格數）, "events": {名稱: 格號},
## "poses": [length + 1 格，每格 {hip, neck, elbow_f, hand_f, elbow_b, hand_b: [x, y], head_roll, wind, seated, point, anchors: {名稱: [x, y]}}]}}}。
## 座標與 CultivatorFigure 相同：成人身高為 1、腳下為原點、−y 向上、人朝 −x；_f 離看畫的人遠（人物的右手），_b 近（左手）。

const PATH := "res://modules/figure/resources/actions.json"
const VECTOR_KEYS := ["hip", "neck", "elbow_f", "hand_f", "elbow_b", "hand_b"]

static var _data := {}
static var _fps := 24.0


static func _load() -> void:
	if not _data.is_empty():
		return
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(PATH))
	if not parsed is Dictionary:
		push_error("動作資料讀不進來：" + PATH)
		return
	_fps = float(parsed.get("fps", 24))
	for name: String in parsed["actions"]:
		var raw: Dictionary = parsed["actions"][name]
		var poses: Array[Dictionary] = []
		for frame: Dictionary in raw["poses"]:
			var pose := {}
			for key in VECTOR_KEYS:
				pose[key] = _vec(frame[key])
			pose["head_roll"] = float(frame["head_roll"])
			pose["wind"] = float(frame["wind"])
			pose["seated"] = bool(frame["seated"])
			pose["point"] = bool(frame["point"])
			var anchors := {}
			for anchor: String in frame["anchors"]:
				anchors[anchor] = _vec(frame["anchors"][anchor])
			pose["anchors"] = anchors
			poses.append(pose)
		_data[name] = {"loop": bool(raw["loop"]), "loop_start": int(raw["loop_start"]), "length": int(raw["length"]),
			"events": raw["events"], "poses": poses}


static func _vec(pair: Array) -> Vector2:
	return Vector2(float(pair[0]), float(pair[1]))


static func names() -> Array:
	_load()
	return _data.keys()


static func fps() -> float:
	_load()
	return _fps


## 動作的格數（第 0 格到最後一格之間的間隔數）。
static func frame_count(action: String) -> int:
	_load()
	return int(_data[action]["length"]) if _data.has(action) else 0


## 動作的長度（秒）。
static func length(action: String) -> float:
	return frame_count(action) / fps()


static func loops(action: String) -> bool:
	_load()
	return bool(_data[action]["loop"]) if _data.has(action) else false


## 循環動作從哪個時間（秒）開始繞圈；之前的進場只播一次。
static func loop_start(action: String) -> float:
	_load()
	return int(_data[action]["loop_start"]) / fps() if _data.has(action) else 0.0


## 動作內的事件發生在第幾秒（例如 struck 的 hit：雷擊中）；沒有這個事件是 −1。
static func event_time(action: String, event: String) -> float:
	_load()
	if not _data.has(action) or not (_data[action]["events"] as Dictionary).has(event):
		return -1.0
	return float(_data[action]["events"][event]) / fps()


## 第 index 格的關節（原樣，不內插）。
static func frame_pose(action: String, index: int) -> Dictionary:
	_load()
	var poses: Array = _data[action]["poses"]
	return poses[clampi(index, 0, poses.size() - 1)]


## 動作在動作內時間 age（秒）的姿勢：循環動作過了進場就在 loop_start 之後繞圈，不循環的停在最後一格；相鄰兩格之間線性內插。
## 回傳的字典直接可以當 CultivatorFigure 的姿勢，另帶 anchors。
static func pose(action: String, age: float) -> Dictionary:
	_load()
	if not _data.has(action):
		return CultivatorFigure.POSES[CultivatorFigure.Pose.STAND]
	var last := frame_count(action)
	var at := age * fps()
	if bool(_data[action]["loop"]):
		var start := float(_data[action]["loop_start"])
		if at >= float(last):
			at = start + fposmod(at - start, float(last) - start)
	at = clampf(at, 0.0, float(last))
	var i := mini(int(floor(at)), last - 1) if last > 0 else 0
	var t := at - float(i)
	var a := frame_pose(action, i)
	if t <= 0.0 or last == 0:
		return a
	var b := frame_pose(action, i + 1)
	var out := {}
	for key in VECTOR_KEYS:
		out[key] = (a[key] as Vector2).lerp(b[key], t)
	out["head_roll"] = lerpf(a["head_roll"], b["head_roll"], t)
	out["wind"] = lerpf(a["wind"], b["wind"], t)
	out["seated"] = a["seated"]
	out["point"] = a["point"]
	var anchors := {}
	for key: String in a["anchors"]:
		anchors[key] = (a["anchors"][key] as Vector2).lerp(b["anchors"][key], t)
	out["anchors"] = anchors
	return out
