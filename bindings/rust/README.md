# Dropworks Rust binding

`dropworks` is a scaffold crate mirroring the TypeScript reference client:

- `DropworksClient::sign_in(app_id, auth_token)` → `POST /api/v1/dropworks/session`
- `DropworksClient::unlock_achievement(achievement_id)` → `POST /api/v1/dropworks/achievement`
- `DropworksClient::current_session()` → the stored `DropworksSession`

HTTP is abstracted behind `Transport`; enable the `http` feature for the
bundled `ReqwestTransport`, or implement `Transport` over your engine's HTTP
stack.

```rust
use dropworks::DropworksClient;

# async fn example(transport: impl dropworks::Transport) {
let mut client = DropworksClient::new(transport, "https://drop.example.com");
let session = client.sign_in("app-id", "auth-token").await?;
client.unlock_achievement("achievement-id").await?;
# Ok::<(), dropworks::DropworksError>(())
# }
```

**Status:** scaffold. The REST calls and session handling are implemented and
unit tested; there is no WebSocket presence client, retry policy, or C-ABI
export layer yet. The C ABI equivalent is
[`../../include/dropworks.h`](../../include/dropworks.h).

```sh
cargo test
```
