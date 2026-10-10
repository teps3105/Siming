extends Logger
## 案例執行期間收下引擎與腳本的錯誤（警告不收），交給執行程式判決。

var _mutex := Mutex.new()
var _capturing := false
var _errors: Array[String] = []


func begin() -> void:
	_mutex.lock()
	_capturing = true
	_errors.clear()
	_mutex.unlock()


func end() -> Array[String]:
	_mutex.lock()
	var captured := _errors.duplicate()
	_capturing = false
	_errors.clear()
	_mutex.unlock()
	return captured


func _log_error(function: String, file: String, line: int, code: String, rationale: String, _editor_notify: bool,
		error_type: int, _script_backtraces: Array) -> void:
	if error_type == ERROR_TYPE_WARNING:
		return
	_mutex.lock()
	if _capturing:
		var text := rationale if not rationale.is_empty() else code
		_errors.append("%s（%s:%d %s）" % [text, file, line, function])
	_mutex.unlock()
