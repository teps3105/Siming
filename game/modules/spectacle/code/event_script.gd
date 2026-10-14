class_name EventScript
extends RefCounted
## 5.13「棋子演出」：事件 → 棋子動作腳本。純資料與計算，不碰畫面。
## 每件演出四段：定場 INTRO（站定）、動作、餘韻 HOLD（停在結束姿勢）、收場 OUTRO（淡出）。
## 對照表 ROWS 以資料寫成：鍵是事件種類（有分支的加 :x 參數）；actors 的順序就是站位（第一位在左，兩人時相向）。
## 每位演員的 steps 依序播；沒有 sync 的步驟接在前一步結束；sync 寫成「演員序號:動作:事件名」，
## 表示從那位演員那個動作的事件時刻起播（例如倒地從出手「命中」那一格開始）。
## 沒有人物的演出（轉世）以 "light": true 標示、actors 為空：光效由 EventStage 另建，動作段長度取 light_len。
## 步驟的 do 寫成陣列就是幾種任選一種（出手 STRIKES）；sync 的動作名寫 * 是對方第一步不論挑到哪一種。
## len 是動作沒有固定長度（循環動作）時的秒數，len_of 取另一個動作的長度；align 配 sync 用：本步驟自己的某個事件對到 sync 的時刻。

const INTRO := 1.5
const HOLD := 2.0
const OUTRO := 0.5
## 定場裡淡入的秒數（其餘時間站定）。
const FADE_IN := 0.5
## 迎擊（對方未死）、行走的動作長度。
const BRACE_SECONDS := 2.0
const WALK_SECONDS := 3.0
## 轉世光效：光自上方落到事件格（定場）、落地後光暈散開（動作段）。
const LIGHT_LAND := 1.5
const LIGHT_SECONDS := 1.5
## 行走速度（身高／秒）：一步 11 格（24 fps）、腳相對髖前後共 0.18 身高。
const WALK_SPEED := 0.393

## 出手的三種動作：依序是修士（並指劍訣）、凡人與武人（揮劍劈）、僧人（掌擊）；出手者的道途（0、1、2）直接當索引。
const STRIKES := ["strike", "chop", "palm"]

const ROWS := {
	# 出手有三種（STRIKES）：並指劍訣、揮劍劈、掌擊；依出手者的道途挑（plan 的 a_path、b_path），沒有就取人物編號除三的餘數；
	# 倒地與格擋從出手的「命中」（三種都叫 hit）那一格開始，sync 的動作名寫 * 就是對方第一步不論挑到哪一種。
	"Killed": {"actors": [
		{"who": "a", "steps": [{"do": STRIKES}]},
		{"who": "b", "steps": [{"do": "fall", "sync": "0:*:hit"}]}]},
	"FeudEscalated": {"actors": [
		{"who": "a", "steps": [{"do": STRIKES}]},
		{"who": "b", "steps": [{"do": "fall", "sync": "0:*:hit"}]}]},
	# 了結：a 是欠債方、b 是債主（Spectacle._debtor_end）；報仇的是債主，倒下的是欠債方。
	"ThreadResolved:Feud": {"actors": [
		{"who": "b", "steps": [{"do": STRIKES}]},
		{"who": "a", "steps": [{"do": "fall", "sync": "0:*:hit"}]}]},
	# 對方沒死：被打的一方後退格擋。
	"ThreadResolved:Feud:Survived": {"actors": [
		{"who": "b", "steps": [{"do": STRIKES}]},
		{"who": "a", "steps": [{"do": "guard", "sync": "0:*:hit"}]}]},
	# 報恩：a 是欠恩方（報恩者）雙手奉物，b 受者伸手接過；接過的 take 對到奉物的 give。
	"ThreadResolved:Grace": {"actors": [
		{"who": "a", "steps": [{"do": "offer"}]},
		{"who": "b", "steps": [{"do": "receive", "sync": "0:offer:give", "align": "take"}]}]},
	# 情緣了結：兩人執手，鬆開後各自背過身。
	"ThreadResolved:Love": {"actors": [
		{"who": "a", "steps": [{"do": "clasp"}, {"do": "away"}]}, {"who": "b", "steps": [{"do": "clasp"}, {"do": "away"}]}]},
	# 釋怨：a 是欠怨方，抱拳深揖；b 拂袖轉身。
	"ThreadResolved:Grudge": {"actors": [
		{"who": "a", "steps": [{"do": "salute"}]}, {"who": "b", "steps": [{"do": "turn"}]}]},
	# 師徒緣了結：a 是徒、b 是師（同 ThreadCreated:Mentor）；徒跪別，師拂袖轉身（兩人列沒有位移，原地背過身）。
	"ThreadResolved:Mentor": {"actors": [
		{"who": "a", "steps": [{"do": "kneel"}]}, {"who": "b", "steps": [{"do": "turn"}]}]},
	"Rebirth": {"light": true, "light_len": LIGHT_SECONDS, "actors": []},
	"Died:Age": {"actors": [{"who": "a", "steps": [{"do": "slump"}]}]},
	"Died:Plague": {"actors": [{"who": "a", "steps": [{"do": "cough"}, {"do": "fall"}]}]},
	"Breakthrough": {"actors": [{"who": "a", "steps": [{"do": "sit"}, {"do": "rise"}]}]},
	"Ascended": {"ascend": true, "actors": [{"who": "a", "steps": [{"do": "rise"}]}]},
	"FellToDemon": {"actors": [{"who": "a", "steps": [{"do": "writhe"}]}]},
	# 拜師：徒跪拜叩首，師抬手撫頂（兩個動作同長，撫頂的 touch 與叩首的 down 都在第 22 格）。
	"ThreadCreated:Mentor": {"actors": [
		{"who": "a", "steps": [{"do": "kneel"}]},
		{"who": "b", "steps": [{"do": "bless"}]}]},
	# 成婚：兩人對拜後執手。
	"Married": {"actors": [
		{"who": "a", "steps": [{"do": "bow"}, {"do": "clasp"}]}, {"who": "b", "steps": [{"do": "bow"}, {"do": "clasp"}]}]},
	"Succession": {"actors": [{"who": "a", "steps": [{"do": "token"}]}]},
	"PowerFounded": {"actors": [{"who": "a", "steps": [{"do": "hail"}]}]},
	"SectSplit": {"actors": [{"who": "a", "steps": [{"do": "turn"}]}]},
	"Traveled:Out": {"travel": true, "actors": [{"who": "a", "steps": [{"do": "walk", "len": 3.0}]}]},
	"Traveled:Back": {"travel": true, "actors": [{"who": "a", "steps": [{"do": "walk", "len": 3.0}]}]},
}


