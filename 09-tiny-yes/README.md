# 09-tiny-yes

Endlessly outputs "y" to stdout, like coreutils `yes`.

## Technique

Uses the `no_std` + libc pattern from example 03. Handles EPIPE (broken pipe)
gracefully so `tiny-yes | head -5` exits cleanly.

## Usage

```sh
cargo build --release
./target/release/tiny-yes | head -5
```
