class_name ChronicleText
extends RefCounted
## GDD 5.12 史書文本：把橋接 `person_chronicle(id)` 的事實清單組成人物卡的小傳與年表。
## 條件文法加句型庫（chronicle.zh-Hant.json）：每種事實有三到六種句型，依事實選用，選擇只看事實與人物、事件編號的雜湊，
## 不用亂數，同一個人每次組出的字逐字相同。執行期不跑語言模型。
##
## 輸出的每段都是 {text, links}：links 的每項是 {text, target}，target 為 `person:`、`province:`、`sect:` 加編號，
## 依字在文中出現的先後排列，交給 LedgerText.paragraph 畫。年表的每列另有 date（年月）與 event（事件編號，點整列開事件卡）。

const LIB_PATH := "res://modules/locale/resources/chronicle.zh-Hant.json"
const DIGITS := "零一二三四五六七八九"
## 同類的事多於這個數量就合併成一句。
const MERGE_OVER := 3

static var _lib: Dictionary = {}
static var _need_re: RegEx = null

var _info: Dictionary
var _lookup: Callable
var _me := -1
var _past: Dictionary = {}
var _entries: Array[Dictionary] = []
var _row_mode := false
var _era := ""
var _prev: Dictionary = {}
var _cur: Dictionary = {}
var _section := ""
var _used_by: Dictionary = {}


## 組出小傳與年表。info 是 `person(id)`，facts 是 `person_chronicle(id)`；lookup(id) 回傳 `person(id)`（收徒挑最高境界者用，可不給）。
static func compose(info: Dictionary, facts: Array, lookup := Callable()) -> Dictionary:
	var c := ChronicleText.new(info, facts, lookup)
	var bio := c.biography()
	return {"bio": bio, "rows": c.timeline(), "used": c._used_by}


func _init(info: Dictionary, facts: Array, lookup := Callable()) -> void:
	_info = info
	_lookup = lookup
	_me = int(info.get("id", -1))
	for fact: Dictionary in facts:
		if fact["kind"] == "past_life":
			_past = fact
		else:
			_entries.append(fact)
	_fold_revenge()


# ── 數字與年月 ────────────────────────────────────────────────

## 漢字數字：十三、二十五、一百零二、一百一十一。
static func han(n: int) -> String:
	if n <= 0:
		return DIGITS[0]
	var parts := [[n / 1000, "千"], [(n / 100) % 10, "百"], [(n / 10) % 10, "十"], [n % 10, ""]]
	var out := ""
	var started := false
	var zero := false
	for part: Array in parts:
		var digit: int = part[0]
		if digit == 0:
			zero = started
			continue
		if zero:
			out += DIGITS[0]
			zero = false
		if digit == 1 and part[1] == "十" and not started:
			out += "十"
		else:
			out += DIGITS[digit] + part[1]
		started = true
	return out


## 年份的寫法：紀元名只在與前一個寫出的年份不同紀元時寫；move 為假（回溯過去的事）不改變目前的紀元。
func _year_text(era: String, year: int, move := true) -> String:
	var head := ""
	if not era.is_empty() and era != _era:
		head = era
		if move:
			_era = era
	return "%s%s年" % [head, han(year)]


func _stamp(e: Dictionary) -> String:
	return "%s%s月" % [_year_text(e["era"], int(e["year"])), han(int(e["month"]))]


# ── 句型庫 ────────────────────────────────────────────────────

static func library() -> Dictionary:
	if _lib.is_empty():
		_lib = JSON.parse_string(FileAccess.get_file_as_string(LIB_PATH))
	return _lib


## 句型的需求與正文：「需求+需求|正文」。
static func _parse(variant: String) -> Array:
	if _need_re == null:
		_need_re = RegEx.create_from_string("^([a-z0-9_]+(?:\\+[a-z0-9_]+)*)[|+]")
	var found := _need_re.search(variant)
	if found == null:
		return [[], variant]
	return [Array(found.get_string(1).split("+")), variant.substr(found.get_end())]


static func _mix(a: int, b: int) -> int:
	var h := ((a & 0xFFFFFFFF) ^ ((b * 0x9E3779B1) & 0xFFFFFFFF)) & 0xFFFFFFFF
	h = (h * 0x85EBCA6B) & 0xFFFFFFFF
	h ^= h >> 13
	h = (h * 0xC2B2AE35) & 0xFFFFFFFF
	return h ^ (h >> 16)


static func _blank(value: Variant) -> bool:
	return value == null or (value is String and value.is_empty()) or (value is Array and value.is_empty())


## 句型的數量與各句型要的需求（測試用）。
static func variant_count(key: String) -> int:
	return (library()[key] as Array).size()


