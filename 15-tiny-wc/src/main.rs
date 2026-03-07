#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        libc::exit(1);
    }
}

unsafe fn write_all(fd: i32, buf: &[u8]) {
    let mut written = 0;
    while written < buf.len() {
        let ret = libc::write(
            fd,
            buf.as_ptr().add(written) as *const libc::c_void,
            buf.len() - written,
        );
        if ret <= 0 {
            libc::exit(if ret == 0 { 1 } else { 0 });
        }
        written += ret as usize;
    }
}

fn format_u64(n: u64, buf: &mut [u8; 20]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut val = n;
    let mut pos = 20;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    // Shift to start of buffer
    let len = 20 - pos;
    let mut i = 0;
    while i < len {
        buf[i] = buf[pos + i];
        i += 1;
    }
    len
}

unsafe fn print_counts(lines: u64, words: u64, bytes: u64) {
    let mut num_buf = [0u8; 20];
    let mut out = [0u8; 80];
    let mut out_pos = 0;

    // Right-align each field in 8-char columns
    let counts = [lines, words, bytes];
    let mut c = 0;
    while c < 3 {
        let len = format_u64(counts[c], &mut num_buf);
        // Pad with spaces
        let mut pad = 0;
        while pad + len < 8 {
            out[out_pos] = b' ';
            out_pos += 1;
            pad += 1;
        }
        let mut j = 0;
        while j < len {
            out[out_pos] = num_buf[j];
            out_pos += 1;
            j += 1;
        }
        c += 1;
    }

    out[out_pos] = b'\n';
    out_pos += 1;
    write_all(1, &out[..out_pos]);
}

#[inline(never)]
fn run() {
    let mut buf = [0u8; 4096];
    let mut lines: u64 = 0;
    let mut words: u64 = 0;
    let mut bytes: u64 = 0;
    let mut in_word = false;

    unsafe {
        loop {
            let n = libc::read(0, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                libc::exit(1);
            }
            if n == 0 {
                break;
            }

            let chunk = &buf[..n as usize];
            bytes += n as u64;

            let mut i = 0;
            while i < chunk.len() {
                let c = chunk[i];
                let is_ws = c == b' ' || c == b'\n' || c == b'\t' || c == b'\r';

                if c == b'\n' {
                    lines += 1;
                }

                if is_ws {
                    in_word = false;
                } else if !in_word {
                    in_word = true;
                    words += 1;
                }

                i += 1;
            }
        }

        print_counts(lines, words, bytes);
        libc::exit(0);
    }
}

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::asm!(
        "and rsp, -16",
        "call {run}",
        run = sym run,
        options(noreturn),
    );
}