## 飛升：動作段前 ASCEND_STILL 比例原地 rise，之後離地上升到 ASCEND_HEIGHT 個身高，餘韻段再以 ASCEND_DRIFT（身高／秒）緩升。
const ASCEND_STILL := 0.3
const ASCEND_HEIGHT := 1.5
const ASCEND_DRIFT := 0.15

## 表裡有的事件種類 → 可演的 x 參數（空陣列是不分支）；post 在最前面用它跳過沒有對應的事件，不建字典、不查位置。
static var _accepted := {}


## 事件的種類（與 x 參數）在表裡有對應：只查種類與參數，不看位置與當事人。
static func accepts(event: Dictionary) -> bool:
	if _accepted.is_empty():
		for key: String in ROWS:
			var parts := key.split(":")
			if not _accepted.has(parts[0]):
				_accepted[parts[0]] = []
			if parts.size() > 1 and not (_accepted[parts[0]] as Array).has(parts[1]):
				(_accepted[parts[0]] as Array).append(parts[1])
	var xs: Variant = _accepted.get(event.get("kind", ""))
	if xs == null:
		return false
	return (xs as Array).is_empty() or (xs as Array).has(event.get("x", ""))


## 事件對到表的哪一列；沒有對應回空字串。
static func row_key(event: Dictionary) -> String:
	var kind := str(event.get("kind", ""))
	var x := str(event.get("x", ""))
	var key := kind
	match kind:
		"Died", "ThreadCreated", "Traveled":
			key = "%s:%s" % [kind, x]
		"ThreadResolved":
			key = "ThreadResolved:" + x
			if x == "Feud" and bool(event.get("survivor_alive", false)):
				key += ":Survived"
	return key if ROWS.has(key) else ""


## 步驟有幾種任選一種（do 是陣列）的列：補上出手者的道途 <who>_path（path_of 由人物編號回傳 0 起算的索引，不知道回 −1），
## 沒有這類步驟的事件原樣回傳。道途由 EventStage 向橋接取：修士 0、凡人與其他 1、僧人 2，與 STRIKES 的順序一致。
static func with_paths(event: Dictionary, path_of: Callable) -> Dictionary:
	var key := row_key(event)
	if key.is_empty():
		return event
	var out := event
	for spec: Dictionary in ROWS[key]["actors"]:
		for step: Dictionary in spec["steps"]:
			if step["do"] is Array:
				var who := str(spec["who"])
				var person := _person_of(event, who)
				if person < 0 or event.has(who + "_path"):
					continue
				if out == event:
					out = event.duplicate()
				out[who + "_path"] = int(path_of.call(person))
	return out


