#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        libc::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

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

fn format_u32(n: u32, buf: &mut [u8; 10]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut val = n;
    let mut pos = 10;
    while val > 0 {
        pos -= 1;
        buf[pos] = b'0' + (val % 10) as u8;
        val /= 10;
    }
    let len = 10 - pos;
    let mut i = 0;
    while i < len {
        buf[i] = buf[pos + i];
        i += 1;
    }
    len
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const LISTEN_PORT: u16 = 9000;
const TARGET_ADDR: u32 = 0x7f000001; // 127.0.0.1
const TARGET_PORT: u16 = 8080;

// ---------------------------------------------------------------------------
// Main logic
// ---------------------------------------------------------------------------

#[inline(never)]
fn run() {
    unsafe {
        let listen_fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if listen_fd < 0 {
            write_all(2, b"tiny-proxy: socket() failed\n");
            libc::exit(1);
        }

        let optval: i32 = 1;
        libc::setsockopt(
            listen_fd,
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            &optval as *const i32 as *const libc::c_void,
            4,
        );

        let addr = libc::sockaddr_in {
            sin_family: libc::AF_INET as u16,
            sin_port: LISTEN_PORT.to_be(),
            sin_addr: libc::in_addr { s_addr: 0 },
            sin_zero: [0; 8],
        };

        if libc::bind(
            listen_fd,
            &addr as *const _ as *const libc::sockaddr,
            core::mem::size_of::<libc::sockaddr_in>() as u32,
        ) < 0
        {
            write_all(2, b"tiny-proxy: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(listen_fd, 8) < 0 {
            write_all(2, b"tiny-proxy: listen() failed\n");
            libc::exit(1);
        }

        let mut num_buf = [0u8; 10];
        write_all(1, b"TCP proxy listening on port ");
        let len = format_u32(LISTEN_PORT as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" -> 127.0.0.1:");
        let len = format_u32(TARGET_PORT as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"\n");

        let mut conn_count: u32 = 0;

        loop {
            let client_fd =
                libc::accept(listen_fd, core::ptr::null_mut(), core::ptr::null_mut());
            if client_fd < 0 {
                continue;
            }

            conn_count += 1;
            write_all(1, b"[conn #");
            let len = format_u32(conn_count, &mut num_buf);
            write_all(1, &num_buf[..len]);
            write_all(1, b"] accepted\n");

            // Connect to target
            let target_fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
            if target_fd < 0 {
                write_all(2, b"  target socket() failed\n");
                libc::close(client_fd);
                continue;
            }

            let target_addr = libc::sockaddr_in {
                sin_family: libc::AF_INET as u16,
                sin_port: TARGET_PORT.to_be(),
                sin_addr: libc::in_addr {
                    s_addr: TARGET_ADDR.to_be(),
                },
                sin_zero: [0; 8],
            };

            if libc::connect(
                target_fd,
                &target_addr as *const _ as *const libc::sockaddr,
                core::mem::size_of::<libc::sockaddr_in>() as u32,
            ) < 0
            {
                write_all(2, b"  connect to target failed\n");
                libc::close(client_fd);
                libc::close(target_fd);
                continue;
            }

            write_all(1, b"  connected to target\n");

            // Bidirectional forwarding with poll()
            let mut buf = [0u8; 4096];
            let mut fds = [
                libc::pollfd {
                    fd: client_fd,
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: target_fd,
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];

            loop {
                fds[0].revents = 0;
                fds[1].revents = 0;
                let ret = libc::poll(fds.as_mut_ptr(), 2, 30000); // 30s timeout
                if ret <= 0 {
                    break; // timeout or error
                }

                // Client -> Target
                if fds[0].revents & libc::POLLIN != 0 {
                    let n = libc::read(
                        client_fd,
                        buf.as_mut_ptr() as *mut libc::c_void,
                        buf.len(),
                    );
                    if n <= 0 {
                        break;
                    }
                    write_all(target_fd, &buf[..n as usize]);
                }

                // Target -> Client
                if fds[1].revents & libc::POLLIN != 0 {
                    let n = libc::read(
                        target_fd,
                        buf.as_mut_ptr() as *mut libc::c_void,
                        buf.len(),
                    );
                    if n <= 0 {
                        break;
                    }
                    write_all(client_fd, &buf[..n as usize]);
                }

                // Error on either fd
                if fds[0].revents & libc::POLLERR != 0
                    || fds[1].revents & libc::POLLERR != 0
                {
                    break;
                }
            }

            libc::close(client_fd);
            libc::close(target_fd);

            write_all(1, b"[conn #");
            let len = format_u32(conn_count, &mut num_buf);
            write_all(1, &num_buf[..len]);
            write_all(1, b"] closed\n");
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
