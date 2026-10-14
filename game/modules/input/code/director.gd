class_name Director
extends RefCounted
## GDD 5.13 導播模式的決策：選去處、算停留、決定何時換鏡頭與何時讓出、接回。
## 不碰引擎時間：外部每幀餵真實秒 delta 與事件，測試用假時間直接推進。鏡頭、字卡、變速由外面接 signal 去做。

## 開始過場：目的州郡與是否為大事。
signal go(province: int, big: bool)
## 玩家接手（含設定關閉、被別的流程占用）。
signal yielded
## 字卡的句子換了；空字典表示收起。
signal caption_changed(event: Dictionary)
## 「對準大事」的狀態變了（大事自動放慢用）。
signal slow_changed(on: bool)
## 要看的那件事換成別件了（過場途中或停留中），鏡頭該不該跟過去由接線決定。
signal refocus(event: Dictionary)

enum State { OFF, IDLE, TRAVEL, STAY, YIELDED }

## 計分窗（真實秒）。
const WINDOW := 10.0
## 最短、一般、最長停留（真實秒，標準節奏）。
const STAY_MIN := 8.0
const STAY_NORMAL := 12.0
const STAY_MAX := 30.0
## 別的州郡要高出目前所在州郡這個倍數才換。
const HYSTERESIS := 1.2
## 剛離開的州郡這麼久之內不再選。
const COOLDOWN := 30.0
## 同一句字卡至少顯示這麼久。
const CAPTION_MIN := 3.0
## 事件的重要度等級最高一級（3.3）才算大事。
const BIG_LEVEL := 5
## 沒有棋子演出的事件，分數打這個折扣。
const PLAIN_WEIGHT := 0.5
## 沒處理到的大事保留多久（真實秒），超過就不去了。
const BIG_KEEP := 15.0
## 事件記錄保留多久（真實秒）；計分只看計分窗內的。
const EVENT_KEEP := 60.0
## 過場最長等多久；鏡頭沒回報到位就當作到了。
const TRAVEL_LIMIT := 6.0
## 節奏三檔對停留的倍率：慢、標準、快。
const PACE_SCALE: Array[float] = [1.5, 1.0, 0.7]

var enabled := false
## 玩家正在用畫面（拖曳中、卡片或天下冊開著）：導播讓出並重新計閒置。
var busy := false
var pace := 1
var idle_resume := 20.0
## 找出演出節點的方式；預設查群組 event_stage（5.13 棋子演出）。
var stages: Callable = _group_stages
## 事件有沒有對應的棋子演出（5.13 字卡選事與計分用）；預設查 EventStage。
var performs: Callable = _event_performs

var state := State.OFF
## 目前所在（或正前往）的州郡；沒有為 -1。
var current := -1
var target := -1
var dwell := 0.0
var clock := 0.0

var _events: Array[Dictionary] = []
var _big: Dictionary = {}
var _left: Dictionary = {}
## 剛離開的州郡；它的大事也受回訪冷卻，其他州郡的大事不受。
var _last := -1
var _idle := 0.0
var _travel := 0.0
var _slow := false
var _caption: Dictionary = {}
var _caption_age := 0.0
var _pending_caption: Dictionary = {}
var _target_event: Dictionary = {}


static func _event_performs(event: Dictionary) -> bool:
	return EventStage.duration_for(event) > 0.0


static func _group_stages() -> Array:
	var loop := Engine.get_main_loop()
	return (loop as SceneTree).get_nodes_in_group("event_stage") if loop is SceneTree else []


## 換了世界：忘掉所有事件與去處，從頭來過。
func reset() -> void:
	if state == State.TRAVEL:
		_release(target)
	_events.clear()
	_big.clear()
	_left.clear()
	_last = -1
	state = State.OFF
	current = -1
	target = -1
	dwell = 0.0
	_leave_scene()
	yielded.emit()


func stay_min() -> float:
	return STAY_MIN * PACE_SCALE[clampi(pace, 0, 2)]


func stay_normal() -> float:
	return STAY_NORMAL * PACE_SCALE[clampi(pace, 0, 2)]


