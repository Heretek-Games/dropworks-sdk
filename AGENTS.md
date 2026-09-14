# AGENTS.md — dropworks-sdk

Dropworks native game SDK (Steamworks replacement) and language bindings (#19).

## Commands

```sh
npm ci
npm run typecheck
npm run build
npm test

cargo test --manifest-path bindings/rust/Cargo.toml
```

This repo has no dependency on `@droposs/plugin-sdk`; it ships a standalone
TypeScript reference client, a C ABI header (`include/dropworks.h`), and
language bindings under `bindings/`.
