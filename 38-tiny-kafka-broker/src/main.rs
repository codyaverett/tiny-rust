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

const MAX_TOPICS: usize = 8;
const MAX_PARTITIONS: usize = 4;
const MAX_MESSAGES: usize = 64;
const MAX_MSG_LEN: usize = 256;
const MAX_NAME_LEN: usize = 32;
const MAX_OFFSETS: usize = 64;

const PORT: u16 = 9092;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[repr(C)]
struct Message {
    data: [u8; MAX_MSG_LEN],
    data_len: usize,
    offset: u64,
}

#[repr(C)]
struct Partition {
    messages: [Message; MAX_MESSAGES],
    next_offset: u64,
    head: usize,
}

#[repr(C)]
struct Topic {
    name: [u8; MAX_NAME_LEN],
    name_len: usize,
    partitions: [Partition; MAX_PARTITIONS],
    num_partitions: usize,
    active: bool,
    next_partition: usize,
}

#[repr(C)]
struct ConsumerOffset {
    group: [u8; MAX_NAME_LEN],
    group_len: usize,
    topic_idx: usize,
    partition_idx: usize,
    offset: u64,
    active: bool,
}

#[repr(C)]
struct Broker {
    topics: [Topic; MAX_TOPICS],
    offsets: [ConsumerOffset; MAX_OFFSETS],
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

fn copy_to(dest: &mut [u8], pos: &mut usize, src: &[u8]) {
    let mut i = 0;
    while i < src.len() && *pos < dest.len() {
        dest[*pos] = src[i];
        *pos += 1;
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_u32(buf: &[u8]) -> u32 {
    let mut val: u32 = 0;
    let mut i = 0;
    while i < buf.len() && buf[i] >= b'0' && buf[i] <= b'9' {
        val = val * 10 + (buf[i] - b'0') as u32;
        i += 1;
    }
    val
}

fn parse_u64(buf: &[u8]) -> u64 {
    let mut val: u64 = 0;
    let mut i = 0;
    while i < buf.len() && buf[i] >= b'0' && buf[i] <= b'9' {
        val = val * 10 + (buf[i] - b'0') as u64;
        i += 1;
    }
    val
}

/// Find next space or end-of-data, return index
fn next_space(buf: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < buf.len() && buf[i] != b' ' && buf[i] != b'\n' && buf[i] != b'\r' {
        i += 1;
    }
    i
}

/// Skip spaces, return index of next non-space
fn skip_spaces(buf: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < buf.len() && buf[i] == b' ' {
        i += 1;
    }
    i
}

// ---------------------------------------------------------------------------
// Broker operations
// ---------------------------------------------------------------------------

fn find_topic(broker: &Broker, name: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < MAX_TOPICS {
        if broker.topics[i].active && bytes_eq(&broker.topics[i].name[..broker.topics[i].name_len], name) {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_or_create_offset(
    broker: &mut Broker,
    group: &[u8],
    topic_idx: usize,
    partition_idx: usize,
) -> Option<usize> {
    // Find existing
    let mut i = 0;
    while i < MAX_OFFSETS {
        if broker.offsets[i].active
            && bytes_eq(&broker.offsets[i].group[..broker.offsets[i].group_len], group)
            && broker.offsets[i].topic_idx == topic_idx
            && broker.offsets[i].partition_idx == partition_idx
        {
            return Some(i);
        }
        i += 1;
    }
    // Create new
    i = 0;
    while i < MAX_OFFSETS {
        if !broker.offsets[i].active {
            broker.offsets[i].active = true;
            broker.offsets[i].topic_idx = topic_idx;
            broker.offsets[i].partition_idx = partition_idx;
            broker.offsets[i].offset = 0;
            broker.offsets[i].group_len = if group.len() > MAX_NAME_LEN {
                MAX_NAME_LEN
            } else {
                group.len()
            };
            let mut j = 0;
            while j < broker.offsets[i].group_len {
                broker.offsets[i].group[j] = group[j];
                j += 1;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

fn cmd_create_topic(broker: &mut Broker, buf: &[u8], len: usize, resp: &mut [u8], rlen: &mut usize) {
    // CREATE_TOPIC name [partitions]
    let start = skip_spaces(buf, 0);
    let name_end = next_space(buf, start);
    if name_end == start || name_end - start > MAX_NAME_LEN {
        copy_to(resp, rlen, b"ERR invalid topic name\n");
        return;
    }
    let name = &buf[start..name_end];

    // Check if already exists
    if find_topic(broker, name).is_some() {
        copy_to(resp, rlen, b"ERR topic already exists\n");
        return;
    }

    // Parse optional partition count
    let mut num_partitions: usize = 1;
    let pstart = skip_spaces(buf, name_end);
    if pstart < len && buf[pstart] >= b'0' && buf[pstart] <= b'9' {
        num_partitions = parse_u32(&buf[pstart..len]) as usize;
        if num_partitions < 1 {
            num_partitions = 1;
        }
        if num_partitions > MAX_PARTITIONS {
            num_partitions = MAX_PARTITIONS;
        }
    }

    // Find free slot
    let mut i = 0;
    while i < MAX_TOPICS {
        if !broker.topics[i].active {
            broker.topics[i].active = true;
            broker.topics[i].name_len = name.len();
            let mut j = 0;
            while j < name.len() {
                broker.topics[i].name[j] = name[j];
                j += 1;
            }
            broker.topics[i].num_partitions = num_partitions;
            broker.topics[i].next_partition = 0;

            copy_to(resp, rlen, b"OK topic created with ");
            let mut nbuf = [0u8; 10];
            let n = format_u32(num_partitions as u32, &mut nbuf);
            copy_to(resp, rlen, &nbuf[..n]);
            copy_to(resp, rlen, b" partition(s)\n");
            return;
        }
        i += 1;
    }
    copy_to(resp, rlen, b"ERR max topics reached\n");
}

fn cmd_produce(broker: &mut Broker, buf: &[u8], len: usize, resp: &mut [u8], rlen: &mut usize) {
    // PRODUCE topic msg
    let start = skip_spaces(buf, 0);
    let topic_end = next_space(buf, start);
    if topic_end == start {
        copy_to(resp, rlen, b"ERR missing topic\n");
        return;
    }
    let topic_name = &buf[start..topic_end];

    let msg_start = skip_spaces(buf, topic_end);
    if msg_start >= len {
        copy_to(resp, rlen, b"ERR missing message\n");
        return;
    }
    // Message is everything from msg_start to end (trim trailing newline)
    let mut msg_end = len;
    while msg_end > msg_start && (buf[msg_end - 1] == b'\n' || buf[msg_end - 1] == b'\r') {
        msg_end -= 1;
    }
    if msg_end == msg_start {
        copy_to(resp, rlen, b"ERR empty message\n");
        return;
    }
    let msg = &buf[msg_start..msg_end];

    let tidx = match find_topic(broker, topic_name) {
        Some(idx) => idx,
        None => {
            copy_to(resp, rlen, b"ERR topic not found\n");
            return;
        }
    };

    let topic = &mut broker.topics[tidx];
    let pidx = topic.next_partition;
    topic.next_partition = (topic.next_partition + 1) % topic.num_partitions;

    let part = &mut topic.partitions[pidx];
    let head = part.head;
    let msg_len = if msg.len() > MAX_MSG_LEN { MAX_MSG_LEN } else { msg.len() };

    let mut j = 0;
    while j < msg_len {
        part.messages[head].data[j] = msg[j];
        j += 1;
    }
    part.messages[head].data_len = msg_len;
    part.messages[head].offset = part.next_offset;

    let offset = part.next_offset;
    part.next_offset += 1;
    part.head = (head + 1) % MAX_MESSAGES;

    copy_to(resp, rlen, b"OK partition=");
    let mut nbuf = [0u8; 10];
    let n = format_u32(pidx as u32, &mut nbuf);
    copy_to(resp, rlen, &nbuf[..n]);
    copy_to(resp, rlen, b" offset=");
    let mut obuf = [0u8; 20];
    let n = format_u64(offset, &mut obuf);
    copy_to(resp, rlen, &obuf[..n]);
    copy_to(resp, rlen, b"\n");
}

fn cmd_consume(broker: &Broker, buf: &[u8], len: usize, resp: &mut [u8], rlen: &mut usize) {
    // CONSUME topic partition offset [count]
    let start = skip_spaces(buf, 0);
    let topic_end = next_space(buf, start);
    if topic_end == start {
        copy_to(resp, rlen, b"ERR missing topic\n");
        return;
    }
    let topic_name = &buf[start..topic_end];

    let p_start = skip_spaces(buf, topic_end);
    let p_end = next_space(buf, p_start);
    if p_end == p_start {
        copy_to(resp, rlen, b"ERR missing partition\n");
        return;
    }
    let partition_idx = parse_u32(&buf[p_start..p_end]) as usize;

    let o_start = skip_spaces(buf, p_end);
    let o_end = next_space(buf, o_start);
    if o_end == o_start {
        copy_to(resp, rlen, b"ERR missing offset\n");
        return;
    }
    let start_offset = parse_u64(&buf[o_start..o_end]);

    let mut count: u32 = 1;
    let c_start = skip_spaces(buf, o_end);
    if c_start < len && buf[c_start] >= b'0' && buf[c_start] <= b'9' {
        count = parse_u32(&buf[c_start..len]);
        if count == 0 {
            count = 1;
        }
    }

    let tidx = match find_topic(broker, topic_name) {
        Some(idx) => idx,
        None => {
            copy_to(resp, rlen, b"ERR topic not found\n");
            return;
        }
    };

    let topic = &broker.topics[tidx];
    if partition_idx >= topic.num_partitions {
        copy_to(resp, rlen, b"ERR invalid partition\n");
        return;
    }

    let part = &topic.partitions[partition_idx];
    let mut offset = start_offset;
    let mut fetched: u32 = 0;

    while fetched < count && offset < part.next_offset {
        let index = (offset % MAX_MESSAGES as u64) as usize;
        if part.messages[index].offset != offset {
            copy_to(resp, rlen, b"ERR message at offset ");
            let mut obuf = [0u8; 20];
            let n = format_u64(offset, &mut obuf);
            copy_to(resp, rlen, &obuf[..n]);
            copy_to(resp, rlen, b" was evicted\n");
            break;
        }
        copy_to(resp, rlen, b"MSG ");
        let mut obuf = [0u8; 20];
        let n = format_u64(offset, &mut obuf);
        copy_to(resp, rlen, &obuf[..n]);
        copy_to(resp, rlen, b" ");
        copy_to(resp, rlen, &part.messages[index].data[..part.messages[index].data_len]);
        copy_to(resp, rlen, b"\n");
        offset += 1;
        fetched += 1;
    }
    copy_to(resp, rlen, b"END\n");
}

fn cmd_list_topics(broker: &Broker, resp: &mut [u8], rlen: &mut usize) {
    let mut i = 0;
    while i < MAX_TOPICS {
        if broker.topics[i].active {
            copy_to(resp, rlen, b"TOPIC ");
            copy_to(resp, rlen, &broker.topics[i].name[..broker.topics[i].name_len]);
            copy_to(resp, rlen, b" ");
            let mut nbuf = [0u8; 10];
            let n = format_u32(broker.topics[i].num_partitions as u32, &mut nbuf);
            copy_to(resp, rlen, &nbuf[..n]);
            copy_to(resp, rlen, b"\n");
        }
        i += 1;
    }
    copy_to(resp, rlen, b"END\n");
}

fn cmd_stats(broker: &Broker, resp: &mut [u8], rlen: &mut usize) {
    let mut topic_count: u32 = 0;
    let mut total_messages: u64 = 0;
    let mut i = 0;
    while i < MAX_TOPICS {
        if broker.topics[i].active {
            topic_count += 1;
            let mut p = 0;
            while p < broker.topics[i].num_partitions {
                total_messages += broker.topics[i].partitions[p].next_offset;
                p += 1;
            }
        }
        i += 1;
    }

    let mut offset_count: u32 = 0;
    i = 0;
    while i < MAX_OFFSETS {
        if broker.offsets[i].active {
            offset_count += 1;
        }
        i += 1;
    }

    copy_to(resp, rlen, b"OK topics=");
    let mut nbuf = [0u8; 10];
    let n = format_u32(topic_count, &mut nbuf);
    copy_to(resp, rlen, &nbuf[..n]);
    copy_to(resp, rlen, b" messages=");
    let mut obuf = [0u8; 20];
    let n = format_u64(total_messages, &mut obuf);
    copy_to(resp, rlen, &obuf[..n]);
    copy_to(resp, rlen, b" consumer_offsets=");
    let n = format_u32(offset_count, &mut nbuf);
    copy_to(resp, rlen, &nbuf[..n]);
    copy_to(resp, rlen, b"\n");
}

fn cmd_subscribe(broker: &mut Broker, buf: &[u8], _len: usize, resp: &mut [u8], rlen: &mut usize) {
    // SUBSCRIBE group topic
    let start = skip_spaces(buf, 0);
    let group_end = next_space(buf, start);
    if group_end == start {
        copy_to(resp, rlen, b"ERR missing group\n");
        return;
    }
    let group = &buf[start..group_end];

    let t_start = skip_spaces(buf, group_end);
    let t_end = next_space(buf, t_start);
    if t_end == t_start {
        copy_to(resp, rlen, b"ERR missing topic\n");
        return;
    }
    let topic_name = &buf[t_start..t_end];

    let tidx = match find_topic(broker, topic_name) {
        Some(idx) => idx,
        None => {
            copy_to(resp, rlen, b"ERR topic not found\n");
            return;
        }
    };

    let num_partitions = broker.topics[tidx].num_partitions;
    let mut p = 0;
    while p < num_partitions {
        if find_or_create_offset(broker, group, tidx, p).is_none() {
            copy_to(resp, rlen, b"ERR max consumer offsets reached\n");
            return;
        }
        p += 1;
    }

    copy_to(resp, rlen, b"OK subscribed ");
    copy_to(resp, rlen, group);
    copy_to(resp, rlen, b" to ");
    copy_to(resp, rlen, topic_name);
    copy_to(resp, rlen, b"\n");
}

fn cmd_poll(broker: &mut Broker, buf: &[u8], _len: usize, resp: &mut [u8], rlen: &mut usize) {
    // POLL group topic
    let start = skip_spaces(buf, 0);
    let group_end = next_space(buf, start);
    if group_end == start {
        copy_to(resp, rlen, b"ERR missing group\n");
        return;
    }
    let group = &buf[start..group_end];

    let t_start = skip_spaces(buf, group_end);
    let t_end = next_space(buf, t_start);
    if t_end == t_start {
        copy_to(resp, rlen, b"ERR missing topic\n");
        return;
    }
    let topic_name = &buf[t_start..t_end];

    let tidx = match find_topic(broker, topic_name) {
        Some(idx) => idx,
        None => {
            copy_to(resp, rlen, b"ERR topic not found\n");
            return;
        }
    };

    // Find the partition with lowest committed offset that has messages
    let num_partitions = broker.topics[tidx].num_partitions;
    let mut best_partition: Option<usize> = None;
    let mut best_offset: u64 = u64::MAX;

    let mut p = 0;
    while p < num_partitions {
        let mut oi = 0;
        while oi < MAX_OFFSETS {
            if broker.offsets[oi].active
                && bytes_eq(&broker.offsets[oi].group[..broker.offsets[oi].group_len], group)
                && broker.offsets[oi].topic_idx == tidx
                && broker.offsets[oi].partition_idx == p
            {
                let off = broker.offsets[oi].offset;
                if off < broker.topics[tidx].partitions[p].next_offset && off < best_offset {
                    best_offset = off;
                    best_partition = Some(p);
                }
                break;
            }
            oi += 1;
        }
        p += 1;
    }

    match best_partition {
        Some(pidx) => {
            let part = &broker.topics[tidx].partitions[pidx];
            let index = (best_offset % MAX_MESSAGES as u64) as usize;
            if part.messages[index].offset != best_offset {
                copy_to(resp, rlen, b"ERR message was evicted\n");
                return;
            }
            copy_to(resp, rlen, b"MSG partition=");
            let mut nbuf = [0u8; 10];
            let n = format_u32(pidx as u32, &mut nbuf);
            copy_to(resp, rlen, &nbuf[..n]);
            copy_to(resp, rlen, b" offset=");
            let mut obuf = [0u8; 20];
            let n = format_u64(best_offset, &mut obuf);
            copy_to(resp, rlen, &obuf[..n]);
            copy_to(resp, rlen, b" ");
            copy_to(resp, rlen, &part.messages[index].data[..part.messages[index].data_len]);
            copy_to(resp, rlen, b"\n");

            // Auto-advance the offset
            let mut oi = 0;
            while oi < MAX_OFFSETS {
                if broker.offsets[oi].active
                    && bytes_eq(&broker.offsets[oi].group[..broker.offsets[oi].group_len], group)
                    && broker.offsets[oi].topic_idx == tidx
                    && broker.offsets[oi].partition_idx == pidx
                {
                    broker.offsets[oi].offset = best_offset + 1;
                    break;
                }
                oi += 1;
            }
        }
        None => {
            copy_to(resp, rlen, b"ERR no messages available\n");
        }
    }
}

fn cmd_commit(broker: &mut Broker, buf: &[u8], _len: usize, resp: &mut [u8], rlen: &mut usize) {
    // COMMIT group topic partition offset
    let start = skip_spaces(buf, 0);
    let group_end = next_space(buf, start);
    if group_end == start {
        copy_to(resp, rlen, b"ERR missing group\n");
        return;
    }
    let group = &buf[start..group_end];

    let t_start = skip_spaces(buf, group_end);
    let t_end = next_space(buf, t_start);
    if t_end == t_start {
        copy_to(resp, rlen, b"ERR missing topic\n");
        return;
    }
    let topic_name = &buf[t_start..t_end];

    let p_start = skip_spaces(buf, t_end);
    let p_end = next_space(buf, p_start);
    if p_end == p_start {
        copy_to(resp, rlen, b"ERR missing partition\n");
        return;
    }
    let partition_idx = parse_u32(&buf[p_start..p_end]) as usize;

    let o_start = skip_spaces(buf, p_end);
    let o_end = next_space(buf, o_start);
    if o_end == o_start {
        copy_to(resp, rlen, b"ERR missing offset\n");
        return;
    }
    let commit_offset = parse_u64(&buf[o_start..o_end]);

    let tidx = match find_topic(broker, topic_name) {
        Some(idx) => idx,
        None => {
            copy_to(resp, rlen, b"ERR topic not found\n");
            return;
        }
    };

    if partition_idx >= broker.topics[tidx].num_partitions {
        copy_to(resp, rlen, b"ERR invalid partition\n");
        return;
    }

    match find_or_create_offset(broker, group, tidx, partition_idx) {
        Some(oi) => {
            broker.offsets[oi].offset = commit_offset;
            copy_to(resp, rlen, b"OK committed\n");
        }
        None => {
            copy_to(resp, rlen, b"ERR max consumer offsets reached\n");
        }
    }
}

// ---------------------------------------------------------------------------
// Main server loop
// ---------------------------------------------------------------------------

fn run() {
    unsafe {
        let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if sock < 0 {
            write_all(2, b"tiny-kafka-broker: socket() failed\n");
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
            write_all(2, b"tiny-kafka-broker: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(sock, 16) < 0 {
            write_all(2, b"tiny-kafka-broker: listen() failed\n");
            libc::exit(1);
        }

        write_all(1, b"tiny-kafka-broker listening on port 9092\n");

        // Allocate broker via mmap
        let broker_size = core::mem::size_of::<Broker>();
        let broker_ptr = libc::mmap(
            core::ptr::null_mut(),
            broker_size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        if broker_ptr == libc::MAP_FAILED {
            write_all(2, b"tiny-kafka-broker: mmap() failed\n");
            libc::exit(1);
        }
        let broker = &mut *(broker_ptr as *mut Broker);

        loop {
            let client = libc::accept(sock, core::ptr::null_mut(), core::ptr::null_mut());
            if client < 0 {
                continue;
            }

            // Read line from client
            let mut buf = [0u8; 4096];
            let mut buf_len: usize = 0;

            loop {
                if buf_len >= buf.len() {
                    break;
                }
                let n = libc::read(
                    client,
                    buf.as_mut_ptr().add(buf_len) as *mut libc::c_void,
                    buf.len() - buf_len,
                );
                if n <= 0 {
                    break;
                }
                buf_len += n as usize;
                // Check for newline
                let mut found = false;
                let mut k = 0;
                while k < buf_len {
                    if buf[k] == b'\n' {
                        found = true;
                        break;
                    }
                    k += 1;
                }
                if found {
                    break;
                }
            }

            if buf_len == 0 {
                libc::close(client);
                continue;
            }

            // Trim trailing newline/cr
            let mut line_len = buf_len;
            while line_len > 0 && (buf[line_len - 1] == b'\n' || buf[line_len - 1] == b'\r') {
                line_len -= 1;
            }

            if line_len == 0 {
                libc::close(client);
                continue;
            }

            // Parse command (first word)
            let cmd_end = next_space(&buf, 0);
            let cmd = &buf[..cmd_end];

            // Log command to stdout
            write_all(1, b"CMD: ");
            write_all(1, &buf[..line_len]);
            write_all(1, b"\n");

            let mut resp = [0u8; 4096];
            let mut rlen: usize = 0;

            let args_start = skip_spaces(&buf, cmd_end);

            if bytes_eq(cmd, b"CREATE_TOPIC") {
                cmd_create_topic(broker, &buf[args_start..line_len], line_len - args_start, &mut resp, &mut rlen);
            } else if bytes_eq(cmd, b"PRODUCE") {
                cmd_produce(broker, &buf[args_start..line_len], line_len - args_start, &mut resp, &mut rlen);
            } else if bytes_eq(cmd, b"CONSUME") {
                cmd_consume(broker, &buf[args_start..line_len], line_len - args_start, &mut resp, &mut rlen);
            } else if bytes_eq(cmd, b"LIST_TOPICS") {
                cmd_list_topics(broker, &mut resp, &mut rlen);
            } else if bytes_eq(cmd, b"STATS") {
                cmd_stats(broker, &mut resp, &mut rlen);
            } else if bytes_eq(cmd, b"SUBSCRIBE") {
                cmd_subscribe(broker, &buf[args_start..line_len], line_len - args_start, &mut resp, &mut rlen);
            } else if bytes_eq(cmd, b"POLL") {
                cmd_poll(broker, &buf[args_start..line_len], line_len - args_start, &mut resp, &mut rlen);
            } else if bytes_eq(cmd, b"COMMIT") {
                cmd_commit(broker, &buf[args_start..line_len], line_len - args_start, &mut resp, &mut rlen);
            } else {
                copy_to(&mut resp, &mut rlen, b"ERR unknown command\n");
            }

            write_all(client, &resp[..rlen]);
            libc::close(client);
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::asm!(
        "and rsp, -16",
        "call {run}",
        run = sym run,
        options(noreturn),
    );
}
