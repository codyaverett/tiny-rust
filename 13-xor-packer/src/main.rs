//! Minimal XOR packer stub for Linux x86-64 (educational/CTF example).
//!
//! The binary stores only XOR-encrypted shellcode. At runtime it:
//!   1. Decrypts the payload
//!   2. Maps an RWX page via the mmap syscall
//!   3. Copies the shellcode into it
//!   4. Jumps to it
//!
//! The payload is a tiny shellcode that calls exit(42).
//! After running, `echo $?` should print 42.
//!
//! Build:  cargo build --release
//! Run:    ./target/release/xor-packer; echo $?   # prints 42

use std::arch::asm;

// --- Payload (plaintext shellcode) ---
// This is what we're packing. It calls exit(42) on x86-64 Linux:
//
//   mov edi, 42          ; bf 2a 00 00 00
//   mov eax, 60          ; b8 3c 00 00 00
//   syscall              ; 0f 05
//
// Raw bytes: [0xbf, 0x2a, 0x00, 0x00, 0x00, 0xb8, 0x3c, 0x00, 0x00, 0x00, 0x0f, 0x05]

const XOR_KEY: u8 = 0xAA;

// Encrypted payload (each byte XORed with 0xAA):
//   0xbf^0xaa=0x15  0x2a^0xaa=0x80  0x00^0xaa=0xaa  0x00^0xaa=0xaa
//   0x00^0xaa=0xaa  0xb8^0xaa=0x12  0x3c^0xaa=0x96  0x00^0xaa=0xaa
//   0x00^0xaa=0xaa  0x00^0xaa=0xaa  0x0f^0xaa=0xa5  0x05^0xaa=0xaf
static ENCRYPTED: [u8; 12] = [
    0x15, 0x80, 0xaa, 0xaa, 0xaa, 0x12, 0x96, 0xaa, 0xaa, 0xaa, 0xa5, 0xaf,
];

/// Raw mmap syscall. Returns pointer to mapped region or -1 on error.
unsafe fn mmap_rwx(len: usize) -> *mut u8 {
    let addr: usize;
    // syscall 9 = mmap
    // rdi=0 (kernel picks address), rsi=len, rdx=7 (RWX), r10=0x22 (MAP_PRIVATE|MAP_ANON),
    // r8=-1 (no fd), r9=0 (offset)
    unsafe {
        asm!(
            "syscall",
            in("rax") 9_u64,       // __NR_mmap
            in("rdi") 0_u64,       // addr = NULL
            in("rsi") len as u64,  // length
            in("rdx") 7_u64,       // prot = PROT_READ|PROT_WRITE|PROT_EXEC
            in("r10") 0x22_u64,    // flags = MAP_PRIVATE|MAP_ANONYMOUS
            in("r8") (-1_i64) as u64, // fd = -1
            in("r9") 0_u64,       // offset = 0
            lateout("rax") addr,
            lateout("rcx") _,      // clobbered by syscall
            lateout("r11") _,      // clobbered by syscall
        );
    }
    addr as *mut u8
}

fn main() {
    // Step 1: Decrypt payload
    let mut shellcode = [0u8; ENCRYPTED.len()];
    for i in 0..ENCRYPTED.len() {
        shellcode[i] = ENCRYPTED[i] ^ XOR_KEY;
    }

    unsafe {
        // Step 2: Allocate an executable page
        let page = mmap_rwx(shellcode.len());
        assert!(!page.is_null() && page as isize != -1, "mmap failed");

        // Step 3: Copy decrypted shellcode into the executable page
        core::ptr::copy_nonoverlapping(shellcode.as_ptr(), page, shellcode.len());

        // Step 4: Jump to it
        let entry: extern "C" fn() -> ! = core::mem::transmute(page);
        entry();
    }
}
