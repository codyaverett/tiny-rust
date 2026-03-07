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

unsafe fn cat_fd(fd: i32) {
    let mut buf = [0u8; 4096];
    loop {
        let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
        if n < 0 {
            libc::exit(1);
        }
        if n == 0 {
            break;
        }
        write_all(1, &buf[..n as usize]);
    }
}

unsafe fn get_args() -> (*const *const u8, usize) {
    // Read argc and argv from the stack via /proc/self/cmdline
    let mut buf = [0u8; 4096];
    let fd = libc::open(
        b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
        libc::O_RDONLY,
    );
    if fd < 0 {
        return (core::ptr::null(), 0);
    }
    let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
    libc::close(fd);
    if n <= 0 {
        return (core::ptr::null(), 0);
    }

    // Count args (null-separated in cmdline)
    let data = &buf[..n as usize];
    let mut count = 0usize;
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0 {
            count += 1;
        }
        i += 1;
    }
    // If last byte wasn't null, count the trailing arg
    if data[data.len() - 1] != 0 {
        count += 1;
    }

    (core::ptr::null(), count)
}

#[inline(never)]
fn run() {
    unsafe {
        let (_, argc) = get_args();

        if argc <= 1 {
            // No file arguments: read stdin
            cat_fd(0);
            libc::exit(0);
        }

        // Re-parse cmdline to get file paths
        let mut cmdline = [0u8; 4096];
        let fd = libc::open(
            b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
            libc::O_RDONLY,
        );
        if fd < 0 {
            libc::exit(1);
        }
        let n = libc::read(fd, cmdline.as_mut_ptr() as *mut libc::c_void, cmdline.len());
        libc::close(fd);
        if n <= 0 {
            libc::exit(1);
        }

        let data = &cmdline[..n as usize];

        // Skip argv[0]
        let mut i = 0;
        while i < data.len() && data[i] != 0 {
            i += 1;
        }
        i += 1; // skip null

        // Process each file argument
        while i < data.len() {
            let start = i;
            while i < data.len() && data[i] != 0 {
                i += 1;
            }

            if i == start {
                i += 1;
                continue;
            }

            let arg = &data[start..i];

            // "-" means stdin
            if arg.len() == 1 && arg[0] == b'-' {
                cat_fd(0);
            } else {
                // Need null-terminated path for open()
                let mut path = [0u8; 256];
                if arg.len() >= path.len() {
                    libc::exit(1);
                }
                let mut j = 0;
                while j < arg.len() {
                    path[j] = arg[j];
                    j += 1;
                }
                path[arg.len()] = 0;

                let file_fd = libc::open(
                    path.as_ptr() as *const libc::c_char,
                    libc::O_RDONLY,
                );
                if file_fd < 0 {
                    // Write error message to stderr
                    let prefix = b"tiny-cat: ";
                    let suffix = b": No such file or directory\n";
                    write_all(2, prefix);
                    write_all(2, arg);
                    write_all(2, suffix);
                    libc::exit(1);
                }
                cat_fd(file_fd);
                libc::close(file_fd);
            }

            i += 1; // skip null separator
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
