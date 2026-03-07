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

fn format_u32(n: u32, buf: &mut [u8; 20]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut val = n;
    let mut pos = 20usize;
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

// Parse /proc/self/cmdline to extract arguments
struct Args {
    buf: [u8; 4096],
    len: usize,
}

impl Args {
    unsafe fn read() -> Self {
        let mut args = Args {
            buf: [0u8; 4096],
            len: 0,
        };
        let fd = libc::open(
            b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
            libc::O_RDONLY,
        );
        if fd >= 0 {
            let n = libc::read(fd, args.buf.as_mut_ptr() as *mut libc::c_void, args.buf.len());
            libc::close(fd);
            if n > 0 {
                args.len = n as usize;
            }
        }
        args
    }

    fn get(&self, n: usize) -> Option<&[u8]> {
        let mut idx = 0;
        let mut i = 0;
        let mut start = 0;
        while i < self.len {
            if self.buf[i] == 0 {
                if idx == n {
                    return Some(&self.buf[start..i]);
                }
                idx += 1;
                start = i + 1;
            }
            i += 1;
        }
        if idx == n && start < self.len {
            return Some(&self.buf[start..self.len]);
        }
        None
    }

    fn count(&self) -> usize {
        if self.len == 0 {
            return 0;
        }
        let mut count = 0;
        let mut i = 0;
        while i < self.len {
            if self.buf[i] == 0 {
                count += 1;
            }
            i += 1;
        }
        if self.buf[self.len - 1] != 0 {
            count += 1;
        }
        count
    }
}

// Copy bytes and null-terminate into a fixed buffer
fn make_cstr(src: &[u8], dst: &mut [u8]) -> bool {
    if src.len() >= dst.len() {
        return false;
    }
    let mut i = 0;
    while i < src.len() {
        dst[i] = src[i];
        i += 1;
    }
    dst[src.len()] = 0;
    true
}

#[inline(never)]
fn run() {
    unsafe {
        let args = Args::read();

        if args.count() < 3 {
            write_all(1, b"tiny-sandbox: chroot + privilege drop + execve\n\n");
            write_all(1, b"Usage: tiny-sandbox <chroot-dir> <uid> [command] [args...]\n\n");
            write_all(1, b"Steps performed:\n");
            write_all(1, b"  1. fork() child process\n");
            write_all(1, b"  2. chroot() to <chroot-dir>\n");
            write_all(1, b"  3. chdir() to /\n");
            write_all(1, b"  4. setgid(uid) + setuid(uid) to drop privileges\n");
            write_all(1, b"  5. execve() the command (default: /bin/sh)\n\n");
            write_all(1, b"Example: sudo ./tiny-sandbox /tmp/jail 1000 /bin/ls /\n\n");
            write_all(1, b"Note: chroot requires root privileges.\n");
            write_all(
                1,
                b"Critical ordering: chroot -> chdir -> setgid -> setuid -> execve\n",
            );
            write_all(
                1,
                b"(setgid before setuid, because after setuid we lose privilege to setgid)\n",
            );
            libc::exit(0);
        }

        let chroot_dir = args.get(1).unwrap_or(b"");
        let uid_str = args.get(2).unwrap_or(b"");

        // Parse UID
        let mut uid: u32 = 0;
        {
            let mut i = 0;
            while i < uid_str.len() {
                if uid_str[i] < b'0' || uid_str[i] > b'9' {
                    write_all(2, b"tiny-sandbox: invalid uid\n");
                    libc::exit(1);
                }
                uid = uid * 10 + (uid_str[i] - b'0') as u32;
                i += 1;
            }
        }

        // Determine command to exec (default: /bin/sh)
        let cmd = args.get(3).unwrap_or(b"/bin/sh");

        // Null-terminate chroot dir and command
        let mut chroot_cstr = [0u8; 256];
        if !make_cstr(chroot_dir, &mut chroot_cstr) {
            write_all(2, b"tiny-sandbox: chroot path too long\n");
            libc::exit(1);
        }

        let mut cmd_cstr = [0u8; 256];
        if !make_cstr(cmd, &mut cmd_cstr) {
            write_all(2, b"tiny-sandbox: command path too long\n");
            libc::exit(1);
        }

        // Build argv for execve: [cmd, extra_args..., NULL]
        let mut exec_argv: [*const libc::c_char; 16] = [core::ptr::null(); 16];
        exec_argv[0] = cmd_cstr.as_ptr() as *const libc::c_char;
        let mut arg_bufs = [[0u8; 256]; 14];
        let mut exec_argc = 1usize;
        let mut arg_idx = 4;
        while arg_idx < args.count() && exec_argc < 15 {
            if let Some(a) = args.get(arg_idx) {
                if make_cstr(a, &mut arg_bufs[exec_argc - 1]) {
                    exec_argv[exec_argc] =
                        arg_bufs[exec_argc - 1].as_ptr() as *const libc::c_char;
                    exec_argc += 1;
                }
            }
            arg_idx += 1;
        }
        // exec_argv is already null-terminated (initialized to null)

        // Empty environment
        let envp: [*const libc::c_char; 1] = [core::ptr::null()];

        write_all(1, b"[sandbox] chroot=");
        write_all(1, chroot_dir);
        write_all(1, b" uid=");
        let mut num_buf = [0u8; 20];
        let len = format_u32(uid, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" cmd=");
        write_all(1, cmd);
        write_all(1, b"\n");

        let pid = libc::fork();
        if pid < 0 {
            write_all(2, b"tiny-sandbox: fork() failed\n");
            libc::exit(1);
        }

        if pid == 0 {
            // Child: chroot -> chdir -> setgid -> setuid -> execve
            if libc::chroot(chroot_cstr.as_ptr() as *const libc::c_char) < 0 {
                write_all(2, b"tiny-sandbox: chroot() failed (need root?)\n");
                libc::exit(1);
            }

            if libc::chdir(b"/\0".as_ptr() as *const libc::c_char) < 0 {
                write_all(2, b"tiny-sandbox: chdir() failed\n");
                libc::exit(1);
            }

            // setgid before setuid (we lose privilege to setgid after setuid)
            if libc::setgid(uid) < 0 {
                write_all(2, b"tiny-sandbox: setgid() failed\n");
                libc::exit(1);
            }

            if libc::setuid(uid) < 0 {
                write_all(2, b"tiny-sandbox: setuid() failed\n");
                libc::exit(1);
            }

            libc::execve(
                cmd_cstr.as_ptr() as *const libc::c_char,
                exec_argv.as_ptr(),
                envp.as_ptr(),
            );

            // execve only returns on error
            write_all(2, b"tiny-sandbox: execve() failed\n");
            libc::exit(1);
        } else {
            // Parent: wait for child
            let mut status: i32 = 0;
            libc::waitpid(pid, &mut status, 0);

            if (status & 0x7f) == 0 {
                let exit_code = (status >> 8) & 0xff;
                write_all(1, b"[sandbox] child exited with status ");
                let len = format_u32(exit_code as u32, &mut num_buf);
                write_all(1, &num_buf[..len]);
                write_all(1, b"\n");
            } else {
                let sig = status & 0x7f;
                write_all(1, b"[sandbox] child killed by signal ");
                let len = format_u32(sig as u32, &mut num_buf);
                write_all(1, &num_buf[..len]);
                write_all(1, b"\n");
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
