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
const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

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

fn format_hex64(val: u64, buf: &mut [u8; 16]) {
    let mut v = val;
    let mut i: usize = 16;
    while i > 0 {
        i -= 1;
        buf[i] = HEX_CHARS[(v & 0xF) as usize];
        v >>= 4;
    }
}

fn parse_hex64(buf: &[u8], len: usize) -> Option<u64> {
    if len != 16 {
        return None;
    }
    let mut val: u64 = 0;
    let mut i = 0;
    while i < 16 {
        let nibble = match buf[i] {
            b'0'..=b'9' => buf[i] - b'0',
            b'a'..=b'f' => buf[i] - b'a' + 10,
            b'A'..=b'F' => buf[i] - b'A' + 10,
            _ => return None,
        };
        val = (val << 4) | nibble as u64;
        i += 1;
    }
    Some(val)
}

// ---------------------------------------------------------------------------
// Object store
// ---------------------------------------------------------------------------

const MAX_OBJECTS: usize = 64;
const MAX_OBJ_SIZE: usize = 4096;

struct Object {
    occupied: bool,
    hash_id: u64,
    data: [u8; MAX_OBJ_SIZE],
    data_len: usize,
}

struct ObjStore {
    objects: [Object; MAX_OBJECTS],
    count: usize,
    total_bytes: usize,
}

