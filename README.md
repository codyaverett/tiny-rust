# tiny-rust

Exploring how small a Rust binary can get, one technique at a time.

Each example builds on the previous one, progressively stripping away overhead
until we reach a sub-kilobyte ELF binary.

## Examples

| #  | Example | Technique | Size |
|----|---------|-----------|------|
| 01 | [release-opts](01-release-opts/) | Release profile: strip, LTO, opt-level=z | ~297 KB |
| 02 | [panic-abort](02-panic-abort/) | Add panic=abort via workspace profile | ~297 KB |
| 03 | [no-std](03-no-std/) | Drop std, use libc for write/exit, custom `_start` | ~13 KB |
| 04 | [raw-syscall](04-raw-syscall/) | Drop libc, inline asm syscalls, custom linker script | 560 B |
| 05 | [tiny-wasm](05-tiny-wasm/) | WebAssembly target, no_std cdylib | 554 B |
| 06 | [build-std](06-build-std/) | Rebuild core with `-Z build-std`, panic_immediate_abort | ~400 B* |
| 07 | [global-asm](07-global-asm/) | Assembly entry stub via `global_asm!` | ~400 B* |
| 08 | [upx-compressed](08-upx-compressed/) | Post-build UPX compression on example 04 | varies |
| 09 | [tiny-yes](09-tiny-yes/) | Endless "y" output, like coreutils `yes` | ~14 KB |
| 10 | [tiny-base64](10-tiny-base64/) | Base64 encode/decode stdin filter | ~14 KB |
| 11 | [tiny-hash](11-tiny-hash/) | FNV-1a 64-bit hash of stdin | ~14 KB |
| 12 | [tiny-random](12-tiny-random/) | Xorshift64 random number generator | ~14 KB |

*Requires nightly toolchain. Exact size depends on toolchain version.

Examples 09-12 are practical utilities proving tiny binaries can do real work,
all using the no_std + libc pattern from example 03.

## Building

Examples 02-04 and 09-12 are workspace members and build together:

```sh
cargo build --release
```

Example 01 has its own release profile and builds independently:

```sh
cd 01-release-opts && cargo build --release
```

Example 05 targets WebAssembly:

```sh
cd 05-tiny-wasm && cargo build --release --target wasm32-unknown-unknown
```

Examples 06-07 require nightly:

```sh
cd 06-build-std && cargo +nightly build --release
cd 07-global-asm && cargo +nightly build --release
```

Example 08 requires [UPX](https://upx.github.io/):

```sh
cd 08-upx-compressed && bash build.sh
```

## Prerequisites

- Rust stable (for examples 01-05)
- Rust nightly (for examples 06-07): `rustup toolchain install nightly`
- wasm32 target (for example 05): `rustup target add wasm32-unknown-unknown`
- [UPX](https://upx.github.io/) (for example 08, optional)
- Linux x86_64 (examples 03-04, 06-07 use Linux syscalls and x86_64 assembly)
