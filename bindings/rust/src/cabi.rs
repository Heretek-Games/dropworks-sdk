//! C ABI for the Dropworks native game SDK (`libdropworks`).
//!
//! Implements the contract declared in `include/dropworks.h`. C, C#, and
//! GDScript bindings link against this shared library; Rust games can keep
//! using the async [`crate::DropworksClient`] directly.
//!
//! The blocking HTTP client is owned by each `dropworks_client`, so the ABI is
//! usable from engines that have no async runtime. A single client is not
//! thread-safe, matching the header contract.
#![cfg(feature = "c-abi")]
#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_uint, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::API_BASE_PATH;

pub const DROPWORKS_OK: c_int = 0;
pub const DROPWORKS_ERR_INVALID_ARGUMENT: c_int = 1;
pub const DROPWORKS_ERR_NETWORK: c_int = 2;
pub const DROPWORKS_ERR_UNAUTHORIZED: c_int = 3;
pub const DROPWORKS_ERR_SERVER: c_int = 4;
pub const DROPWORKS_ERR_NOT_SIGNED_IN: c_int = 5;
pub const DROPWORKS_ERR_INTERNAL: c_int = 6;
pub const DROPWORKS_ERR_NOT_IMPLEMENTED: c_int = 7;

/// Sentinel for an output flag whose value the server did not report.
pub const DROPWORKS_BOOL_UNKNOWN: c_int = -1;

const DEFAULT_TIMEOUT_MS: u64 = 10_000;
const MAX_STRING_BYTES: usize = 64 * 1024;

static NEXT_CLIENT_ID: AtomicU64 = AtomicU64::new(1);

/// C-visible client configuration (mirrors `dropworks_config`).
#[repr(C)]
pub struct dropworks_config {
    pub base_url: *const c_char,
    pub timeout_ms: c_uint,
}

/// Opaque client handle.
pub struct dropworks_client {
    id: u64,
    base_url: String,
    http: reqwest::blocking::Client,
    last_error: Option<CString>,
}

/// Opaque signed-in session handle.
pub struct dropworks_session {
    client_id: u64,
    app_id: CString,
    user_id: CString,
    auth_token: CString,
}

/// Run an `extern "C"` body, converting a Rust panic into an error code instead
/// of letting unwinding cross the ABI boundary (which is undefined behaviour).
fn guard(fallback: c_int, body: impl FnOnce() -> c_int) -> c_int {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(fallback)
}

/// Length of a required NUL-terminated string, or 0 for a null pointer.
unsafe fn cstr_len(value: *const c_char) -> usize {
    if value.is_null() {
        return 0;
    }
    CStr::from_ptr(value).to_bytes().len()
}

/// Read an explicit `(ptr, len)` string, rejecting an out-of-range length.
unsafe fn read_slice(value: *const c_char, len: usize) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if len > MAX_STRING_BYTES {
        return None;
    }
    let bytes = std::slice::from_raw_parts(value as *const u8, len);
    std::str::from_utf8(bytes).ok().map(ToString::to_string)
}

unsafe fn read_cstr(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let bytes = CStr::from_ptr(value).to_bytes();
    if bytes.len() > MAX_STRING_BYTES {
        return None;
    }
    std::str::from_utf8(bytes).ok().map(ToString::to_string)
}

fn session_matches(client: &dropworks_client, session: &dropworks_session) -> bool {
    client.id == session.client_id
}

fn set_error(client: &mut dropworks_client, message: impl Into<String>) {
    let message = message.into();
    client.last_error = CString::new(message).ok();
}

fn trim_trailing_slash(mut url: String) -> String {
    while url.ends_with('/') {
        url.pop();
    }
    url
}

