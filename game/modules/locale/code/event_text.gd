class_name EventText
extends RefCounted
## GDD 7.2 事件的一句話與印章：把橋接帶出的事件（SimWorld.drain_events）填進 locale 的模板。
## 模板在 ui.zh-Hant.json 的 event_<代號>（有分支的加 _<分支>），印章在 seal_<代號>。


## 世界大事（WorldChanged）：事件代號 change 與開啟的紀元名 era。
static func world_line(change: String, era: String) -> String:
	var line := Locale.t("world_" + change)
	return line if era.is_empty() else Locale.t("world_new_era").format({"event": line, "era": era})


## 事件的一句話。
static func line(event: Dictionary) -> String:
	var kind: String = event["kind"]
	if kind == "WorldChanged":
		return world_line(event["change"], event["era"])
	var key := _key(event)
	if not Locale.has(key):
		key = "event_unknown"
	var n: int = event.get("n", 0)
	var line := Locale.t(key).format({
		"A": event.get("a", ""),
		"B": event.get("b", ""),
		"F": event.get("f", ""),
		"G": event.get("g", ""),
		"P": event.get("p", ""),
		"Q": event.get("q", ""),
		"X": _word(event),
		"L": Locale.t("node_level_%d" % n),
		"N": str(n),
	})
	if event["kind"] == "ProvinceLost" and n > 0:
		line += Locale.t("event_ProvinceLost_drowned").format({"N": str(n)})
	return line


## 事件的印章用字（7.2「印」欄）。
static func seal(kind: String) -> String:
	var key := "seal_" + kind
	return Locale.t(key if Locale.has(key) else "seal_unknown")


## 模板的鍵：有分支的事件依參數選一句。
static func _key(event: Dictionary) -> String:
	var kind: String = event["kind"]
	var x: String = event.get("x", "")
	match kind:
		"ComeOfAge":
			return "event_ComeOfAge_" + ("join" if not String(event.get("f", "")).is_empty() else "self")
		"Tribulation", "Traveled", "LandLinked":
			return "event_%s_%s" % [kind, x]
		"ThreadResolved":
			return "event_ThreadResolved" if int(event.get("n", 0)) >= 2 else "event_ThreadResolved_once"
		"ProvinceLost":
			return "event_ProvinceLost" if not String(event.get("q", "")).is_empty() else "event_ProvinceLost_none"
		"NodeShifted":
			var to: int = event.get("n", 0)
			var from: int = event.get("m", 0)
			if to < from:
				return "event_NodeShifted_out" if to == 0 else "event_NodeShifted_down"
			return "event_NodeShifted_up"
	if kind == "Succession" and String(event.get("x", "")) == "Dynasty":
		return "event_Succession_dynasty"
	return "event_" + kind


## 模板的 {X}：代號字串換成介面用語，沒有對照的原樣使用（名字類）。
static func _word(event: Dictionary) -> String:
	var x: String = event.get("x", "")
	match String(event["kind"]):
		"Died":
			return Locale.t("death_" + x)
		"Breakthrough":
			return Locale.t("realm_%d" % int(event.get("n", 0)))
		"Tribulation":
			return Locale.t("tribulation_%d" % int(event.get("n", 0)))
		"ThreadCreated", "ThreadOmen", "ThreadResolved":
			return Locale.t("thread_" + x)
		"Judged":
			return Locale.t("path_" + x)
		"ComeOfAge":
			return Locale.t("role_" + x)
		"BookEntry":
			return Locale.t("book_" + x)
	return x
