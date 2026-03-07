#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};

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
            return;
        }
        written += ret as usize;
    }
}

static GOT_SIGINT: AtomicBool = AtomicBool::new(false);
static GOT_SIGUSR1: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigint(_sig: libc::c_int) {
    GOT_SIGINT.store(true, Ordering::SeqCst);
}

extern "C" fn handle_sigusr1(_sig: libc::c_int) {
    GOT_SIGUSR1.store(true, Ordering::SeqCst);
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

#[inline(never)]
fn run() {
    unsafe {
        // Install SIGINT handler (Ctrl+C)
        let mut sa: libc::sigaction = core::mem::zeroed();
        sa.sa_sigaction = handle_sigint as usize;
        libc::sigemptyset(&mut sa.sa_mask);
        sa.sa_flags = 0;

        if libc::sigaction(libc::SIGINT, &sa, core::ptr::null_mut()) != 0 {
            write_all(2, b"Failed to install SIGINT handler\n");
            libc::exit(1);
        }

        // Install SIGUSR1 handler
        sa.sa_sigaction = handle_sigusr1 as usize;
        if libc::sigaction(libc::SIGUSR1, &sa, core::ptr::null_mut()) != 0 {
            write_all(2, b"Failed to install SIGUSR1 handler\n");
            libc::exit(1);
        }

        // Print PID so user can send signals
        let pid = libc::getpid();
        let mut num_buf = [0u8; 20];
        write_all(1, b"PID: ");
        let len = format_u64(pid as u64, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"\n");

        write_all(1, b"Waiting for signals... (Ctrl+C to quit, kill -USR1 <pid> to ping)\n");

        let mut sigusr1_count: u64 = 0;

        loop {
            // pause() sleeps until a signal is delivered
            libc::pause();

            if GOT_SIGUSR1.swap(false, Ordering::SeqCst) {
                sigusr1_count += 1;
                write_all(1, b"Caught SIGUSR1 (#");
                let len = format_u64(sigusr1_count, &mut num_buf);
                write_all(1, &num_buf[..len]);
                write_all(1, b")\n");
            }

            if GOT_SIGINT.swap(false, Ordering::SeqCst) {
                write_all(1, b"\nCaught SIGINT, exiting gracefully.\n");
                write_all(1, b"Total SIGUSR1 received: ");
                let len = format_u64(sigusr1_count, &mut num_buf);
                write_all(1, &num_buf[..len]);
                write_all(1, b"\n");
                libc::exit(0);
            }
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
