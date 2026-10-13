class_name SpectacleQueue
extends RefCounted
## GDD 5.6 奇觀佇列：同時最多 12 個，超過時依重要度捨棄（同重要度捨棄先開始的）；
## 速度 >= 20× 時重要度 < 4 的奇觀只留簡化版光點，光點屬小型奇觀，不計入上限。
## 時長是實際秒；推進多少秒由呼叫端決定（暫停時不推進）。純邏輯，不碰畫面。

const MAX_ACTIVE := 12
## 光點雖不計上限，仍留一道保險，免得快轉時無限堆積。
const MAX_SPOTS := 64
const FAST_SPEED := 20
const FULL_IMPORTANCE := 4
const SPOT_SECONDS := 0.8
## 5.6 表的時長（實際秒）。
const SECONDS := {"Tribulation": 4.0, "Ascended": 6.0, "ThreadResolved": 3.0}

var _items: Array[Dictionary] = []
var _spots: Array[Dictionary] = []
var _serial := 0


## 加入一個奇觀。spec 至少有 kind、importance、pos（地圖座標）；delay 秒後才開始（重播等鏡頭飛到）。
## fast 為真且重要度不到 4 時只留光點。回傳這個奇觀是否留下（被捨棄為 false）。
func add(spec: Dictionary, fast: bool, delay := 0.0) -> bool:
	_serial += 1
	var item := spec.duplicate()
	item["order"] = _serial
	item["age"] = -delay
	if fast and int(spec["importance"]) < FULL_IMPORTANCE:
		item["of"] = spec["kind"]
		item["kind"] = "Spot"
		item["duration"] = SPOT_SECONDS
		if _spots.size() >= MAX_SPOTS:
			_spots.pop_front()
		_spots.append(item)
		return true
	item["duration"] = float(SECONDS.get(spec["kind"], 3.0))
	if _items.size() >= MAX_ACTIVE:
		var victim := -1
		for i in _items.size():
			if victim < 0 or int(_items[i]["importance"]) < int(_items[victim]["importance"]):
				victim = i
		if int(item["importance"]) < int(_items[victim]["importance"]):
			return false
		_items.remove_at(victim)
	_items.append(item)
	return true


## 時間過了 seconds 秒；到時長的移除。
func advance(seconds: float) -> void:
	_items = _aged(_items, seconds)
	_spots = _aged(_spots, seconds)


static func _aged(list: Array[Dictionary], seconds: float) -> Array[Dictionary]:
	var keep: Array[Dictionary] = []
	for item in list:
		item["age"] = float(item["age"]) + seconds
		if float(item["age"]) < float(item["duration"]):
			keep.append(item)
	return keep


func clear() -> void:
	_items.clear()
	_spots.clear()


## 計入上限的奇觀（開始的先後序）。
func items() -> Array[Dictionary]:
	return _items


func spots() -> Array[Dictionary]:
	return _spots


func count() -> int:
	return _items.size()


## 某事件的奇觀是否正在放（含光點與尚未開始的重播）。
func has_event(event_id: int) -> bool:
	for list in [_items, _spots]:
		for item: Dictionary in list:
			if int(item.get("id", -1)) == event_id:
				return true
	return false
