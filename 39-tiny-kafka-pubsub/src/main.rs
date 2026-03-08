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
// Constants
// ---------------------------------------------------------------------------

const PORT: u16 = 9093;
const MAX_SUBSCRIBERS: usize = 16;
const MAX_FILTER_LEN: usize = 32;

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

fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
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

/// Check if `haystack` starts with `prefix`.
fn starts_with(haystack: &[u8], prefix: &[u8]) -> bool {
    if prefix.len() > haystack.len() {
        return false;
    }
    let mut i = 0;
    while i < prefix.len() {
        if haystack[i] != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

struct Subscriber {
    fd: i32,
    filter: [u8; MAX_FILTER_LEN],
    filter_len: usize,
    active: bool,
    subscribed: bool,
}

struct Publisher {
    subscribers: [Subscriber; MAX_SUBSCRIBERS],
    _listen_fd: i32,
}

impl Publisher {
    fn new(listen_fd: i32) -> Self {
        const EMPTY_SUB: Subscriber = Subscriber {
            fd: -1,
            filter: [0u8; MAX_FILTER_LEN],
            filter_len: 0,
            active: false,
            subscribed: false,
        };
        Publisher {
            subscribers: [EMPTY_SUB; MAX_SUBSCRIBERS],
            _listen_fd: listen_fd,
        }
    }

    fn add_subscriber(&mut self, fd: i32) -> bool {
        let mut i = 0;
        while i < MAX_SUBSCRIBERS {
            if !self.subscribers[i].active {
                self.subscribers[i].fd = fd;
                self.subscribers[i].filter_len = 0;
                self.subscribers[i].active = true;
                self.subscribers[i].subscribed = false;
                return true;
            }
            i += 1;
        }
        false
    }

    fn remove_subscriber(&mut self, idx: usize) {
        if idx < MAX_SUBSCRIBERS && self.subscribers[idx].active {
            unsafe {
                libc::close(self.subscribers[idx].fd);
            }
            self.subscribers[idx].active = false;
            self.subscribers[idx].fd = -1;
            self.subscribers[idx].filter_len = 0;
            self.subscribers[idx].subscribed = false;
        }
    }

    /// Broadcast a message to all subscribed subscribers whose filter matches the topic.
    /// Returns the number of subscribers notified.
    fn broadcast(&self, topic: &[u8], message: &[u8]) -> u32 {
        let mut count: u32 = 0;
        let mut i = 0;
        while i < MAX_SUBSCRIBERS {
            if self.subscribers[i].active && self.subscribers[i].subscribed {
                let filter = &self.subscribers[i].filter[..self.subscribers[i].filter_len];
                // empty filter = match all, otherwise prefix match
                if filter.len() == 0 || starts_with(topic, filter) {
                    // Build: MSG topic 0 message\n
                    unsafe {
                        write_all(self.subscribers[i].fd, b"MSG ");
                        write_all(self.subscribers[i].fd, topic);
                        write_all(self.subscribers[i].fd, b" 0 ");
                        write_all(self.subscribers[i].fd, message);
                        write_all(self.subscribers[i].fd, b"\n");
                    }
                    count += 1;
                }
            }
            i += 1;
        }
        count
    }

    /// Handle a SUBSCRIBE command for subscriber at index `idx`.
    fn handle_subscribe(&mut self, idx: usize, filter: &[u8]) {
        self.subscribers[idx].subscribed = true;
        let copy_len = if filter.len() < MAX_FILTER_LEN {
            filter.len()
        } else {
            MAX_FILTER_LEN
        };
        let mut i = 0;
        while i < copy_len {
            self.subscribers[idx].filter[i] = filter[i];
            i += 1;
        }
        self.subscribers[idx].filter_len = copy_len;
        unsafe {
            write_all(self.subscribers[idx].fd, b"OK subscribed\n");
        }
    }
}

// ---------------------------------------------------------------------------
// Argv parsing via /proc/self/cmdline
// ---------------------------------------------------------------------------

unsafe fn argv1_is_sub() -> bool {
    let mut buf = [0u8; 256];
    let fd = libc::open(
        b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
        libc::O_RDONLY,
    );
    if fd < 0 {
        return false;
    }
    let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
    libc::close(fd);
    if n <= 0 {
        return false;
    }

    let data = &buf[..n as usize];
    // Skip argv[0]
    let mut i = 0;
    while i < data.len() && data[i] != 0 {
        i += 1;
    }
    i += 1; // skip null separator

    // Extract argv[1]
    if i >= data.len() {
        return false;
    }
    let start = i;
    while i < data.len() && data[i] != 0 {
        i += 1;
    }
    let arg = &data[start..i];
    bytes_eq(arg, b"sub")
}

// ---------------------------------------------------------------------------
// Publisher mode
// ---------------------------------------------------------------------------

fn run_publisher() {
    unsafe {
        let listen_fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if listen_fd < 0 {
            write_all(2, b"tiny-kafka: socket() failed\n");
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
            sin_port: PORT.to_be(),
            sin_addr: libc::in_addr { s_addr: 0 },
            sin_zero: [0; 8],
        };

        if libc::bind(
            listen_fd,
            &addr as *const _ as *const libc::sockaddr,
            core::mem::size_of::<libc::sockaddr_in>() as u32,
        ) < 0
        {
            write_all(2, b"tiny-kafka: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(listen_fd, 8) < 0 {
            write_all(2, b"tiny-kafka: listen() failed\n");
            libc::exit(1);
        }

        let mut num_buf = [0u8; 10];
        write_all(1, b"tiny-kafka: publisher listening on port ");
        let len = format_u32(PORT as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"\n");

        let mut pub_state = Publisher::new(listen_fd);

        // pollfd array: index 0 = listen_fd, index 1..MAX_SUBSCRIBERS = subscriber fds
        let mut fds: [libc::pollfd; 1 + MAX_SUBSCRIBERS] = core::mem::zeroed();
        let mut read_buf = [0u8; 1024];

        loop {
            // Build pollfd array
            fds[0] = libc::pollfd {
                fd: listen_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let mut nfds: usize = 1;
            let mut i = 0;
            while i < MAX_SUBSCRIBERS {
                if pub_state.subscribers[i].active {
                    fds[nfds] = libc::pollfd {
                        fd: pub_state.subscribers[i].fd,
                        events: libc::POLLIN,
                        revents: 0,
                    };
                    nfds += 1;
                }
                i += 1;
            }

            let ret = libc::poll(fds.as_mut_ptr(), nfds as libc::nfds_t, 1000);
            if ret < 0 {
                continue;
            }
            if ret == 0 {
                continue;
            }

            // Check listen socket for new connections
            if fds[0].revents & libc::POLLIN != 0 {
                let client_fd =
                    libc::accept(listen_fd, core::ptr::null_mut(), core::ptr::null_mut());
                if client_fd >= 0 {
                    if pub_state.add_subscriber(client_fd) {
                        write_all(client_fd, b"OK connected\n");
                        write_all(1, b"tiny-kafka: new client connected\n");
                    } else {
                        write_all(client_fd, b"ERR max subscribers\n");
                        libc::close(client_fd);
                    }
                }
            }

            // Check each subscriber fd
            let mut poll_idx: usize = 1;
            let mut sub_idx: usize = 0;
            while sub_idx < MAX_SUBSCRIBERS {
                if !pub_state.subscribers[sub_idx].active {
                    sub_idx += 1;
                    continue;
                }
                if poll_idx >= nfds {
                    break;
                }

                let sub_fd = pub_state.subscribers[sub_idx].fd;
                if fds[poll_idx].fd != sub_fd {
                    sub_idx += 1;
                    continue;
                }

                if fds[poll_idx].revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0 {
                    let n = libc::read(
                        sub_fd,
                        read_buf.as_mut_ptr() as *mut libc::c_void,
                        read_buf.len(),
                    );
                    if n <= 0 {
                        // Disconnected
                        write_all(1, b"tiny-kafka: client disconnected\n");
                        pub_state.remove_subscriber(sub_idx);
                    } else {
                        // Parse command(s) in buffer
                        let data = &read_buf[..n as usize];
                        // Find newline-terminated command
                        let mut cmd_start: usize = 0;
                        let mut ci: usize = 0;
                        while ci < data.len() {
                            if data[ci] == b'\n' {
                                let line = &data[cmd_start..ci];
                                handle_command(&mut pub_state, sub_idx, line, &mut num_buf);
                                cmd_start = ci + 1;
                            }
                            ci += 1;
                        }
                        // Handle line without trailing newline
                        if cmd_start < data.len() {
                            let line = &data[cmd_start..data.len()];
                            if line.len() > 0 {
                                handle_command(&mut pub_state, sub_idx, line, &mut num_buf);
                            }
                        }
                    }
                }

                poll_idx += 1;
                sub_idx += 1;
            }
        }
    }
}

fn handle_command(pub_state: &mut Publisher, sub_idx: usize, line: &[u8], num_buf: &mut [u8; 10]) {
    // Strip trailing \r if present
    let line = if line.len() > 0 && line[line.len() - 1] == b'\r' {
        &line[..line.len() - 1]
    } else {
        line
    };

    if line.len() == 0 {
        return;
    }

    if bytes_eq(line, b"SUBSCRIBE") || starts_with(line, b"SUBSCRIBE ") {
        // Extract optional filter
        let filter = if line.len() > 10 {
            &line[10..]
        } else {
            &[]
        };
        pub_state.handle_subscribe(sub_idx, filter);
        unsafe {
            write_all(1, b"tiny-kafka: client subscribed");
            if filter.len() > 0 {
                write_all(1, b" filter=");
                write_all(1, filter);
            }
            write_all(1, b"\n");
        }
    } else if starts_with(line, b"PUBLISH ") {
        // PUBLISH topic message
        // Find topic (first word after PUBLISH )
        let rest = &line[8..];
        let mut topic_end: usize = 0;
        while topic_end < rest.len() && rest[topic_end] != b' ' {
            topic_end += 1;
        }
        if topic_end == 0 || topic_end >= rest.len() {
            unsafe {
                write_all(pub_state.subscribers[sub_idx].fd, b"ERR bad publish\n");
            }
            return;
        }
        let topic = &rest[..topic_end];
        let message = &rest[topic_end + 1..];

        let count = pub_state.broadcast(topic, message);
        unsafe {
            write_all(pub_state.subscribers[sub_idx].fd, b"OK published ");
            let len = format_u32(count, num_buf);
            write_all(pub_state.subscribers[sub_idx].fd, &num_buf[..len]);
            write_all(pub_state.subscribers[sub_idx].fd, b"\n");

            write_all(1, b"tiny-kafka: published to ");
            let len = format_u32(count, num_buf);
            write_all(1, &num_buf[..len]);
            write_all(1, b" subscriber(s) topic=");
            write_all(1, topic);
            write_all(1, b"\n");
        }
    } else {
        unsafe {
            write_all(pub_state.subscribers[sub_idx].fd, b"ERR unknown command\n");
        }
    }
}

// ---------------------------------------------------------------------------
// Subscriber mode
// ---------------------------------------------------------------------------

fn run_subscriber() {
    unsafe {
        let fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if fd < 0 {
            write_all(2, b"tiny-kafka: socket() failed\n");
            libc::exit(1);
        }

        let addr = libc::sockaddr_in {
            sin_family: libc::AF_INET as u16,
            sin_port: PORT.to_be(),
            sin_addr: libc::in_addr {
                s_addr: 0x0100007f_u32.to_be(), // 127.0.0.1 in network byte order
            },
            sin_zero: [0; 8],
        };

        if libc::connect(
            fd,
            &addr as *const _ as *const libc::sockaddr,
            core::mem::size_of::<libc::sockaddr_in>() as u32,
        ) < 0
        {
            write_all(2, b"tiny-kafka: connect() failed\n");
            libc::exit(1);
        }

        // Send subscribe command
        write_all(fd, b"SUBSCRIBE\n");

        // Read and print messages
        let mut buf = [0u8; 1024];
        loop {
            let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n <= 0 {
                write_all(2, b"tiny-kafka: disconnected\n");
                libc::close(fd);
                libc::exit(0);
            }
            write_all(1, &buf[..n as usize]);
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[inline(never)]
fn run() {
    unsafe {
        if argv1_is_sub() {
            run_subscriber();
        } else {
            run_publisher();
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
