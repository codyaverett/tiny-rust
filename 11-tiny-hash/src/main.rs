#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        libc::exit(1);
    }
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

#[inline(never)]
fn run() {
    let mut hash: u64 = FNV_OFFSET;
    let mut buf = [0u8; 4096];

    unsafe {
        loop {
            let n = libc::read(0, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                libc::exit(1);
            }
            if n == 0 {
                break;
            }
            let mut i = 0;
            while i < n as usize {
                hash ^= buf[i] as u64;
                hash = hash.wrapping_mul(FNV_PRIME);
                i += 1;
            }
        }

        // format as 16-char hex
        let mut out = [0u8; 17];
        let mut i: usize = 16;
        let mut v = hash;
        while i > 0 {
            i -= 1;
            out[i] = HEX_CHARS[(v & 0xF) as usize];
            v >>= 4;
        }
        out[16] = b'\n';
        libc::write(1, out.as_ptr() as *const libc::c_void, 17);
        libc::exit(0);
    }
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    // The kernel enters _start with %rsp 16-byte aligned, but the C ABI
    // expects 16n+8 (as if a `call` pushed a return address). Fix the
    // alignment before calling any Rust code that may use SSE aligned moves.
    core::arch::asm!(
        "and rsp, -16",
        "call {run}",
        run = sym run,
        options(noreturn),
    );
}
