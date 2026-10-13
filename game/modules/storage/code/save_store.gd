class_name SaveStore
extends RefCounted
## GDD 4.8 存檔：手動 10 欄、自動存檔留最新 5 份，與分享碼。
## 一個檔一份存檔（JSON）：欄位資訊（世界名、日期、存檔時間）加上模擬端的存檔文字（SimWorld.save_text）。

const DEFAULT_ROOT := "user://saves"
const MANUAL_SLOTS := 10
const AUTO_KEEP := 5
const FORMAT := 1
## 自動存檔的間隔：一世（30 年）。
const AUTO_INTERVAL_MONTHS := 360
## tick 以日為單位（2.6）：每月 30 日。
const DAYS_PER_MONTH := 30
const SHARE_PREFIX := "SMG1-"
## 分享碼解壓後的大小上限（位元組）。
const SHARE_MAX_BYTES := 16 * 1024 * 1024
const SHARE_DIGEST_BYTES := 4

## 存檔資料夾；測試會改到別處。
static var root := DEFAULT_ROOT


## 手動欄位，共 MANUAL_SLOTS 格；空欄是空字典。
static func manual_slots() -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	for slot in MANUAL_SLOTS:
		out.append(_read_entry(_manual_path(slot)))
	return out


## 自動存檔，最新的排最前。
static func auto_saves() -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	if not DirAccess.dir_exists_absolute(root):
		return out
	for name in DirAccess.get_files_at(root):
		if name.begins_with("auto_") and name.ends_with(".json"):
			var entry := _read_entry(root.path_join(name))
			if not entry.is_empty():
				out.append(entry)
	out.sort_custom(_newer)
	return out


## 最近存的一份（手動與自動一起比）；沒有存檔時是空字典。
static func latest() -> Dictionary:
	var all := auto_saves()
	for entry in manual_slots():
		if not entry.is_empty():
			all.append(entry)
	all.sort_custom(_newer)
	return {} if all.is_empty() else all[0]


## 存到手動欄位 slot（覆寫）。失敗回傳原因，成功回傳空字串。
static func save_manual(slot: int, world: SimWorld) -> String:
	return _write(_manual_path(slot), "manual", slot, world)


## 存一份自動存檔，並刪掉超過 AUTO_KEEP 份的舊檔。
static func save_auto(world: SimWorld) -> String:
	var error := _write(root.path_join("auto_%010d_%d.json" % [world.tick(), Time.get_ticks_usec()]), "auto", -1, world)
	if error.is_empty():
		var autos := auto_saves()
		for i in range(AUTO_KEEP, autos.size()):
			delete(autos[i]["path"])
	return error


## 從 from_tick 推進到 to_tick 之間是否跨過了自動存檔的時刻（每世一次）。
static func auto_due(from_tick: int, to_tick: int) -> bool:
	return _period(to_tick) > _period(from_tick)


## 存檔裡的模擬端存檔文字（交給 SimWorld.load_begin）；讀不到時回傳空字串。
static func read_text(path: String) -> String:
	var data := _read_json(path)
	return data.get("save", "") if data.get("save", "") is String else ""


static func delete(path: String) -> void:
	if FileAccess.file_exists(path):
		DirAccess.remove_absolute(path)


## 分享碼：存檔文字壓縮後，前面加 4 個位元組的校驗，以網址安全的 Base64 表示，再加版本字首。
static func share_code(save_text: String) -> String:
	var body := save_text.to_utf8_buffer().compress(FileAccess.COMPRESSION_DEFLATE)
	var packed := _digest(body)
	packed.append_array(body)
	return SHARE_PREFIX + Marshalls.raw_to_base64(packed).replace("+", "-").replace("/", "_").trim_suffix("=").trim_suffix("=")


