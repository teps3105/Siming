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


## 傳聞簽全螢幕的篩選歸類（5.7）：每個事件種類只屬一類。
## 大事＝世界大事與天地之變（地形、洞天、天災、魔劫量劫、妖患）；人物＝一個人的生老病死與作為；
## 宗門＝門派的創立、更替、爭鬥與覆滅（`Succession` 王朝與宗門共用同一代號，歸宗門）；
## 王朝＝國家的立滅、戰事、政令與賑濟；宿緣＝宿緣的結下、將盡、了結與消散。
const FILTER_KINDS := {
	"world": ["WorldChanged", "NodeShifted", "ProvinceLost", "LandLinked", "SeatMoved", "ForestCleared", "Desertified", "BookEntry",
		"Famine", "Plague", "BeastAwakened", "BeastRaid", "DemonDen", "CataclysmBegan", "CataclysmEnded", "HeavenWrath",
		"KalpaOmen", "Kalpa", "LeyDamaged", "KalpaSettled"],
	"person": ["Born", "ComeOfAge", "Married", "Died", "WraithBorn", "Breakthrough", "Tribulation", "TribulationOmen", "Ascended", "FellToDemon",
		"FruitAttained", "Vow", "Nirvana", "Treatise", "GreatScholar", "Sage", "Killed", "Converted", "Rebirth", "Judged", "SoulDissolved", "Traveled", "Remonstrance"],
	"sect": ["PowerFounded", "Succession", "SectSplit", "SectWar", "PowerFallen"],
	"dynasty": ["DynastyFounded", "Annexed", "WarDeclared", "Battle", "CityFell", "Massacre", "Rebellion", "Secession", "Usurpation", "CivilWar",
		"CapitalMoved", "DynastyChanged", "DynastyCollapsed", "Peace", "Fengshan", "Relief", "Purge", "BanTao"],
	"karma": ["Genesis", "ThreadCreated", "ThreadOmen", "ThreadResolved", "FeudEscalated", "ThreadFaded"],
}


## 事件種類的篩選歸類；沒歸類的退到「大事」，不會從全螢幕的清單消失。
static func filter_of(kind: String) -> String:
	for cls: String in FILTER_KINDS:
		if kind in FILTER_KINDS[cls]:
			return cls
	return "world"


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
	return (maxi(int(event["tick"]), 1) - 1) / 30 % 12 + 1


## ── 日期（GDD 2.6「日期顯示」，決策 #91）──────────────────────────────────────────────
## 每個日期都同時寫紀元時間（紀元名、紀元內第幾年，中文數字）與世界時間（開天以來第幾年，阿拉伯數字）。
## 全專案的日期文字只由下面這組函式產生，其他地方不自行拼字串。
## 帶空格的寫法（_label）：題簽、傳聞簽、卡片欄位、表格；句子裡的寫法（_inline）：全形括號、不留空格。

const CJK_DIGITS := "〇一二三四五六七八九"
const CJK_MONTHS := ["", "一", "二", "三", "四", "五", "六", "七", "八", "九", "十", "十一", "十二"]
## 兩種寫法的日期（含只到年的）；折行時整段不拆（LedgerText 用它找出要黏在一起的範圍），驗收也用它掃描。
const DATE_PATTERN := "[^\\s，。、；：！？（）()]{2}紀元 ?[〇一二三四五六七八九]+年 ?[（(][0-9]+年[）)]( ?(十[一二]?|[一二三四五六七八九])月)?"
static var _date_regex: RegEx
## 題簽預留的位數：紀元年四位、世界年六位。
const ERA_YEAR_DIGITS := 4
const WORLD_YEAR_DIGITS := 6


## 逐位寫的中文數字：105 → 一〇五。
static func cjk_digits(n: int) -> String:
	var out := ""
	for digit in str(maxi(n, 0)):
		out += CJK_DIGITS[int(digit)]
	return out


## 月份照念法：1 → 一月、11 → 十一月。
static func cjk_month(month: int) -> String:
	return CJK_MONTHS[clampi(month, 1, 12)] + Locale.t("date_month_unit")