## 挑句型：需求都滿足的才可用，需求最多者優先，同級由人物與事件編號的雜湊選一。
func _pick(key: String, params: Dictionary, salt: int) -> String:
	var variants: Array = library()[key]
	var best := -1
	var pool: Array = []
	for i in variants.size():
		var parsed := _parse(variants[i])
		var ok := true
		for need: String in parsed[0]:
			if _blank(params.get(need)):
				ok = false
				break
		if not ok:
			continue
		if parsed[0].size() > best:
			best = parsed[0].size()
			pool.clear()
		if parsed[0].size() == best:
			pool.append([i, parsed[1]])
	var choice: Array = pool[_mix(_mix(_me, salt), key.hash()) % pool.size()]
	if not _row_mode:
		if not _used_by.has(_section):
			_used_by[_section] = []
		_used_by[_section].append("%s#%d" % [key, choice[0]])
	return choice[1]


func _append(out: Dictionary, value: Variant) -> void:
	if value is String:
		out["t"] += value
	elif value is Array:
		for part: Variant in value:
			_append(out, part)
	elif value is Dictionary:
		if value.has("t"):
			out["t"] += value["t"]
			for link: Dictionary in value["l"]:
				out["l"].append(link)
		else:
			if not String(value["text"]).is_empty():
				out["l"].append({"text": value["text"], "target": value["target"]})
			out["t"] += value["text"]


## 代入句型。when 是小傳裡的年份（年表傳空字串）；dated 為假的句型（接在前一句後面的尾巴）不補年份。
func _say(key: String, params: Dictionary, salt: int, when := "", dated := true) -> Dictionary:
	var template: String = _pick(key, params, salt)
	if dated and not template.contains("@"):
		template = "@" + template
	var out := {"t": "", "l": [] as Array[Dictionary]}
	var i := 0
	while i < template.length():
		var ch := template[i]
		if ch == "@":
			out["t"] += when
		elif ch == "{":
			var close := template.find("}", i)
			var token := template.substr(i + 1, close - i - 1)
			var lead := token.begins_with(",")
			var name := token.trim_prefix(",")
			if not params.has(name):
				out["t"] += "{%s}" % token
			elif not (lead and _blank(params[name])):
				if lead:
					out["t"] += "，"
				_append(out, params[name])
			i = close
		else:
			out["t"] += ch
		i += 1
	return out


func _join(parts: Array, sep := "，") -> Dictionary:
	var out := {"t": "", "l": [] as Array[Dictionary]}
	for i in parts.size():
		if i > 0:
			out["t"] += sep
		_append(out, parts[i])
	return out


# ── 稱呼與連結 ────────────────────────────────────────────────

func _roll() -> void:
	_prev = _cur
	_cur = {}


func _reset_refs() -> void:
	_prev = {}
	_cur = {}


## 人名：本人不連結；同段前一句出現過的人改用代稱（pron）。
func _ref(pid: int, name: String, pron := "") -> Variant:
	if name.is_empty():
		return ""
	if pid >= 0:
		var seen := _prev.has(pid)
		_cur[pid] = true
		if pid == _me:
			return name
		if seen and not pron.is_empty():
			return pron
		return {"text": name, "target": "person:%d" % pid}
	return name


static func _place(id: int, name: String, scheme: String) -> Variant:
	if name.is_empty():
		return ""
	return {"text": name, "target": "%s:%d" % [scheme, id]} if id >= 0 else name


func _prov(id: int, name: String) -> Variant:
	return _place(id, name, "province")


func _sect(id: int, name: String) -> Variant:
	return _place(id, name, "sect")


## 對方：結緣當時用過不同名字（轉世）時寫「前名轉世之今名」。
func _other(e: Dictionary, pron := "其人") -> Variant:
	var other: Dictionary = e
	var now: Variant = _ref(int(other["other"]), other["other_name"], pron)
	var bond_name: String = other.get("other_bond_name", "")
	if bond_name.is_empty() or bond_name == other["other_name"]:
		return now
	return [_ref(int(other["other_bond"]), bond_name), "轉世之", now]


func _is_past(e: Dictionary) -> bool:
	return e.has("because") and int(e["because"]["me"]) != _me and int(e["because"]["me"]) >= 0


## 後果的前因：這條緣是哪一世、因什麼事結下。
func _because(e: Dictionary, salt: int) -> Variant:
	if not e.has("because"):
		return ""
	var b: Dictionary = e["because"]
	var key := "bt_%s_%s" % [b["thread"], e["role"]]
	if not library().has(key):
		return ""
	var params := {
		"y0": _year_text(b["era"], int(b["year"]), false),
		"me0": _ref(int(b["me"]), b["me_name"]),
		"b0": _ref(int(b["other"]), b["other_name"]),
	}
	return _say(key, params, salt * 5 + 3, "", false)


## 這條緣相對於本人是前世的（本人另投一世）還是往日的（本人仍是同一世，是對方轉了世）。
func _world_word(past: bool) -> String:
	return "前世" if past else "往日"


# ── 事實的整理 ────────────────────────────────────────────────

