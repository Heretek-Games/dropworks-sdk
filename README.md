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
| `include/dropworks.h` | C ABI declarations; implemented by `libdropworks`. |
| `bindings/rust/` | `dropworks` crate: async client over a pluggable transport, plus the C ABI cdylib (`--features c-abi`). REST only; no presence client yet. |
| `bindings/c/` | Links `libdropworks` and `include/dropworks.h`. |
| `bindings/csharp/` | `Dropworks.Client` P/Invoke wrapper over the C ABI, with a buildable smoke test. |
| `bindings/gdscript/` | Planned Godot 4 autoload; will use a GDExtension over the C ABI. |

The server side of this contract is implemented in
[`Heretek-Games/drop`](https://github.com/Heretek-Games/drop)
(`server/server/api/v1/dropworks/`, commit `45515274`).

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
