#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        libc::exit(1);
    }
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

unsafe fn seed_from_urandom() -> u64 {
    let fd = libc::open(
        b"/dev/urandom\0".as_ptr() as *const libc::c_char,
        libc::O_RDONLY,
    );
    if fd < 0 {
        libc::exit(1);
    }
    let mut seed: u64 = 0;
    let n = libc::read(
        fd,
        &mut seed as *mut u64 as *mut libc::c_void,
        core::mem::size_of::<u64>(),
    );
    libc::close(fd);
    if n != core::mem::size_of::<u64>() as isize {
        libc::exit(1);
    }
    if seed == 0 {
        seed = 0xdeadbeefcafe1234;
    }
    seed
}

unsafe fn write_u64_decimal(fd: i32, value: u64) {
    let mut buf = [0u8; 21];
    let mut pos = 20;
    buf[pos] = b'\n';

    if value == 0 {
        pos -= 1;
        buf[pos] = b'0';
    } else {
        let mut v = value;
        while v > 0 {
            pos -= 1;
            buf[pos] = b'0' + (v % 10) as u8;
            v /= 10;
        }
    }

    libc::write(
        fd,
        buf.as_ptr().add(pos) as *const libc::c_void,
        21 - pos,
    );
}

unsafe fn write_raw_bytes(fd: i32, state: &mut u64, count: usize) {
    let mut buf = [0u8; 4096];
    let mut remaining = count;

    while remaining > 0 {
        let chunk = if remaining < buf.len() {
            remaining
        } else {
            buf.len()
        };
        let mut i = 0;
        while i < chunk {
            let val = xorshift64(state);
            let bytes = val.to_le_bytes();
            let mut j = 0;
            while j < 8 && i < chunk {
                buf[i] = bytes[j];
                i += 1;
                j += 1;
            }
        }
        let mut written = 0;
        while written < chunk {
            let ret = libc::write(
                fd,
                buf.as_ptr().add(written) as *const libc::c_void,
                chunk - written,
            );
            if ret <= 0 {
                libc::exit(if ret == 0 { 1 } else { 0 });
            }
            written += ret as usize;
        }
        remaining -= chunk;
    }
}

unsafe fn parse_u64(s: &[u8]) -> u64 {
    let mut result: u64 = 0;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c < b'0' || c > b'9' {
            libc::exit(1);
        }
        result = result.wrapping_mul(10).wrapping_add((c - b'0') as u64);
        i += 1;
    }
    result
}

unsafe fn parse_args() -> (u64, bool) {
    let mut buf = [0u8; 512];
    let fd = libc::open(
        b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
        libc::O_RDONLY,
    );
    if fd < 0 {
        return (1, false);
    }
    let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
    libc::close(fd);
    if n <= 0 {
        return (1, false);
    }

    let data = &buf[..n as usize];
    let mut args: [&[u8]; 8] = [&[]; 8];
    let mut arg_count = 0;
    let mut i = 0;

    while i < data.len() && data[i] != 0 {
        i += 1;
    }
    i += 1;

    while i < data.len() && arg_count < 8 {
        let start = i;
        while i < data.len() && data[i] != 0 {
            i += 1;
        }
        if i > start {
            args[arg_count] = &data[start..i];
            arg_count += 1;
        }
        i += 1;
    }

    let mut count: u64 = 1;
    let mut raw = false;
    let mut j = 0;

    while j < arg_count {
        let arg = args[j];
        if arg.len() == 2 && arg[0] == b'-' && arg[1] == b'n' {
            if j + 1 < arg_count {
                count = parse_u64(args[j + 1]);
                j += 2;
                continue;
            }
        } else if arg.len() == 2 && arg[0] == b'-' && arg[1] == b'b' {
            if j + 1 < arg_count {
                count = parse_u64(args[j + 1]);
                raw = true;
                j += 2;
                continue;
            }
        }
        j += 1;
    }

    (count, raw)
}

#[inline(never)]
fn run() {
    unsafe {
        let mut state = seed_from_urandom();
        let (count, raw) = parse_args();

        if raw {
            write_raw_bytes(1, &mut state, count as usize);
        } else {
            let mut i: u64 = 0;
            while i < count {
                let val = xorshift64(&mut state);
                write_u64_decimal(1, val);
                i += 1;
            }
        }

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