## 報仇、世仇發生在殺人當下：併入殺人（或被殺）那一筆，一件事只說一次。
func _fold_revenge() -> void:
	for i in _entries.size():
		var e := _entries[i]
		if not (e["kind"] in ["thread_resolved", "feud_escalated"]) or not (e["thread"] in ["feud", "grudge"]):
			continue
		if int(e["other"]) < 0:
			continue
		for k: Dictionary in _entries:
			if k["kind"] in ["killed", "died"] and int(k["tick"]) == int(e["tick"]) and int(k["other"]) == int(e["other"]):
				var copy := e.duplicate()
				copy["group"] = k["group"]
				_entries[i] = copy
				break


const SECTION_OF := {
	"come_of_age": "school", "sect_joined": "school", "sect_left": "school", "apprenticed": "school", "mentor_ended": "school",
	"married": "home", "widowed": "home",
	"breakthrough": "cult", "tribulation": "cult", "tribulation_omen": "cult", "fell_to_demon": "cult",
	"traveled": "deeds", "sect_founded": "deeds", "sect_split": "deeds", "succeeded": "deeds", "throne_succeeded": "deeds",
	"dynasty_founded": "deeds", "rebellion": "deeds", "secession": "deeds", "usurpation": "deeds", "fengshan": "deeds",
	"killed": "karma", "thread_created": "karma", "thread_omen": "karma", "thread_resolved": "karma", "feud_escalated": "karma",
	"thread_faded": "karma",
	"died": "end", "ascended": "end",
}


## 把同組的事實併成一件（unit）：anchor 是主紀錄，members 是同組各筆；依年月排序。
func _units() -> Array[Dictionary]:
	var by_group := {}
	var order: Array = []
	for e in _entries:
		var group := int(e["group"])
		if not by_group.has(group):
			by_group[group] = []
			order.append(group)
		by_group[group].append(e)
	var units: Array[Dictionary] = []
	for group in order:
		var members: Array = by_group[group]
		var anchor: Dictionary = members[0]
		for m: Dictionary in members:
			if int(m["id"]) == int(group):
				anchor = m
		units.append({"anchor": anchor, "members": members, "section": SECTION_OF.get(anchor["kind"], ""), "consumed": false, "prev": {}})
	units.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var x: Dictionary = a["anchor"]
		var y: Dictionary = b["anchor"]
		return [int(x["tick"]), int(x["id"])] < [int(y["tick"]), int(y["id"])])
	_pair(units)
	return units


func _same_month(a: Dictionary, b: Dictionary) -> bool:
	return a["era"] == b["era"] and int(a["year"]) == int(b["year"]) and int(a["month"]) == int(b["month"])


## 師亡後改拜、喪偶後再婚、渡劫與突破：前後相連的事併成一句。年表只併同年月的。
func _pair(units: Array[Dictionary]) -> void:
	for i in units.size():
		var u := units[i]
		var a: Dictionary = u["anchor"]
		match a["kind"]:
			"mentor_ended", "widowed":
				var is_master: bool = a["kind"] == "mentor_ended"
				if is_master and (a["role"] != "disciple" or a["end_reason"] == "disciple_left"):
					continue
				for j in range(i + 1, units.size()):
					var b: Dictionary = units[j]["anchor"]
					if b["kind"] == a["kind"]:
						break
					if b["kind"] == ("apprenticed" if is_master else "married") and (not is_master or b["role"] == "disciple"):
						if not _row_mode or _same_month(a, b):
							u["consumed"] = true
							units[j]["prev"] = a
						break
			"tribulation":
				for j in range(units.size()):
					var b: Dictionary = units[j]["anchor"]
					if b["kind"] == "breakthrough" and int(b["tick"]) == int(a["tick"]) and int(b["n"]) == int(a["n"]) and a["detail"] == "Passed":
						units[j]["consumed"] = true
						u["gain_from"] = b
					elif b["kind"] == "died" and int(b["tick"]) == int(a["tick"]) and a["detail"] != "Passed":
						u["consumed"] = true


func _first_of(u: Dictionary, kind: String) -> Dictionary:
	for m: Dictionary in u["members"]:
		if m["kind"] == kind:
			return m
	return {}


func _gain(n: int) -> String:
	if n <= 1:
		return "築基"
	var key := "realm_gain_%d" % n
	return Locale.t(key) if Locale.has(key) else Locale.t("realm_%d" % n)


func _trib_name(n: int) -> String:
	var key := "tribulation_%d" % n
	return Locale.t(key) if Locale.has(key) else "天劫"


# ── 單件事的句子 ──────────────────────────────────────────────

