#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        libc::exit(1);
    }
}

const ENCODE_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const DECODE_TABLE: [u8; 256] = {
    let mut table = [0xFFu8; 256];
    let enc = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut i = 0;
    while i < 64 {
        table[enc[i] as usize] = i as u8;
        i += 1;
    }
    table[b'=' as usize] = 0;
    table
};

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

fn encode() {
    let mut buf = [0u8; 4096];
    let mut out = [0u8; 6144];
    let mut remainder = [0u8; 3];
    let mut rem_len: usize = 0;

    unsafe {
        loop {
            let n = libc::read(0, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                libc::exit(1);
            }
            if n == 0 {
                if rem_len > 0 {
                    let mut i = rem_len;
                    while i < 3 {
                        remainder[i] = 0;
                        i += 1;
                    }
                    let b0 = remainder[0] as u32;
                    let b1 = remainder[1] as u32;
                    let b2 = remainder[2] as u32;
                    let triple = (b0 << 16) | (b1 << 8) | b2;

                    let mut final_out = [0u8; 4];
                    final_out[0] = ENCODE_TABLE[((triple >> 18) & 0x3F) as usize];
                    final_out[1] = ENCODE_TABLE[((triple >> 12) & 0x3F) as usize];
                    if rem_len >= 2 {
                        final_out[2] = ENCODE_TABLE[((triple >> 6) & 0x3F) as usize];
                    } else {
                        final_out[2] = b'=';
                    }
                    final_out[3] = b'=';
                    write_all(1, &final_out);
                }
                write_all(1, b"\n");
                libc::exit(0);
            }

            let input = &buf[..n as usize];
            let mut pos = 0;
            let mut out_pos = 0;

            if rem_len > 0 {
                while rem_len < 3 && pos < input.len() {
                    remainder[rem_len] = input[pos];
                    rem_len += 1;
                    pos += 1;
                }
                if rem_len == 3 {
                    let b0 = remainder[0] as u32;
                    let b1 = remainder[1] as u32;
                    let b2 = remainder[2] as u32;
                    let triple = (b0 << 16) | (b1 << 8) | b2;
                    out[out_pos] = ENCODE_TABLE[((triple >> 18) & 0x3F) as usize];
                    out[out_pos + 1] = ENCODE_TABLE[((triple >> 12) & 0x3F) as usize];
                    out[out_pos + 2] = ENCODE_TABLE[((triple >> 6) & 0x3F) as usize];
                    out[out_pos + 3] = ENCODE_TABLE[(triple & 0x3F) as usize];
                    out_pos += 4;
                    rem_len = 0;
                }
            }

            while pos + 3 <= input.len() {
                let b0 = input[pos] as u32;
                let b1 = input[pos + 1] as u32;
                let b2 = input[pos + 2] as u32;
                let triple = (b0 << 16) | (b1 << 8) | b2;
                out[out_pos] = ENCODE_TABLE[((triple >> 18) & 0x3F) as usize];
                out[out_pos + 1] = ENCODE_TABLE[((triple >> 12) & 0x3F) as usize];
                out[out_pos + 2] = ENCODE_TABLE[((triple >> 6) & 0x3F) as usize];
                out[out_pos + 3] = ENCODE_TABLE[(triple & 0x3F) as usize];
                out_pos += 4;
                pos += 3;
            }

            while pos < input.len() {
                remainder[rem_len] = input[pos];
                rem_len += 1;
                pos += 1;
            }

            if out_pos > 0 {
                write_all(1, &out[..out_pos]);
            }
        }
    }
}

fn decode() {
    let mut buf = [0u8; 4096];
    let mut out = [0u8; 3072];
    let mut quad = [0u8; 4];
    let mut quad_len: usize = 0;

    unsafe {
        loop {
            let n = libc::read(0, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n < 0 {
                libc::exit(1);
            }
            if n == 0 {
                libc::exit(0);
            }

            let input = &buf[..n as usize];
            let mut pos = 0;
            let mut out_pos = 0;

            while pos < input.len() {
                let c = input[pos];
                pos += 1;

                if c == b'\n' || c == b'\r' || c == b' ' || c == b'\t' {
                    continue;
                }

                quad[quad_len] = c;
                quad_len += 1;

                if quad_len == 4 {
                    let a = DECODE_TABLE[quad[0] as usize] as u32;
                    let b = DECODE_TABLE[quad[1] as usize] as u32;
                    let c = DECODE_TABLE[quad[2] as usize] as u32;
                    let d = DECODE_TABLE[quad[3] as usize] as u32;
                    let triple = (a << 18) | (b << 12) | (c << 6) | d;

                    out[out_pos] = (triple >> 16) as u8;
                    out_pos += 1;
                    if quad[2] != b'=' {
                        out[out_pos] = (triple >> 8) as u8;
                        out_pos += 1;
                    }
                    if quad[3] != b'=' {
                        out[out_pos] = triple as u8;
                        out_pos += 1;
                    }
                    quad_len = 0;
                }
            }

            if out_pos > 0 {
                write_all(1, &out[..out_pos]);
            }
        }
    }
}

unsafe fn args_contain_d() -> bool {
    let mut buf = [0u8; 256];
    let fd = libc::open(
        b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
        libc::O_RDONLY,
    );
    if fd < 0 {
        return false;
    }
    let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
    libc::close(fd);
    if n <= 0 {
        return false;
    }

    let data = &buf[..n as usize];
    let mut i = 0;
    while i < data.len() && data[i] != 0 {
        i += 1;
    }
    i += 1;

    while i < data.len() {
        let start = i;
        while i < data.len() && data[i] != 0 {
            i += 1;
        }
        let arg = &data[start..i];
        if arg.len() == 2 && arg[0] == b'-' && arg[1] == b'd' {
            return true;
        }
        i += 1;
    }
    false
}

#[inline(never)]
fn run() {
    unsafe {
        if args_contain_d() {
            decode();
        } else {
            encode();
        }
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