## 解開分享碼：回傳 text（存檔文字）與 error（空字串為無誤）。
static func parse_share_code(code: String) -> Dictionary:
	var body := code.strip_edges()
	if not body.begins_with(SHARE_PREFIX):
		return {"text": "", "error": Locale.t("share_error_prefix")}
	body = body.trim_prefix(SHARE_PREFIX).replace("-", "+").replace("_", "/")
	for ch in body:
		if not (ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"):
			return {"text": "", "error": Locale.t("share_error_broken")}
	while body.length() % 4 != 0:
		body += "="
	var packed := Marshalls.base64_to_raw(body)
	if packed.size() <= SHARE_DIGEST_BYTES or packed.slice(0, SHARE_DIGEST_BYTES) != _digest(packed.slice(SHARE_DIGEST_BYTES)):
		return {"text": "", "error": Locale.t("share_error_broken")}
	var raw := packed.slice(SHARE_DIGEST_BYTES).decompress_dynamic(SHARE_MAX_BYTES, FileAccess.COMPRESSION_DEFLATE)
	var text := raw.get_string_from_utf8() if not raw.is_empty() else ""
	if text.is_empty() or not text.begins_with("{"):
		return {"text": "", "error": Locale.t("share_error_broken")}
	return {"text": text, "error": ""}


## 校驗：SHA-256 的前 4 個位元組（抄錯或截斷的分享碼在解壓前就擋下）。
static func _digest(data: PackedByteArray) -> PackedByteArray:
	var hasher := HashingContext.new()
	hasher.start(HashingContext.HASH_SHA256)
	hasher.update(data)
	return hasher.finish().slice(0, SHARE_DIGEST_BYTES)


static func _period(tick: int) -> int:
	return maxi(tick - 1, 0) / (AUTO_INTERVAL_MONTHS * DAYS_PER_MONTH)


static func _manual_path(slot: int) -> String:
	return root.path_join("slot_%02d.json" % slot)


static func _newer(a: Dictionary, b: Dictionary) -> bool:
	if a["saved_at"] != b["saved_at"]:
		return a["saved_at"] > b["saved_at"]
	return a["order"] > b["order"]


static func _write(path: String, kind: String, slot: int, world: SimWorld) -> String:
	if not world.is_ready():
		return Locale.t("save_error_no_world")
	DirAccess.make_dir_recursive_absolute(root)
	var options := world.world_options()
	var data := {
		"format": FORMAT,
		"kind": kind,
		"slot": slot,
		"name": options.get("name", ""),
		"date": EventText.now_label(world),
		"tick": world.tick(),
		"saved_at": Time.get_unix_time_from_system(),
		"order": Time.get_ticks_usec(),
		"options": options,
		"save": world.save_text(),
	}
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return Locale.t("save_error_write")
	file.store_string(JSON.stringify(data))
	file.close()
	return ""


static func _read_json(path: String) -> Dictionary:
	if not FileAccess.file_exists(path):
		return {}
	var json := JSON.new()
	if json.parse(FileAccess.get_file_as_string(path)) != OK or not json.data is Dictionary:
		return {}
	return json.data


## 欄位資訊（不含存檔文字本身；options 是開局選項），另加 path。檔案不在或讀不懂時是空字典。
static func _read_entry(path: String) -> Dictionary:
	var data := _read_json(path)
	if data.is_empty() or int(data.get("format", 0)) != FORMAT or not data.has("save"):
		return {}
	return {
		"path": path,
		"kind": str(data.get("kind", "manual")),
		"slot": int(data.get("slot", -1)),
		"name": str(data.get("name", "")),
		"date": str(data.get("date", "")),
		"tick": int(data.get("tick", 0)),
		"saved_at": float(data.get("saved_at", 0.0)),
		"order": float(data.get("order", 0.0)),
		"options": _options_of(data),
	}


## 存檔裡的開局選項；JSON 讀回的種子是浮點數，轉回整數。
static func _options_of(data: Dictionary) -> Dictionary:
	var options: Dictionary = data.get("options", {})
	if options.has("seed"):
		options["seed"] = int(options["seed"])
	return options