func _render(u: Dictionary, when: String) -> Dictionary:
	var a: Dictionary = u["anchor"]
	var salt := int(a["id"])
	match a["kind"]:
		"come_of_age", "sect_joined":
			return _r_join(u, when, salt)
		"sect_left":
			var why: String = a["why"]
			var key: String = {"left_for_demon": "leave_demon", "sect_destroyed": "leave_destroyed", "followed_master": "leave_followed"}.get(why, "leave_other")
			return _say(key, {"sect": _sect(int(a["power"]), a["power_name"])}, salt, when)
		"apprenticed":
			return _r_apprenticed(u, when, salt)
		"mentor_ended":
			return _r_mentor_ended(u, when, salt)
		"married":
			return _r_married(u, when, salt)
		"widowed":
			return _say("widowed", {"spouse": _ref(int(a["other"]), a["other_name"], _spouse_pronoun())}, salt, when)
		"breakthrough":
			return _say("breakthrough", {"gain": _gain(int(a["n"]))}, salt, when)
		"tribulation":
			return _r_tribulation(u, when, salt)
		"tribulation_omen":
			return _say("omen_one", {}, salt, when)
		"fell_to_demon":
			var key2: String = {"sin_demon": "demon_sin", "past_life_mark": "demon_mark"}.get(a["why"], "demon_default")
			return _say(key2, {}, salt, when)
		"traveled":
			if a["detail"] == "back":
				return _say("travel_back", {"target": _prov(int(a["target"]), a["target_name"]), "prov": _prov(int(a["province"]), a["province_name"])}, salt, when)
			return _say("travel_out", {"target": _prov(int(a["target"]), a["target_name"])}, salt, when)
		"sect_founded":
			return _say("sect_founded", {"sect": _sect(int(a["power"]), a["power_name"])}, salt, when)
		"sect_split":
			return _say("sect_split", {"sect": _sect(int(a["power"]), a["power_name"]), "from": _sect(int(a["power_from"]), a["power_from_name"])}, salt, when)
		"succeeded":
			return _say("succeeded", {"sect": _sect(int(a["power"]), a["power_name"])}, salt, when)
		"throne_succeeded":
			if int(a["power"]) >= 0:
				return _say("throne", {"sect": _sect(int(a["power"]), a["power_name"])}, salt, when)
			return _say("throne_bare", {}, salt, when)
		"dynasty_founded":
			return _say("dynasty_founded", {}, salt, when)
		"rebellion":
			return _say("rebellion_low" if a["why"] == "low_mandate" else "rebellion", {}, salt, when)
		"secession":
			return _say("secession", {}, salt, when)
		"usurpation":
			return _say("usurpation_noheir" if a["why"] == "no_heir" else "usurpation", {}, salt, when)
		"fengshan":
			return _say("fengshan", {}, salt, when)
		"killed":
			return _r_killed(u, when, salt)
		"died", "ascended":
			return _r_died(u, when, salt)
		"thread_omen", "thread_resolved", "thread_faded", "feud_escalated", "thread_created":
			return _r_thread(u, when, salt)
	return {"t": "", "l": [] as Array[Dictionary]}


func _spouse_pronoun() -> String:
	return "其妻" if _info.get("gender", "") == "Male" else "其夫"


func _r_join(u: Dictionary, when: String, salt: int) -> Dictionary:
	var a: Dictionary = u["anchor"]
	var parts: Array = []
	if a["kind"] == "come_of_age" and int(a["power"]) < 0:
		var role: String = a["detail"]
		var key := "coa_" + role
		if role in ["General", "GeneralCandidate"]:
			key = "coa_General"
		if not library().has(key):
			key = "coa_rogue" if role == "Cultivator" else "coa_other"
		if role == "Commoner" or role == "Child":
			key = "coa_commoner"
		parts.append(_say(key, {"act": Locale.t("role_" + role) if Locale.has("role_" + role) else ""}, salt, when))
	else:
		var key2 := "join_born"
		if a["kind"] == "sect_joined":
			key2 = {"rogue_joined": "join_rogue", "followed_master": "join_followed", "recruited": "join_recruited"}.get(a["why"], "join_born")
		parts.append(_say(key2, {"sect": _sect(int(a["power"]), a["power_name"])}, salt, when))
	var master := _first_of(u, "apprenticed")
	if not master.is_empty() and master["role"] == "disciple":
		parts.append(_say("apprentice_tail", {"master": _ref(int(master["other"]), master["other_name"], "其師")}, salt + 1, "", false))
		_tail_resolved(u, parts, "mentor", salt)
	return _join(parts)


## 同組裡了結前世之緣的那一筆，接在句尾。
func _tail_resolved(u: Dictionary, parts: Array, thread: String, salt: int) -> void:
	var r := _first_of(u, "thread_resolved")
	if r.is_empty() or r["thread"] != thread:
		return
	var b: Variant = _because(r, salt)
	parts.append(_say("tail_resolved_" + thread, {"pw": _world_word(_is_past(r)), "bt": b}, salt + 2, "", false))


func _r_apprenticed(u: Dictionary, when: String, salt: int) -> Dictionary:
	var a: Dictionary = u["anchor"]
	var parts: Array = []
	if a["role"] == "master":
		parts.append(_say("take_disciple", {"d": _ref(int(a["other"]), a["other_name"], "其徒")}, salt, when))
		return _join(parts)
	var master: Variant = _ref(int(a["other"]), a["other_name"], "其師")
	var prev: Dictionary = u["prev"]
	if not prev.is_empty():
		var key := "reapprentice_died" if prev["end_reason"] == "master_died" else "reapprentice_left"
		parts.append(_say(key, {"pm": _ref(int(prev["other"]), prev["other_name"], "其師"), "master": master}, salt, when))
	elif _earlier_master(a):
		parts.append(_say("apprentice_again", {"master": master}, salt, when))
	else:
		parts.append(_say("apprentice", {"master": master}, salt, when))
	_tail_resolved(u, parts, "mentor", salt)
	return _join(parts)