fn status_for_http(status: u16) -> c_int {
    match status {
        401 | 403 => DROPWORKS_ERR_UNAUTHORIZED,
        _ => DROPWORKS_ERR_SERVER,
    }
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_client_create(
    config: *const dropworks_config,
    out_client: *mut *mut dropworks_client,
) -> c_int {
    guard(DROPWORKS_ERR_INTERNAL, || {
        if config.is_null() || out_client.is_null() {
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }
        let Some(base_url) = read_cstr((*config).base_url) else {
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        };
        if base_url.trim().is_empty() {
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }
        let timeout_ms = if (*config).timeout_ms == 0 {
            DEFAULT_TIMEOUT_MS
        } else {
            u64::from((*config).timeout_ms)
        };
        let http = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
        {
            Ok(client) => client,
            Err(_) => return DROPWORKS_ERR_INTERNAL,
        };
        let client = Box::new(dropworks_client {
            id: NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed),
            base_url: trim_trailing_slash(base_url),
            http,
            last_error: None,
        });
        *out_client = Box::into_raw(client);
        DROPWORKS_OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_client_destroy(client: *mut dropworks_client) {
    if client.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the pointer came from `Box::into_raw` in `_create`.
        drop(Box::from_raw(client));
    }));
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_sign_in(
    client: *mut dropworks_client,
    app_id: *const c_char,
    auth_token: *const c_char,
    out_session: *mut *mut dropworks_session,
) -> c_int {
    dropworks_sign_in_n(
        client,
        app_id,
        cstr_len(app_id),
        auth_token,
        cstr_len(auth_token),
        out_session,
    )
}

/// Length-prefixed variant of [`dropworks_sign_in`] for callers that carry
/// strings without a NUL terminator.
#[no_mangle]
pub unsafe extern "C" fn dropworks_sign_in_n(
    client: *mut dropworks_client,
    app_id: *const c_char,
    app_id_len: usize,
    auth_token: *const c_char,
    auth_token_len: usize,
    out_session: *mut *mut dropworks_session,
) -> c_int {
    guard(DROPWORKS_ERR_INTERNAL, || {
        if client.is_null() || out_session.is_null() {
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }
        let client = &mut *client;
        let (Some(app_id), Some(auth_token)) = (
            read_slice(app_id, app_id_len),
            read_slice(auth_token, auth_token_len),
        ) else {
            set_error(client, "app id and auth token are required");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        };
        if app_id.is_empty() || auth_token.is_empty() {
            set_error(client, "app id and auth token are required");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }

        let body = serde_json::json!({ "appId": app_id, "authToken": auth_token });
        let url = format!("{}{API_BASE_PATH}/session", client.base_url);
        let response = match client.http.post(&url).json(&body).send() {
            Ok(response) => response,
            Err(error) => {
                set_error(client, error.to_string());
                return DROPWORKS_ERR_NETWORK;
            }
        };

        let status = response.status().as_u16();
        let body = response.text().unwrap_or_default();
        if !(200..300).contains(&status) {
            set_error(client, format!("sign-in failed with HTTP {status}"));
            return status_for_http(status);
        }

        let value: serde_json::Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(error) => {
                set_error(client, format!("invalid sign-in response: {error}"));
                return DROPWORKS_ERR_INTERNAL;
            }
        };
        let Some(user_id) = value.get("userId").and_then(|value| value.as_str()) else {
            set_error(client, "sign-in response was missing userId");
            return DROPWORKS_ERR_INTERNAL;
        };

        let (Ok(app_id), Ok(user_id), Ok(auth_token)) = (
            CString::new(app_id),
            CString::new(user_id),
            CString::new(auth_token),
        ) else {
            set_error(client, "credentials contained an interior NUL byte");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        };

        let session = Box::new(dropworks_session {
            client_id: client.id,
            app_id,
            user_id,
            auth_token,
        });
        *out_session = Box::into_raw(session);
        DROPWORKS_OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_session_destroy(session: *mut dropworks_session) {
    if session.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: the pointer came from `Box::into_raw`.
        drop(Box::from_raw(session));
    }));
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_session_app_id(
    session: *const dropworks_session,
) -> *const c_char {
    if session.is_null() {
        return ptr::null();
    }
    catch_unwind(AssertUnwindSafe(|| (*session).app_id.as_ptr())).unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_session_user_id(
    session: *const dropworks_session,
) -> *const c_char {
    if session.is_null() {
        return ptr::null();
    }
    catch_unwind(AssertUnwindSafe(|| (*session).user_id.as_ptr())).unwrap_or(ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_unlock_achievement(
    client: *mut dropworks_client,
    session: *const dropworks_session,
    achievement_id: *const c_char,
    out_unlocked: *mut c_int,
) -> c_int {
    dropworks_unlock_achievement_n(
        client,
        session,
        achievement_id,
        cstr_len(achievement_id),
        out_unlocked,
    )
}

/// Length-prefixed variant of [`dropworks_unlock_achievement`].
#[no_mangle]
pub unsafe extern "C" fn dropworks_unlock_achievement_n(
    client: *mut dropworks_client,
    session: *const dropworks_session,
    achievement_id: *const c_char,
    achievement_id_len: usize,
    out_unlocked: *mut c_int,
) -> c_int {
    guard(DROPWORKS_ERR_INTERNAL, || {
        if client.is_null() || session.is_null() || out_unlocked.is_null() {
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }
        let client = &mut *client;
        let session = &*session;
        if !session_matches(client, session) {
            set_error(client, "session does not belong to this client");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }
        let Some(achievement_id) = read_slice(achievement_id, achievement_id_len) else {
            set_error(client, "achievement id is required");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        };
        if achievement_id.is_empty() {
            set_error(client, "achievement id is required");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }

        let body = serde_json::json!({
            "appId": session.app_id.to_string_lossy(),
            "userId": session.user_id.to_string_lossy(),
            "achievementId": achievement_id,
        });
        let url = format!("{}{API_BASE_PATH}/achievement", client.base_url);
        let response = match client
            .http
            .post(&url)
            .bearer_auth(session.auth_token.to_string_lossy().as_ref())
            .json(&body)
            .send()
        {
            Ok(response) => response,
            Err(error) => {
                *out_unlocked = 0;
                set_error(client, error.to_string());
                return DROPWORKS_ERR_NETWORK;
            }
        };

        let status = response.status().as_u16();
        if (200..300).contains(&status) {
            *out_unlocked = 1;
            DROPWORKS_OK
        } else {
            *out_unlocked = 0;
            set_error(client, format!("unlock failed with HTTP {status}"));
            status_for_http(status)
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_set_presence(
    client: *mut dropworks_client,
    session: *const dropworks_session,
    game_id: *const c_char,
    status: *const c_char,
) -> c_int {
    dropworks_set_presence_n(
        client,
        session,
        game_id,
        cstr_len(game_id),
        status,
        cstr_len(status),
    )
}

/// Length-prefixed variant of [`dropworks_set_presence`].
#[no_mangle]
pub unsafe extern "C" fn dropworks_set_presence_n(
    client: *mut dropworks_client,
    session: *const dropworks_session,
    game_id: *const c_char,
    game_id_len: usize,
    status: *const c_char,
    status_len: usize,
) -> c_int {
    guard(DROPWORKS_ERR_INTERNAL, || {
        if client.is_null() || session.is_null() {
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }
        let client = &mut *client;
        let session = &*session;
        if !session_matches(client, session) {
            set_error(client, "session does not belong to this client");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }
        let Some(status) = read_slice(status, status_len) else {
            set_error(client, "status is required");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        };
        if status.is_empty() {
            set_error(client, "status is required");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }
        let game_id = if game_id.is_null() {
            None
        } else {
            match read_slice(game_id, game_id_len) {
                Some(game_id) => Some(game_id),
                None => {
                    set_error(client, "game id is invalid or too long");
                    return DROPWORKS_ERR_INVALID_ARGUMENT;
                }
            }
        };

        let mut body = serde_json::json!({
            "appId": session.app_id.to_string_lossy(),
            "userId": session.user_id.to_string_lossy(),
            "status": status,
        });
        if let Some(game_id) = game_id {
            body["gameId"] = serde_json::Value::String(game_id);
        }

        let url = format!("{}{API_BASE_PATH}/presence", client.base_url);
        let response = match client
            .http
            .post(&url)
            .bearer_auth(session.auth_token.to_string_lossy().as_ref())
            .json(&body)
            .send()
        {
            Ok(response) => response,
            Err(error) => {
                set_error(client, error.to_string());
                return DROPWORKS_ERR_NETWORK;
            }
        };

        let status_code = response.status().as_u16();
        if (200..300).contains(&status_code) {
            DROPWORKS_OK
        } else {
            set_error(
                client,
                format!("presence update failed with HTTP {status_code}"),
            );
            status_for_http(status_code)
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_submit_score(
    client: *mut dropworks_client,
    session: *const dropworks_session,
    leaderboard_key: *const c_char,
    score: f64,
    out_improved: *mut c_int,
) -> c_int {
    dropworks_submit_score_n(
        client,
        session,
        leaderboard_key,
        cstr_len(leaderboard_key),
        score,
        out_improved,
    )
}

/// Length-prefixed variant of [`dropworks_submit_score`].
#[no_mangle]
pub unsafe extern "C" fn dropworks_submit_score_n(
    client: *mut dropworks_client,
    session: *const dropworks_session,
    leaderboard_key: *const c_char,
    leaderboard_key_len: usize,
    score: f64,
    out_improved: *mut c_int,
) -> c_int {
    guard(DROPWORKS_ERR_INTERNAL, || {
        if client.is_null() || session.is_null() || out_improved.is_null() {
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }
        let client = &mut *client;
        let session = &*session;
        if !session_matches(client, session) {
            set_error(client, "session does not belong to this client");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }
        let Some(key) = read_slice(leaderboard_key, leaderboard_key_len) else {
            set_error(client, "leaderboard key is required");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        };
        if key.is_empty() || !score.is_finite() {
            set_error(client, "leaderboard key and a finite score are required");
            return DROPWORKS_ERR_INVALID_ARGUMENT;
        }

        let body = serde_json::json!({
            "appId": session.app_id.to_string_lossy(),
            "userId": session.user_id.to_string_lossy(),
            "key": key,
            "score": score,
        });
        let url = format!("{}{API_BASE_PATH}/leaderboard", client.base_url);
        let response = match client
            .http
            .post(&url)
            .bearer_auth(session.auth_token.to_string_lossy().as_ref())
            .json(&body)
            .send()
        {
            Ok(response) => response,
            Err(error) => {
                *out_improved = DROPWORKS_BOOL_UNKNOWN;
                set_error(client, error.to_string());
                return DROPWORKS_ERR_NETWORK;
            }
        };

        let status = response.status().as_u16();
        if (200..300).contains(&status) {
            let body = response.text().unwrap_or_default();
            let improved = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|value| value.get("improved").and_then(|value| value.as_bool()));
            // Do not assume "improved" when the server omits the field.
            *out_improved = match improved {
                Some(true) => 1,
                Some(false) => 0,
                None => DROPWORKS_BOOL_UNKNOWN,
            };
            DROPWORKS_OK
        } else {
            *out_improved = 0;
            set_error(
                client,
                format!("score submission failed with HTTP {status}"),
            );
            status_for_http(status)
        }
    })
}

#[no_mangle]
pub extern "C" fn dropworks_status_string(status: c_int) -> *const c_char {
    let value: &'static [u8] = match status {
        DROPWORKS_OK => b"DROPWORKS_OK\0",
        DROPWORKS_ERR_INVALID_ARGUMENT => b"DROPWORKS_ERR_INVALID_ARGUMENT\0",
        DROPWORKS_ERR_NETWORK => b"DROPWORKS_ERR_NETWORK\0",
        DROPWORKS_ERR_UNAUTHORIZED => b"DROPWORKS_ERR_UNAUTHORIZED\0",
        DROPWORKS_ERR_SERVER => b"DROPWORKS_ERR_SERVER\0",
        DROPWORKS_ERR_NOT_SIGNED_IN => b"DROPWORKS_ERR_NOT_SIGNED_IN\0",
        DROPWORKS_ERR_INTERNAL => b"DROPWORKS_ERR_INTERNAL\0",
        DROPWORKS_ERR_NOT_IMPLEMENTED => b"DROPWORKS_ERR_NOT_IMPLEMENTED\0",
        _ => b"DROPWORKS_ERR_UNKNOWN\0",
    };
    value.as_ptr().cast()
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_last_error(client: *const dropworks_client) -> *const c_char {
    if client.is_null() {
        return ptr::null();
    }
    match &(*client).last_error {
        Some(error) => error.as_ptr(),
        None => ptr::null(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn base_config() -> dropworks_config {
        dropworks_config {
            base_url: b"https://drop.example.com\0".as_ptr().cast(),
            timeout_ms: 0,
        }
    }

    #[test]
    fn status_strings_are_stable() {
        let cases = [
            (DROPWORKS_OK, "DROPWORKS_OK"),
            (
                DROPWORKS_ERR_INVALID_ARGUMENT,
                "DROPWORKS_ERR_INVALID_ARGUMENT",
            ),
            (DROPWORKS_ERR_NETWORK, "DROPWORKS_ERR_NETWORK"),
            (DROPWORKS_ERR_UNAUTHORIZED, "DROPWORKS_ERR_UNAUTHORIZED"),
            (DROPWORKS_ERR_SERVER, "DROPWORKS_ERR_SERVER"),
            (DROPWORKS_ERR_NOT_SIGNED_IN, "DROPWORKS_ERR_NOT_SIGNED_IN"),
            (DROPWORKS_ERR_INTERNAL, "DROPWORKS_ERR_INTERNAL"),
            (
                DROPWORKS_ERR_NOT_IMPLEMENTED,
                "DROPWORKS_ERR_NOT_IMPLEMENTED",
            ),
        ];
        for (code, expected) in cases {
            let rendered = unsafe {
                CStr::from_ptr(dropworks_status_string(code))
                    .to_str()
                    .unwrap()
            };
            assert_eq!(rendered, expected);
        }
    }

    #[test]
    fn create_rejects_null_pointers() {
        unsafe {
            let mut client: *mut dropworks_client = ptr::null_mut();
            assert_eq!(
                dropworks_client_create(ptr::null(), &mut client),
                DROPWORKS_ERR_INVALID_ARGUMENT
            );
            assert_eq!(
                dropworks_client_create(&base_config(), ptr::null_mut()),
                DROPWORKS_ERR_INVALID_ARGUMENT
            );
        }
    }

    #[test]
    fn sign_in_and_unlock_validate_arguments() {
        unsafe {
            let mut client: *mut dropworks_client = ptr::null_mut();
            assert_eq!(
                dropworks_client_create(&base_config(), &mut client),
                DROPWORKS_OK
            );
            let mut session: *mut dropworks_session = ptr::null_mut();
            assert_eq!(
                dropworks_sign_in(
                    client,
                    ptr::null(),
                    b"token\0".as_ptr().cast(),
                    &mut session,
                ),
                DROPWORKS_ERR_INVALID_ARGUMENT
            );
            let mut unlocked: c_int = 9;
            assert_eq!(
                dropworks_unlock_achievement(client, ptr::null(), ptr::null(), &mut unlocked),
                DROPWORKS_ERR_INVALID_ARGUMENT
            );
            let mut improved: c_int = 9;
            assert_eq!(
                dropworks_submit_score(
                    client,
                    ptr::null(),
                    b"high\0".as_ptr().cast(),
                    1.0,
                    &mut improved
                ),
                DROPWORKS_ERR_INVALID_ARGUMENT
            );
            assert_eq!(
                dropworks_set_presence(client, ptr::null(), ptr::null(), ptr::null()),
                DROPWORKS_ERR_INVALID_ARGUMENT
            );
            assert!(!dropworks_last_error(client).is_null());
            dropworks_client_destroy(client);
        }
    }

    #[test]
    fn session_getters_handle_null() {
        unsafe {
            assert!(dropworks_session_app_id(ptr::null()).is_null());
            assert!(dropworks_session_user_id(ptr::null()).is_null());
            dropworks_session_destroy(ptr::null_mut());
            dropworks_client_destroy(ptr::null_mut());
        }
    }

    #[test]
    fn guard_converts_panics_into_error_codes() {
        let code = guard(DROPWORKS_ERR_INTERNAL, || panic!("boom"));
        assert_eq!(code, DROPWORKS_ERR_INTERNAL);
        assert_eq!(guard(DROPWORKS_ERR_INTERNAL, || DROPWORKS_OK), DROPWORKS_OK);
    }

    #[test]
    fn read_slice_validates_length_and_utf8() {
        unsafe {
            assert_eq!(
                read_slice(b"hello\0".as_ptr().cast(), 5).as_deref(),
                Some("hello")
            );
            assert_eq!(read_slice(b"x".as_ptr().cast(), 1).as_deref(), Some("x"));
            assert_eq!(read_slice(ptr::null(), 0), None);
            // Over-cap lengths are rejected before dereferencing.
            assert_eq!(read_slice(b"x".as_ptr().cast(), MAX_STRING_BYTES + 1), None);
            // Invalid UTF-8 is rejected.
            assert_eq!(read_slice([0xffu8, 0xfe].as_ptr().cast(), 2), None);
        }
    }

    #[test]
    fn operations_reject_a_session_from_another_client() {
        unsafe {
            let mut client: *mut dropworks_client = ptr::null_mut();
            assert_eq!(
                dropworks_client_create(&base_config(), &mut client),
                DROPWORKS_OK
            );

            let foreign = Box::new(dropworks_session {
                client_id: 999_999,
                app_id: CString::new("app").unwrap(),
                user_id: CString::new("user").unwrap(),
                auth_token: CString::new("token").unwrap(),
            });
            let session_ptr = Box::into_raw(foreign);

            let mut unlocked: c_int = 9;
            assert_eq!(
                dropworks_unlock_achievement(
                    client,
                    session_ptr,
                    b"ach\0".as_ptr().cast(),
                    &mut unlocked,
                ),
                DROPWORKS_ERR_INVALID_ARGUMENT
            );
            assert_eq!(unlocked, 9, "out param must be untouched on rejection");
            assert!(!dropworks_last_error(client).is_null());

            dropworks_session_destroy(session_ptr);
            dropworks_client_destroy(client);
        }
    }

    #[test]
    fn length_prefixed_sign_in_validates_arguments() {
        unsafe {
            let mut client: *mut dropworks_client = ptr::null_mut();
            assert_eq!(
                dropworks_client_create(&base_config(), &mut client),
                DROPWORKS_OK
            );
            let mut session: *mut dropworks_session = ptr::null_mut();
            assert_eq!(
                dropworks_sign_in_n(
                    client,
                    b"app\0".as_ptr().cast(),
                    3,
                    ptr::null(),
                    0,
                    &mut session,
                ),
                DROPWORKS_ERR_INVALID_ARGUMENT
            );
            dropworks_client_destroy(client);
        }
    }

    /// Reads one complete HTTP request (headers plus declared body) from a
    /// connection so mock-server tests can assert on the JSON body.
    fn read_full_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                return request;
            }
            request.extend_from_slice(&buffer[..read]);
            let Some(head_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };
            let head = String::from_utf8_lossy(&request[..head_end]);
            let length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.trim()
                        .eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if request.len() >= head_end + 4 + length {
                return request;
            }
        }
    }

    /// Serves exactly one request with a fixed JSON body. Returns the base URL
    /// and a handle yielding the raw request bytes.
    fn serve_json(body: &'static str) -> (String, std::thread::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_full_request(&mut stream);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
            request
        });
        (format!("http://{address}"), handle)
    }

    /// Builds a session bound to `client` without going through the network.
    unsafe fn session_for(client: *mut dropworks_client) -> *mut dropworks_session {
        Box::into_raw(Box::new(dropworks_session {
            client_id: (*client).id,
            app_id: CString::new("app").unwrap(),
            user_id: CString::new("user").unwrap(),
            auth_token: CString::new("token").unwrap(),
        }))
    }

    #[test]
    fn submit_score_maps_improved_tri_state_through_the_c_abi() {
        let cases: [(&str, c_int); 3] = [
            (r#"{"improved":true}"#, 1),
            (r#"{"improved":false}"#, 0),
            ("{}", DROPWORKS_BOOL_UNKNOWN),
        ];
        for (payload, expected) in cases {
            let (base_url, server) = serve_json(payload);
            let base_url = CString::new(base_url).unwrap();
            let config = dropworks_config {
                base_url: base_url.as_ptr(),
                timeout_ms: 1_000,
            };
            unsafe {
                let mut client: *mut dropworks_client = ptr::null_mut();
                assert_eq!(dropworks_client_create(&config, &mut client), DROPWORKS_OK);
                let session = session_for(client);
                let mut improved: c_int = 9;
                assert_eq!(
                    dropworks_submit_score_n(
                        client,
                        session,
                        b"high-score\0".as_ptr().cast(),
                        10,
                        1.0,
                        &mut improved,
                    ),
                    DROPWORKS_OK,
                    "payload {payload}"
                );
                assert_eq!(improved, expected, "payload {payload}");
                dropworks_session_destroy(session);
                dropworks_client_destroy(client);
            }
            server.join().unwrap();
        }
    }

    #[test]
    fn set_presence_rejects_unreadable_game_ids() {
        unsafe {
            let mut client: *mut dropworks_client = ptr::null_mut();
            assert_eq!(
                dropworks_client_create(&base_config(), &mut client),
                DROPWORKS_OK
            );
            let session = session_for(client);
            let status = b"in-game\0".as_ptr().cast::<c_char>();

            // A non-NULL game id whose declared length exceeds the 64 KiB cap is
            // rejected before the pointer is dereferenced.
            assert_eq!(
                dropworks_set_presence_n(
                    client,
                    session,
                    b"game\0".as_ptr().cast(),
                    MAX_STRING_BYTES + 1,
                    status,
                    7,
                ),
                DROPWORKS_ERR_INVALID_ARGUMENT
            );
            // Invalid UTF-8 is rejected the same way instead of being dropped.
            assert_eq!(
                dropworks_set_presence_n(
                    client,
                    session,
                    [0xffu8, 0xfe].as_ptr().cast(),
                    2,
                    status,
                    7,
                ),
                DROPWORKS_ERR_INVALID_ARGUMENT
            );
            assert!(!dropworks_last_error(client).is_null());

            dropworks_session_destroy(session);
            dropworks_client_destroy(client);
        }
    }

    #[test]
    fn set_presence_keeps_a_null_game_id_optional() {
        let (base_url, server) = serve_json("{}");
        let base_url = CString::new(base_url).unwrap();
        let config = dropworks_config {
            base_url: base_url.as_ptr(),
            timeout_ms: 1_000,
        };
        unsafe {
            let mut client: *mut dropworks_client = ptr::null_mut();
            assert_eq!(dropworks_client_create(&config, &mut client), DROPWORKS_OK);
            let session = session_for(client);
            assert_eq!(
                dropworks_set_presence_n(
                    client,
                    session,
                    ptr::null(),
                    0,
                    b"in-game\0".as_ptr().cast(),
                    7,
                ),
                DROPWORKS_OK
            );
            let request = String::from_utf8(server.join().unwrap()).unwrap();
            assert!(request.contains("\"status\":\"in-game\""));
            assert!(!request.contains("gameId"));
            dropworks_session_destroy(session);
            dropworks_client_destroy(client);
        }
    }
}
