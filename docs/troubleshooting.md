# Troubleshooting Tiny Rust Binaries

Common pitfalls when building minimal Rust binaries, and the tools to debug
them.

## The `_start` Stack Alignment SIGSEGV

This is the most insidious bug in `no_std` + `_start` binaries. It may not
appear until your code becomes complex enough for the compiler to emit SSE
instructions.

### Symptom

The program crashes with SIGSEGV (`si_addr=NULL`) even though all syscalls
succeed. `strace` shows reads completing normally, then the process dies:

```
read(0, "hello\n", 4096)              = 6
--- SIGSEGV {si_signo=SIGSEGV, si_code=SEGV_ACCERR, si_addr=NULL} ---
```

### Root cause

The x86_64 System V ABI requires the stack to be 16-byte aligned **before** a
`call` instruction. After `call` pushes the 8-byte return address, `%rsp` is at
`16n+8` -- which is what the compiler assumes inside any function.

But the Linux kernel enters `_start` with `%rsp` at exactly `16n` (no return
address was pushed). The compiler doesn't know this. When it emits:

```asm
movaps %xmm0, (%rsp)    ; requires 16-byte aligned %rsp
```

...it assumes `%rsp` is at `16n+8` (post-call alignment), subtracts to align,
and lands on a 16-byte boundary. But since `%rsp` started at `16n` instead of
`16n+8`, the subtraction puts it at `16n+8` -- **misaligned** for `movaps`.

### Why simple examples don't trigger it

Examples 01-04 are simple enough that the compiler never emits SSE aligned
memory operations. The bug only surfaces when functions use enough local
variables or temporaries that the compiler decides to use `movaps` for stack
spills.

### How to diagnose

1. **`strace`** confirms syscalls succeed before the crash:
   ```sh
   strace ./target/release/tiny-hash < /dev/null
   ```

2. **`objdump -d`** reveals the `movaps` instruction:
   ```sh
   objdump -d target/release/tiny-hash | grep movaps
   ```

3. **`readelf -h`** shows the entry point address:
   ```sh
   readelf -h target/release/tiny-hash | grep Entry
   ```

### The fix

Align the stack in `_start` before calling any Rust code:

```rust
#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::asm!(
        "and rsp, -16",   // force 16-byte alignment
        "call {run}",     // call pushes return addr -> 16n+8, as expected
        run = sym run,
        options(noreturn),
    );
}
```

The `and rsp, -16` rounds down to 16-byte alignment. The subsequent `call`
pushes a return address, putting `%rsp` at `16n+8` -- exactly what the
compiler expects.

The actual work goes in a separate `#[inline(never)]` function:

```rust
#[inline(never)]
fn run() {
    // all program logic here
}
```

**See:** [11-tiny-hash/src/main.rs](../11-tiny-hash/src/main.rs) for the
complete implementation.

## Debugging Tools Reference

### `readelf` -- ELF structure inspector

Shows ELF headers, sections, program headers, and symbol tables.

```sh
# Show ELF header (entry point, type, machine)
readelf -h target/release/tiny-hash

# List all sections with sizes
readelf -S target/release/tiny-hash

# Show program headers (what gets loaded into memory)
readelf -l target/release/tiny-hash

# Show dynamic section (linked libraries)
readelf -d target/release/tiny-hash
```

**Good for:** Understanding binary structure, verifying the entry point address,
checking which sections exist and their sizes.

**Not for:** Reading actual code -- use `objdump` for disassembly.

### `objdump` -- disassembler

Disassembles binary code and shows section contents.

```sh
# Full disassembly with Intel syntax
objdump -d -M intel target/release/tiny-hash

# Find specific instructions (e.g., SSE moves)
objdump -d target/release/tiny-hash | grep movaps

# Show all sections with contents
objdump -s target/release/tiny-hash

# Show only the .text section
objdump -d -j .text target/release/tiny-hash
```

**Good for:** Finding specific instructions, understanding what the compiler
generated, tracing code flow from `_start`.

**Not for:** High-level structure -- use `readelf` for that.

### `strace` -- syscall tracer

Traces every system call the program makes.

```sh
# Basic trace
strace ./target/release/tiny-hash < /dev/null

# With timestamps (to see timing)
strace -t ./target/release/tiny-hash < /dev/null

# Filter to specific syscalls
strace -e trace=write,read ./target/release/tiny-hash < /dev/null
```

**Good for:** Seeing what succeeded before a crash, verifying syscall arguments
and return values, confirming the program reaches specific points.

