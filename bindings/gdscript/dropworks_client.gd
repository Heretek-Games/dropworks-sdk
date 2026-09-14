## DropworksClient — Godot 4 GDScript binding for the Dropworks REST contract.
##
## Native Godot games add this as an autoload and call the methods after
## `sign_in`. Requests are synchronous (`HTTPClient`), which is appropriate for
## session setup and discrete events (achievements, scores, presence) rather than
## per-frame calls.
##
## The static builders are pure and unit-tested headlessly; the network methods
## mirror the TypeScript and Rust clients.
class_name DropworksClient
extends RefCounted

const API_BASE := "/api/v1/dropworks"

var base_url: String
var app_id: String = ""
var user_id: String = ""
var auth_token: String = ""


func _init(server_base_url: String) -> void:
	base_url = server_base_url.trim_suffix("/")


func is_signed_in() -> bool:
	return user_id != ""


static func build_session_body(p_app_id: String, p_auth_token: String) -> Dictionary:
	return {"appId": p_app_id, "authToken": p_auth_token}


static func build_achievement_body(
	p_app_id: String, p_user_id: String, achievement_id: String
) -> Dictionary:
	return {
		"appId": p_app_id,
		"userId": p_user_id,
		"achievementId": achievement_id,
	}


static func build_score_body(
	p_app_id: String, p_user_id: String, key: String, score: float
) -> Dictionary:
	return {"appId": p_app_id, "userId": p_user_id, "key": key, "score": score}


static func build_presence_body(
	p_app_id: String, p_user_id: String, status: String, game_id: String
) -> Dictionary:
	var body := {
		"appId": p_app_id,
		"userId": p_user_id,
		"status": status,
	}
	if game_id != "":
		body["gameId"] = game_id
	return body


## Splits an absolute URL into the host/port/path an `HTTPClient` needs.
static func parse_endpoint(url: String) -> Dictionary:
	var use_tls := url.begins_with("https://")
	var stripped := url.trim_prefix("https://") if use_tls else url.trim_prefix("http://")
	var slash := stripped.find("/")
	var authority := stripped.substr(0, slash) if slash != -1 else stripped
	var request_path := stripped.substr(slash) if slash != -1 else "/"
	var port := 443 if use_tls else 80
	var colon := authority.rfind(":")
	var host := authority
	if colon != -1:
		host = authority.substr(0, colon)
		port = int(authority.substr(colon + 1))
	return {"tls": use_tls, "host": host, "port": port, "path": request_path}


## Signs in and stores the session. Returns a result dictionary.
func sign_in(p_app_id: String, p_auth_token: String) -> Dictionary:
	var response := _post("/session", build_session_body(p_app_id, p_auth_token), "")
	if not response["ok"]:
		return {"ok": false, "error": response["error"]}
	var payload: Variant = response["json"]
	var uid := ""
	if payload is Dictionary:
		uid = str(payload.get("userId", ""))
	if uid == "":
		return {"ok": false, "error": "sign-in response missing userId"}
	app_id = p_app_id
	auth_token = p_auth_token
	user_id = uid
	return {"ok": true, "userId": uid}


func unlock_achievement(achievement_id: String) -> bool:
	if not is_signed_in():
		return false
	var response := _post(
		"/achievement",
		build_achievement_body(app_id, user_id, achievement_id),
		auth_token,
	)
	return response["ok"]


func submit_score(key: String, score: float) -> bool:
	if not is_signed_in():
		return false
	var response := _post(
		"/leaderboard",
		build_score_body(app_id, user_id, key, score),
		auth_token,
	)
	return response["ok"]


func set_presence(status: String, game_id: String = "") -> bool:
	if not is_signed_in():
		return false
	var response := _post(
		"/presence",
		build_presence_body(app_id, user_id, status, game_id),
		auth_token,
	)
	return response["ok"]


func _post(path: String, body: Dictionary, token: String) -> Dictionary:
	var endpoint := parse_endpoint(base_url + API_BASE + path)
	var http := HTTPClient.new()
	var tls: TLSOptions = TLSOptions.client() if endpoint["tls"] else null
	var err := http.connect_to_host(endpoint["host"], endpoint["port"], tls)
	if err != OK:
		return _failure("connect failed (%d)" % err)

	while http.get_status() in [HTTPClient.STATUS_CONNECTING, HTTPClient.STATUS_RESOLVING]:
		http.poll()
		OS.delay_msec(10)

	var headers := PackedStringArray(["Content-Type: application/json"])
	if token != "":
		headers.append("Authorization: Bearer " + token)

	err = http.request(HTTPClient.METHOD_POST, endpoint["path"], headers, JSON.stringify(body))
	if err != OK:
		return _failure("request failed (%d)" % err)

	while http.get_status() == HTTPClient.STATUS_REQUESTING:
		http.poll()
		OS.delay_msec(10)

	var chunks := PackedByteArray()
	while http.get_status() == HTTPClient.STATUS_BODY:
		http.poll()
		var chunk := http.read_response_body_chunk()
		if chunk.size() == 0:
			OS.delay_msec(10)
		else:
			chunks.append_array(chunk)

	var status := http.get_response_code()
	var text := chunks.get_string_from_utf8()
	var parsed: Variant = JSON.parse_string(text) if text != "" else null
	if status >= 200 and status < 300:
		return {"ok": true, "status": status, "json": parsed, "error": ""}
	return {"ok": false, "status": status, "json": parsed, "error": "HTTP %d" % status}


func _failure(message: String) -> Dictionary:
	return {"ok": false, "status": 0, "json": null, "error": message}
