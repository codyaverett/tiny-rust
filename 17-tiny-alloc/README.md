# 17-tiny-alloc

Bump allocator demo showing heap-like allocation without std or a global allocator.

## Technique

Uses the `no_std` + libc pattern from example 03. Implements a simple bump
allocator backed by a 4 KB static byte array. Supports aligned allocation
and bulk reset. Demonstrates that dynamic memory is possible without `alloc`
or `std`.

## Usage

```sh
cargo build --release
./target/release/tiny-alloc
```

Output shows allocations of u32, u64, and string values with their addresses,
heap usage stats, and reset behavior.
