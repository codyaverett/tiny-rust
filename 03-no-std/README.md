# 03 - No Standard Library

Drop std entirely, use `libc` crate for write and exit.

## Technique

- `#![no_std]` - don't link the standard library
- `#![no_main]` - don't use the standard entry point
- Custom `_start` entry point with `extern "C"` calling convention
- `libc` crate (with `default-features = false`) for `write()` and `exit()`
- Custom panic handler that calls `libc::exit(1)`
- `build.rs` passes `-nostartfiles` to skip the C runtime startup

## Build & Run

```sh
# From workspace root
cargo build --release
./target/release/no-std
```

## Result

~13 KB. A massive drop from ~297 KB. Most of the remaining size is the ELF
structure, libc linkage overhead, and default linker sections.

## What Changed vs 02

- Added `#![no_std]` and `#![no_main]`
- Replaced `println!` with direct `libc::write()` syscall
- Added `libc` dependency (no default features)
- Added `build.rs` for `-nostartfiles` linker flag
- Added custom `#[panic_handler]`
