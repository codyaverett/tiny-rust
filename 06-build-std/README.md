# 06 - Build Std (Nightly)

Rebuild the `core` library from source with size optimizations.

## Technique

- `-Z build-std=core` recompiles the core library with our release profile
  settings (LTO, opt-level=z) instead of using the precompiled version
- `build-std-features = ["panic_immediate_abort"]` eliminates all panic
  formatting machinery, making the panic handler truly zero-cost
- Configured via `.cargo/config.toml`

## Build & Run

Requires nightly:

```sh
cargo +nightly build --release
./target/x86_64-unknown-linux-gnu/release/build-std
```

## Result

~400 bytes (varies by toolchain version). May be slightly smaller than
example 04 because `core` itself is optimized for size and panic formatting
is completely eliminated.

## What Changed vs 04

- Added `.cargo/config.toml` with `build-std` and `panic_immediate_abort`
- Requires nightly toolchain
- Output goes to `target/x86_64-unknown-linux-gnu/` (explicit target)
- Same code, but the compiler-provided `core` library is now size-optimized
