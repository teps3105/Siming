class_name GameSettings
extends RefCounted
## GDD 9.1 設定：存在使用者資料夾的一個設定檔；其他模組只經這裡讀寫。

const DEFAULT_PATH := "user://settings.cfg"
const SECTION := "settings"
const UI_SCALE_MIN := 0.8
const UI_SCALE_MAX := 1.5
const UI_SCALE_STEP := 0.1
## 視窗模式下可選的大小。
const WINDOW_SIZES: Array[Vector2i] = [Vector2i(1280, 720), Vector2i(1600, 900), Vector2i(1920, 1080), Vector2i(2560, 1440)]
## 各項的預設值。
const DEFAULTS := {
	"fullscreen": false,
	"window_size": 2,
	"vsync": true,
	"ui_scale": 1.0,
	"edge_scroll": false,
	# 地圖上的文字標籤（地名、治所名、宗門名）；關掉時只留墨點與圖樣。
	"map_labels": true,
	"tutorial_done": false,
	"tutorial_step": 0,
	# 導播模式（GDD 5.13）。節奏 0 慢、1 標準、2 快。
	"director": true,
	"director_pace": 1,
	"director_idle": 20,
	"director_slow": true,
	"director_card": true,
	"director_reduce": false,
}
## 程式覆寫的預設值（不寫入設定檔）：測試執行器用它預設關閉導播，使用者存過的值仍優先。
static var default_overrides := {}

## 設定檔的位置；測試會改到別處。
static var file_path := DEFAULT_PATH
static var _file: ConfigFile


## 改用另一份設定檔；fresh 為真時先刪掉舊檔，從預設值開始（測試與拍圖工具用，不碰玩家的設定檔）。
static func use_file(path: String, fresh := false) -> void:
	file_path = path
	if fresh and FileAccess.file_exists(path):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(path))
	load_file()


static func load_file() -> void:
	_file = ConfigFile.new()
	_file.load(file_path)


static func value(key: String) -> Variant:
	if _file == null:
		load_file()
	var fallback: Variant = default_overrides.get(key, DEFAULTS[key])
	var stored: Variant = _file.get_value(SECTION, key, fallback)
	return stored if typeof(stored) == typeof(DEFAULTS[key]) else fallback


static func flag(key: String) -> bool:
	return value(key)


## 介面縮放（0.8–1.5）。
static func ui_scale() -> float:
	return clampf(value("ui_scale"), UI_SCALE_MIN, UI_SCALE_MAX)


static func window_size() -> Vector2i:
	return WINDOW_SIZES[clampi(value("window_size"), 0, WINDOW_SIZES.size() - 1)]


## 改一項並立刻寫入設定檔。
static func set_value(key: String, new_value: Variant) -> void:
	if _file == null:
		load_file()
	_file.set_value(SECTION, key, new_value)
	DirAccess.make_dir_recursive_absolute(file_path.get_base_dir())
	_file.save(file_path)


## 把顯示相關的設定套到視窗上（無視窗模式不做）。
static func apply_display(window: Window) -> void:
	if DisplayServer.get_name() == "headless":
		return
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_ENABLED if flag("vsync") else DisplayServer.VSYNC_DISABLED)
	var want := Window.MODE_FULLSCREEN if flag("fullscreen") else Window.MODE_WINDOWED
	if window.mode != want:
		window.mode = want
	if not flag("fullscreen") and window.size != window_size():
		var screen := DisplayServer.screen_get_usable_rect(window.current_screen)
		window.size = window_size().min(screen.size)
		window.position = screen.position + (screen.size - window.size) / 2