func stay_max() -> float:
	return STAY_MAX * PACE_SCALE[clampi(pace, 0, 2)]


## 送到畫面的事件（已濾掉不上傳聞的）：發生的州郡、重要度、原事件字典。
func post(province: int, importance: int, payload := {}) -> void:
	if province < 0 or importance <= 0:
		return
	var weight := float(importance) * (1.0 if performs.call(payload) else PLAIN_WEIGHT)
	_events.append({"t": clock, "province": province, "importance": importance, "weight": weight, "payload": payload})
	if importance >= BIG_LEVEL:
		if (state == State.STAY or state == State.TRAVEL) and province == (target if state == State.TRAVEL else current):
			_set_slow(true)
		else:
			_big[province] = {"t": clock, "payload": payload}
	if (state == State.STAY or state == State.TRAVEL) and province == (target if state == State.TRAVEL else current):
		_offer_caption(payload, importance)


## 分數：計分窗內該州郡的重要度總和。
func score(province: int) -> float:
	var total := 0.0
	for e in _events:
		if int(e["province"]) == province and clock - float(e["t"]) <= WINDOW:
			total += float(e["weight"])
	return total


## 計分窗內該州郡最值得講的事件：重要度高者；同級有棋子演出的優先；仍同級取新的；沒有為空字典。
func best_event(province: int) -> Dictionary:
	var best: Dictionary = {}
	for e in _events:
		if int(e["province"]) != province or clock - float(e["t"]) > WINDOW:
			continue
		if best.is_empty() or _rank(e["payload"], int(e["importance"])) >= _rank(best["payload"], int(best["importance"])):
			best = e
	return {} if best.is_empty() else best["payload"]


func caption() -> Dictionary:
	return _caption


## 這趟過場要看的那件事（計分窗內分數最高、同級取新的；沒有則是大事的原事件）；沒有為空字典。
func target_event() -> Dictionary:
	return _target_event


func slowing() -> bool:
	return _slow


## 玩家做了一個操作（拖曳、縮放、換視角、點選）：導播讓出，閒置重新計。
func player_acted() -> void:
	if state == State.OFF:
		return
	_yield()
	_idle = 0.0


## 鏡頭到位（過場結束）：放開等待的演出、停留從此刻起算。
func arrive() -> void:
	if state != State.TRAVEL:
		return
	state = State.STAY
	current = target
	dwell = 0.0
	_release(target)
	_big.erase(current)
	var spoken: Dictionary = _target_event if not _target_event.is_empty() else best_event(current)
	_set_caption(spoken)


## 每幀呼叫：delta 為真實秒，running 為遊戲時間是否在走（暫停時導播不動）。
func update(delta: float, running: bool) -> void:
	if running:
		clock += delta
		_prune()
	if not enabled:
		if state != State.OFF:
			_stand_down()
		return
	if state == State.OFF:
		state = State.IDLE
		current = -1
	if busy:
		if state != State.YIELDED:
			_yield()
		_idle = 0.0
		return
	if state == State.YIELDED:
		_idle += delta
		if _idle < idle_resume:
			return
		state = State.IDLE
		current = -1
	if state == State.TRAVEL:
		_travel += delta
		if _travel >= TRAVEL_LIMIT:
			arrive()
		return
	if not running:
		return
	if state == State.IDLE:
		var pick := _pick(-1)
		if pick >= 0:
			_begin(pick)
		return
	dwell += delta
	_caption_age += delta
	if not _pending_caption.is_empty() and _caption_age >= CAPTION_MIN:
		_set_caption(_pending_caption)
	_decide()


func _decide() -> void:
	if dwell < stay_min():
		return
	for node in stages.call():
		if node.has_method("remaining_seconds") and float(node.remaining_seconds(current)) > 0.0:
			return
	var big := _big_pending()
	if big >= 0 and big != current:
		_begin(big)
		return
	if dwell < stay_normal():
		return
	var best := _pick(current)
	if best < 0:
		return
	if dwell >= stay_max() or float(score(best)) > HYSTERESIS * float(score(current)):
		_begin(best)


