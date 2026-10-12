class_name EventText
extends RefCounted
## GDD 7.2 事件的一句話與印章：把橋接帶出的事件（SimWorld.drain_events）填進 locale 的模板。
## 模板在 ui.zh-Hant.json 的 event_<代號>（有分支的加 _<分支>），印章在 seal_<代號>。
## 模板以 {I} 起頭的，{I} 是人物的身分全名（門派、職分、姓名、境界），句子依空間由全到簡逐級縮短，結果不被截。
## 7.2「傳聞的合成與標記」：例行事件（category 非空）同月同類合成一則統計；reason 是每則至多一個的「為什麼值得看」。

## 高境界：新境界或當事人境界達化神（4）起，不再當例行（7.2 的重要度在化神起升到 3，人數約是金丹的 1/4）。
const HIGH_REALM := 4
## 勢力之首更替與開宗立派的事件。
const LEAD_KINDS := ["PowerFounded", "SectSplit", "Succession", "Usurpation", "DynastyFounded", "Secession"]
## 身死（失敗而亡）的死因。
const DEATH_CAUSES := ["BreakthroughFailed", "Tribulation", "ShedBody", "Smite"]


## 世界大事（WorldChanged）：事件代號 change 與開啟的紀元名 era。
static func world_line(change: String, era: String) -> String:
	var line := Locale.t("world_" + change)
	return line if era.is_empty() else Locale.t("world_new_era").format({"event": line, "era": era})


## 事件的一句話。
static func line(event: Dictionary) -> String:
	return describe(event)["line"]


## 事件的印章用字（7.2「印」欄）。
static func seal(kind: String) -> String:
	var key := "seal_" + kind
	return Locale.t(key if Locale.has(key) else "seal_unknown")


## 事件的月份（1..12）：橋接給了 month 就用，否則由 tick 依曆法（GDD 2.6）算：tick 1 起算，第一個月為 1。
static func month_of(event: Dictionary) -> int:
	if event.has("month"):
		return int(event["month"])
	return (maxi(int(event["tick"]), 1) - 1) % 12 + 1


## 「71年3月」。
static func stamp(year: int, month: int) -> String:
	return Locale.t("rumor_month").format({"y": year, "m": month})


## 事件的完整描述：句子、分級縮短的版本、分類與標記。
## 回傳 line（全句）、variants（[[首, 尾], …] 由全到簡，首可截、尾不截）、category（例行類別，異常為空）、
## reason（原因代號：world、lead、high、death、fate，沒有為空）、consequence（後果，得不出為空）、
## who、who_variants、what（名單一行用的身分與結果）。
static func describe(event: Dictionary) -> Dictionary:
	var kind: String = event["kind"]
	var out := {"category": category(event), "reason": reason(event), "consequence": consequence(event), "who": "", "who_variants": [], "tag": "", "what": ""}
	if kind == "WorldChanged":
		var text := world_line(event["change"], event["era"])
		out["line"] = text
		out["variants"] = [["", text]]
		return out
	var key := _key(event)
	if not Locale.has(key):
		key = "event_unknown"
	var params := _params(event)
	var template := Locale.t(key)
	var tail_extra := ""
	if kind == "ProvinceLost" and int(event.get("n", 0)) > 0:
		tail_extra = Locale.t("event_ProvinceLost_drowned").format({"N": str(int(event["n"]))})
	var variants: Array = []
	if template.begins_with("{I}"):
		var heads := identity_variants(event)
		var tag := realm_tag(_event_realm(event))
		var rest := tag + template.substr(3).format(params) + tail_extra
		for head in heads:
			variants.append([head, rest])
		if Locale.has(key + "_short"):
			variants.append([heads[-1], tag + Locale.t(key + "_short").format(params) + tail_extra])
		out["line"] = heads[0] + rest
	else:
		var text := template.format(params) + tail_extra
		out["line"] = text
		variants.append(["", text])
		if Locale.has(key + "_short"):
			variants.append(["", Locale.t(key + "_short").format(params) + tail_extra])
	out["variants"] = variants
	var who := identity_variants(event)
	out["tag"] = realm_tag(_event_realm(event))
	out["who"] = who[0] + out["tag"]
	out["who_variants"] = who
	out["what"] = _result(event, params)
	return out


## 人物的身分與姓名，由全到簡：「鶴微宗長老霍靜沉」「長老霍靜沉」「霍靜沉」。境界另放在 realm_tag，空間不夠時只截這一段。
static func identity_variants(event: Dictionary) -> Array[String]:
	var person: String = event.get("a", "")
	var code: String = event.get("a_role", "")
	var role := _role_word(code, _event_realm(event))
	var power: String = event.get("a_power", "") if code == "Leader" or code == "Member" else ""
	var out: Array[String] = []
	for text in [power + role + person, role + person, person]:
		if not out.has(text):
			out.append(text)
	return out


## 境界的括號標記「（元嬰）」；非修士（−1）為空。
static func realm_tag(realm: int) -> String:
	return Locale.t("identity_realm").format({"R": Locale.t("realm_%d" % realm)}) if realm >= 0 else ""


## 職分代號換成稱呼；宗門成員依境界分長老（金丹起，3.5）與弟子。
static func _role_word(code: String, realm: int) -> String:
	if code == "Member":
		code = "Elder" if realm >= 2 else "Disciple"
	if code.is_empty():
		return ""
	var key := "identity_role_" + code
	return Locale.t(key) if Locale.has(key) else ""


## 事件當時主角的境界：突破與渡劫是進入新境界前的境界（新境界 − 1），其餘取橋接帶的目前境界。
static func _event_realm(event: Dictionary) -> int:
	var kind: String = event["kind"]
	if kind == "Tribulation" or kind == "Breakthrough":
		return maxi(int(event.get("n", 0)) - 1, 0)
	return int(event.get("a_realm", -1))


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