## 「咸寧紀元 一二三年 (12356年) 六月」；month 為 0 時只到年。
static func date_label(era: String, era_year: int, world_year: int, month: int = 0) -> String:
	return _date("date_label", era, era_year, world_year, month)


## 「咸寧紀元一二三年（12356年）六月」；month 為 0 時只到年。
static func date_inline(era: String, era_year: int, world_year: int, month: int = 0) -> String:
	return _date("date_inline", era, era_year, world_year, month)


static func year_label(era: String, era_year: int, world_year: int) -> String:
	return date_label(era, era_year, world_year, 0)


static func year_inline(era: String, era_year: int, world_year: int) -> String:
	return date_inline(era, era_year, world_year, 0)


static func _date(key: String, era: String, era_year: int, world_year: int, month: int) -> String:
	var text := Locale.t(key).format({"E": era, "Y": cjk_digits(era_year), "W": str(world_year)})
	if month <= 0:
		return text
	return text + Locale.t(key + "_month").format({"M": cjk_month(month)})


## 事件（橋接的事件字典）發生的日期：紀元名在 date_era（橋接的 event）或 era（個人歷程、門史）。
static func event_label(event: Dictionary) -> String:
	return date_label(_event_era(event), int(event["year"]), int(event["world_year"]), int(event["month"]))


static func event_inline(event: Dictionary) -> String:
	return date_inline(_event_era(event), int(event["year"]), int(event["world_year"]), int(event["month"]))


static func event_year_inline(event: Dictionary) -> String:
	return year_inline(_event_era(event), int(event["year"]), int(event["world_year"]))


static func _event_era(event: Dictionary) -> String:
	return String(event["date_era"] if event.has("date_era") else event["era"])


## 現在的日期（題簽、存檔列表）；world 是 SimWorld。
static func now_label(world: Object) -> String:
	var parts: Dictionary = world.date_parts()
	if parts.is_empty():
		return ""
	return date_label(parts["era"], parts["era_year"], parts["world_year"], parts["month"])


## 帶空格的日期折成兩行：「竭黃紀元 一五年」「(11145年) 十二月」（題簽放不下單行時用）。
static func label_lines(label: String) -> PackedStringArray:
	var at := label.find(" (")
	if at < 0:
		return PackedStringArray([label])
	return PackedStringArray([label.substr(0, at), label.substr(at + 1)])


## 題簽最寬的日期（單行）：紀元名兩個字、紀元年四位、世界年六位、十二月；數字各位取該字體最寬者。
static func widest_label(size: int) -> float:
	return _widest_text(date_label(Locale.t("date_era_sample"), 0, 0, 12), size)


## 題簽兩行時各行最寬的寬度：第一行紀元名與紀元年，第二行世界年與月。
static func widest_label_lines(size: int) -> float:
	var lines := label_lines(date_label(Locale.t("date_era_sample"), 0, 0, 12))
	return maxf(_widest_text(lines[0], size), _widest_text(lines[1], size))


## 樣本字串裡的「〇」「0」各是一位數字的佔位：換成該字體最寬的那個字，位數照預留的算。
static func _widest_text(sample: String, size: int) -> float:
	var cjk := sample.count("〇")
	var arabic := sample.count("0")
	var base := Ink.text_size(sample, size).x - cjk * Ink.text_size("〇", size).x - arabic * Ink.text_size("0", size).x
	return base + (ERA_YEAR_DIGITS * _widest_glyph(CJK_DIGITS, size) if cjk > 0 else 0.0) + (WORLD_YEAR_DIGITS * _widest_glyph("0123456789", size) if arabic > 0 else 0.0)


static func _widest_glyph(glyphs: String, size: int) -> float:
	var widest := 0.0
	for glyph in glyphs:
		widest = maxf(widest, Ink.text_size(glyph, size).x)
	return widest