## 事件的演出腳本：{key, actors: [{who, person, steps: [{do, start, length}], end}], action_len, total, travel, light, ascend, to}；
## 沒有對應動作、沒有位置、沒有當事人（或行走沒有目的州郡）回空字典。
static func plan(event: Dictionary) -> Dictionary:
	var key := row_key(event)
	if key.is_empty() or not event.has("pos"):
		return {}
	var row: Dictionary = ROWS[key]
	var travel := bool(row.get("travel", false))
	if travel and int(event.get("q_province", -1)) < 0:
		return {}
	var actors: Array[Dictionary] = []
	for spec: Dictionary in row["actors"]:
		var person := _person_of(event, str(spec["who"]))
		if person < 0:
			continue
		actors.append({"who": spec["who"], "person": person, "spec": spec})
	var light := bool(row.get("light", false))
	if actors.is_empty() and not light:
		return {}
	var action_len := float(row["light_len"]) if light else 0.0
	for index in actors.size():
		var steps: Array[Dictionary] = []
		var cursor := 0.0
		for step: Dictionary in (actors[index]["spec"] as Dictionary)["steps"]:
			var action := _pick(step["do"], event, str(actors[index]["who"]), int(actors[index]["person"]))
			var length := float(step["len"]) if step.has("len") else (FigureMotion.length(str(step["len_of"])) if step.has("len_of") else FigureMotion.length(action))
			var start := cursor
			if step.has("sync"):
				start = _sync_time(actors, str(step["sync"]))
				# align：本步驟自己的這個事件對到 sync 的時刻（接過的 take 對奉物的 give），本步驟因此提早開始。
				if step.has("align"):
					start = maxf(start - maxf(FigureMotion.event_time(action, str(step["align"])), 0.0), 0.0)
			steps.append({"do": action, "start": start, "length": length})
			cursor = start + length
		actors[index]["steps"] = steps
		actors[index]["end"] = cursor
		actors[index].erase("spec")
		action_len = maxf(action_len, cursor)
	return {"key": key, "actors": actors, "action_len": action_len, "total": INTRO + action_len + HOLD + OUTRO,
		"travel": travel, "light": light, "ascend": bool(row.get("ascend", false)), "to": int(event.get("q_province", -1))}


## 事件的演出長度（秒，四段合計）；沒有對應動作回 0。
static func duration_for(event: Dictionary) -> float:
	var p := plan(event)
	return float(p["total"]) if not p.is_empty() else 0.0


## 此刻（演出時間 time）某位演員該做的動作與動作內的時間：定場站定，之後依步驟；已播完的步驟停在結束姿勢。
## 站定與停格期間回傳的時間不再增加（姿勢不變就不必重畫）；keep_moving 為真（行走）時時間照走。
static func pose_at(actor: Dictionary, time: float, keep_moving := false) -> Dictionary:
	var t := time - INTRO
	var current: Dictionary = {}
	for step: Dictionary in actor["steps"]:
		if float(step["start"]) <= t:
			current = step
	if t < 0.0 or current.is_empty():
		return {"action": "stand", "time": 0.0}
	var inside := t - float(current["start"])
	return {"action": current["do"], "time": inside if keep_moving else minf(inside, float(current["length"]))}


## 飛升的離地高度（身高的倍數）：動作段前段原地，之後升到 ASCEND_HEIGHT，餘韻再緩升；不是飛升回 0。
static func lift_at(plan: Dictionary, time: float) -> float:
	if not bool(plan.get("ascend", false)):
		return 0.0
	var t := time - INTRO
	var span := float(plan["action_len"])
	var still := span * ASCEND_STILL
	if t <= still:
		return 0.0
	var k := clampf((t - still) / maxf(span - still, 1e-4), 0.0, 1.0)
	return ASCEND_HEIGHT * k * k * (3.0 - 2.0 * k) + maxf(t - span, 0.0) * ASCEND_DRIFT


## 整段演出在時間 time 的不透明度：淡入、站定、淡出。
static func opacity_at(total: float, time: float) -> float:
	return clampf(time / FADE_IN, 0.0, 1.0) * clampf((total - time) / OUTRO, 0.0, 1.0)


## 步驟的動作：寫成陣列就是幾種任選一種，取事件的「<who>_path」（出手者的道途，0 起算），沒有就取人物編號除以種數的餘數。
static func _pick(choice: Variant, event: Dictionary, who: String, person: int) -> String:
	if not choice is Array:
		return str(choice)
	var options: Array = choice
	var path := int(event.get(who + "_path", -1))
	return str(options[path if path >= 0 and path < options.size() else person % options.size()])


static func _person_of(event: Dictionary, who: String) -> int:
	var persons: PackedInt32Array = event.get("persons", PackedInt32Array())
	var index := 0 if who == "a" else 1
	var id := int(event.get("a_person" if who == "a" else "b_person", -1))
	if id < 0 and persons.size() > index:
		id = persons[index]
	return id


static func _sync_time(actors: Array[Dictionary], sync: String) -> float:
	var parts := sync.split(":")
	var other := int(parts[0])
	if other >= actors.size() or not actors[other].has("steps"):
		return 0.0
	for step: Dictionary in actors[other]["steps"]:
		if parts[1] == "*" or str(step["do"]) == parts[1]:
			return float(step["start"]) + maxf(FigureMotion.event_time(str(step["do"]), parts[2]), 0.0)
	return 0.0