## 模板的填字表。
static func _params(event: Dictionary) -> Dictionary:
	var n: int = event.get("n", 0)
	return {
		"A": event.get("a", ""),
		"B": event.get("b", ""),
		"F": event.get("f", ""),
		"G": event.get("g", ""),
		"P": event.get("p", ""),
		"Q": event.get("q", ""),
		"X": _word(event),
		"L": Locale.t("node_level_%d" % n),
		"N": str(n),
		"T": Locale.t("tribulation_%d" % n) if Locale.has("tribulation_%d" % n) else "",
		"R": Locale.t("realm_%d" % n) if Locale.has("realm_%d" % n) else "",
		"W": gain(n),
	}


## 渡過劫、進入某境界後成就了什麼：「結成金丹」。
static func gain(realm: int) -> String:
	var key := "realm_gain_%d" % realm
	return Locale.t(key) if Locale.has(key) else Locale.t("realm_%d" % realm)


## 名單一行的「結果」。
static func _result(event: Dictionary, params: Dictionary) -> String:
	var kind: String = event["kind"]
	var key := "result_" + kind
	if kind == "Tribulation":
		key = "result_Tribulation_Passed" if event.get("x", "") == "Passed" else ""
	if not key.is_empty() and Locale.has(key):
		return Locale.t(key).format(params)
	var template := Locale.t(_key(event)) if Locale.has(_key(event)) else ""
	return template.trim_prefix("{I}").replace("{A}", "").format(params)


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


## 例行類別（同月同類合成一則統計）；異常事件為空字串。見 7.2「傳聞的合成與標記」。
static func category(event: Dictionary) -> String:
	var kind: String = event["kind"]
	var n: int = event.get("n", 0)
	match kind:
		"Tribulation":
			return "Tribulation" if event.get("x", "") == "Passed" and n < HIGH_REALM else ""
		"Breakthrough":
			return "Breakthrough" if n < HIGH_REALM else ""
		"ThreadCreated":
			return "ThreadCreated" if int(event["importance"]) <= 2 else ""
		"Married":
			return "Married"
		"Died":
			var role: String = event.get("a_role", "")
			if event.get("x", "") == "Age" and int(event.get("a_realm", -1)) < HIGH_REALM and role != "Leader" and role != "Ruler":
				return "Died"
		"TribulationOmen":
			return "TribulationOmen" if int(event.get("a_realm", -1)) < HIGH_REALM else ""
		"ComeOfAge":
			return "ComeOfAge"
		"Traveled":
			return "Traveled"
	return ""


## 為什麼值得看：依序取第一個成立的，至多一個（世界大事；開宗立派或勢力之首更替；高境界；身死；宿緣了結）。
static func reason(event: Dictionary) -> String:
	var kind: String = event["kind"]
	if kind == "WorldChanged":
		return "world"
	if kind in LEAD_KINDS:
		return "lead"
	var realm := int(event.get("n", 0)) if kind == "Tribulation" or kind == "Breakthrough" else int(event.get("a_realm", -1))
	if realm >= HIGH_REALM:
		return "high"
	if (kind == "Tribulation" and event.get("x", "") != "Passed") or (kind == "Died" and event.get("x", "") in DEATH_CAUSES):
		return "death"
	if kind == "ThreadResolved":
		return "fate"
	return ""


## 現在能從事件直接得出的後果；得不出為空字串。
static func consequence(event: Dictionary) -> String:
	var kind: String = event["kind"]
	var key := ""
	var power: String = event.get("f", "")
	if power.is_empty():
		power = event.get("a_power", "")
	match kind:
		"Died", "Tribulation":
			if kind == "Tribulation" and event.get("x", "") == "Passed":
				return ""
			var role: String = event.get("a_role", "")
			if role == "Leader" and not power.is_empty():
				key = "conseq_Died_leader"
		"PowerFounded", "SectSplit", "PowerFallen", "ProvinceLost", "DynastyFounded", "DynastyCollapsed", "Secession", "Usurpation", "CivilWar":
			key = "conseq_" + kind
	if key.is_empty() or not Locale.has(key):
		return ""
	return Locale.t(key).format({"F": power, "P": event.get("p", "")})


## 一組同月同類的例行事件（members 是 describe 的結果加上 n、x）合成的統計：回傳 [[首, 尾], …]（由全到簡）。
static func group_variants(category_name: String, members: Array) -> Array:
	var count := members.size()
	var counts := {}
	for m in members:
		var n := int(m.get("n", 0))
		counts[n] = int(counts.get(n, 0)) + 1
	var full := Locale.t("group_" + category_name)
	var out: Array = []
	if category_name == "Tribulation" or category_name == "Breakthrough":
		var realms: Array = counts.keys()
		realms.sort()
		var parts: Array[String] = []
		var short_parts: Array[String] = []
		for realm: int in realms:
			parts.append(Locale.t("group_%s_part" % category_name).format({"K": str(counts[realm]), "W": gain(realm), "R": Locale.t("realm_%d" % realm)}))
			short_parts.append(Locale.t("group_%s_short_part" % category_name).format({"K": str(counts[realm]), "R": Locale.t("realm_%d" % realm)}))
		out.append(["", full.format({"N": str(count), "L": Locale.t("group_separator").join(parts)})])
		out.append(["", Locale.t("group_%s_short" % category_name).format({"N": str(count), "L": Locale.t("group_separator_short").join(short_parts)})])
	else:
		out.append(["", full.format({"N": str(count)})])
	return out