## 文字裡所有日期（兩種寫法）的範圍 [起, 迄)，供折行整段不拆。
static func date_spans(text: String) -> Array[Vector2i]:
	if _date_regex == null:
		_date_regex = RegEx.create_from_string(DATE_PATTERN)
	var out: Array[Vector2i] = []
	for found in _date_regex.search_all(text):
		out.append(Vector2i(found.get_start(), found.get_end()))
	return out


## 低境界身死時可以同月同死因合成統計的死因（7.2）；被殺（有兇手與宿緣脈絡）與飛升、涅槃、成聖不在內。
const MERGE_CAUSES := ["Age", "BreakthroughFailed", "Tribulation", "ShedBody", "Smite", "War", "Plague", "Famine", "Kalpa"]
## 死於天劫的兩種死因：當事人是在衝擊下一境時身死，境界要再加一階才是他正要去的境界。
const TRIBULATION_CAUSES := ["Tribulation", "ShedBody"]


## 身死事件判境界高低用的境界：死於天劫者算他正要晉入的新境界，與渡劫事件的門檻一致。
static func death_realm(event: Dictionary) -> int:
	var realm := int(event.get("a_realm", -1))
	return realm + 1 if realm >= 0 and event.get("x", "") in TRIBULATION_CAUSES else realm


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


## 個人歷程（人物卡）的一句話：主角是 owner 本人時省略他的身分與姓名，直接以動詞起句；
## 事件裡的其他人名照舊（歷程端把它們做成可點）。沒有境界括號：歷程每筆都是同一個人，渡劫寫出成就的境界即可。
static func history_line(event: Dictionary, owner: int) -> String:
	var key := _key(event)
	if not Locale.has(key):
		key = "event_unknown"
	var subject := int(event.get("a_person", -1))
	var own := subject == owner or subject < 0
	var template := Locale.t(key + "_own") if own and Locale.has(key + "_own") else Locale.t(key)
	# 師徒：事件的 A 是徒、B 是師；本人是師時寫「收某為徒」。
	if key == "event_ThreadCreated_Mentor" and int(event.get("b_person", -1)) == owner and subject != owner:
		template = Locale.t(key + "_master")
	if template.begins_with("{I}") or template.begins_with("{A}"):
		template = template.substr(3) if own else "{A}" + template.substr(3)
	var tail_extra := ""
	if event["kind"] == "ProvinceLost" and int(event.get("n", 0)) > 0:
		tail_extra = Locale.t("event_ProvinceLost_drowned").format({"N": str(int(event["n"]))})
	return template.format(_params(event)) + tail_extra


## 歷程去掉同一件事的第二筆：渡劫成功後緊接的突破（新境界 ≥ 金丹）、成婚同時結下的宿緣。
static func fold_history(history: Array) -> Array:
	var out: Array = []
	for entry: Dictionary in history:
		if not _is_echo(entry, history):
			out.append(entry)
	return out


## 因樹（先根順序攤平，每筆有 level 與 parent）套同一條規則：去掉的節點，它底下的因接到上一層（level 與 parent 重算）。
## extra 是不在樹裡、但可作對照的事件（事件卡的根事件）。
static func fold_tree(nodes: Array, extra: Array = []) -> Array:
	var reference: Array = nodes + extra
	var dropped: Array[bool] = []
	for node: Dictionary in nodes:
		dropped.append(_is_echo(node, reference))
	var out: Array = []
	var new_index := {}
	for i in nodes.size():
		if dropped[i]:
			continue
		var parent := int(nodes[i].get("parent", -1))
		while parent >= 0 and dropped[parent]:
			parent = int(nodes[parent].get("parent", -1))
		var copy: Dictionary = (nodes[i] as Dictionary).duplicate()
		if parent >= 0 and new_index.has(parent):
			copy["parent"] = new_index[parent]
			copy["level"] = int(out[new_index[parent]]["level"]) + 1
		else:
			copy["parent"] = -1
			copy["level"] = 1
		new_index[i] = out.size()
		out.append(copy)
	return out


