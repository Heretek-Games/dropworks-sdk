extends SceneTree

const DropworksClientScript := preload("res://dropworks_client.gd")

var failures := 0


func _initialize() -> void:
	_check(
		DropworksClientScript.build_session_body("app", "tok") == {"appId": "app", "authToken": "tok"},
		"session body"
	)
	_check(
		DropworksClientScript.build_achievement_body("app", "user", "ach")
			== {"appId": "app", "userId": "user", "achievementId": "ach"},
		"achievement body"
	)
	_check(
		DropworksClientScript.build_score_body("app", "user", "high", 42.0)
			== {"appId": "app", "userId": "user", "key": "high", "score": 42.0},
		"score body"
	)
	_check(
		DropworksClientScript.build_presence_body("app", "user", "in-game", "")
			== {"appId": "app", "userId": "user", "status": "in-game"},
		"presence body omits an empty game id"
	)
	_check(
		DropworksClientScript.build_presence_body("app", "user", "in-game", "game-1")
			== {"appId": "app", "userId": "user", "status": "in-game", "gameId": "game-1"},
		"presence body includes the game id"
	)

	var https := DropworksClientScript.parse_endpoint("https://drop.example.com/api/v1/dropworks/session")
	_check(
		https["tls"]
			and https["host"] == "drop.example.com"
			and https["port"] == 443
			and https["path"] == "/api/v1/dropworks/session",
		"https endpoint"
	)
	var http := DropworksClientScript.parse_endpoint("http://localhost:8080/api/v1/dropworks/session")
	_check(
		not http["tls"]
			and http["host"] == "localhost"
			and http["port"] == 8080
			and http["path"] == "/api/v1/dropworks/session",
		"http endpoint"
	)

	var client := DropworksClientScript.new("https://drop.example.com/")
	_check(client.base_url == "https://drop.example.com", "base url trimmed")
	_check(not client.is_signed_in(), "not signed in initially")
	_check(not client.unlock_achievement("ach"), "unlock requires sign-in")
	_check(not client.submit_score("high", 1.0), "score requires sign-in")
	_check(not client.set_presence("in-game"), "presence requires sign-in")

	if failures == 0:
		print("dropworks GDScript binding test passed")
		quit(0)
	else:
		printerr("%d check(s) failed" % failures)
		quit(1)


func _check(condition: bool, what: String) -> void:
	if not condition:
		printerr("FAIL: " + what)
		failures += 1