func _earlier_master(a: Dictionary) -> bool:
	for e in _entries:
		if e["kind"] == "apprenticed" and e["role"] == "disciple" and [int(e["tick"]), int(e["id"])] < [int(a["tick"]), int(a["id"])]:
			return true
	return false


func _r_mentor_ended(u: Dictionary, when: String, salt: int) -> Dictionary:
	var a: Dictionary = u["anchor"]
	var key := "mentor_end_died"
	if a["role"] == "master":
		key = "mentor_end_master"
	elif a["end_reason"] == "master_left":
		key = "mentor_end_left"
	elif a["end_reason"] == "disciple_left":
		key = "mentor_end_self"
	return _say(key, {"pm": _ref(int(a["other"]), a["other_name"], "其師" if a["role"] == "disciple" else "其徒")}, salt, when)


func _r_married(u: Dictionary, when: String, salt: int) -> Dictionary:
	var a: Dictionary = u["anchor"]
	var verb := "嫁" if _info.get("gender", "") == "Female" else "娶"
	var prev: Dictionary = u["prev"]
	var spouse: Variant = _ref(int(a["other"]), a["other_name"], _spouse_pronoun())
	var parts: Array = []
	if not prev.is_empty():
		var ps: Variant = _ref(int(prev["other"]), prev["other_name"], _spouse_pronoun())
		parts.append(_say("remarry", {"ps": ps, "verb": verb, "spouse": spouse}, salt, when))
	elif _earlier_marriage(a):
		parts.append(_say("remarry_plain", {"verb": verb, "spouse": spouse}, salt, when))
	else:
		var params := {"spouse": spouse}
		match a["why"]:
			"same_hometown":
				params["home"] = "1"
			"same_sect":
				params["school"] = "1"
			"fate_tug":
				params["fate"] = "1"
		parts.append(_say("marry_f" if verb == "嫁" else "marry_m", params, salt, when))
	_tail_resolved(u, parts, "love", salt)
	return _join(parts)


func _earlier_marriage(a: Dictionary) -> bool:
	for e in _entries:
		if e["kind"] == "married" and [int(e["tick"]), int(e["id"])] < [int(a["tick"]), int(a["id"])]:
			return true
	return false


func _r_tribulation(u: Dictionary, when: String, salt: int) -> Dictionary:
	var a: Dictionary = u["anchor"]
	var n := int(a["n"])
	var trib := _trib_name(n)
	if a["detail"] == "Passed":
		if u.has("gain_from"):
			return _say("trib_passed", {"trib": trib, "gain": _gain(n)}, salt, when)
		return _say("trib_passed_bare", {"trib": trib}, salt, when)
	return _say("trib_failed", {"trib": trib}, salt, when)


func _feud_tail(u: Dictionary, salt: int) -> String:
	var made := _first_of(u, "thread_created")
	if made.is_empty() or not (made["thread"] in ["feud", "grudge"]):
		return ""
	var key := "grudge_tail" if made["thread"] == "grudge" else ("feud_tail_killer" if made["role"] == "killer" else "feud_tail_victim")
	return _say(key, {}, salt + 4, "", false)["t"]


func _r_killed(u: Dictionary, when: String, salt: int) -> Dictionary:
	var a: Dictionary = u["anchor"]
	var resolved := _first_of(u, "thread_resolved")
	var escalated := _first_of(u, "feud_escalated")
	var victim: Variant = _ref(int(a["other"]), a["other_name"], "其人")
	var prov: Variant = _prov(int(a["province"]), a["province_name"])
	if not resolved.is_empty() and resolved["role"] == "victim":
		return _say("kill_revenge", {"victim": _other(resolved), "prov": prov, "pw": _world_word(_is_past(resolved)), "bt": _because(resolved, salt)}, salt, when)
	if not escalated.is_empty():
		return _say("kill_feud", {"victim": victim, "prov": prov, "bt": _because(escalated, salt)}, salt, when)
	var key: String = {"vendetta": "kill_vendetta", "slay_demon": "kill_slay", "demon_assassin": "kill_assassin", "sect_war_death": "kill_war"}.get(a["why"], "kill_default")
	var parts: Array = [_say(key, {"victim": victim, "prov": prov, "feud": _feud_tail(u, salt)}, salt, when)]
	if not resolved.is_empty():
		parts.append(_say("tail_settled", {"pw": _world_word(_is_past(resolved)), "bt": _because(resolved, salt)}, salt + 1, "", false))
	return _join(parts)


func _death_cause_word(code: String) -> String:
	return Locale.t("death_" + code)


