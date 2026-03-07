# 07 - Global Assembly Entry

Use `global_asm!` for the ELF entry point, keeping Rust only for logic.

## Technique

- `global_asm!` defines `_start` in pure assembly
- The assembly stub calls `rust_main()` and uses its return value as the
  exit code for a `sys_exit` syscall
- Rust code focuses on logic only (`rust_main` returns a status code)
- Separates the platform entry/exit boilerplate from application logic

## Build & Run

```sh
cargo build --release
./target/release/global-asm
```

## Result

~560 bytes. Similar size to example 04 - the technique is more about code
organization than size reduction. The assembly entry stub is a few bytes
smaller than the Rust `extern "C" fn _start()`, but the difference is
minimal.

## What Changed vs 04

- `_start` is now written in assembly via `global_asm!`
- Added `rust_main()` function that returns an exit code
- Assembly handles the `sys_exit` syscall after `rust_main` returns
- Cleaner separation between platform boilerplate and application logic
