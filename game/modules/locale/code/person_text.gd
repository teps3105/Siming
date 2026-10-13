class_name PersonText
extends RefCounted
## GDD 5.8 人物的身分一行與基本資料用語：把橋接的 person()、people_in() 字典換成介面用語。


## 身分的名詞（修士、僧人…）。
static func role_word(role: String) -> String:
	var key := "identity_" + role
	return Locale.t(key) if Locale.has(key) else role


## 境界或果位；沒有為空字串。
static func stage(info: Dictionary) -> String:
	if int(info.get("realm", -1)) >= 0:
		return Locale.t("realm_%d" % int(info["realm"]))
	if int(info.get("fruit", -1)) >= 0:
		return Locale.t("fruit_%d" % int(info["fruit"]))
	return ""


## 門派與職分：「太乙宗掌門」「太乙宗弟子」；修士沒有門派為「散修」。
static func affiliation(info: Dictionary) -> String:
	var power: String = info.get("power_name", "")
	if power.is_empty():
		return Locale.t("identity_loose") if info.get("role", "") == "Cultivator" else ""
	return power + Locale.t("power_leader" if info.get("is_leader", false) else "power_member")


## 身分一行：「太乙宗掌門 · 金丹」「僧人 · 羅漢」「孩童」。
static func identity_line(info: Dictionary) -> String:
	var parts: Array[String] = []
	var role: String = info.get("role", "")
	if role == "Cultivator":
		parts.append(affiliation(info))
	else:
		parts.append(role_word(role))
		var power: String = info.get("power_name", "")
		if not power.is_empty():
			parts.append(power)
	var level := stage(info)
	if not level.is_empty():
		parts.append(level)
	return " · ".join(parts)


static func gender_word(gender: String) -> String:
	return Locale.t("male" if gender == "Male" else "female")


static func death_word(cause: String) -> String:
	return Locale.t("death_" + cause)