func _r_died(u: Dictionary, when: String, salt: int) -> Dictionary:
	var a: Dictionary = u["anchor"]
	var prov: Variant = _prov(int(a["province"]), a["province_name"])
	if a["kind"] == "ascended":
		return _say("ascend", {"prov": prov}, salt, when)
	var age := han(int(a["n"]))
	var cause: String = a["detail"]
	if cause == "Killed" and int(a["other"]) >= 0:
		var killer: Variant = _ref(int(a["other"]), a["other_name"], "其人")
		var resolved := _first_of(u, "thread_resolved")
		var escalated := _first_of(u, "feud_escalated")
		if not resolved.is_empty():
			return _say("death_killed_revenge", {"prov": prov, "killer": killer, "age": age, "pw": _world_word(_is_past(resolved)), "bt": _because(resolved, salt)}, salt, when)
		if not escalated.is_empty():
			return _say("death_killed_feud", {"prov": prov, "killer": killer, "age": age, "bt": _because(escalated, salt)}, salt, when)
		var key: String = {"vendetta": "death_killed_vendetta", "slay_demon": "death_killed_slay", "demon_assassin": "death_killed_assassin", "sect_war_death": "death_killed_war"}.get(a["why"], "death_killed")
		return _say(key, {"prov": prov, "killer": killer, "age": age, "feud": _feud_tail(u, salt)}, salt, when)
	if cause == "Age":
		return _say("death_natural", {"prov": prov, "age": age}, salt, when)
	if cause in ["Tribulation", "ShedBody"]:
		for e in _entries:
			if e["kind"] == "tribulation" and int(e["tick"]) == int(a["tick"]):
				return _say("death_trib", {"prov": prov, "age": age, "trib": _trib_name(int(e["n"])), "cause": _death_cause_word(cause)}, salt, when)
	return _say("death_cause", {"prov": prov, "age": age, "cause": _death_cause_word(cause)}, salt, when)


func _r_thread(u: Dictionary, when: String, salt: int) -> Dictionary:
	var a: Dictionary = u["anchor"]
	var thread: String = a["thread"]
	var role: String = a["role"]
	var params := {"other": _other(a), "pw": _world_word(_is_past(a)), "bt": _because(a, salt),
		"rel": library()["rel"].get("%s/%s" % [thread, role], "有緣人")}
	match a["kind"]:
		"thread_omen":
			return _say("omen", params, salt, when)
		"thread_faded":
			return _say("fade", params, salt, when)
		"feud_escalated":
			return _say("escalate", params, salt, when)
		"thread_created":
			var key := "created_%s_%s" % [thread, role]
			if not library().has(key):
				return {"t": "", "l": [] as Array[Dictionary]}
			return _say(key, params, salt, when)
		_:
			var rkey := "resolve_" + thread
			if thread == "grace":
				rkey += "_" + role
			if not library().has(rkey):
				return {"t": "", "l": [] as Array[Dictionary]}
			return _say(rkey, params, salt, when)


# ── 小傳 ──────────────────────────────────────────────────────

func _ident() -> Array:
	var role: String = _info.get("role", "")
	var power := int(_info.get("power", -1))
	var power_name: String = _info.get("power_name", "")
	if role == "Cultivator":
		if power_name.is_empty() or power < 0:
			return [Locale.t("identity_loose")]
		return [_sect(power, power_name), Locale.t("power_leader" if _info.get("is_leader", false) else "power_member")]
	var out: Array = []
	if power >= 0 and not power_name.is_empty():
		out.append(_sect(power, power_name))
	out.append(PersonText.role_word(role))
	return out


func _past_ident(p: Dictionary) -> Array:
	var role: String = p["identity"]
	var power := int(p["power"])
	if role == "Cultivator" and power < 0:
		return [Locale.t("identity_loose")]
	var out: Array = []
	if power >= 0 and not String(p["power_name"]).is_empty():
		out.append(_sect(power, p["power_name"]))
	out.append(PersonText.role_word(role))
	return out


## 前世一段：怎麼死的、死後去哪、何時何地轉生。
func _past_clause(p: Dictionary, with_dates: bool) -> Dictionary:
	var salt := int(p["id"]) if int(p["id"]) >= 0 else int(p["tick"])
	var dwhen := _year_text(p["era"], int(p["year"])) if with_dates else ""
	var killer: Dictionary = {}
	var parts: Array = []
	var params := {"pid": _past_ident(p), "pname": _ref(int(p["person"]), p["name"]), "dwhen": dwhen, "age": han(int(p["age"]))}
	if p["cause"] == "Killed" and not String(p.get("killer_name", "")).is_empty():
		params["killer"] = _ref(int(p["killer"]), p["killer_name"])
		parts.append(_say("past_killed", params, salt, "", false))
	else:
		params["cause"] = _death_cause_word(p["cause"])
		parts.append(_say("past_died", params, salt, "", false))
	var path: String = p.get("path", "")
	var reborn := {
		"path": Locale.t("path_" + path) if not path.is_empty() and Locale.has("path_" + path) else "",
		"rwhen": ("同年" if p["era"] == p["reborn_era"] and int(p["year"]) == int(p["reborn_year"]) else _year_text(p["reborn_era"], int(p["reborn_year"]))) if with_dates else "",
		"rprov": _prov(int(p["reborn_province"]), p["reborn_province_name"]),
	}
	parts.append(_say("past_reborn", reborn, salt + 1, "", false))
	return _join(parts)


