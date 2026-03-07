# 12-tiny-random

Generates random numbers using xorshift64 PRNG, seeded from `/dev/urandom`.

## Technique

Uses the `no_std` + libc pattern from example 03. Xorshift64 is a fast,
minimal PRNG. Seeds from `/dev/urandom` for non-deterministic output.

## Usage

```sh
cargo build --release

# Single random number
./target/release/tiny-random

# Multiple random numbers
./target/release/tiny-random -n 5

# Raw random bytes (e.g., 1024 bytes)
./target/release/tiny-random -b 1024 > random.bin
```
