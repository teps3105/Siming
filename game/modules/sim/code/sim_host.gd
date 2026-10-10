class_name SimHost
extends Node
## GDD 6.4 模擬驅動：Godot 端唯一持有 SimWorld 的節點。
## 其他模組經由本節點讀取世界與推進時間，不直接建立 SimWorld。

signal world_created
signal advanced(months: int)

var world: SimWorld = SimWorld.new()


## GDD 4.6 開天。size 為 "small"、"medium"、"large"。失敗時回傳原因，成功回傳空字串。
func create_world(world_seed: int, size: String = "medium") -> String:
	var error := world.create(world_seed, size)
	if error.is_empty():
		world_created.emit()
	return error


## GDD 6.3 推進 months 個月。
func advance(months: int) -> void:
	if not world.is_ready():
		return
	world.step(months)
	advanced.emit(months)
