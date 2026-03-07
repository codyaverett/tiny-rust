# 14-tiny-cat

Concatenate files or stdin to stdout, like coreutils `cat`.

## Technique

Uses the `no_std` + libc pattern from example 03. Introduces file descriptor
handling with `open`/`read`/`write`/`close`. Supports multiple file arguments,
`-` for stdin, and prints errors to stderr.

## Usage

```sh
cargo build --release

# Read from stdin
echo "hello" | ./target/release/tiny-cat

# Read files
./target/release/tiny-cat file1.txt file2.txt

# Mix stdin and files
echo "from stdin" | ./target/release/tiny-cat file1.txt - file2.txt
```
