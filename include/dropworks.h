/*
 * dropworks.h — C ABI for the Dropworks native game SDK.
 *
 * Dropworks is the Steamworks-replacement surface exposed by a Drop server:
 *   POST /api/v1/dropworks/session      (sign in)
 *   POST /api/v1/dropworks/achievement  (unlock an achievement)
 *   POST /api/v1/dropworks/leaderboard  (submit a leaderboard score)
 *   POST /api/v1/dropworks/presence     (set rich presence)
 *
 * This header is the shared ABI target for the C, C#, GDScript, and Rust
 * bindings in `bindings/`. The reference implementation (`libdropworks`) is
 * built from the Rust crate in `bindings/rust` with the `c-abi` feature:
 *
 *     cargo build --release --manifest-path bindings/rust/Cargo.toml --features c-abi
 *
 * which produces `libdropworks.{so,dylib,dll}`. The header is the stable
 * contract every binding links against.
 *
 * Memory ownership:
 *   - `dropworks_client*` and `dropworks_session*` are owned by the caller and
 *     released with the matching `_destroy` function.
 *   - String getters return borrowed, NUL-terminated pointers owned by the
 *     session/client. They stay valid until the owning object is destroyed.
 *   - `dropworks_last_error` returns a borrowed pointer valid until the next
 *     call on that client.
 *
 * Threading: a single client is not thread-safe; use one client per thread or
 * add your own synchronization.
 */

#ifndef DROPWORKS_H
#define DROPWORKS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define DROPWORKS_VERSION_MAJOR 0
#define DROPWORKS_VERSION_MINOR 1
#define DROPWORKS_VERSION_PATCH 0
#define DROPWORKS_API_VERSION 1

#if defined(_WIN32) || defined(__CYGWIN__)
#  if defined(DROPWORKS_BUILD_SHARED)
#    define DROPWORKS_API __declspec(dllexport)
#  elif defined(DROPWORKS_USE_SHARED)
#    define DROPWORKS_API __declspec(dllimport)
#  else
#    define DROPWORKS_API
#  endif
#else
#  if defined(DROPWORKS_BUILD_SHARED) || defined(DROPWORKS_USE_SHARED)
#    define DROPWORKS_API __attribute__((visibility("default")))
#  else
#    define DROPWORKS_API
#  endif
#endif

#ifdef __cplusplus
#  define DROPWORKS_NULL nullptr
#else
#  define DROPWORKS_NULL NULL
#endif

/** Opaque client handle. */
typedef struct dropworks_client dropworks_client;

/** Opaque signed-in session handle. */
typedef struct dropworks_session dropworks_session;

typedef enum dropworks_status {
  DROPWORKS_OK = 0,
  DROPWORKS_ERR_INVALID_ARGUMENT = 1,
  DROPWORKS_ERR_NETWORK = 2,
  DROPWORKS_ERR_UNAUTHORIZED = 3,
  DROPWORKS_ERR_SERVER = 4,
  DROPWORKS_ERR_NOT_SIGNED_IN = 5,
  DROPWORKS_ERR_INTERNAL = 6,
  DROPWORKS_ERR_NOT_IMPLEMENTED = 7
} dropworks_status;

/**
 * Sentinel written to an output flag when the server did not report a value
 * (for example an \`improved\` field omitted from the leaderboard response).
 */
#define DROPWORKS_BOOL_UNKNOWN (-1)

typedef struct dropworks_config {
  /** Server base URL, e.g. "https://drop.example.com". Required. */
  const char* base_url;
  /** Per-request timeout in milliseconds. 0 selects the default. */
  uint32_t timeout_ms;
} dropworks_config;

/** Creates a client. The base URL is copied; `out_client` must not be NULL. */
DROPWORKS_API dropworks_status dropworks_client_create(
    const dropworks_config* config,
    dropworks_client** out_client);

/** Destroys a client. Safe to call with DROPWORKS_NULL. */
DROPWORKS_API void dropworks_client_destroy(dropworks_client* client);

/**
 * Signs in with an app id and Drop auth token (POST /api/v1/dropworks/session).
 * On success the caller owns `*out_session` until `dropworks_session_destroy`.
 */
DROPWORKS_API dropworks_status dropworks_sign_in(
    dropworks_client* client,
    const char* app_id,
    const char* auth_token,
    dropworks_session** out_session);

/**
 * Length-prefixed variant of dropworks_sign_in for callers whose strings are
 * not NUL-terminated. `app_id_len`/`auth_token_len` are byte lengths.
 */
DROPWORKS_API dropworks_status dropworks_sign_in_n(
    dropworks_client* client,
    const char* app_id,
    size_t app_id_len,
    const char* auth_token,
    size_t auth_token_len,
    dropworks_session** out_session);

/** Destroys a session. Safe to call with DROPWORKS_NULL. */
DROPWORKS_API void dropworks_session_destroy(dropworks_session* session);

/** Borrowed app id. Returns DROPWORKS_NULL for a NULL session. */
DROPWORKS_API const char* dropworks_session_app_id(
    const dropworks_session* session);

/** Borrowed user id. Returns DROPWORKS_NULL for a NULL session. */
DROPWORKS_API const char* dropworks_session_user_id(
    const dropworks_session* session);

/**
 * Unlocks an achievement (POST /api/v1/dropworks/achievement).
 * `out_unlocked` is set to 1 when the server accepted the unlock, 0 when it
 * rejected it; it must not be NULL.
 */
DROPWORKS_API dropworks_status dropworks_unlock_achievement(
    dropworks_client* client,
    const dropworks_session* session,
    const char* achievement_id,
    int* out_unlocked);

/** Length-prefixed variant of dropworks_unlock_achievement. */
DROPWORKS_API dropworks_status dropworks_unlock_achievement_n(
    dropworks_client* client,
    const dropworks_session* session,
    const char* achievement_id,
    size_t achievement_id_len,
    int* out_unlocked);

/**
 * Submits a global leaderboard score (POST /api/v1/dropworks/leaderboard).
 * `out_improved` is set to 1 when the score became the player's best, 0 when it
 * did not, and DROPWORKS_BOOL_UNKNOWN when the server did not report the field;
 * it must not be NULL.
 */
DROPWORKS_API dropworks_status dropworks_submit_score(
    dropworks_client* client,
    const dropworks_session* session,
    const char* leaderboard_key,
    double score,
    int* out_improved);

/** Length-prefixed variant of dropworks_submit_score. */
DROPWORKS_API dropworks_status dropworks_submit_score_n(
    dropworks_client* client,
    const dropworks_session* session,
    const char* leaderboard_key,
    size_t leaderboard_key_len,
    double score,
    int* out_improved);

/**
 * Sets the player's rich presence for a game
 * (POST /api/v1/dropworks/presence). `game_id` may be NULL, in which case the
 * server uses the signed-in app id.
 */
DROPWORKS_API dropworks_status dropworks_set_presence(
    dropworks_client* client,
    const dropworks_session* session,
    const char* game_id,
    const char* status);

/** Length-prefixed variant of dropworks_set_presence. */
DROPWORKS_API dropworks_status dropworks_set_presence_n(
    dropworks_client* client,
    const dropworks_session* session,
    const char* game_id,
    size_t game_id_len,
    const char* status,
    size_t status_len);

/** Borrowed, stable human-readable name for a status code. */
DROPWORKS_API const char* dropworks_status_string(dropworks_status status);

/** Borrowed message for the last failure on this client, or DROPWORKS_NULL. */
DROPWORKS_API const char* dropworks_last_error(
    const dropworks_client* client);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* DROPWORKS_H */
