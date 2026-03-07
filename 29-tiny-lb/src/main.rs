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

/// Find \r\n\r\n in buffer
fn find_header_end(buf: &[u8], len: usize) -> Option<usize> {
    if len < 4 {
        return None;
    }
    let mut i = 0;
    while i + 3 < len {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' && buf[i + 2] == b'\r' && buf[i + 3] == b'\n' {
            return Some(i + 4);
        }
        i += 1;
    }
    None
}

/// Extract request line length (up to first \r\n)
fn request_line_len(buf: &[u8], len: usize) -> usize {
    let mut i = 0;
    while i + 1 < len {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            return i;
        }
        i += 1;
    }
    len
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const LISTEN_PORT: u16 = 9001;
const NUM_BACKENDS: usize = 3;
const BACKEND_PORTS: [u16; NUM_BACKENDS] = [8081, 8082, 8083];
const BACKEND_ADDR: u32 = 0x7f000001; // 127.0.0.1

// ---------------------------------------------------------------------------
// Main logic
// ---------------------------------------------------------------------------

#[inline(never)]
fn run() {
    unsafe {
        let listen_fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if listen_fd < 0 {
            write_all(2, b"tiny-lb: socket() failed\n");
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
            write_all(2, b"tiny-lb: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(listen_fd, 16) < 0 {
            write_all(2, b"tiny-lb: listen() failed\n");
            libc::exit(1);
        }

        let mut num_buf = [0u8; 10];
        write_all(1, b"Round-robin LB on port ");
        let len = format_u32(LISTEN_PORT as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" -> backends ");
        let mut b = 0;
        while b < NUM_BACKENDS {
            write_all(1, b":");
            let len = format_u32(BACKEND_PORTS[b] as u32, &mut num_buf);
            write_all(1, &num_buf[..len]);
            if b + 1 < NUM_BACKENDS {
                write_all(1, b" ");
            }
            b += 1;
        }
        write_all(1, b"\n");

        let mut rr_index: usize = 0;
        let mut req_count: u32 = 0;

        loop {
            let client_fd =
                libc::accept(listen_fd, core::ptr::null_mut(), core::ptr::null_mut());
            if client_fd < 0 {
                continue;
            }

            req_count += 1;

            // Read HTTP request headers
            let mut req_buf = [0u8; 8192];
            let mut req_len: usize = 0;
            loop {
                if req_len >= req_buf.len() {
                    break;
                }
                let n = libc::read(
                    client_fd,
                    req_buf.as_mut_ptr().add(req_len) as *mut libc::c_void,
                    req_buf.len() - req_len,
                );
                if n <= 0 {
                    break;
                }
                req_len += n as usize;
                if find_header_end(&req_buf, req_len).is_some() {
                    break;
                }
            }

            if find_header_end(&req_buf, req_len).is_none() {
                write_all(client_fd, b"HTTP/1.1 400 Bad Request\r\n\r\n");
                libc::close(client_fd);
                continue;
            }

            let rline_len = request_line_len(&req_buf, req_len);

            // Try backends in round-robin order, skip unavailable ones
            let mut backend_fd: i32 = -1;
            let mut chosen: usize = 0;
            let mut attempts = 0;

            while attempts < NUM_BACKENDS {
                chosen = (rr_index + attempts) % NUM_BACKENDS;

                let bfd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
                if bfd < 0 {
                    attempts += 1;
                    continue;
                }

                let backend_addr = libc::sockaddr_in {
                    sin_family: libc::AF_INET as u16,
                    sin_port: BACKEND_PORTS[chosen].to_be(),
                    sin_addr: libc::in_addr {
                        s_addr: BACKEND_ADDR.to_be(),
                    },
                    sin_zero: [0; 8],
                };

                if libc::connect(
                    bfd,
                    &backend_addr as *const _ as *const libc::sockaddr,
                    core::mem::size_of::<libc::sockaddr_in>() as u32,
                ) < 0
                {
                    libc::close(bfd);
                    attempts += 1;
                    continue;
                }

                backend_fd = bfd;
                break;
            }

            // Advance round-robin index regardless of which backend was used
            rr_index = (rr_index + 1) % NUM_BACKENDS;

            if backend_fd < 0 {
                write_all(1, b"[#");
                let len = format_u32(req_count, &mut num_buf);
                write_all(1, &num_buf[..len]);
                write_all(1, b"] all backends down\n");
                write_all(
                    client_fd,
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 22\r\n\r\nAll backends are down.",
                );
                libc::close(client_fd);
                continue;
            }

            // Log
            write_all(1, b"[#");
            let len = format_u32(req_count, &mut num_buf);
            write_all(1, &num_buf[..len]);
            write_all(1, b"] -> :");
            let len = format_u32(BACKEND_PORTS[chosen] as u32, &mut num_buf);
            write_all(1, &num_buf[..len]);
            write_all(1, b"  ");
            write_all(1, &req_buf[..rline_len]);
            write_all(1, b"\n");

            // Forward request to backend
            write_all(backend_fd, &req_buf[..req_len]);

            // Relay response from backend to client
            let mut relay_buf = [0u8; 4096];
            loop {
                let n = libc::read(
                    backend_fd,
                    relay_buf.as_mut_ptr() as *mut libc::c_void,
                    relay_buf.len(),
                );
                if n <= 0 {
                    break;
                }
                write_all(client_fd, &relay_buf[..n as usize]);
            }

            libc::close(backend_fd);
            libc::close(client_fd);
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
