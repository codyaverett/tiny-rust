# 15-tiny-wc

Count lines, words, and bytes from stdin, like coreutils `wc`.

## Technique

Uses the `no_std` + libc pattern from example 03. Implements a simple
state-machine parser to count lines (`\n`), words (whitespace-delimited), and
bytes in a streaming fashion. Output is formatted in right-aligned columns
matching `wc` output style.

## Usage

```sh
cargo build --release

echo "hello world" | ./target/release/tiny-wc
#        1       2      12

cat somefile.txt | ./target/release/tiny-wc
```