## 分數最高的可去州郡（排除 skip 與冷卻中的）；全零為 -1。
func _pick(skip: int) -> int:
	var big := _big_pending()
	if big >= 0 and big != skip:
		return big
	var best := -1
	var best_score := 0.0
	var seen := {}
	for e in _events:
		var province := int(e["province"])
		if seen.has(province) or province == skip:
			continue
		seen[province] = true
		if clock - float(_left.get(province, -1000.0)) < COOLDOWN:
			continue
		var s := score(province)
		if s > best_score:
			best = province
			best_score = s
	return best


func _big_pending() -> int:
	var pick := -1
	var stamp := -1.0
	for province: int in _big:
		if province == _last and clock - float(_left.get(province, -1000.0)) < COOLDOWN:
			continue
		if float(_big[province]["t"]) > stamp:
			pick = province
			stamp = float(_big[province]["t"])
	return pick


func _begin(province: int) -> void:
	var was_big := _big.has(province)
	var held: Dictionary = _big[province]["payload"] if was_big else {}
	if current >= 0:
		_left[current] = clock
		_last = current
	_set_slow(was_big)
	_big.erase(province)
	target = province
	state = State.TRAVEL
	_travel = 0.0
	_target_event = best_event(province)
	if _target_event.is_empty():
		_target_event = held
	_caption_age = 0.0
	_pending_caption = {}
	_set_caption({})
	_hold(province)
	_focus(_target_event)
	go.emit(province, was_big)


func _yield() -> void:
	if state == State.TRAVEL:
		_release(target)
	state = State.YIELDED
	_leave_scene()
	yielded.emit()


func _stand_down() -> void:
	if state == State.TRAVEL:
		_release(target)
	state = State.OFF
	current = -1
	_leave_scene()
	yielded.emit()


func _leave_scene() -> void:
	_idle = 0.0
	_set_slow(false)
	_set_caption({})
	_pending_caption = {}
	_target_event = {}


func _prune() -> void:
	while not _events.is_empty() and clock - float(_events[0]["t"]) > EVENT_KEEP:
		_events.pop_front()
	for province: int in _big.keys():
		if clock - float(_big[province]["t"]) > BIG_KEEP:
			_big.erase(province)


func _offer_caption(payload: Dictionary, importance: int) -> void:
	if state == State.TRAVEL:
		if _target_event.is_empty() or _rank(payload, importance) >= _rank(_target_event, _importance_of(_target_event)):
			_target_event = payload
			_focus(payload)
			refocus.emit(payload)
		return
	if _rank(payload, importance) <= _rank(_caption, _importance_of(_caption)):
		return
	if _caption_age >= CAPTION_MIN:
		_set_caption(payload)
	else:
		_pending_caption = payload


## 字卡選事的排序：重要度優先，同級有棋子演出的勝過沒有的。
func _rank(event: Dictionary, importance: int) -> int:
	return importance * 2 + (1 if not event.is_empty() and performs.call(event) else 0)


static func _importance_of(event: Dictionary) -> int:
	return int(event.get("importance", 0))


func _set_caption(event: Dictionary) -> void:
	var changed := not event.is_empty() and state == State.STAY and not _caption.is_empty()
	if state == State.STAY and not event.is_empty():
		_target_event = event
		_focus(event)
	_caption = event
	_caption_age = 0.0
	_pending_caption = {}
	caption_changed.emit(event)
	if changed:
		refocus.emit(event)


func _set_slow(on: bool) -> void:
	if on == _slow:
		return
	_slow = on
	slow_changed.emit(on)


func _hold(province: int) -> void:
	for node in stages.call():
		if node.has_method("hold"):
			node.hold(province)


## 要看的那件事選定或換了：通知演出節點把它的演出排在最前、等鏡頭到位才開播。
func _focus(event: Dictionary) -> void:
	for node in stages.call():
		if node.has_method("focus"):
			node.focus(event)


func _release(province: int) -> void:
	for node in stages.call():
		if node.has_method("release"):
			node.release(province)
