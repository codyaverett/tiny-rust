# tiny-base32

Encodes and decodes RFC 4648 Base32 and Crockford Base32 with proper 5-bit grouping and padding, the encoding used for TOTP secrets and onion addresses.

## What you learn

* How to encode and decode Base32 strings
* How to implement case-insensitive decoding

## Build and Run

* `cargo build --release`
* `cargo run --example <example_name>`

## Measured Size

* 2.3 KB (release build)