**Not for:** Understanding *why* code crashed -- it shows the last successful
syscall, not the faulting instruction.

### `size` -- section size summary

Quick text/data/bss breakdown.

```sh
size target/release/tiny-hash
#    text    data     bss     dec     hex filename
#     887     384       0    1271     4f7 target/release/tiny-hash
```

**Good for:** Quick comparison between builds, seeing where bytes are going.

**Not for:** Detailed analysis -- use `readelf -S` for per-section breakdown.

### `nm` -- symbol inspector

Lists symbols (functions, variables) in the binary.

```sh
# All symbols
nm target/release/tiny-hash

# Dynamic symbols only
nm -D target/release/tiny-hash

# Sort by size
nm --size-sort target/release/tiny-hash
```

Note: With `strip = true` in the release profile, `nm` will show no symbols.
Build without stripping to inspect symbols, or use `nm` on the unstripped
binary before the strip step.

**Good for:** Finding what functions exist and their sizes, verifying dead code
was eliminated.

**Not for:** Stripped binaries (no symbols to show).

### `file` -- binary identification

Quick identification of binary type.

```sh
file target/release/tiny-hash
# target/release/tiny-hash: ELF 64-bit LSB pie executable, x86-64, ...
```

**Good for:** Confirming the binary type, architecture, whether it's statically
or dynamically linked.

**Not for:** Anything beyond identification.

### `xxd` / `hexdump` -- raw byte inspection

View raw binary contents.

```sh
# First 128 bytes in hex + ASCII
xxd -l 128 target/release/tiny-hash

# ELF magic number verification
xxd -l 4 target/release/tiny-hash
# 00000000: 7f45 4c46  .ELF
```

**Good for:** Inspecting ELF magic bytes, verifying section contents, debugging
linker script issues.

**Not for:** Understanding code -- use `objdump` for that.

### `ldd` -- shared library dependencies

Lists shared libraries a dynamically linked binary needs.

```sh
ldd target/release/tiny-hash
# Output for a statically linked binary:
#     not a dynamic executable
```

**Good for:** Verifying a binary is truly statically linked, finding unexpected
dynamic dependencies.

**Not for:** Static binaries (it just says "not a dynamic executable").

## Other Common Pitfalls

### Missing `#[panic_handler]` in `no_std`

Every `no_std` binary must provide a panic handler:

```rust
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { libc::exit(1); }
}
```

Without it, the compiler error is clear but can be confusing if you're new to
`no_std`:

```
error[E0152]: found duplicate lang item `panic_impl`
```

or:

```
error: `#[panic_handler]` function required, but not found
```

### Forgetting `_start` with `-nostartfiles`

When using `-nostartfiles` (which the `no_std` examples do via `build.rs` or
linker flags), you must provide a `_start` function. Without it, the linker
will complain:

```
error: linking with `cc` failed: exit status: 1
  = note: /usr/bin/ld: cannot find entry symbol _start
```

The function must be `#[no_mangle]` and `pub extern "C"`:

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // ...
}
```

### `libc` with `default-features = false`

The `libc` crate's default features include `std`. In a `no_std` binary, you
must disable them:

```toml
[dependencies]
libc = { version = "0.2", default-features = false }
```

Forgetting this will pull in `std` and negate the `no_std` size savings.

### Workspace vs standalone builds

Some examples can't share workspace settings. In this project:

- **Example 01** is excluded from the workspace because it needs its own
  release profile (no `panic = "abort"`) to show the baseline size.
- **Examples 05-08** are excluded because they have special build requirements
  (wasm target, nightly toolchain, UPX post-processing).
- **Examples 02-04 and 09-12** are workspace members and share the release
  profile.

If you add a new example, decide whether it can use the workspace profile or
needs standalone configuration.

### Bounds checks inflating binary size

Array and slice indexing in Rust generates bounds checks that call into panic
formatting code. In `no_std` binaries, this can pull in unexpected code.

Mitigation strategies:
- Use iterators instead of indexing where possible
- Use `get_unchecked()` in performance-critical `unsafe` blocks when bounds
  are provably correct
- Build with `-Z build-std-features=panic_immediate_abort` (nightly) to
  eliminate panic formatting entirely

### Stack buffer sizing

In `no_std` binaries without an allocator, all buffers live on the stack. Be
mindful of stack size -- the default thread stack is 8 MB, but in `_start`
there's no guarantee of a large stack. The examples use 4096-byte buffers
which is safe for typical use.
