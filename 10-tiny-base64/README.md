# 10-tiny-base64

Base64 encode/decode filter for stdin to stdout.

## Technique

Uses the `no_std` + libc pattern from example 03. Static encode/decode lookup
tables, reads stdin in chunks, handles padding correctly.

## Usage

```sh
cargo build --release

# Encode
echo "Hello, tiny world!" | ./target/release/tiny-base64

# Decode
echo "Hello, tiny world!" | ./target/release/tiny-base64 | ./target/release/tiny-base64 -d

# Roundtrip verification
echo "Hello, tiny world!" | ./target/release/tiny-base64 | ./target/release/tiny-base64 -d
```
