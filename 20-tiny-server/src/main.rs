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

const RESPONSE_BODY: &[u8] = b"<html><body><h1>Hello from tiny-server!</h1><p>A ~14KB HTTP server with no standard library.</p></body></html>";

const PORT: u16 = 9999;

#[inline(never)]
fn run() {
    unsafe {
        // Create socket
        let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if sock < 0 {
            write_all(2, b"tiny-server: socket() failed\n");
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
            sin_addr: libc::in_addr {
                s_addr: 0, // INADDR_ANY
            },
            sin_zero: [0; 8],
        };

        if libc::bind(
            sock,
            &addr as *const libc::sockaddr_in as *const libc::sockaddr,
            core::mem::size_of::<libc::sockaddr_in>() as u32,
        ) < 0
        {
            write_all(2, b"tiny-server: bind() failed\n");
            libc::exit(1);
        }

        // Listen
        if libc::listen(sock, 16) < 0 {
            write_all(2, b"tiny-server: listen() failed\n");
            libc::exit(1);
        }

        let mut num_buf = [0u8; 20];
        write_all(1, b"Listening on port ");
        let len = format_u64(PORT as u64, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"\n");

        let mut request_count: u64 = 0;

        loop {
            // Accept connection
            let client = libc::accept(sock, core::ptr::null_mut(), core::ptr::null_mut());
            if client < 0 {
                continue;
            }

            request_count += 1;

            // Read request (we don't parse it, just drain it)
            let mut req_buf = [0u8; 1024];
            libc::read(client, req_buf.as_mut_ptr() as *mut libc::c_void, req_buf.len());

            // Build HTTP response
            // Content-Length header
            let body_len = RESPONSE_BODY.len();
            let cl_len = format_u64(body_len as u64, &mut num_buf);

            let header = b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\nContent-Length: ";
            write_all(client, header);
            write_all(client, &num_buf[..cl_len]);
            write_all(client, b"\r\n\r\n");
            write_all(client, RESPONSE_BODY);

            libc::close(client);

            // Log to stdout
            write_all(1, b"[");
            let len = format_u64(request_count, &mut num_buf);
            write_all(1, &num_buf[..len]);
            write_all(1, b"] 200 OK\n");
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
