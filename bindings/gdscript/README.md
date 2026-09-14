# Dropworks GDScript binding

Godot 4 binding for the Dropworks REST contract, implemented with the engine's
built-in `HTTPClient` (no GDExtension or native library required).

## Layout

- `dropworks_client.gd` — `DropworksClient` (`class_name`), with pure static
  builders (`build_session_body`, `build_achievement_body`, `build_score_body`,
  `build_presence_body`, `parse_endpoint`) and synchronous network methods
  (`sign_in`, `unlock_achievement`, `submit_score`, `set_presence`).
- `test_dropworks.gd` — a headless `SceneTree` test exercising the pure logic.
- `project.godot` — minimal project used only to run the test.

## Usage

```gdscript
var client := DropworksClient.new("https://drop.example.com")
var result := client.sign_in("app-id", "auth-token")
if result["ok"]:
    client.unlock_achievement("beat-the-boss")
    client.submit_score("high-score", 9001.0)
    client.set_presence("in-game", "game-id")
```

## Test

```sh
godot --headless --path bindings/gdscript --script test_dropworks.gd
# -> dropworks GDScript binding test passed
```

CI downloads Godot 4.3 and runs the same command.
