# 08 - UPX Compressed

Post-build compression with UPX on the raw-syscall binary.

## Technique

[UPX](https://upx.github.io/) (Ultimate Packer for eXecutables) compresses
an ELF binary into a self-extracting executable. At runtime, it decompresses
into memory before execution.

This is not a Rust crate - just a shell script that builds example 04 and
compresses the result.

## Build & Run

Requires UPX to be installed:

```sh
# Ubuntu/Debian
sudo apt install upx-ucl

# Then build
bash build.sh
./raw-syscall-upx
```

## Result

The compressed size depends on UPX version and the input binary. For a 560
byte binary, UPX may actually make it larger due to the decompressor overhead.
UPX shines on larger binaries (example 01 or 02 would see significant gains).

## Tradeoffs

- Adds startup decompression time (negligible for small binaries)
- Some environments block UPX-packed binaries (antivirus, certain deployments)
- The binary is not inspectable with standard tools until decompressed
- Most useful for distribution size, not runtime performance
