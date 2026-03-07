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
// SMTP helpers
// ---------------------------------------------------------------------------

unsafe fn smtp_send(fd: i32, msg: &[u8]) {
    write_all(fd, msg);
}

/// Case-insensitive prefix match (ASCII only)
fn starts_with_ci(buf: &[u8], len: usize, prefix: &[u8]) -> bool {
    if len < prefix.len() {
        return false;
    }
    let mut i = 0;
    while i < prefix.len() {
        let a = if buf[i] >= b'A' && buf[i] <= b'Z' {
            buf[i] + 32
        } else {
            buf[i]
        };
        let b = if prefix[i] >= b'A' && prefix[i] <= b'Z' {
            prefix[i] + 32
        } else {
            prefix[i]
        };
        if a != b {
            return false;
        }
        i += 1;
    }
    true
}

/// Find \r\n in buffer, return index of first byte of the sequence
fn find_crlf(buf: &[u8], len: usize) -> Option<usize> {
    if len < 2 {
        return None;
    }
    let mut i = 0;
    while i + 1 < len {
        if buf[i] == b'\r' && buf[i + 1] == b'\n' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Find \r\n.\r\n (dot-stuffing terminator) in buffer
fn find_dot_terminator(buf: &[u8], len: usize) -> Option<usize> {
    if len < 5 {
        return None;
    }
    let mut i = 0;
    while i + 4 < len {
        if buf[i] == b'\r'
            && buf[i + 1] == b'\n'
            && buf[i + 2] == b'.'
            && buf[i + 3] == b'\r'
            && buf[i + 4] == b'\n'
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Extract value after "MAIL FROM:" or "RCPT TO:" -- everything between < and >
/// Returns (start, end) indices into buf, or None
fn extract_angle_addr(buf: &[u8], len: usize) -> Option<(usize, usize)> {
    let mut i = 0;
    while i < len {
        if buf[i] == b'<' {
            let start = i;
            i += 1;
            while i < len {
                if buf[i] == b'>' {
                    return Some((start, i + 1));
                }
                i += 1;
            }
            return None;
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// SMTP state machine
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum State {
    Init,
    Greeted,
    Mail,
    Rcpt,
    Data,
}

const MAX_MSG_SIZE: usize = 10240;
const LISTEN_PORT: u16 = 2525;

unsafe fn handle_client(fd: i32, msg_count: &mut u32) {
    smtp_send(fd, b"220 tiny-smtp ready\r\n");

    let mut state = State::Init;
    let mut line_buf = [0u8; 512];
    let mut line_len: usize = 0;

    // Sender/recipient storage for logging
    let mut from_buf = [0u8; 256];
    let mut from_len: usize = 0;
    let mut to_buf = [0u8; 256];
    let mut to_len: usize = 0;

    // DATA accumulation buffer
    let mut data_buf = [0u8; MAX_MSG_SIZE];
    let mut data_len: usize = 0;

    loop {
        if state == State::Data {
            // In DATA mode: read until we find \r\n.\r\n
            loop {
                if data_len >= MAX_MSG_SIZE {
                    smtp_send(fd, b"552 Message too large\r\n");
                    state = State::Greeted;
                    data_len = 0;
                    break;
                }
                let n = libc::read(
                    fd,
                    data_buf.as_mut_ptr().add(data_len) as *mut libc::c_void,
                    MAX_MSG_SIZE - data_len,
                );
                if n <= 0 {
                    return; // Client disconnected
                }
                data_len += n as usize;

                if let Some(pos) = find_dot_terminator(&data_buf, data_len) {
                    // Message complete
                    *msg_count += 1;
                    let body_size = pos as u32;

                    // Log: [#N] MAIL FROM:<...> TO:<...> (size bytes)
                    let mut num_buf = [0u8; 10];
                    write_all(1, b"[#");
                    let len = format_u32(*msg_count, &mut num_buf);
                    write_all(1, &num_buf[..len]);
                    write_all(1, b"] MAIL FROM:");
                    write_all(1, &from_buf[..from_len]);
                    write_all(1, b" TO:");
                    write_all(1, &to_buf[..to_len]);
                    write_all(1, b" (");
                    let len = format_u32(body_size, &mut num_buf);
                    write_all(1, &num_buf[..len]);
                    write_all(1, b" bytes)\n");

                    smtp_send(fd, b"250 OK\r\n");
                    state = State::Greeted;
                    data_len = 0;
                    break;
                }
            }
            continue;
        }

        // Command mode: read one line at a time
        let n = libc::read(
            fd,
            line_buf.as_mut_ptr().add(line_len) as *mut libc::c_void,
            line_buf.len() - line_len,
        );
        if n <= 0 {
            return; // Client disconnected
        }
        line_len += n as usize;

        // Process all complete lines in the buffer
        loop {
            let crlf = match find_crlf(&line_buf, line_len) {
                Some(pos) => pos,
                None => break,
            };

            // We have a complete command from line_buf[0..crlf]
            let cmd_len = crlf;

            if starts_with_ci(&line_buf, cmd_len, b"quit") {
                smtp_send(fd, b"221 Bye\r\n");
                return;
            } else if starts_with_ci(&line_buf, cmd_len, b"noop") {
                smtp_send(fd, b"250 OK\r\n");
            } else if starts_with_ci(&line_buf, cmd_len, b"rset") {
                if state == State::Init {
                    smtp_send(fd, b"503 Say EHLO first\r\n");
                } else {
                    state = State::Greeted;
                    from_len = 0;
                    to_len = 0;
                    smtp_send(fd, b"250 OK\r\n");
                }
            } else if starts_with_ci(&line_buf, cmd_len, b"ehlo")
                || starts_with_ci(&line_buf, cmd_len, b"helo")
            {
                state = State::Greeted;
                from_len = 0;
                to_len = 0;
                if starts_with_ci(&line_buf, cmd_len, b"ehlo") {
                    smtp_send(fd, b"250-tiny-smtp\r\n");
                    smtp_send(fd, b"250-SIZE 10240\r\n");
                    smtp_send(fd, b"250 OK\r\n");
                } else {
                    smtp_send(fd, b"250 tiny-smtp\r\n");
                }
            } else if starts_with_ci(&line_buf, cmd_len, b"mail from:") {
                if state == State::Init {
                    smtp_send(fd, b"503 Say EHLO first\r\n");
                } else if state != State::Greeted {
                    smtp_send(fd, b"503 Bad sequence\r\n");
                } else {
                    // Extract sender address
                    if let Some((s, e)) = extract_angle_addr(&line_buf, cmd_len) {
                        from_len = e - s;
                        let mut i = 0;
                        while i < from_len && i < from_buf.len() {
                            from_buf[i] = line_buf[s + i];
                            i += 1;
                        }
                    } else {
                        // Use everything after "MAIL FROM:"
                        let offset = 10;
                        from_len = cmd_len - offset;
                        if from_len > from_buf.len() {
                            from_len = from_buf.len();
                        }
                        let mut i = 0;
                        while i < from_len {
                            from_buf[i] = line_buf[offset + i];
                            i += 1;
                        }
                    }
                    state = State::Mail;
                    smtp_send(fd, b"250 OK\r\n");
                }
            } else if starts_with_ci(&line_buf, cmd_len, b"rcpt to:") {
                if state == State::Init || state == State::Greeted {
                    smtp_send(fd, b"503 Bad sequence\r\n");
                } else if state != State::Mail && state != State::Rcpt {
                    smtp_send(fd, b"503 Bad sequence\r\n");
                } else {
                    // Extract recipient address
                    if let Some((s, e)) = extract_angle_addr(&line_buf, cmd_len) {
                        to_len = e - s;
                        let mut i = 0;
                        while i < to_len && i < to_buf.len() {
                            to_buf[i] = line_buf[s + i];
                            i += 1;
                        }
                    } else {
                        let offset = 8;
                        to_len = cmd_len - offset;
                        if to_len > to_buf.len() {
                            to_len = to_buf.len();
                        }
                        let mut i = 0;
                        while i < to_len {
                            to_buf[i] = line_buf[offset + i];
                            i += 1;
                        }
                    }
                    state = State::Rcpt;
                    smtp_send(fd, b"250 OK\r\n");
                }
            } else if starts_with_ci(&line_buf, cmd_len, b"data") {
                if state != State::Rcpt {
                    smtp_send(fd, b"503 Bad sequence\r\n");
                } else {
                    smtp_send(fd, b"354 Start mail input; end with <CRLF>.<CRLF>\r\n");
                    state = State::Data;
                    data_len = 0;

                    // Any remaining data after this line belongs to DATA
                    let consumed = crlf + 2;
                    let remaining = line_len - consumed;
                    if remaining > 0 {
                        let mut i = 0;
                        while i < remaining && i < MAX_MSG_SIZE {
                            data_buf[i] = line_buf[consumed + i];
                            i += 1;
                        }
                        data_len = remaining;
                    }
                    line_len = 0;

                    // Check if the leftover already contains the terminator
                    if data_len > 0 {
                        if let Some(_pos) = find_dot_terminator(&data_buf, data_len) {
                            // Unlikely but handle it
                        }
                    }
                    break;
                }
            } else {
                smtp_send(fd, b"500 Unrecognized command\r\n");
            }

            // Shift remaining data in line_buf
            let consumed = crlf + 2;
            let remaining = line_len - consumed;
            let mut i = 0;
            while i < remaining {
                line_buf[i] = line_buf[consumed + i];
                i += 1;
            }
            line_len = remaining;
        }

        if line_len >= line_buf.len() {
            // Line too long, reject
            smtp_send(fd, b"500 Line too long\r\n");
            line_len = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// Main logic
// ---------------------------------------------------------------------------

#[inline(never)]
fn run() {
    unsafe {
        let listen_fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if listen_fd < 0 {
            write_all(2, b"tiny-smtp: socket() failed\n");
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
            write_all(2, b"tiny-smtp: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(listen_fd, 16) < 0 {
            write_all(2, b"tiny-smtp: listen() failed\n");
            libc::exit(1);
        }

        let mut num_buf = [0u8; 10];
        write_all(1, b"SMTP server listening on port ");
        let len = format_u32(LISTEN_PORT as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"\n");

        let mut msg_count: u32 = 0;

        loop {
            let client_fd =
                libc::accept(listen_fd, core::ptr::null_mut(), core::ptr::null_mut());
            if client_fd < 0 {
                continue;
            }

            handle_client(client_fd, &mut msg_count);
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