func _sentences(clauses: Array) -> Dictionary:
	var out := {"t": "", "l": [] as Array[Dictionary]}
	for i in clauses.size():
		_append(out, clauses[i])
		out["t"] += "。" if i % 2 == 1 or i == clauses.size() - 1 else "；"
	return out


func _section_clauses(units: Array[Dictionary], section: String) -> Array:
	var mine: Array[Dictionary] = []
	for u in units:
		if u["section"] == section and not u["consumed"]:
			mine.append(u)
	_section = section
	var clauses: Array = []
	var draw := func(u: Dictionary) -> void:
		var a: Dictionary = u["anchor"]
		var clause := _render(u, _year_text(a["era"], int(a["year"])))
		if not String(clause["t"]).is_empty():
			clauses.append(clause)
		_roll()
	match section:
		"school":
			var pupils: Array[Dictionary] = []
			for u in mine:
				if u["anchor"]["kind"] == "apprenticed" and u["anchor"]["role"] == "master":
					pupils.append(u)
				else:
					draw.call(u)
			if pupils.size() > MERGE_OVER:
				clauses.append(_take_many(pupils))
				_roll()
			else:
				for u in pupils:
					draw.call(u)
		"home":
			var wed: Array[Dictionary] = []
			for u in mine:
				if u["anchor"]["kind"] == "married":
					wed.append(u)
			if wed.size() > MERGE_OVER:
				var last: Dictionary = wed[-1]["anchor"]
				var verb := "嫁" if _info.get("gender", "") == "Female" else "娶"
				clauses.append(_say("married_many", {"n": han(wed.size()), "verb": verb, "spouse": _ref(int(last["other"]), last["other_name"])}, int(last["id"])))
				_roll()
				for u in mine:
					if u["anchor"]["kind"] == "widowed" and int(u["anchor"]["tick"]) > int(last["tick"]):
						draw.call(u)
			else:
				for u in mine:
					draw.call(u)
		"cult":
			_cult_clauses(mine, clauses, draw)
		"deeds":
			var trips: Array[Dictionary] = []
			for u in mine:
				if u["anchor"]["kind"] == "traveled":
					if u["anchor"]["detail"] == "out":
						trips.append(u)
				else:
					draw.call(u)
			if trips.size() > MERGE_OVER:
				clauses.append(_travel_many(trips))
				_roll()
			else:
				for u in trips:
					draw.call(u)
		"karma":
			var faded: Array[Dictionary] = []
			for u in mine:
				if u["anchor"]["kind"] == "thread_faded":
					faded.append(u)
			for u in mine:
				if faded.size() <= MERGE_OVER or u["anchor"]["kind"] != "thread_faded":
					draw.call(u)
			if faded.size() > MERGE_OVER:
				clauses.append(_say("fade_many", {"n": han(faded.size())}, int(faded[0]["anchor"]["id"]), "", false))
		_:
			for u in mine:
				draw.call(u)
	return clauses


## 收徒多於三人：「先後收徒五人」，只點出境界最高者。
func _take_many(pupils: Array[Dictionary]) -> Dictionary:
	var best_rank := -2
	var best: Dictionary = {}
	for u in pupils:
		var a: Dictionary = u["anchor"]
		var rank := _rank(int(a["other"]))
		if rank > best_rank:
			best_rank = rank
			best = a
	var params := {"n": han(pupils.size())}
	if best_rank >= 0 and _lookup.is_valid():
		params["nb"] = _ref(int(best["other"]), best["other_name"])
	return _say("take_many", params, int(pupils[0]["anchor"]["id"]), "", false)


## 境界高低：境界或果位的序數，查不到為 -1。
func _rank(pid: int) -> int:
	if not _lookup.is_valid():
		return -1
	var p: Dictionary = _lookup.call(pid)
	if p.is_empty():
		return -1
	return maxi(int(p.get("realm", -1)), int(p.get("fruit", -1)))


func _cult_clauses(mine: Array[Dictionary], clauses: Array, draw: Callable) -> void:
	var levels: Array[Dictionary] = []
	var omens: Array[Dictionary] = []
	var rest: Array[Dictionary] = []
	for u in mine:
		var a: Dictionary = u["anchor"]
		if a["kind"] == "breakthrough" or (a["kind"] == "tribulation" and a["detail"] == "Passed"):
			levels.append(u)
		elif a["kind"] == "tribulation_omen":
			omens.append(u)
		else:
			rest.append(u)
	if levels.size() > MERGE_OVER:
		var first: Dictionary = levels[0]["anchor"]
		var last: Dictionary = levels[-1]["anchor"]
		var y1 := _year_text(first["era"], int(first["year"]))
		var y2 := _year_text(last["era"], int(last["year"]))
		clauses.append(_say("cult_summary", {"n": han(levels.size()), "y1": y1, "g1": _gain(int(first["n"])), "y2": y2, "g2": _gain(int(last["n"]))}, int(first["id"]), "", false))
		_roll()
	else:
		for u in levels:
			draw.call(u)
	if omens.size() > 1:
		clauses.append(_say("omen_many", {"n": han(omens.size())}, int(omens[0]["anchor"]["id"]), "", false))
		_roll()
	else:
		for u in omens:
			draw.call(u)
	for u in rest:
		draw.call(u)


