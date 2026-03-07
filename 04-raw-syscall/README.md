# 04 - Raw Syscalls

Drop libc entirely, use inline assembly for Linux syscalls.

## Technique

- Inline `asm!` for `sys_write` (syscall 1) and `sys_exit` (syscall 60)
- Custom linker script (`linker.ld`) to control ELF layout and discard
  unnecessary sections (.eh_frame, .note, .gnu.hash, etc.)
- `build.rs` passes `-nostartfiles`, `-nodefaultlibs`, `-static`, and the
  linker script

## Build & Run

```sh
# From workspace root
cargo build --release
./target/release/raw-syscall
```

## Result

560 bytes. The binary contains just the ELF header, our code, and the string
literal.

## What Changed vs 03

- Removed `libc` dependency
- Implemented `syscall_write()` and `syscall_exit()` with inline assembly
- Added custom linker script to discard all non-essential ELF sections
- Added `-nodefaultlibs` and `-static` linker flags
