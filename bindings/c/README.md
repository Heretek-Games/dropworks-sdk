# Dropworks C binding

The C ABI lives in [`../../include/dropworks.h`](../../include/dropworks.h).
It declares the `dropworks_client`/`dropworks_session` handles plus
`dropworks_sign_in`, `dropworks_unlock_achievement`, and the presence stub,
matching the TypeScript reference client.

**Status:** implemented. `libdropworks` is built from the Rust crate in
[`../rust`](../rust) with the `c-abi` feature:

```sh
cargo build --release --manifest-path ../rust/Cargo.toml --features c-abi
# produces ../rust/target/release/libdropworks.{so,dylib,dll}
```

Link against that shared library (and the header) from C, C#, or GDScript.

`smoke.c` is a compile-link-run check of the ABI that needs no network:

```sh
cc -Iinclude bindings/c/smoke.c \
  -Lbindings/rust/target/debug -ldropworks -o /tmp/dropworks-smoke
LD_LIBRARY_PATH=bindings/rust/target/debug /tmp/dropworks-smoke
```

Usage:

```c
#include "dropworks.h"

dropworks_client* client = NULL;
dropworks_config config = { "https://drop.example.com", 5000 };
if (dropworks_client_create(&config, &client) != DROPWORKS_OK) {
  /* handle */
}

dropworks_session* session = NULL;
if (dropworks_sign_in(client, "app-id", "auth-token", &session) == DROPWORKS_OK) {
  int unlocked = 0;
  dropworks_unlock_achievement(client, session, "achievement-id", &unlocked);
}

dropworks_session_destroy(session);
dropworks_client_destroy(client);
```

See the header for ownership and threading rules.
