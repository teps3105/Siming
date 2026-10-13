extends SceneTree
## 測試的執行程式（GDD 6.7）：找出 res://tests 底下全部 test_*.gd，逐一跑完每個案例並判決，
## 最後以結束碼回報（0 全過、1 有失敗）。在 repo 根目錄執行：
##   Godot_v4.7.1-stable_win64_console.exe --headless --audio-driver Dummy --path game --script res://tests/harness/run.gd
## 後面可加 -- --suite=套件名稱 --case=案例名稱的一部分 --report=報告.json 的路徑。
## 判決：斷言失敗、案例期間出現沒有事先宣告的錯誤、一個斷言都沒有、套件載入失敗、沒有執行任何案例，都算失敗。

const Suite := preload("res://tests/harness/suite.gd")
const ErrorLog := preload("res://tests/harness/error_log.gd")
const ROOT := "res://tests"
## 測試的世界用短預跑（人口期 1,500 年、人物期 100 年），不然每個案例開局都要等十秒以上。
const TEST_POP_YEARS := 1500
const TEST_PRERUN_YEARS := 100

var _log := ErrorLog.new()


func _initialize() -> void:
	_main.call_deferred()


func _main() -> void:
	var args := {}
	for arg in OS.get_cmdline_user_args():
		var pair := arg.trim_prefix("--").split("=", true, 1)
		args[pair[0]] = pair[1] if pair.size() > 1 else ""
	OS.add_logger(_log)
	SimWorld.set_default_prerun(TEST_POP_YEARS, TEST_PRERUN_YEARS)
	var results: Array[Dictionary] = []
	var load_errors: Array[String] = []
	var paths := _discover(ROOT)
	paths.sort()
	for path in paths:
		var script := load(path) as GDScript
		var suite: Variant = script.new() if script != null and script.can_instantiate() else null
		if suite == null or not suite is Suite:
			load_errors.append(path)
			continue
		if args.has("suite") and suite.suite_name() != args.suite:
			continue
		suite.tree = self
		for method in _cases(suite):
			if args.has("case") and not method.contains(args.case):
				continue
			results.append(await _run_case(suite, method))
	OS.remove_logger(_log)
	var failed := results.filter(func(result: Dictionary) -> bool: return not result.passed)
	for result in failed:
		print("✗ %s.%s：%s" % [result.suite, result.case, result.message])
	for path in load_errors:
		print("✗ 載入失敗：%s" % path)
	var suites := {}
	for result in results:
		suites[result.suite] = true
	print("測試：%d 個套件 %d 項，通過 %d，失敗 %d，載入失敗 %d" % [suites.size(), results.size(),
		results.size() - failed.size(), failed.size(), load_errors.size()])
	if args.has("report"):
		var file := FileAccess.open(args.report, FileAccess.WRITE)
		file.store_string(JSON.stringify({"results": results, "load_errors": load_errors}, "  "))
	Suite.wipe_storage()
	var empty := results.is_empty()
	if empty:
		print("✗ 沒有執行任何案例")
	quit(1 if not failed.is_empty() or not load_errors.is_empty() or empty else 0)


func _run_case(suite: Suite, method: String) -> Dictionary:
	suite._begin_case()
	_log.begin()
	await suite.setup()
	await suite.call(method)
	await suite.teardown()
	var errors := _log.end()
	await suite._finish_case()
	var unexpected: Array[String] = []
	var expected := suite._expected_errors.duplicate()
	for error in errors:
		var index := -1
		for i in expected.size():
			if error.contains(expected[i]):
				index = i
				break
		if index < 0:
			unexpected.append(error)
		else:
			expected.remove_at(index)
	var message := ""
	if suite._failed:
		message = suite._message
	elif not unexpected.is_empty():
		message = "出現錯誤：%s" % "；".join(unexpected)
	elif not expected.is_empty():
		message = "宣告的錯誤沒有出現：%s" % "；".join(expected)
	elif suite._assertions == 0:
		message = "沒有任何斷言"
	return {"suite": suite.suite_name(), "case": method, "passed": message.is_empty(), "message": message,
		"assertions": suite._assertions}


func _cases(suite: Object) -> Array[String]:
	var names: Array[String] = []
	for method in suite.get_method_list():
		var method_name: String = method.name
		if method_name.begins_with("test_"):
			names.append(method_name)
	names.sort()
	return names


## dir 底下（含子資料夾，不含 harness）全部 test_*.gd 的路徑。
func _discover(dir: String) -> Array[String]:
	var found: Array[String] = []
	for name in DirAccess.get_files_at(dir):
		if name.begins_with("test_") and name.ends_with(".gd"):
			found.append(dir.path_join(name))
	for name in DirAccess.get_directories_at(dir):
		if name != "harness":
			found.append_array(_discover(dir.path_join(name)))
	return found
