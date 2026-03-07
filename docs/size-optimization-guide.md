# Rust Binary Size Optimization Guide

A practical guide to making Rust binaries smaller, based on the techniques
explored in this project. Each technique is demonstrated with measured results
from real builds.

## Why Small Binaries?

### Embedded / IoT constrained environments

Microcontrollers and edge devices often have kilobytes, not megabytes, of flash.
A 300 KB binary simply won't fit on many targets.

*Counterpoint:* Most embedded work uses cross-compilation with dedicated
toolchains (e.g., `thumbv7em-none-eabihf`) that already exclude `std`. You'll
use `no_std` regardless of size concerns.

### Container images / serverless cold starts

Smaller binaries mean smaller container layers and faster cold starts in
serverless environments. A 14 KB binary loads meaningfully faster than a 300 KB
one when multiplied across thousands of function invocations.

*Counterpoint:* The base image layer (even `scratch` needs a loader for
dynamically linked binaries) typically dwarfs the application binary. Optimizing
the app from 300 KB to 14 KB saves nothing if it sits in a 5 MB image.

### Attack surface reduction

Fewer bytes means less code. Less code means fewer potential vulnerabilities
and a smaller surface for reverse engineering.

*Counterpoint:* Stripping symbols and reducing binary size are not security
measures. A determined attacker will decompile regardless. Real security comes
from correct code, not small code.

### Deployment bandwidth / edge distribution

When distributing binaries to many endpoints (CDNs, firmware updates, P2P
networks), every kilobyte multiplied by millions of targets adds up.

*Counterpoint:* Compression (gzip, zstd) handles this effectively. A 300 KB
binary compresses well; the delta between compressed 300 KB and compressed 14 KB
is much smaller than the raw difference suggests.

### Intellectual curiosity

Understanding what goes into a binary -- ELF headers, sections, the runtime,
standard library -- teaches you how the toolchain works from source to
executable. No counterpoint needed.

## Techniques Ranked by Impact

Ordered from largest size reduction to smallest, with measured sizes from our
builds.

### 1. Drop std (`no_std` + libc)

**297 KB -> 14 KB (~95% reduction)**

The standard library includes the allocator, panic runtime, formatting
machinery, thread-local storage, and stack unwinding support. Dropping it
removes all of that.

With `no_std`, you use `libc` for system calls (write, read, exit) and
provide your own `_start` entry point and `#[panic_handler]`.

```rust
#![no_std]
#![no_main]

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { libc::exit(1); }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let msg = b"Hello, tiny world!\n";
    unsafe {
        libc::write(1, msg.as_ptr() as *const libc::c_void, msg.len());
        libc::exit(0);
    }
}
```

**See:** [03-no-std](../03-no-std/)

### 2. Drop libc (raw syscalls + custom linker script)

**14 KB -> 560 B (~96% further reduction)**

The `libc` crate statically links C library code. By replacing libc calls with
inline assembly syscalls and using a custom linker script to discard unnecessary
ELF sections, you eliminate all external dependencies.

```rust
fn syscall_write(fd: u64, buf: *const u8, len: u64) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 1u64,  // sys_write
            in("rdi") fd,
            in("rsi") buf,
            in("rdx") len,
            out("rcx") _, out("r11") _, lateout("rax") _,
        );
    }
}
```

The custom linker script discards `.comment`, `.note.*`, `.eh_frame`,
`.gnu.hash`, `.dynsym`, `.dynstr`, `.dynamic`, `.got`, `.plt`, `.rela.*`,
and `.debug_*` sections.

**See:** [04-raw-syscall](../04-raw-syscall/)

### 3. Release profile settings

These are configured once in `Cargo.toml` and apply to all workspace members:

```toml
[profile.release]
strip = true       # Remove symbol tables and debug info
lto = true         # Link-time optimization across all crates
opt-level = "z"    # Optimize for size over speed
codegen-units = 1  # Single codegen unit enables better LTO
panic = "abort"    # No unwinding tables or landing pads
```

Individual impact:
- **`strip = true`**: Removes symbol table and debug sections. Significant on
  larger binaries, minimal effect on already-tiny ones.
- **`lto = true`**: Enables cross-crate inlining and dead code elimination.
  Most impactful with multiple dependencies.
- **`opt-level = "z"`**: Tells LLVM to prefer smaller code. May be slower than
  `opt-level = 3` for hot loops but typically negligible.
- **`codegen-units = 1`**: Gives LLVM a whole-program view. Slower compilation,
  better optimization.
- **`panic = "abort"`**: Eliminates unwinding tables and cleanup landing pads.
  On `no_std` binaries this is already implicit.

### 4. Rebuild core with `-Z build-std`

**560 B -> ~400 B (nightly only)**

The prebuilt `core` library includes panic formatting code. Rebuilding it from
source with `panic_immediate_abort` removes even those remnants:

```sh
cargo +nightly build --release \
    -Z build-std=core \
    -Z build-std-features=panic_immediate_abort
```

This is nightly-only and the exact savings depend on toolchain version.

**See:** [06-build-std](../06-build-std/)

### 5. Post-build compression (UPX)

UPX packs executables into self-extracting binaries. On a 300 KB binary, this
can yield meaningful savings. On a 560 B binary, there's nothing left to
compress -- the UPX header itself may be larger than the original.

**See:** [08-upx-compressed](../08-upx-compressed/)

## The Sweet Spot: `no_std` + libc

Examples 09-12 prove that the `no_std` + libc pattern (from example 03) is the
practical sweet spot:

- **Stable Rust** -- no nightly required
- **~14 KB binaries** -- small enough for any deployment scenario
- **Real functionality** -- base64 encoding, hashing, PRNG, I/O loops
- **Readable code** -- straightforward Rust with `libc` for syscalls
- **Portable** -- works on any Linux x86_64 system

Going smaller (raw syscalls, custom linker scripts) gains you another order of
magnitude but ties you to a specific architecture and makes the code harder to
maintain.

## Quick Reference

| # | Example | Technique | Size | Stable? | Portable? |
|---|---------|-----------|------|---------|-----------|
| 01 | release-opts | Release profile only | 297 KB | Yes | Yes |
| 02 | panic-abort | + panic=abort | 297 KB | Yes | Yes |
| 03 | no-std | + no_std, libc, custom _start | 14 KB | Yes | Linux x86_64 |
| 04 | raw-syscall | + raw asm syscalls, linker script | 560 B | Yes | Linux x86_64 |
| 05 | tiny-wasm | WebAssembly no_std cdylib | 554 B | Yes | wasm32 |
| 06 | build-std | + rebuild core, panic_immediate_abort | ~400 B | Nightly | Linux x86_64 |
| 07 | global-asm | + global_asm! entry stub | ~400 B | Nightly | Linux x86_64 |
| 08 | upx-compressed | + UPX on example 04 | varies | Yes | Linux x86_64 |
| 09 | tiny-yes | Endless `y` output (coreutils yes) | 14 KB | Yes | Linux x86_64 |
| 10 | tiny-base64 | Base64 encode/decode stdin | 14 KB | Yes | Linux x86_64 |
| 11 | tiny-hash | FNV-1a 64-bit hash of stdin | 14 KB | Yes | Linux x86_64 |
| 12 | tiny-random | Xorshift64 random number generator | 14 KB | Yes | Linux x86_64 |

All sizes measured on Linux x86_64 with Rust stable (except 06-07 which require nightly).
