# 21-tiny-sha256

SHA-256 hash of stdin, like `sha256sum`.

## Technique

Uses the `no_std` + libc pattern from example 03. Implements the full NIST
SHA-256 algorithm: Merkle-Damgard construction with 64-round compression,
K constants table, and proper padding with bit-length suffix. Uses `read` to
stream stdin through 4KB chunks.

## New syscall

- `fstat` -- file metadata (available but hash is computed via streaming reads)

## Usage

```sh
cargo build --release
echo -n "hello" | ./target/release/tiny-sha256
# 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824 -

# Verify against system sha256sum:
echo -n "hello" | sha256sum
# 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824  -

# Empty input:
echo -n "" | ./target/release/tiny-sha256
# e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 -
```
