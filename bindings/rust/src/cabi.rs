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
use std::ptr;
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

const DEFAULT_TIMEOUT_MS: u64 = 10_000;

/// C-visible client configuration (mirrors `dropworks_config`).
#[repr(C)]
pub struct dropworks_config {
    pub base_url: *const c_char,
    pub timeout_ms: c_uint,
}

/// Opaque client handle.
pub struct dropworks_client {
    base_url: String,
    http: reqwest::blocking::Client,
    last_error: Option<CString>,
}

/// Opaque signed-in session handle.
pub struct dropworks_session {
    app_id: CString,
    user_id: CString,
    auth_token: CString,
}

unsafe fn read_cstr(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // SAFETY: the caller contract requires a valid NUL-terminated string.
    CStr::from_ptr(value).to_str().ok().map(ToString::to_string)
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
        base_url: trim_trailing_slash(base_url),
        http,
        last_error: None,
    });
    *out_client = Box::into_raw(client);
    DROPWORKS_OK
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_client_destroy(client: *mut dropworks_client) {
    if !client.is_null() {
        // SAFETY: the pointer came from `Box::into_raw` in `_create`.
        drop(Box::from_raw(client));
    }
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_sign_in(
    client: *mut dropworks_client,
    app_id: *const c_char,
    auth_token: *const c_char,
    out_session: *mut *mut dropworks_session,
) -> c_int {
    if client.is_null() || out_session.is_null() {
        return DROPWORKS_ERR_INVALID_ARGUMENT;
    }
    let client = &mut *client;
    let (Some(app_id), Some(auth_token)) = (read_cstr(app_id), read_cstr(auth_token)) else {
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
        app_id,
        user_id,
        auth_token,
    });
    *out_session = Box::into_raw(session);
    DROPWORKS_OK
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_session_destroy(session: *mut dropworks_session) {
    if !session.is_null() {
        // SAFETY: the pointer came from `Box::into_raw`.
        drop(Box::from_raw(session));
    }
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_session_app_id(
    session: *const dropworks_session,
) -> *const c_char {
    if session.is_null() {
        return ptr::null();
    }
    (*session).app_id.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_session_user_id(
    session: *const dropworks_session,
) -> *const c_char {
    if session.is_null() {
        return ptr::null();
    }
    (*session).user_id.as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_unlock_achievement(
    client: *mut dropworks_client,
    session: *const dropworks_session,
    achievement_id: *const c_char,
    out_unlocked: *mut c_int,
) -> c_int {
    if client.is_null() || session.is_null() || out_unlocked.is_null() {
        return DROPWORKS_ERR_INVALID_ARGUMENT;
    }
    let client = &mut *client;
    let Some(achievement_id) = read_cstr(achievement_id) else {
        set_error(client, "achievement id is required");
        return DROPWORKS_ERR_INVALID_ARGUMENT;
    };
    if achievement_id.is_empty() {
        set_error(client, "achievement id is required");
        return DROPWORKS_ERR_INVALID_ARGUMENT;
    }
    let session = &*session;

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
}

#[no_mangle]
pub unsafe extern "C" fn dropworks_set_presence(
    _client: *mut dropworks_client,
    _session: *const dropworks_session,
    _game_id: *const c_char,
    _status: *const c_char,
) -> c_int {
    DROPWORKS_ERR_NOT_IMPLEMENTED
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
            assert_eq!(
                dropworks_set_presence(client, ptr::null(), ptr::null(), ptr::null()),
                DROPWORKS_ERR_NOT_IMPLEMENTED
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
}
