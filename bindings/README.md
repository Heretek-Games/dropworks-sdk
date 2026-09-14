# Language bindings

Each binding targets the Dropworks contract described in `../src/index.ts`.
`include/dropworks.h` is the single source of truth for the native bindings;
`libdropworks` is built from `bindings/rust` with `--features c-abi`.

| Binding     | Status                                                                                                                                                          |
| :---------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `c/`        | Links the C ABI (`include/dropworks.h`) and `libdropworks`; `smoke.c` is a compile-link-run test.                                                               |
| `rust/`     | `dropworks` crate: async client over a pluggable `Transport`, the C ABI cdylib (`--features c-abi`), and session/achievement/leaderboard/presence methods.       |
| `csharp/`   | `Dropworks.Client` P/Invoke wrapper over `include/dropworks.h`, with a buildable smoke test.                                                                    |
| `gdscript/` | Godot 4 `HTTPClient` binding (`DropworksClient`) with a headless test.                                                                                          |
