# 01 - Release Optimizations

Baseline: a standard Rust binary with aggressive release profile settings.

## Technique

All optimization happens in `Cargo.toml` profile settings:

- `strip = true` - remove symbol tables and debug info
- `lto = true` - link-time optimization across all crates
- `opt-level = "z"` - optimize for smallest binary size
- `codegen-units = 1` - single codegen unit for better optimization

The code itself is just `println!("Hello, tiny world!")`.

## Build & Run

```sh
cargo build --release
./target/release/release-opts
```

## Result

~297 KB. Still large because the Rust standard library (formatting, I/O, panic
handling) is linked in.

## What Changed

This is the starting point - a normal Rust binary with size-focused profile
options. The next example adds `panic = "abort"` to eliminate unwinding.
