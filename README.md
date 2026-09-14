# Dropworks SDK

Dropworks native game SDK (Steamworks replacement) and language bindings (#19).

The TypeScript package in `src/` is the reference client for the REST contract
served by a Drop instance:

```
POST /api/v1/dropworks/session      sign in
POST /api/v1/dropworks/achievement  unlock an achievement
WS   /api/v1/dropworks/presence     presence (not implemented yet)
```

## Layout

| Path | Status |
| :--- | :--- |
| `src/` | Reference TypeScript client (`DropworksClient`). |
| `include/dropworks.h` | C ABI declarations shared by every native binding. **Header only** — the `libdropworks` implementation is tracked in #19. |
| `bindings/rust/` | `dropworks` crate scaffold: transport-abstracted client mirroring the TS API, with unit tests. REST only; no presence client yet. |
| `bindings/c/` | README pointing at `include/dropworks.h`; implementation pending #19. |
| `bindings/csharp/` | Planned `Dropworks.Client`; will P/Invoke the C ABI. |
| `bindings/gdscript/` | Planned Godot 4 autoload; will use a GDExtension over the C ABI. |

## Build

```sh
npm ci
npm run typecheck
npm run build
npm test
```

Rust scaffold:

```sh
cargo test --manifest-path bindings/rust/Cargo.toml
```
