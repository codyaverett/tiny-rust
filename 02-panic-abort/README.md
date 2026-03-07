# 02 - Panic Abort

Add `panic = "abort"` to eliminate stack unwinding machinery.

## Technique

The workspace `Cargo.toml` sets `panic = "abort"` in the release profile. This
tells the compiler to abort immediately on panic instead of unwinding the stack,
removing all the unwinding tables and code.

This example has no per-crate profile settings - it inherits everything from
the workspace.

## Build & Run

```sh
# From workspace root
cargo build --release
./target/release/panic-abort
```

## Result

~297 KB. The size difference from example 01 is minimal because std still
pulls in a large amount of code. The real savings come when we drop std
entirely.

## What Changed vs 01

- Moved release profile settings to workspace-level `Cargo.toml`
- Added `panic = "abort"` to the workspace release profile
- Per-crate `Cargo.toml` has no profile section (inherits from workspace)
