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

// Extract basename from argv[0] via /proc/self/cmdline
unsafe fn get_argv0(buf: &mut [u8; 256]) -> usize {
    let fd = libc::open(
        b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
        libc::O_RDONLY,
    );
    if fd < 0 {
        return 0;
    }
    let mut cmdline = [0u8; 256];
    let n = libc::read(fd, cmdline.as_mut_ptr() as *mut libc::c_void, cmdline.len());
    libc::close(fd);
    if n <= 0 {
        return 0;
    }

    // Find end of argv[0]
    let mut end = 0;
    while end < n as usize && cmdline[end] != 0 {
        end += 1;
    }

    // Find last '/' for basename
    let mut last_slash = 0;
    let mut found_slash = false;
    let mut i = 0;
    while i < end {
        if cmdline[i] == b'/' {
            last_slash = i + 1;
            found_slash = true;
        }
        i += 1;
    }
    let start = if found_slash { last_slash } else { 0 };
    let len = end - start;

    i = 0;
    while i < len && i < buf.len() {
        buf[i] = cmdline[start + i];
        i += 1;
    }
    len
}

fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

// --- Applet: yes ---
fn applet_yes() -> ! {
    let msg = b"y\n";
    unsafe {
        loop {
            let ret = libc::write(1, msg.as_ptr() as *const libc::c_void, msg.len());
            if ret < 0 {
                libc::exit(0);
            }
        }
    }
}

// --- Applet: true ---
fn applet_true() -> ! {
    unsafe { libc::exit(0) }
}

// --- Applet: false ---
fn applet_false() -> ! {
    unsafe { libc::exit(1) }
}

// --- Applet: echo ---
fn applet_echo() -> ! {
    unsafe {
        let mut buf = [0u8; 4096];
        let fd = libc::open(
            b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
            libc::O_RDONLY,
        );
        if fd < 0 {
            libc::exit(1);
        }
        let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
        libc::close(fd);
        if n <= 0 {
            write_all(1, b"\n");
            libc::exit(0);
        }

        let data = &buf[..n as usize];

        // Skip argv[0]
        let mut i = 0;
        while i < data.len() && data[i] != 0 {
            i += 1;
        }
        i += 1;

        let mut first = true;
        while i < data.len() {
            let start = i;
            while i < data.len() && data[i] != 0 {
                i += 1;
            }
            if i > start {
                if !first {
                    write_all(1, b" ");
                }
                write_all(1, &data[start..i]);
                first = false;
            }
            i += 1;
        }
        write_all(1, b"\n");
        libc::exit(0);
    }
}

#[inline(never)]
fn run() {
    unsafe {
        let mut name_buf = [0u8; 256];
        let name_len = get_argv0(&mut name_buf);
        let name = &name_buf[..name_len];

        if bytes_eq(name, b"yes") {
            applet_yes();
        } else if bytes_eq(name, b"true") {
            applet_true();
        } else if bytes_eq(name, b"false") {
            applet_false();
        } else if bytes_eq(name, b"echo") {
            applet_echo();
        } else {
            // Default: print available applets
            write_all(1, b"tiny-multicall: BusyBox-style multi-call binary\n");
            write_all(1, b"Available applets: yes, true, false, echo\n");
            write_all(1, b"Create symlinks to invoke: ln -s tiny-multicall yes\n");
            libc::exit(0);
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
