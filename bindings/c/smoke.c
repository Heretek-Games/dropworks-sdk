/*
 * smoke.c — end-to-end smoke test for the Dropworks C ABI.
 *
 * Builds and links against the `libdropworks` shared library, then exercises
 * the parts of the contract that need no network: client creation, argument
 * validation, status strings, and cleanup. Exits non-zero on any failure.
 *
 * Build (from the repository root):
 *   cargo build --manifest-path bindings/rust/Cargo.toml --features c-abi
 *   cc -Iinclude bindings/c/smoke.c \
 *     -Lbindings/rust/target/debug -ldropworks -o /tmp/dropworks-smoke
 *   LD_LIBRARY_PATH=bindings/rust/target/debug /tmp/dropworks-smoke
 */

#include <stdio.h>
#include <string.h>

#include "dropworks.h"

static int failures = 0;

static void check(int condition, const char *what) {
  if (!condition) {
    fprintf(stderr, "FAIL: %s\n", what);
    failures++;
  }
}

int main(void) {
  check(strcmp(dropworks_status_string(DROPWORKS_OK), "DROPWORKS_OK") == 0,
        "status_string(OK)");
  check(strcmp(dropworks_status_string(DROPWORKS_ERR_UNAUTHORIZED),
               "DROPWORKS_ERR_UNAUTHORIZED") == 0,
        "status_string(UNAUTHORIZED)");

  dropworks_client *client = NULL;
  dropworks_config config = { "https://drop.example.com", 1000 };
  check(dropworks_client_create(&config, &client) == DROPWORKS_OK,
        "client_create");
  check(client != NULL, "client_create returns a handle");

  /* NULL arguments must be rejected, not crash. */
  check(dropworks_client_create(NULL, &client) == DROPWORKS_ERR_INVALID_ARGUMENT,
        "client_create rejects NULL config");

  dropworks_session *session = NULL;
  check(dropworks_sign_in(client, NULL, "token", &session) ==
            DROPWORKS_ERR_INVALID_ARGUMENT,
        "sign_in rejects NULL app id");
  check(session == NULL, "no session on failure");

  int improved = 9;
  check(dropworks_submit_score(client, NULL, "high-score", 1.0, &improved) ==
            DROPWORKS_ERR_INVALID_ARGUMENT,
        "submit_score rejects NULL session");

  check(dropworks_set_presence(client, NULL, "game", "online") ==
            DROPWORKS_ERR_NOT_IMPLEMENTED,
        "set_presence is declared but not implemented");

  check(dropworks_session_app_id(NULL) == DROPWORKS_NULL,
        "session_app_id(NULL) is NULL");

  dropworks_session_destroy(NULL);
  dropworks_client_destroy(client);

  if (failures == 0) {
    printf("dropworks C ABI smoke test passed\n");
    return 0;
  }
  fprintf(stderr, "%d check(s) failed\n", failures);
  return 1;
}
