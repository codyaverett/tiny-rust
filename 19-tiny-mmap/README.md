# 19-tiny-mmap

Memory-mapped file viewer using `mmap` for zero-copy I/O.

## Technique

Uses the `no_std` + libc pattern from example 03. Opens a file, determines its
size with `lseek`, maps it into memory with `mmap`, and writes the contents to
stdout directly from the mapping. No intermediate buffer copies.

## Usage

```sh
cargo build --release
./target/release/tiny-mmap somefile.txt
```