func _travel_many(trips: Array[Dictionary]) -> Dictionary:
	var seen := {}
	var names: Array = []
	for u in trips:
		var a: Dictionary = u["anchor"]
		if not seen.has(int(a["target"])):
			seen[int(a["target"])] = true
			names.append(a)
	var shown: Array = []
	for i in mini(3, names.size()):
		if i > 0:
			shown.append("、")
		shown.append(_prov(int(names[i]["target"]), names[i]["target_name"]))
	return _say("travel_many", {"n": han(trips.size()), "k": han(seen.size()), "ps": shown}, int(trips[0]["anchor"]["id"]), "", false)


## 小傳：一到四段文字，每段 {text, links}。
func biography() -> Array[Dictionary]:
	_row_mode = false
	_reset_refs()
	var units := _units()
	_era = _past["era"] if not _past.is_empty() else (_entries[0]["era"] if not _entries.is_empty() else "")
	var paragraphs: Array[Dictionary] = []
	# 出身與前世。
	_section = "origin"
	var alive: bool = _info.get("alive", true)
	var params := {"me": _info["name"], "home": _prov(int(_info.get("home", -1)), _info.get("home_name", "")), "ident": _ident(), "title": _info.get("title", "")}
	var lead: Array = [_say("origin_alive" if alive else "origin_dead", params, _me)]
	_roll()
	if not _past.is_empty():
		_section = "past"
		lead.append(_past_clause(_past, true))
		_roll()
	var para := {"t": "", "l": [] as Array[Dictionary]}
	for i in lead.size():
		_append(para, lead[i])
		para["t"] += "。"
	paragraphs.append(para)
	# 其餘各段，依序合成三段。
	for group: Array in [["school", "home"], ["cult", "deeds"], ["karma", "end"]]:
		_reset_refs()
		var text := {"t": "", "l": [] as Array[Dictionary]}
		for section: String in group:
			_append(text, _sentences(_section_clauses(units, section)))
		if not String(text["t"]).is_empty():
			paragraphs.append(text)
	var out: Array[Dictionary] = []
	for p in paragraphs:
		out.append({"text": p["t"], "links": p["l"]})
	return out


# ── 年表 ──────────────────────────────────────────────────────

## 年表：一年月一條，舊的在上，不含主詞的完整句子，不合併不省略；前世的事只在第一條交代。
func timeline() -> Array[Dictionary]:
	_row_mode = true
	_reset_refs()
	_section = "rows"
	var units := _units()
	var rows: Array[Dictionary] = []
	if not _past.is_empty():
		_era = _past["reborn_era"]
		var stamp := "%s%s月" % [_year_text(_past["reborn_era"], int(_past["reborn_year"])), han(int(_past["reborn_month"]))]
		var clause := _past_clause(_past, false)
		rows.append(_row(stamp, [clause], int(_past["id"])))
	elif not units.is_empty():
		_era = units[0]["anchor"]["era"]
	var bucket: Array = []
	var bucket_head: Dictionary = {}
	for u in units:
		if u["consumed"]:
			continue
		var a: Dictionary = u["anchor"]
		if not bucket.is_empty() and not _same_month(bucket_head, a):
			_flush(bucket, bucket_head, rows)
			bucket = []
		if bucket.is_empty():
			bucket_head = a
		bucket.append(u)
	_flush(bucket, bucket_head, rows)
	return rows


func _flush(bucket: Array, head: Dictionary, rows: Array[Dictionary]) -> void:
	if bucket.is_empty():
		return
	_reset_refs()
	var clauses: Array = []
	var stamp := _stamp(head)
	for u: Dictionary in bucket:
		var clause := _render(u, "")
		_roll()
		if not String(clause["t"]).is_empty():
			clauses.append(clause)
	if clauses.is_empty():
		return
	rows.append(_row(stamp, clauses, int(head["id"])))


func _row(stamp: String, clauses: Array, event: int) -> Dictionary:
	var text := _join(clauses, "；")
	text["t"] += "。"
	return {"date": stamp, "text": text["t"], "links": text["l"], "event": event}


static var _keep_re: RegEx = null


## 折行時整段不拆的片段：漢字數字的年月（「一百零二年二月」）。
static func keep_pieces(text: String) -> Array[String]:
	if _keep_re == null:
		_keep_re = RegEx.create_from_string("[零一二三四五六七八九十百千]+年(?:[零一二三四五六七八九十]+月)?")
	var out: Array[String] = []
	for found in _keep_re.search_all(text):
		var piece := found.get_string()
		if not out.has(piece):
			out.append(piece)
	return out
