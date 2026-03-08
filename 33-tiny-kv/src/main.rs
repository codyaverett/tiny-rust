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
// HTTP parsing helpers
// ---------------------------------------------------------------------------

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

fn parse_content_length(buf: &[u8], header_end: usize) -> usize {
    let needle = b"Content-Length: ";
    let mut i = 0;
    while i + needle.len() < header_end {
        let mut matched = true;
        let mut j = 0;
        while j < needle.len() {
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

fn parse_request_line(buf: &[u8], len: usize) -> (usize, usize, usize) {
    let mut method_end = 0;
    while method_end < len && buf[method_end] != b' ' {
        method_end += 1;
    }
    let path_start = method_end + 1;
    let mut path_end = path_start;
    while path_end < len && buf[path_end] != b' ' {
        path_end += 1;
    }
    (method_end, path_start, path_end)
}

fn method_is(buf: &[u8], method_end: usize, expected: &[u8]) -> bool {
    if method_end != expected.len() {
        return false;
    }
    let mut i = 0;
    while i < expected.len() {
        if buf[i] != expected[i] {
            return false;
        }
        i += 1;
    }
    true
}

fn path_eq(buf: &[u8], start: usize, end: usize, expected: &[u8]) -> bool {
    let len = end - start;
    if len != expected.len() {
        return false;
    }
    let mut i = 0;
    while i < len {
        if buf[start + i] != expected[i] {
            return false;
        }
        i += 1;
    }
    true
}

fn path_starts_with(buf: &[u8], start: usize, end: usize, prefix: &[u8]) -> bool {
    let len = end - start;
    if len < prefix.len() {
        return false;
    }
    let mut i = 0;
    while i < prefix.len() {
        if buf[start + i] != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}

// ---------------------------------------------------------------------------
// FNV-1a hash
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = FNV_OFFSET;
    let mut i = 0;
    while i < data.len() {
        hash ^= data[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

// ---------------------------------------------------------------------------
// Key-Value store
// ---------------------------------------------------------------------------

const MAX_ENTRIES: usize = 256;
const MAX_KEY_LEN: usize = 64;
const MAX_VAL_LEN: usize = 256;

#[allow(dead_code)] // Empty is constructed via core::mem::zeroed()
#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
enum SlotState {
    Empty = 0,
    Occupied,
    Tombstone,
}

struct Entry {
    state: SlotState,
    key: [u8; MAX_KEY_LEN],
    key_len: usize,
    val: [u8; MAX_VAL_LEN],
    val_len: usize,
}

struct KvStore {
    entries: [Entry; MAX_ENTRIES],
    count: usize,
}

fn kv_find(store: &KvStore, key: &[u8], key_len: usize) -> Option<usize> {
    let hash = fnv1a_hash(&key[..key_len]);
    let mut idx = (hash as usize) % MAX_ENTRIES;
    let mut probes = 0;
    while probes < MAX_ENTRIES {
        match store.entries[idx].state {
            SlotState::Empty => return None,
            SlotState::Occupied => {
                if store.entries[idx].key_len == key_len {
                    let mut eq = true;
                    let mut i = 0;
                    while i < key_len {
                        if store.entries[idx].key[i] != key[i] {
                            eq = false;
                            break;
                        }
                        i += 1;
                    }
                    if eq {
                        return Some(idx);
                    }
                }
            }
            SlotState::Tombstone => {}
        }
        idx = (idx + 1) % MAX_ENTRIES;
        probes += 1;
    }
    None
}

fn kv_put(store: &mut KvStore, key: &[u8], key_len: usize, val: &[u8], val_len: usize) -> bool {
    // Check load factor: don't fill beyond 75%
    if store.count >= MAX_ENTRIES * 3 / 4 {
        return false;
    }

    // Check if key already exists (update in place)
    if let Some(idx) = kv_find(store, key, key_len) {
        let entry = &mut store.entries[idx];
        let copy_len = if val_len > MAX_VAL_LEN {
            MAX_VAL_LEN
        } else {
            val_len
        };
        let mut i = 0;
        while i < copy_len {
            entry.val[i] = val[i];
            i += 1;
        }
        entry.val_len = copy_len;
        return true;
    }

    // Find empty or tombstone slot
    let hash = fnv1a_hash(&key[..key_len]);
    let mut idx = (hash as usize) % MAX_ENTRIES;
    let mut probes = 0;
    while probes < MAX_ENTRIES {
        if store.entries[idx].state != SlotState::Occupied {
            let entry = &mut store.entries[idx];
            entry.state = SlotState::Occupied;
            let klen = if key_len > MAX_KEY_LEN {
                MAX_KEY_LEN
            } else {
                key_len
            };
            let mut i = 0;
            while i < klen {
                entry.key[i] = key[i];
                i += 1;
            }
            entry.key_len = klen;
            let vlen = if val_len > MAX_VAL_LEN {
                MAX_VAL_LEN
            } else {
                val_len
            };
            i = 0;
            while i < vlen {
                entry.val[i] = val[i];
                i += 1;
            }
            entry.val_len = vlen;
            store.count += 1;
            return true;
        }
        idx = (idx + 1) % MAX_ENTRIES;
        probes += 1;
    }
    false
}

fn kv_delete(store: &mut KvStore, key: &[u8], key_len: usize) -> bool {
    if let Some(idx) = kv_find(store, key, key_len) {
        store.entries[idx].state = SlotState::Tombstone;
        store.count -= 1;
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// HTTP response helpers
// ---------------------------------------------------------------------------

unsafe fn send_response(fd: i32, status: &[u8], content_type: &[u8], body: &[u8]) {
    let mut num_buf = [0u8; 10];
    write_all(fd, b"HTTP/1.1 ");
    write_all(fd, status);
    write_all(fd, b"\r\nContent-Type: ");
    write_all(fd, content_type);
    write_all(fd, b"\r\nConnection: close\r\nContent-Length: ");
    let cl_len = format_u32(body.len() as u32, &mut num_buf);
    write_all(fd, &num_buf[..cl_len]);
    write_all(fd, b"\r\n\r\n");
    write_all(fd, body);
}

// ---------------------------------------------------------------------------
// Main logic
// ---------------------------------------------------------------------------

const PORT: u16 = 7879;

fn copy_to(dest: &mut [u8], pos: &mut usize, src: &[u8]) {
    let mut i = 0;
    while i < src.len() && *pos < dest.len() {
        dest[*pos] = src[i];
        *pos += 1;
        i += 1;
    }
}

fn log_request(count: u32, route: &[u8], status: &[u8]) {
    unsafe {
        let mut num_buf = [0u8; 10];
        write_all(1, b"[#");
        let n = format_u32(count, &mut num_buf);
        write_all(1, &num_buf[..n]);
        write_all(1, b"] ");
        write_all(1, route);
        write_all(1, b" -> ");
        write_all(1, status);
        write_all(1, b"\n");
    }
}

#[inline(never)]
fn run() {
    unsafe {
        let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if sock < 0 {
            write_all(2, b"tiny-kv: socket() failed\n");
            libc::exit(1);
        }

        let optval: i32 = 1;
        libc::setsockopt(
            sock,
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
            sock,
            &addr as *const _ as *const libc::sockaddr,
            core::mem::size_of::<libc::sockaddr_in>() as u32,
        ) < 0
        {
            write_all(2, b"tiny-kv: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(sock, 16) < 0 {
            write_all(2, b"tiny-kv: listen() failed\n");
            libc::exit(1);
        }

        write_all(1, b"tiny-kv listening on port 7879\n");

        // Initialize store
        let mut store: KvStore = core::mem::zeroed();
        // zeroed memory has all SlotState bytes = 0, which is Empty

        let mut req_count: u32 = 0;

        loop {
            let client = libc::accept(sock, core::ptr::null_mut(), core::ptr::null_mut());
            if client < 0 {
                continue;
            }

            req_count += 1;

            // Read request headers
            let mut req_buf = [0u8; 4096];
            let mut req_len: usize = 0;

            loop {
                if req_len >= req_buf.len() {
                    break;
                }
                let n = libc::read(
                    client,
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

            let header_end = match find_header_end(&req_buf, req_len) {
                Some(end) => end,
                None => {
                    write_all(client, b"HTTP/1.1 400 Bad Request\r\n\r\n");
                    libc::close(client);
                    continue;
                }
            };

            let (method_end, path_start, path_end) = parse_request_line(&req_buf, req_len);

            if method_is(&req_buf, method_end, b"GET")
                && path_eq(&req_buf, path_start, path_end, b"/stats")
            {
                let mut resp = [0u8; 256];
                let mut rlen: usize = 0;
                let mut num_buf = [0u8; 10];

                copy_to(&mut resp, &mut rlen, b"tiny-kv\ncount: ");
                let n = format_u32(store.count as u32, &mut num_buf);
                copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                copy_to(&mut resp, &mut rlen, b"\ncapacity: ");
                let n = format_u32(MAX_ENTRIES as u32, &mut num_buf);
                copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                copy_to(&mut resp, &mut rlen, b"\n");

                send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
                log_request(req_count, b"GET /stats", b"200");
            } else if method_is(&req_buf, method_end, b"GET")
                && path_starts_with(&req_buf, path_start, path_end, b"/key/")
            {
                // Extract key name from path
                let key_start = path_start + 5; // skip "/key/"
                let key_len = path_end - key_start;

                if key_len == 0 || key_len > MAX_KEY_LEN {
                    send_response(client, b"400 Bad Request", b"text/plain", b"invalid key\n");
                    log_request(req_count, b"GET /key/", b"400");
                } else if let Some(idx) = kv_find(&store, &req_buf[key_start..], key_len) {
                    let entry = &store.entries[idx];
                    send_response(
                        client,
                        b"200 OK",
                        b"application/octet-stream",
                        &entry.val[..entry.val_len],
                    );
                    log_request(req_count, b"GET /key/*", b"200");
                } else {
                    send_response(client, b"404 Not Found", b"text/plain", b"not found\n");
                    log_request(req_count, b"GET /key/*", b"404");
                }
            } else if method_is(&req_buf, method_end, b"PUT")
                && path_starts_with(&req_buf, path_start, path_end, b"/key/")
            {
                let key_start = path_start + 5;
                let key_len = path_end - key_start;

                if key_len == 0 || key_len > MAX_KEY_LEN {
                    send_response(client, b"400 Bad Request", b"text/plain", b"invalid key\n");
                    log_request(req_count, b"PUT /key/", b"400");
                } else {
                    // Read body (value)
                    let content_length = parse_content_length(&req_buf, header_end);
                    let body_in_buf = req_len - header_end;
                    let mut val = [0u8; MAX_VAL_LEN];
                    let mut val_len: usize;

                    let copy_len = if body_in_buf > MAX_VAL_LEN {
                        MAX_VAL_LEN
                    } else {
                        body_in_buf
                    };
                    let mut i = 0;
                    while i < copy_len {
                        val[i] = req_buf[header_end + i];
                        i += 1;
                    }
                    val_len = copy_len;

                    let total_want = if content_length > MAX_VAL_LEN {
                        MAX_VAL_LEN
                    } else {
                        content_length
                    };
                    while val_len < total_want {
                        let n = libc::read(
                            client,
                            val.as_mut_ptr().add(val_len) as *mut libc::c_void,
                            total_want - val_len,
                        );
                        if n <= 0 {
                            break;
                        }
                        val_len += n as usize;
                    }

                    if kv_put(&mut store, &req_buf[key_start..], key_len, &val, val_len) {
                        let mut resp = [0u8; 128];
                        let mut rlen: usize = 0;
                        copy_to(&mut resp, &mut rlen, b"stored ");
                        copy_to(&mut resp, &mut rlen, &req_buf[key_start..key_start + key_len]);
                        copy_to(&mut resp, &mut rlen, b"\n");

                        send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
                        log_request(req_count, b"PUT /key/*", b"200");
                    } else {
                        send_response(
                            client,
                            b"507 Insufficient Storage",
                            b"text/plain",
                            b"store full\n",
                        );
                        log_request(req_count, b"PUT /key/*", b"507");
                    }
                }
            } else if method_is(&req_buf, method_end, b"DELETE")
                && path_starts_with(&req_buf, path_start, path_end, b"/key/")
            {
                let key_start = path_start + 5;
                let key_len = path_end - key_start;

                if key_len == 0 || key_len > MAX_KEY_LEN {
                    send_response(client, b"400 Bad Request", b"text/plain", b"invalid key\n");
                    log_request(req_count, b"DELETE /key/", b"400");
                } else if kv_delete(&mut store, &req_buf[key_start..], key_len) {
                    let mut resp = [0u8; 128];
                    let mut rlen: usize = 0;
                    copy_to(&mut resp, &mut rlen, b"deleted ");
                    copy_to(&mut resp, &mut rlen, &req_buf[key_start..key_start + key_len]);
                    copy_to(&mut resp, &mut rlen, b"\n");

                    send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
                    log_request(req_count, b"DELETE /key/*", b"200");
                } else {
                    send_response(client, b"404 Not Found", b"text/plain", b"not found\n");
                    log_request(req_count, b"DELETE /key/*", b"404");
                }
            } else {
                send_response(client, b"404 Not Found", b"text/plain", b"not found\n");
                log_request(req_count, b"?", b"404");
            }

            libc::close(client);
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
