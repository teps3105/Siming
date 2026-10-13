extends SceneTree
## 量測：一批棋子同時演出時的重畫次數、單次重畫耗時與每幀耗時。
## 用法：godot --path game --position 2600,40 --audio-driver Dummy -s res://tests/harness/figure_perf.gd -- [--count=8] [--frames=240]

var _count := 8
var _frames := 240
var _n := 0
var _actors: Array[FigureActor] = []
var _cam: Camera3D
var _start := 0


func _initialize() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--count="):
			_count = int(arg.trim_prefix("--count="))
		elif arg.begins_with("--frames="):
			_frames = int(arg.trim_prefix("--frames="))
	_cam = Camera3D.new()
	_cam.projection = Camera3D.PROJECTION_ORTHOGONAL
	_cam.size = 6.0
	root.add_child(_cam)
	_cam.global_transform = Transform3D(Basis.looking_at(Vector3(0, -0.87, -0.5).normalized(), Vector3.UP), Vector3(0, 8, 5))
	for i in _count:
		var a := FigureActor.new()
		root.add_child(a)
		a.setup(FigureActor.look_of(i + 3))
		_actors.append(a)


func _process(_delta: float) -> bool:
	_n += 1
	for i in _actors.size():
		var a := _actors[i]
		a.play(["stand", "brace", "struck"][i % 3], _n / 60.0)
		a.place(Vector3((i % 4) * 1.2 - 2.0, 0, (i / 4) * 1.2 - 1.0), Vector3.UP, _cam, 1.0)
	if _n == 30:
		_start = Time.get_ticks_usec()
		FigureActor.redraws = 0
		FigureActor.redraw_usec = 0
	if _n == 30 + _frames:
		var total := Time.get_ticks_usec() - _start
		print("棋子數 %d；%d 幀共 %.1f ms（每幀 %.2f ms）；重畫 %d 次、平均 %.0f us/次、佔每幀 %.2f ms" % [
			_count, _frames, total / 1000.0, total / 1000.0 / _frames, FigureActor.redraws,
			float(FigureActor.redraw_usec) / maxf(1.0, FigureActor.redraws), FigureActor.redraw_usec / 1000.0 / _frames])
		quit()
	return false
