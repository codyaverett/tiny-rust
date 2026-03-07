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
            return;
        }
        written += ret as usize;
    }
}

fn format_i32(n: i32, buf: &mut [u8; 20]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let negative = n < 0;
    let mut val = if negative { -(n as i64) } else { n as i64 };
    let mut pos = 20;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    if negative {
        pos -= 1;
        buf[pos] = b'-';
    }
    let len = 20 - pos;
    let mut i = 0;
    while i < len {
        buf[i] = buf[pos + i];
        i += 1;
    }
    len
}

const MESSAGES: [&[u8]; 3] = [
    b"Hello from the child process!\n",
    b"Pipes are a Unix IPC mechanism.\n",
    b"This is the last message. Goodbye!\n",
];

#[inline(never)]
fn run() {
    unsafe {
        let mut pipefd: [i32; 2] = [0; 2];

        // Create pipe: pipefd[0] = read end, pipefd[1] = write end
        if libc::pipe(pipefd.as_mut_ptr()) < 0 {
            write_all(2, b"tiny-pipe: pipe() failed\n");
            libc::exit(1);
        }

        let pid = libc::fork();
        if pid < 0 {
            write_all(2, b"tiny-pipe: fork() failed\n");
            libc::exit(1);
        }

        let mut num_buf = [0u8; 20];

        if pid == 0 {
            // Child: close read end, write messages
            libc::close(pipefd[0]);

            write_all(1, b"[child  pid=");
            let len = format_i32(libc::getpid(), &mut num_buf);
            write_all(1, &num_buf[..len]);
            write_all(1, b"] sending ");

            let mut msg_count = [0u8; 4];
            msg_count[0] = b'0' + MESSAGES.len() as u8;
            write_all(1, &msg_count[..1]);
            write_all(1, b" messages through pipe\n");

            let mut i = 0;
            while i < MESSAGES.len() {
                write_all(pipefd[1], MESSAGES[i]);
                i += 1;
            }

            libc::close(pipefd[1]);
            libc::exit(0);
        } else {
            // Parent: close write end, read messages
            libc::close(pipefd[1]);

            write_all(1, b"[parent pid=");
            let len = format_i32(libc::getpid(), &mut num_buf);
            write_all(1, &num_buf[..len]);
            write_all(1, b"] forked child pid=");
            let len = format_i32(pid, &mut num_buf);
            write_all(1, &num_buf[..len]);
            write_all(1, b"\n");

            write_all(1, b"[parent] reading from pipe:\n");

            let mut buf = [0u8; 256];
            loop {
                let n = libc::read(pipefd[0], buf.as_mut_ptr() as *mut libc::c_void, buf.len());
                if n <= 0 {
                    break;
                }
                write_all(1, b"  > ");
                write_all(1, &buf[..n as usize]);
            }
            libc::close(pipefd[0]);

            // Wait for child
            let mut status: i32 = 0;
            libc::waitpid(pid, &mut status, 0);

            // WIFEXITED: (status & 0x7f) == 0
            // WEXITSTATUS: (status >> 8) & 0xff
            if (status & 0x7f) == 0 {
                let exit_code = (status >> 8) & 0xff;
                write_all(1, b"[parent] child exited with status ");
                let len = format_i32(exit_code, &mut num_buf);
                write_all(1, &num_buf[..len]);
                write_all(1, b"\n");
            } else {
                write_all(1, b"[parent] child terminated abnormally\n");
            }
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
