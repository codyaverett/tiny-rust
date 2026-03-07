# 13 - XOR Packer

Minimal XOR packer stub that decrypts and executes shellcode at runtime.

## Technique

An educational/CTF-style packer for Linux x86-64. The binary stores only
XOR-encrypted shellcode. At runtime it:

1. Decrypts the payload using a single-byte XOR key
2. Maps an RWX page via the `mmap` syscall (inline assembly)
3. Copies the decrypted shellcode into the executable page
4. Jumps to it

The included payload is a tiny shellcode that calls `exit(42)`.

## Build & Run

```sh
cargo build --release
./target/release/xor-packer; echo $?   # prints 42
```

## Result

A self-contained packer stub demonstrating inline assembly for raw syscalls
(`mmap`), XOR encryption/decryption, and runtime code execution. The focus
is on the technique rather than minimal binary size.

## Security Note

This is strictly for educational purposes and CTF challenges. XOR packing
is a well-known technique in malware analysis and reverse engineering
coursework. The payload here is harmless (`exit(42)`).
