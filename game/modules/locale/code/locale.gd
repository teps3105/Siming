class_name Locale
extends RefCounted
## GDD 8.6、9.5 介面字串：顯示文字一律由此取用，不寫在場景或其他腳本裡。

const UI_PATH := "res://modules/locale/resources/ui.zh-Hant.json"

static var _ui: Dictionary = {}


## 取介面字串；找不到時回傳鍵名本身，方便在畫面上發現缺漏。
static func t(key: String) -> String:
	if _ui.is_empty():
		_ui = JSON.parse_string(FileAccess.get_file_as_string(UI_PATH))
	return _ui.get(key, key)


## 有沒有這個鍵的字串。
static func has(key: String) -> bool:
	t("")
	return _ui.has(key)


## 人口等大數：一萬以上以「萬」為單位，保留一位小數。
static func count(value: float) -> String:
	if value >= 10000.0:
		return "%.1f%s" % [value / 10000.0, t("unit_wan")]
	return str(int(round(value)))
