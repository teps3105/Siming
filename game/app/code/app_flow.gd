class_name AppFlow
extends RefCounted
## 場景之間的切換（GDD 9.1）：標題畫面 ↔ 遊戲場景。世界由選單端備妥後交給遊戲場景。

const TITLE := "res://app/scenes/title.tscn"
const GAME := "res://app/scenes/bootstrap.tscn"


## 帶著備妥的世界進遊戲場景，取代 from 所在的場景。loaded 為真（讀檔）時不放開場運鏡、先暫停。
static func enter_game(from: Node, world: SimWorld, loaded: bool) -> void:
	var game: Node = (load(GAME) as PackedScene).instantiate()
	game.start_world = world
	game.opening_flight = not loaded
	game.start_paused = loaded
	_replace(from, game)


static func enter_title(from: Node) -> void:
	_replace(from, (load(TITLE) as PackedScene).instantiate())


## F12 把目前的畫面存成 PNG（user://screenshots/）。
static func screenshot_on_f12(event: InputEvent, viewport: Viewport) -> void:
	var key := event as InputEventKey
	if key == null or not key.pressed or key.echo or key.keycode != KEY_F12:
		return
	DirAccess.make_dir_recursive_absolute("user://screenshots")
	var stamp := Time.get_datetime_string_from_system().replace(":", "").replace("-", "")
	viewport.get_texture().get_image().save_png("user://screenshots/%s.png" % stamp)


static func _replace(from: Node, next: Node) -> void:
	var tree := from.get_tree()
	var parent := from.get_parent()
	var was_current := tree.current_scene == from
	parent.remove_child(from)
	from.queue_free()
	parent.add_child(next)
	if was_current:
		tree.current_scene = next
