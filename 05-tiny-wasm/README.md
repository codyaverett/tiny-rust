# 05 - Tiny WebAssembly

A minimal no_std Rust library compiled to WebAssembly.

## Technique

- `crate-type = ["cdylib"]` to produce a C-compatible dynamic library (.wasm)
- `#![no_std]` with `wasm32::unreachable()` as the panic handler
- Exports simple `add` and `fib` functions
- Includes an HTML page that loads and exercises the WASM module

## Build & Run

```sh
cargo build --release --target wasm32-unknown-unknown
```

To test in a browser, copy the wasm file and serve:

```sh
cp target/wasm32-unknown-unknown/release/tiny_wasm.wasm .
python3 -m http.server 8080
# Open http://localhost:8080/index.html
```

## Result

554 bytes for a WASM module with two exported functions.

## What Changed vs 04

- Different target: WebAssembly instead of Linux x86_64
- Library crate (`cdylib`) instead of binary
- No syscalls needed - WASM functions are called from JavaScript
- Panic handler uses `wasm32::unreachable()` trap instruction
