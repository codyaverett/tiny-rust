# 11-tiny-hash

Computes FNV-1a 64-bit hash of stdin and outputs a 16-character hex digest.

## Technique

Uses the `no_std` + libc pattern from example 03. FNV-1a is a simple,
non-cryptographic hash function ideal for tiny binaries. Reads stdin in
4096-byte chunks for efficient streaming.

## Usage

```sh
cargo build --release

echo "Hello, tiny world!" | ./target/release/tiny-hash
```