static func _is_echo(entry: Dictionary, history: Array) -> bool:
	var kind: String = entry["kind"]
	if kind != "ThreadCreated" and not (kind == "Breakthrough" and int(entry.get("n", 0)) >= 2):
		return false
	for other: Dictionary in history:
		if other["tick"] != entry["tick"]:
			continue
		if kind == "Breakthrough":
			if other["kind"] == "Tribulation" and other["a_person"] == entry["a_person"]:
				return true
		elif other["kind"] == "Married":
			var pair := [int(other["a_person"]), int(other["b_person"])]
			if pair.has(int(entry["a_person"])) and pair.has(int(entry["b_person"])):
				return true
	return false


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
			# 事件自己帶宗門才寫入某門；沒帶的依成了什麼身分寫（散修、未得入道、或僧儒魔等的「自行…」）。
			if not String(event.get("f", "")).is_empty():
				return "event_ComeOfAge_join"
			match x:
				"Cultivator":
					return "event_ComeOfAge_rogue"
				"Commoner":
					return "event_ComeOfAge_commoner"
			return "event_ComeOfAge_self"
		"Tribulation", "Traveled", "LandLinked":
			return "event_%s_%s" % [kind, x]
		"ThreadCreated":
			return "event_ThreadCreated_Mentor" if x == "Mentor" else "event_ThreadCreated"
		"ThreadOmen", "FeudEscalated", "ThreadFaded":
			# n 是這條緣跨了幾世：隔過世的是前世結下的緣。
			return "event_%s_past" % kind if int(event.get("n", 0)) >= 1 else "event_" + kind
		"SoulDissolved":
			# m 是隨魂散而斷的未了宿緣數。
			return "event_SoulDissolved_threads" if int(event.get("m", 0)) >= 1 else "event_SoulDissolved"
		"ThreadResolved":
			return "event_ThreadResolved" if int(event.get("n", 0)) >= 1 else "event_ThreadResolved_once"
		"ProvinceLost":
			return "event_ProvinceLost" if not String(event.get("q", "")).is_empty() else "event_ProvinceLost_none"
		"NodeShifted":
			var to: int = event.get("n", 0)
			var from: int = event.get("m", 0)
			if to < from:
				return "event_NodeShifted_out" if to == 0 else "event_NodeShifted_down"
			return "event_NodeShifted_up"
	if kind == "TribulationOmen":
		# 宗門中人的劫雲聚在自家山門；散修等沒有山門的寫所在州郡，連州郡都沒有就不寫地點。
		if not _sect_of(event).is_empty():
			return "event_TribulationOmen"
		return "event_TribulationOmen_roam" if not String(event.get("p", "")).is_empty() else "event_TribulationOmen_bare"
	if kind == "Succession" and String(event.get("x", "")) == "Dynasty":
		return "event_Succession_dynasty"
	return "event_" + kind


## 主角所屬的宗門名；不是宗門中人（散修、僧、儒、君臣等）為空。
static func _sect_of(event: Dictionary) -> String:
	var code: String = event.get("a_role", "")
	return event.get("a_power", "") if code == "Leader" or code == "Member" else ""


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
		"S": _sect_of(event),
		"X": _word(event),
		"L": Locale.t("node_level_%d" % n),
		"N": str(n),
		"M": str(event.get("m", 0)),
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
	elif kind == "Died" and event.get("x", "") != "Age":
		key = ""
	elif kind == "ThreadCreated" and event.get("x", "") == "Mentor":
		key = "result_ThreadCreated_Mentor"
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
		"ThreadCreated", "ThreadOmen", "ThreadResolved", "ThreadFaded", "FeudEscalated":
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
		"SoulDissolved":
			# 重要度 2 的魂散是尋常的消逝，同月合成一則；重要度 3（有牽掛的宿緣、修為高）各自成則。
			return "SoulDissolved" if int(event["importance"]) <= 2 else ""
		"Died":
			var cause: String = event.get("x", "")
			var role: String = event.get("a_role", "")
			if cause in MERGE_CAUSES and death_realm(event) < HIGH_REALM and role != "Leader" and role != "Ruler":
				return "Died" if cause == "Age" else "Died_" + cause
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
