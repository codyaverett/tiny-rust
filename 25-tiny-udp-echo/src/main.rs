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

fn str_eq(a: &[u8], b: &[u8]) -> bool {
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

// Trim trailing whitespace/newlines
fn trim_end(s: &[u8]) -> &[u8] {
    let mut len = s.len();
    while len > 0 && (s[len - 1] == b'\n' || s[len - 1] == b'\r' || s[len - 1] == b' ') {
        len -= 1;
    }
    &s[..len]
}

const PORT: u16 = 9998;

#[inline(never)]
fn run() {
    unsafe {
        // Create UDP socket
        let sock = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);
        if sock < 0 {
            write_all(2, b"tiny-udp-echo: socket() failed\n");
            libc::exit(1);
        }

        // SO_REUSEADDR
        let optval: i32 = 1;
        libc::setsockopt(
            sock,
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            &optval as *const i32 as *const libc::c_void,
            4,
        );

        // Bind
        let addr = libc::sockaddr_in {
            sin_family: libc::AF_INET as u16,
            sin_port: PORT.to_be(),
            sin_addr: libc::in_addr { s_addr: 0 },
            sin_zero: [0; 8],
        };

        if libc::bind(
            sock,
            &addr as *const libc::sockaddr_in as *const libc::sockaddr,
            core::mem::size_of::<libc::sockaddr_in>() as u32,
        ) < 0
        {
            write_all(2, b"tiny-udp-echo: bind() failed\n");
            libc::exit(1);
        }

        // Create epoll instance
        let epfd = libc::epoll_create1(0);
        if epfd < 0 {
            write_all(2, b"tiny-udp-echo: epoll_create1() failed\n");
            libc::exit(1);
        }

        // Add UDP socket to epoll
        let mut ev = libc::epoll_event {
            events: libc::EPOLLIN as u32,
            u64: sock as u64,
        };
        if libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, sock, &mut ev) < 0 {
            write_all(2, b"tiny-udp-echo: epoll_ctl(sock) failed\n");
            libc::exit(1);
        }

        // Add stdin to epoll
        let mut ev_stdin = libc::epoll_event {
            events: libc::EPOLLIN as u32,
            u64: 0, // stdin fd = 0
        };
        if libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, 0, &mut ev_stdin) < 0 {
            write_all(2, b"tiny-udp-echo: epoll_ctl(stdin) failed\n");
            libc::exit(1);
        }

        let mut num_buf = [0u8; 20];
        write_all(1, b"UDP echo server listening on port ");
        let len = format_u64(PORT as u64, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"\n");
        write_all(1, b"Commands on stdin: stats, quit\n");

        let mut echo_count: u64 = 0;
        let mut total_bytes: u64 = 0;

        let mut events: [libc::epoll_event; 4] = [libc::epoll_event { events: 0, u64: 0 }; 4];

        loop {
            let nfds = libc::epoll_wait(epfd, events.as_mut_ptr(), 4, -1);
            if nfds < 0 {
                continue;
            }

            let mut i = 0;
            while i < nfds as usize {
                let fd = events[i].u64 as i32;

                if fd == 0 {
                    // Stdin ready - read command
                    let mut cmd_buf = [0u8; 64];
                    let n = libc::read(0, cmd_buf.as_mut_ptr() as *mut libc::c_void, cmd_buf.len());
                    if n <= 0 {
                        // stdin closed, remove from epoll
                        libc::epoll_ctl(epfd, libc::EPOLL_CTL_DEL, 0, core::ptr::null_mut());
                    } else {
                        let cmd = trim_end(&cmd_buf[..n as usize]);
                        if str_eq(cmd, b"quit") {
                            write_all(1, b"Shutting down.\n");
                            libc::close(sock);
                            libc::close(epfd);
                            libc::exit(0);
                        } else if str_eq(cmd, b"stats") {
                            write_all(1, b"Echoed ");
                            let len = format_u64(echo_count, &mut num_buf);
                            write_all(1, &num_buf[..len]);
                            write_all(1, b" datagrams, ");
                            let len = format_u64(total_bytes, &mut num_buf);
                            write_all(1, &num_buf[..len]);
                            write_all(1, b" bytes total\n");
                        } else {
                            write_all(1, b"Unknown command. Try: stats, quit\n");
                        }
                    }
                } else if fd == sock {
                    // UDP data ready - recvfrom and echo back with sendto
                    let mut buf = [0u8; 1500];
                    let mut src_addr: libc::sockaddr_in = core::mem::zeroed();
                    let mut addr_len: libc::socklen_t =
                        core::mem::size_of::<libc::sockaddr_in>() as u32;

                    let n = libc::recvfrom(
                        sock,
                        buf.as_mut_ptr() as *mut libc::c_void,
                        buf.len(),
                        0,
                        &mut src_addr as *mut libc::sockaddr_in as *mut libc::sockaddr,
                        &mut addr_len,
                    );

                    if n > 0 {
                        // Echo back
                        libc::sendto(
                            sock,
                            buf.as_ptr() as *const libc::c_void,
                            n as usize,
                            0,
                            &src_addr as *const libc::sockaddr_in as *const libc::sockaddr,
                            addr_len,
                        );

                        echo_count += 1;
                        total_bytes += n as u64;

                        // Log
                        write_all(1, b"[echo] ");
                        let len = format_u64(n as u64, &mut num_buf);
                        write_all(1, &num_buf[..len]);
                        write_all(1, b" bytes\n");
                    }
                }

                i += 1;
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
