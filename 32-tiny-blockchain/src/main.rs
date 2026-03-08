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

/// Parse "GET /path HTTP/1.x" -> (method_end, path_start, path_end)
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

// ---------------------------------------------------------------------------
// FNV-1a hash
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;
const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

fn format_hex64(val: u64, buf: &mut [u8; 16]) {
    let mut v = val;
    let mut i: usize = 16;
    while i > 0 {
        i -= 1;
        buf[i] = HEX_CHARS[(v & 0xF) as usize];
        v >>= 4;
    }
}

// ---------------------------------------------------------------------------
// Blockchain data structures
// ---------------------------------------------------------------------------

const MAX_BLOCKS: usize = 64;
const MAX_DATA_LEN: usize = 256;

struct Block {
    index: u32,
    timestamp: i64,
    data: [u8; MAX_DATA_LEN],
    data_len: usize,
    prev_hash: u64,
    hash: u64,
}

struct Chain {
    blocks: [Block; MAX_BLOCKS],
    len: usize,
}

fn compute_block_hash(block: &Block) -> u64 {
    let mut hash: u64 = FNV_OFFSET;

    // Hash index bytes
    let idx_bytes = block.index.to_le_bytes();
    let mut i = 0;
    while i < 4 {
        hash ^= idx_bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }

    // Hash timestamp bytes
    let ts_bytes = block.timestamp.to_le_bytes();
    i = 0;
    while i < 8 {
        hash ^= ts_bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }

    // Hash data
    i = 0;
    while i < block.data_len {
        hash ^= block.data[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }

    // Hash prev_hash bytes
    let ph_bytes = block.prev_hash.to_le_bytes();
    i = 0;
    while i < 8 {
        hash ^= ph_bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }

    hash
}

fn chain_init(chain: &mut Chain) {
    let genesis = &mut chain.blocks[0];
    genesis.index = 0;
    unsafe {
        genesis.timestamp = libc::time(core::ptr::null_mut());
    }
    let genesis_data = b"genesis";
    let mut i = 0;
    while i < genesis_data.len() {
        genesis.data[i] = genesis_data[i];
        i += 1;
    }
    genesis.data_len = genesis_data.len();
    genesis.prev_hash = 0;
    genesis.hash = compute_block_hash(genesis);
    chain.len = 1;
}

fn chain_add(chain: &mut Chain, data: &[u8], data_len: usize) -> bool {
    if chain.len >= MAX_BLOCKS {
        return false;
    }
    let idx = chain.len;
    let prev_hash = chain.blocks[idx - 1].hash;

    let block = &mut chain.blocks[idx];
    block.index = idx as u32;
    unsafe {
        block.timestamp = libc::time(core::ptr::null_mut());
    }
    let copy_len = if data_len > MAX_DATA_LEN {
        MAX_DATA_LEN
    } else {
        data_len
    };
    let mut i = 0;
    while i < copy_len {
        block.data[i] = data[i];
        i += 1;
    }
    block.data_len = copy_len;
    block.prev_hash = prev_hash;
    block.hash = compute_block_hash(block);
    chain.len += 1;
    true
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

const PORT: u16 = 7878;

#[inline(never)]
fn run() {
    unsafe {
        let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if sock < 0 {
            write_all(2, b"tiny-blockchain: socket() failed\n");
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
            write_all(2, b"tiny-blockchain: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(sock, 16) < 0 {
            write_all(2, b"tiny-blockchain: listen() failed\n");
            libc::exit(1);
        }

        write_all(1, b"tiny-blockchain listening on port 7878\n");

        // Initialize chain with genesis block
        let mut chain: Chain = core::mem::zeroed();
        chain_init(&mut chain);

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

            // Build response into a buffer
            let mut resp = [0u8; 4096];
            let mut rlen: usize = 0;

            if method_is(&req_buf, method_end, b"GET") && path_eq(&req_buf, path_start, path_end, b"/") {
                // Chain info
                let mut num_buf = [0u8; 10];
                let mut hex_buf = [0u8; 16];

                let msg = b"tiny-blockchain\nblocks: ";
                copy_to(&mut resp, &mut rlen, msg);
                let n = format_u32(chain.len as u32, &mut num_buf);
                copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                copy_to(&mut resp, &mut rlen, b"\nlatest: ");
                format_hex64(chain.blocks[chain.len - 1].hash, &mut hex_buf);
                copy_to(&mut resp, &mut rlen, &hex_buf);
                copy_to(&mut resp, &mut rlen, b"\n");

                send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
                log_request(req_count, b"GET /", b"200");
            } else if method_is(&req_buf, method_end, b"GET")
                && path_eq(&req_buf, path_start, path_end, b"/chain")
            {
                // Dump all blocks
                let mut num_buf = [0u8; 10];
                let mut hex_buf = [0u8; 16];

                let mut bi = 0;
                while bi < chain.len {
                    let block = &chain.blocks[bi];
                    copy_to(&mut resp, &mut rlen, b"[");
                    let n = format_u32(block.index, &mut num_buf);
                    copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                    copy_to(&mut resp, &mut rlen, b"] ");
                    format_hex64(block.hash, &mut hex_buf);
                    copy_to(&mut resp, &mut rlen, &hex_buf);
                    copy_to(&mut resp, &mut rlen, b" prev=");
                    format_hex64(block.prev_hash, &mut hex_buf);
                    copy_to(&mut resp, &mut rlen, &hex_buf);
                    copy_to(&mut resp, &mut rlen, b" data=");
                    copy_to(&mut resp, &mut rlen, &block.data[..block.data_len]);
                    copy_to(&mut resp, &mut rlen, b"\n");
                    bi += 1;
                }

                send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
                log_request(req_count, b"GET /chain", b"200");
            } else if method_is(&req_buf, method_end, b"POST")
                && path_eq(&req_buf, path_start, path_end, b"/block")
            {
                // Read body
                let content_length = parse_content_length(&req_buf, header_end);
                let body_in_buf = req_len - header_end;
                let mut body = [0u8; MAX_DATA_LEN];
                let mut body_len: usize;

                // Copy body already in buffer
                let copy_len = if body_in_buf > MAX_DATA_LEN {
                    MAX_DATA_LEN
                } else {
                    body_in_buf
                };
                let mut i = 0;
                while i < copy_len {
                    body[i] = req_buf[header_end + i];
                    i += 1;
                }
                body_len = copy_len;

                // Read remaining body
                let total_want = if content_length > MAX_DATA_LEN {
                    MAX_DATA_LEN
                } else {
                    content_length
                };
                while body_len < total_want {
                    let n = libc::read(
                        client,
                        body.as_mut_ptr().add(body_len) as *mut libc::c_void,
                        total_want - body_len,
                    );
                    if n <= 0 {
                        break;
                    }
                    body_len += n as usize;
                }

                if chain_add(&mut chain, &body, body_len) {
                    let mut num_buf = [0u8; 10];
                    let mut hex_buf = [0u8; 16];

                    let added_block = &chain.blocks[chain.len - 1];
                    copy_to(&mut resp, &mut rlen, b"added block ");
                    let n = format_u32(added_block.index, &mut num_buf);
                    copy_to(&mut resp, &mut rlen, &num_buf[..n]);
                    copy_to(&mut resp, &mut rlen, b"\nhash=");
                    format_hex64(added_block.hash, &mut hex_buf);
                    copy_to(&mut resp, &mut rlen, &hex_buf);
                    copy_to(&mut resp, &mut rlen, b"\n");

                    send_response(client, b"200 OK", b"text/plain", &resp[..rlen]);
                    log_request(req_count, b"POST /block", b"200");
                } else {
                    send_response(client, b"507 Insufficient Storage", b"text/plain", b"chain full\n");
                    log_request(req_count, b"POST /block", b"507");
                }
            } else {
                send_response(client, b"404 Not Found", b"text/plain", b"not found\n");
                log_request(req_count, b"?", b"404");
            }

            libc::close(client);
        }
    }
}

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

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::asm!(
        "and rsp, -16",
        "call {run}",
        run = sym run,
        options(noreturn),
    );
}
