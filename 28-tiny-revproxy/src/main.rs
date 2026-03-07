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

/// Format an IPv4 address (network byte order) as "A.B.C.D"
fn format_ip(ip: u32, buf: &mut [u8; 16]) -> usize {
    let bytes = ip.to_ne_bytes();
    let mut pos = 0;
    let mut octet = 0;
    while octet < 4 {
        let mut num_buf = [0u8; 10];
        let len = format_u32(bytes[octet] as u32, &mut num_buf);
        let mut j = 0;
        while j < len {
            buf[pos] = num_buf[j];
            pos += 1;
            j += 1;
        }
        if octet < 3 {
            buf[pos] = b'.';
            pos += 1;
        }
        octet += 1;
    }
    pos
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

/// Find \r\n\r\n in buffer, return offset past it (start of body)
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

/// Extract the request line (first line) length including \r\n
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

/// Parse Content-Length value from headers
fn parse_content_length(buf: &[u8], header_end: usize) -> usize {
    // Search for "Content-Length: " (case-sensitive for simplicity)
    let needle = b"Content-Length: ";
    let mut i = 0;
    while i + needle.len() < header_end {
        let mut matched = true;
        let mut j = 0;
        while j < needle.len() {
            // Case-insensitive compare for the header name
            let a = if buf[i + j] >= b'A' && buf[i + j] <= b'Z' {
                buf[i + j] + 32
            } else {
                buf[i + j]
            };
            let b = if needle[j] >= b'A' && needle[j] <= b'Z' {
                needle[j] + 32
            } else {
                needle[j]
            };
            if a != b {
                matched = false;
                break;
            }
            j += 1;
        }
        if matched {
            // Parse the number
            let start = i + needle.len();
            let mut val: usize = 0;
            let mut k = start;
            while k < header_end && buf[k] >= b'0' && buf[k] <= b'9' {
                val = val * 10 + (buf[k] - b'0') as usize;
                k += 1;
            }
            return val;
        }
        i += 1;
    }
    0
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const LISTEN_PORT: u16 = 8888;
const BACKEND_ADDR: u32 = 0x7f000001; // 127.0.0.1
const BACKEND_PORT: u16 = 8080;

// ---------------------------------------------------------------------------
// Main logic
// ---------------------------------------------------------------------------

#[inline(never)]
fn run() {
    unsafe {
        let listen_fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if listen_fd < 0 {
            write_all(2, b"tiny-revproxy: socket() failed\n");
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
            write_all(2, b"tiny-revproxy: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(listen_fd, 16) < 0 {
            write_all(2, b"tiny-revproxy: listen() failed\n");
            libc::exit(1);
        }

        let mut num_buf = [0u8; 10];
        write_all(1, b"HTTP reverse proxy listening on port ");
        let len = format_u32(LISTEN_PORT as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" -> 127.0.0.1:");
        let len = format_u32(BACKEND_PORT as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"\n");

        let mut req_count: u32 = 0;

        loop {
            let mut client_addr: libc::sockaddr_in = core::mem::zeroed();
            let mut addr_len: libc::socklen_t =
                core::mem::size_of::<libc::sockaddr_in>() as u32;

            let client_fd = libc::accept(
                listen_fd,
                &mut client_addr as *mut _ as *mut libc::sockaddr,
                &mut addr_len,
            );
            if client_fd < 0 {
                continue;
            }

            req_count += 1;

            // Read HTTP request (headers + possibly body)
            let mut req_buf = [0u8; 8192];
            let mut req_len: usize = 0;

            // Read until we have complete headers
            let header_end;
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

            header_end = match find_header_end(&req_buf, req_len) {
                Some(end) => end,
                None => {
                    write_all(client_fd, b"HTTP/1.1 400 Bad Request\r\n\r\n");
                    libc::close(client_fd);
                    continue;
                }
            };

            // Log request line
            let rline_len = request_line_len(&req_buf, req_len);
            let mut ip_buf = [0u8; 16];
            let ip_len = format_ip(client_addr.sin_addr.s_addr, &mut ip_buf);

            write_all(1, b"[#");
            let len = format_u32(req_count, &mut num_buf);
            write_all(1, &num_buf[..len]);
            write_all(1, b"] ");
            write_all(1, &ip_buf[..ip_len]);
            write_all(1, b" ");
            write_all(1, &req_buf[..rline_len]);
            write_all(1, b"\n");

            // Read remaining body if Content-Length present
            let content_length = parse_content_length(&req_buf, header_end);
            let body_received = req_len - header_end;
            let body_remaining = if content_length > body_received {
                content_length - body_received
            } else {
                0
            };

            // Connect to backend
            let backend_fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
            if backend_fd < 0 {
                write_all(
                    client_fd,
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 15\r\n\r\n502 Bad Gateway",
                );
                libc::close(client_fd);
                continue;
            }

            let backend_addr = libc::sockaddr_in {
                sin_family: libc::AF_INET as u16,
                sin_port: BACKEND_PORT.to_be(),
                sin_addr: libc::in_addr {
                    s_addr: BACKEND_ADDR.to_be(),
                },
                sin_zero: [0; 8],
            };

            if libc::connect(
                backend_fd,
                &backend_addr as *const _ as *const libc::sockaddr,
                core::mem::size_of::<libc::sockaddr_in>() as u32,
            ) < 0
            {
                write_all(
                    client_fd,
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 15\r\n\r\n502 Bad Gateway",
                );
                libc::close(client_fd);
                libc::close(backend_fd);
                continue;
            }

            // Forward request headers to backend
            write_all(backend_fd, &req_buf[..header_end - 2]); // before final \r\n

            // Inject X-Forwarded-For header
            write_all(backend_fd, b"X-Forwarded-For: ");
            write_all(backend_fd, &ip_buf[..ip_len]);
            write_all(backend_fd, b"\r\n");
            write_all(backend_fd, b"\r\n"); // end of headers

            // Forward any body already received
            if body_received > 0 {
                write_all(backend_fd, &req_buf[header_end..req_len]);
            }

            // Forward remaining body from client
            if body_remaining > 0 {
                let mut remaining = body_remaining;
                let mut body_buf = [0u8; 4096];
                while remaining > 0 {
                    let to_read = if remaining < body_buf.len() {
                        remaining
                    } else {
                        body_buf.len()
                    };
                    let n = libc::read(
                        client_fd,
                        body_buf.as_mut_ptr() as *mut libc::c_void,
                        to_read,
                    );
                    if n <= 0 {
                        break;
                    }
                    write_all(backend_fd, &body_buf[..n as usize]);
                    remaining -= n as usize;
                }
            }

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
