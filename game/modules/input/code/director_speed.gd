class_name DirectorSpeed
extends RefCounted
## GDD 5.13 時間：導播對準大事時把遊戲速度降到細步最慢一檔（每秒 1 日），離開時恢復降速前的速度；
## 期間玩家自己改了速度或按了暫停，就以玩家的為準，不再恢復。

var enabled := true

var _sim: SimHost
var _engaged := false
var _saved := 0
var _touched := false
var _own := false


func _init(sim: SimHost) -> void:
	_sim = sim
	sim.time_state_changed.connect(func() -> void:
		if _engaged and not _own:
			_touched = true)


func engaged() -> bool:
	return _engaged


func engage() -> void:
	if _engaged or not enabled or _sim.paused:
		return
	_engaged = true
	_touched = false
	_saved = _sim.speed_index
	if _saved != 0:
		_own = true
		_sim.set_speed(0)
		_own = false


func release() -> void:
	if not _engaged:
		return
	_engaged = false
	if _saved != 0 and not _touched and not _sim.paused:
		_own = true
		_sim.set_speed(_saved)
		_own = false
