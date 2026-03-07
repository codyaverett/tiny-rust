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

// Parse "a.b.c.d" into network-byte-order u32
fn parse_ipv4(s: &[u8]) -> Option<u32> {
    let mut octets = [0u32; 4];
    let mut octet_idx = 0;
    let mut current: u32 = 0;
    let mut has_digit = false;

    let mut i = 0;
    while i < s.len() {
        if s[i] == b'.' {
            if !has_digit || octet_idx >= 3 || current > 255 {
                return None;
            }
            octets[octet_idx] = current;
            octet_idx += 1;
            current = 0;
            has_digit = false;
        } else if s[i] >= b'0' && s[i] <= b'9' {
            current = current * 10 + (s[i] - b'0') as u32;
            has_digit = true;
        } else {
            return None;
        }
        i += 1;
    }

    if !has_digit || octet_idx != 3 || current > 255 {
        return None;
    }
    octets[3] = current;

    // Return in network byte order (big-endian on the wire)
    Some(
        ((octets[0] as u32) << 24)
            | ((octets[1] as u32) << 16)
            | ((octets[2] as u32) << 8)
            | (octets[3] as u32),
    )
}

fn parse_u16(s: &[u8]) -> Option<u16> {
    if s.is_empty() {
        return None;
    }
    let mut val: u32 = 0;
    let mut i = 0;
    while i < s.len() {
        if s[i] < b'0' || s[i] > b'9' {
            return None;
        }
        val = val * 10 + (s[i] - b'0') as u32;
        if val > 65535 {
            return None;
        }
        i += 1;
    }
    Some(val as u16)
}

const TIMEOUT_MS: i32 = 200;

unsafe fn scan_port(ip_net: u32, port: u16) -> bool {
    let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
    if sock < 0 {
        return false;
    }

    // Set non-blocking
    let flags = libc::fcntl(sock, libc::F_GETFL);
    libc::fcntl(sock, libc::F_SETFL, flags | libc::O_NONBLOCK);

    let addr = libc::sockaddr_in {
        sin_family: libc::AF_INET as u16,
        sin_port: port.to_be(),
        sin_addr: libc::in_addr {
            s_addr: ip_net.to_be(),
        },
        sin_zero: [0; 8],
    };

    let ret = libc::connect(
        sock,
        &addr as *const libc::sockaddr_in as *const libc::sockaddr,
        core::mem::size_of::<libc::sockaddr_in>() as u32,
    );

    let open = if ret == 0 {
        true
    } else if *libc::__errno_location() == libc::EINPROGRESS {
        let mut pfd = libc::pollfd {
            fd: sock,
            events: libc::POLLOUT,
            revents: 0,
        };
        let poll_ret = libc::poll(&mut pfd, 1, TIMEOUT_MS);
        if poll_ret > 0 && (pfd.revents & libc::POLLOUT) != 0 {
            let mut err: i32 = 0;
            let mut err_len: u32 = 4;
            libc::getsockopt(
                sock,
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                &mut err as *mut i32 as *mut libc::c_void,
                &mut err_len,
            );
            err == 0
        } else {
            false
        }
    } else {
        false
    };

    libc::close(sock);
    open
}

// Parse /proc/self/cmdline to extract arguments
struct Args {
    buf: [u8; 1024],
    len: usize,
}

impl Args {
    unsafe fn read() -> Self {
        let mut args = Args {
            buf: [0u8; 1024],
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

    // Get the nth argument (0-indexed, argv[0] is the program name)
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
        // Handle last arg without trailing null
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

#[inline(never)]
fn run() {
    unsafe {
        let args = Args::read();
        if args.count() < 4 {
            write_all(2, b"Usage: tiny-portscan <ip> <start-port> <end-port>\n");
            write_all(2, b"Example: tiny-portscan 127.0.0.1 9990 10000\n");
            libc::exit(1);
        }

        let ip_str = args.get(1).unwrap_or(b"");
        let start_str = args.get(2).unwrap_or(b"");
        let end_str = args.get(3).unwrap_or(b"");

        let ip_net = match parse_ipv4(ip_str) {
            Some(ip) => ip,
            None => {
                write_all(2, b"tiny-portscan: invalid IP address\n");
                libc::exit(1);
            }
        };

        let start_port = match parse_u16(start_str) {
            Some(p) => p,
            None => {
                write_all(2, b"tiny-portscan: invalid start port\n");
                libc::exit(1);
            }
        };

        let end_port = match parse_u16(end_str) {
            Some(p) => p,
            None => {
                write_all(2, b"tiny-portscan: invalid end port\n");
                libc::exit(1);
            }
        };

        write_all(1, b"Scanning ");
        write_all(1, ip_str);
        write_all(1, b" ports ");
        let mut num_buf = [0u8; 20];
        let len = format_u32(start_port as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"-");
        let len = format_u32(end_port as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" (timeout ");
        let len = format_u32(TIMEOUT_MS as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"ms)\n");

        let mut open_count: u32 = 0;
        let mut port = start_port;
        while port <= end_port {
            if scan_port(ip_net, port) {
                write_all(1, b"  OPEN  ");
                let len = format_u32(port as u32, &mut num_buf);
                write_all(1, &num_buf[..len]);
                write_all(1, b"\n");
                open_count += 1;
            }
            port += 1;
        }

        write_all(1, b"Scan complete: ");
        let len = format_u32(open_count, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" open port(s)\n");
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
