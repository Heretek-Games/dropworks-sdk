# Dropworks C# binding

Managed P/Invoke wrapper over the Dropworks C ABI (`libdropworks`).

## Layout

- `Dropworks/` — `Dropworks.Client` class library: `DropworksNative` (raw
  P/Invoke), `DropworksClient`, `DropworksSession`, `DropworksException`.
- `Dropworks.Smoke/` — a console smoke test that links the real shared library
  and exercises client creation, argument validation, and error reporting.

## Build

First build the native library (from the repository root):

```sh
cargo build --manifest-path bindings/rust/Cargo.toml --features c-abi
```

Then build the managed binding and run the smoke test:

```sh
dotnet build bindings/csharp/Dropworks.Smoke/Dropworks.Smoke.csproj -c Release
LD_LIBRARY_PATH=bindings/rust/target/debug \
  dotnet run --project bindings/csharp/Dropworks.Smoke/Dropworks.Smoke.csproj -c Release --no-build
```

On Windows, place `dropworks.dll` next to the managed assembly instead of setting
`LD_LIBRARY_PATH`.

## Usage

```csharp
using Dropworks;

using var client = new DropworksClient("https://drop.example.com");
using var session = client.SignIn("app-id", "auth-token");

client.UnlockAchievement(session, "beat-the-boss");
client.SubmitScore(session, "high-score", 9001);
client.SetPresence(session, "in-game", "game-id");
```

`SubmitScore` returns `true` only when the server reported the score as a new
best. When the server omits the `improved` field the native ABI reports
`DROPWORKS_BOOL_UNKNOWN` (`-1`) and the managed wrapper returns `false`.
