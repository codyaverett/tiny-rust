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
    let len = 20 - pos;
    let mut i = 0;
    while i < len {
        buf[i] = buf[pos + i];
        i += 1;
    }
    len
}

/// Get file path from argv[1] via /proc/self/cmdline
unsafe fn get_argv1(buf: &mut [u8; 256]) -> usize {
    let mut cmdline = [0u8; 4096];
    let fd = libc::open(
        b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
        libc::O_RDONLY,
    );
    if fd < 0 {
        return 0;
    }
    let n = libc::read(fd, cmdline.as_mut_ptr() as *mut libc::c_void, cmdline.len());
    libc::close(fd);
    if n <= 0 {
        return 0;
    }

    let data = &cmdline[..n as usize];

    // Skip argv[0]
    let mut i = 0;
    while i < data.len() && data[i] != 0 {
        i += 1;
    }
    i += 1;

    // Read argv[1]
    let start = i;
    while i < data.len() && data[i] != 0 {
        i += 1;
    }
    let len = i - start;
    if len == 0 || len >= buf.len() {
        return 0;
    }

    let mut j = 0;
    while j < len {
        buf[j] = data[start + j];
        j += 1;
    }
    len
}

#[inline(never)]
fn run() {
    unsafe {
        let mut path_buf = [0u8; 256];
        let path_len = get_argv1(&mut path_buf);
        if path_len == 0 {
            write_all(2, b"Usage: tiny-mmap <file>\n");
            libc::exit(1);
        }
        // Null-terminate
        path_buf[path_len] = 0;

        // Open the file
        let fd = libc::open(
            path_buf.as_ptr() as *const libc::c_char,
            libc::O_RDONLY,
        );
        if fd < 0 {
            write_all(2, b"tiny-mmap: cannot open file\n");
            libc::exit(1);
        }

        // Get file size via lseek
        let size = libc::lseek(fd, 0, libc::SEEK_END);
        if size < 0 {
            write_all(2, b"tiny-mmap: cannot determine file size\n");
            libc::close(fd);
            libc::exit(1);
        }
        if size == 0 {
            libc::close(fd);
            libc::exit(0);
        }

        // Memory-map the file
        let ptr = libc::mmap(
            core::ptr::null_mut(),
            size as usize,
            libc::PROT_READ,
            libc::MAP_PRIVATE,
            fd,
            0,
        );
        libc::close(fd);

        if ptr == libc::MAP_FAILED {
            write_all(2, b"tiny-mmap: mmap failed\n");
            libc::exit(1);
        }

        // Print file info
        let mut num_buf = [0u8; 20];
        write_all(1, b"File: ");
        write_all(1, &path_buf[..path_len]);
        write_all(1, b"\nSize: ");
        let len = format_u64(size as u64, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" bytes (memory-mapped)\n\n");

        // Write file contents to stdout via the mapping (zero-copy)
        let data = core::slice::from_raw_parts(ptr as *const u8, size as usize);
        write_all(1, data);

        // Clean up
        libc::munmap(ptr, size as usize);
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
