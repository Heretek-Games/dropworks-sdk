# Language bindings

Each binding targets the Dropworks contract described in `../src/index.ts`.

| Binding | Status |
| :--- | :--- |
| `c/` | Uses the C ABI header at [`../include/dropworks.h`](../include/dropworks.h). Header only until `libdropworks` lands (#19). |
| `rust/` | `dropworks` crate scaffold with a pluggable `Transport`, session handling, and achievement unlock; unit tested. |
| `csharp/` | Planned `Dropworks.Client` P/Invoke wrapper over `include/dropworks.h`. |
| `gdscript/` | Planned Godot 4 autoload backed by a GDExtension over `include/dropworks.h`. |

When `libdropworks` is implemented, the C ABI in `include/dropworks.h` becomes
the single source of truth for all native bindings.