fn obj_find(store: &ObjStore, hash_id: u64) -> Option<usize> {
    let mut i = 0;
    while i < MAX_OBJECTS {
        if store.objects[i].occupied && store.objects[i].hash_id == hash_id {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Store object with dedup. Returns (hash_id, is_new).
fn obj_store(store: &mut ObjStore, data: &[u8], data_len: usize) -> Option<(u64, bool)> {
    let hash_id = fnv1a_hash(&data[..data_len]);

    // Check for existing (dedup)
    if obj_find(store, hash_id).is_some() {
        return Some((hash_id, false));
    }

    if store.count >= MAX_OBJECTS {
        return None;
    }

    // Find empty slot
    let mut i = 0;
    while i < MAX_OBJECTS {
        if !store.objects[i].occupied {
            let obj = &mut store.objects[i];
            obj.occupied = true;
            obj.hash_id = hash_id;
            let copy_len = if data_len > MAX_OBJ_SIZE {
                MAX_OBJ_SIZE
            } else {
                data_len
            };
            let mut j = 0;
            while j < copy_len {
                obj.data[j] = data[j];
                j += 1;
            }
            obj.data_len = copy_len;
            store.count += 1;
            store.total_bytes += copy_len;
            return Some((hash_id, true));
        }
        i += 1;
    }
    None
}

fn obj_delete(store: &mut ObjStore, hash_id: u64) -> bool {
    if let Some(idx) = obj_find(store, hash_id) {
        store.total_bytes -= store.objects[idx].data_len;
        store.objects[idx].occupied = false;
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

const PORT: u16 = 7880;

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
            write_all(2, b"tiny-objstore: socket() failed\n");
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
            write_all(2, b"tiny-objstore: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(sock, 16) < 0 {
            write_all(2, b"tiny-objstore: listen() failed\n");
            libc::exit(1);
        }

        write_all(1, b"tiny-objstore listening on port 7880\n");

        // Initialize store -- zeroed memory means all objects unoccupied
        let mut store: ObjStore = core::mem::zeroed();

        let mut req_count: u32 = 0;

        loop {
            let client = libc::accept(sock, core::ptr::null_mut(), core::ptr::null_mut());
            if client < 0 {
                continue;
            }

            req_count += 1;

            // Read request headers
            let mut req_buf = [0u8; 8192];
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

                copy_to(&mut resp, &mut rlen, b"tiny-objstore\nobjects: ");
                let n = format_u32(store.count as u32, &mut num_buf);
                copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                copy_to(&mut resp, &mut rlen, b"\nbytes: ");
                let n = format_u32(store.total_bytes as u32, &mut num_buf);
                copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                copy_to(&mut resp, &mut rlen, b"\ncapacity: ");
                let n = format_u32(MAX_OBJECTS as u32, &mut num_buf);
                copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                copy_to(&mut resp, &mut rlen, b"\n");

                send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
                log_request(req_count, b"GET /stats", b"200");
            } else if method_is(&req_buf, method_end, b"PUT")
                && path_eq(&req_buf, path_start, path_end, b"/obj")
            {
                // Read body
                let content_length = parse_content_length(&req_buf, header_end);
                let body_in_buf = req_len - header_end;
                let mut data = [0u8; MAX_OBJ_SIZE];
                let mut data_len: usize;

                let copy_len = if body_in_buf > MAX_OBJ_SIZE {
                    MAX_OBJ_SIZE
                } else {
                    body_in_buf
                };
                let mut i = 0;
                while i < copy_len {
                    data[i] = req_buf[header_end + i];
                    i += 1;
                }
                data_len = copy_len;

                let total_want = if content_length > MAX_OBJ_SIZE {
                    MAX_OBJ_SIZE
                } else {
                    content_length
                };
                while data_len < total_want {
                    let n = libc::read(
                        client,
                        data.as_mut_ptr().add(data_len) as *mut libc::c_void,
                        total_want - data_len,
                    );
                    if n <= 0 {
                        break;
                    }
                    data_len += n as usize;
                }

                match obj_store(&mut store, &data, data_len) {
                    Some((hash_id, is_new)) => {
                        let mut resp = [0u8; 128];
                        let mut rlen: usize = 0;
                        let mut hex_buf = [0u8; 16];
                        let mut num_buf = [0u8; 10];

                        if is_new {
                            copy_to(&mut resp, &mut rlen, b"stored\nid=");
                        } else {
                            copy_to(&mut resp, &mut rlen, b"exists\nid=");
                        }
                        format_hex64(hash_id, &mut hex_buf);
                        copy_to(&mut resp, &mut rlen, &hex_buf);
                        copy_to(&mut resp, &mut rlen, b"\nsize=");
                        let n = format_u32(data_len as u32, &mut num_buf);
                        copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                        copy_to(&mut resp, &mut rlen, b"\n");

                        send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
                        log_request(req_count, b"PUT /obj", b"200");
                    }
                    None => {
                        send_response(
                            client,
                            b"507 Insufficient Storage",
                            b"text/plain",
                            b"store full\n",
                        );
                        log_request(req_count, b"PUT /obj", b"507");
                    }
                }
            } else if method_is(&req_buf, method_end, b"GET")
                && path_starts_with(&req_buf, path_start, path_end, b"/obj/")
            {
                let id_start = path_start + 5; // skip "/obj/"
                let id_len = path_end - id_start;

                match parse_hex64(&req_buf[id_start..], id_len) {
                    Some(hash_id) => {
                        if let Some(idx) = obj_find(&store, hash_id) {
                            let obj = &store.objects[idx];
                            send_response(
                                client,
                                b"200 OK",
                                b"application/octet-stream",
                                &obj.data[..obj.data_len],
                            );
                            log_request(req_count, b"GET /obj/*", b"200");
                        } else {
                            send_response(
                                client,
                                b"404 Not Found",
                                b"text/plain",
                                b"not found\n",
                            );
                            log_request(req_count, b"GET /obj/*", b"404");
                        }
                    }
                    None => {
                        send_response(
                            client,
                            b"400 Bad Request",
                            b"text/plain",
                            b"invalid hex id\n",
                        );
                        log_request(req_count, b"GET /obj/*", b"400");
                    }
                }
            } else if method_is(&req_buf, method_end, b"DELETE")
                && path_starts_with(&req_buf, path_start, path_end, b"/obj/")
            {
                let id_start = path_start + 5;
                let id_len = path_end - id_start;

                match parse_hex64(&req_buf[id_start..], id_len) {
                    Some(hash_id) => {
                        if obj_delete(&mut store, hash_id) {
                            let mut resp = [0u8; 64];
                            let mut rlen: usize = 0;
                            let mut hex_buf = [0u8; 16];

                            copy_to(&mut resp, &mut rlen, b"deleted ");
                            format_hex64(hash_id, &mut hex_buf);
                            copy_to(&mut resp, &mut rlen, &hex_buf);
                            copy_to(&mut resp, &mut rlen, b"\n");

                            send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
                            log_request(req_count, b"DELETE /obj/*", b"200");
                        } else {
                            send_response(
                                client,
                                b"404 Not Found",
                                b"text/plain",
                                b"not found\n",
                            );
                            log_request(req_count, b"DELETE /obj/*", b"404");
                        }
                    }
                    None => {
                        send_response(
                            client,
                            b"400 Bad Request",
                            b"text/plain",
                            b"invalid hex id\n",
                        );
                        log_request(req_count, b"DELETE /obj/*", b"400");
                    }
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
