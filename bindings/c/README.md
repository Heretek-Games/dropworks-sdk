# Dropworks C binding

The C ABI lives in [`../../include/dropworks.h`](../../include/dropworks.h).
It declares the `dropworks_client`/`dropworks_session` handles plus
`dropworks_sign_in`, `dropworks_unlock_achievement`, and the presence stub,
matching the TypeScript reference client.

**Status:** header only. The `libdropworks` reference implementation is tracked
in [#19](https://github.com/Heretek-Games/dropworks-sdk/issues/19); linking
against the header will only succeed once that library exists.

Expected usage once implemented:

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
