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

const PORT: u16 = 9094;

const MAX_TOPICS: usize = 8;
const MAX_PARTITIONS: usize = 4;
const MAX_MESSAGES: usize = 64;
const MAX_MSG_LEN: usize = 128;
const MAX_KEY_LEN: usize = 32;
const MAX_NAME_LEN: usize = 32;
const MAX_CLIENTS: usize = 16;
const MAX_GROUPS: usize = 8;
const MAX_MEMBERS: usize = 8;

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;

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

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    let mut i = 0;
    while i < data.len() {
        hash ^= data[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

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

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[repr(C)]
struct Message {
    key: [u8; MAX_KEY_LEN],
    key_len: usize,
    data: [u8; MAX_MSG_LEN],
    data_len: usize,
    offset: u64,
    timestamp: u64,
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
}

#[repr(C)]
struct GroupMember {
    client_fd: i32,
    partitions: [usize; MAX_PARTITIONS],
    num_partitions: usize,
    active: bool,
}

#[repr(C)]
struct ConsumerGroup {
    name: [u8; MAX_NAME_LEN],
    name_len: usize,
    topic_idx: usize,
    members: [GroupMember; MAX_MEMBERS],
    offsets: [u64; MAX_PARTITIONS],
    active: bool,
}

#[repr(C)]
struct Client {
    fd: i32,
    active: bool,
}

#[repr(C)]
struct Broker {
    topics: [Topic; MAX_TOPICS],
    groups: [ConsumerGroup; MAX_GROUPS],
    clients: [Client; MAX_CLIENTS],
    listen_fd: i32,
}

// ---------------------------------------------------------------------------
// Argv parsing via /proc/self/cmdline
// ---------------------------------------------------------------------------

struct Args {
    buf: [u8; 512],
    len: usize,
    offsets: [usize; 8],
    lengths: [usize; 8],
    count: usize,
}

fn parse_args() -> Args {
    let mut args = Args {
        buf: [0u8; 512],
        len: 0,
        offsets: [0; 8],
        lengths: [0; 8],
        count: 0,
    };
    unsafe {
        let fd = libc::open(
            b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
            libc::O_RDONLY,
        );
        if fd < 0 {
            return args;
        }
        let n = libc::read(fd, args.buf.as_mut_ptr() as *mut libc::c_void, args.buf.len());
        libc::close(fd);
        if n <= 0 {
            return args;
        }
        args.len = n as usize;
    }

    // Skip argv[0]
    let mut i = 0;
    while i < args.len && args.buf[i] != 0 {
        i += 1;
    }
    i += 1; // skip null terminator

    // Parse remaining args
    while i < args.len && args.count < 8 {
        let start = i;
        while i < args.len && args.buf[i] != 0 {
            i += 1;
        }
        let arg_len = i - start;
        if arg_len > 0 {
            args.offsets[args.count] = start;
            args.lengths[args.count] = arg_len;
            args.count += 1;
        }
        i += 1;
    }
    args
}

fn arg_slice<'a>(args: &'a Args, idx: usize) -> &'a [u8] {
    &args.buf[args.offsets[idx]..args.offsets[idx] + args.lengths[idx]]
}

// ---------------------------------------------------------------------------
// Topic helpers
// ---------------------------------------------------------------------------

fn find_topic(broker: &Broker, name: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < MAX_TOPICS {
        if broker.topics[i].active
            && broker.topics[i].name_len == name.len()
            && bytes_eq(&broker.topics[i].name[..broker.topics[i].name_len], name)
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_group(broker: &Broker, name: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < MAX_GROUPS {
        if broker.groups[i].active
            && broker.groups[i].name_len == name.len()
            && bytes_eq(&broker.groups[i].name[..broker.groups[i].name_len], name)
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// Consumer group rebalancing
// ---------------------------------------------------------------------------

fn rebalance_group(broker: &mut Broker, gidx: usize) {
    let topic_idx = broker.groups[gidx].topic_idx;
    let num_partitions = broker.topics[topic_idx].num_partitions;

    // Count active members
    let mut active_count = 0usize;
    let mut active_indices = [0usize; MAX_MEMBERS];
    let mut m = 0;
    while m < MAX_MEMBERS {
        if broker.groups[gidx].members[m].active {
            active_indices[active_count] = m;
            active_count += 1;
        }
        m += 1;
    }

    if active_count == 0 {
        return;
    }

    // Clear all partition assignments
    m = 0;
    while m < MAX_MEMBERS {
        broker.groups[gidx].members[m].num_partitions = 0;
        m += 1;
    }

    // Round-robin assign partitions to active members
    let mut p = 0;
    while p < num_partitions {
        let member_idx = active_indices[p % active_count];
        let member = &mut broker.groups[gidx].members[member_idx];
        member.partitions[member.num_partitions] = p;
        member.num_partitions += 1;
        p += 1;
    }
}

// ---------------------------------------------------------------------------
// Command parsing and execution
// ---------------------------------------------------------------------------

fn skip_spaces(buf: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < buf.len() && (buf[i] == b' ' || buf[i] == b'\t') {
        i += 1;
    }
    i
}

fn next_token_end(buf: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < buf.len() && buf[i] != b' ' && buf[i] != b'\t' && buf[i] != b'\n' && buf[i] != b'\r' {
        i += 1;
    }
    i
}

fn find_line_end(buf: &[u8], len: usize) -> Option<usize> {
    let mut i = 0;
    while i < len {
        if buf[i] == b'\n' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn handle_command(broker: &mut Broker, client_fd: i32, cmd: &[u8], cmd_len: usize) {
    let line = &cmd[..cmd_len];

    // Parse command name
    let start = skip_spaces(line, 0);
    let end = next_token_end(line, start);
    if start == end {
        return;
    }
    let command = &line[start..end];

    let mut resp = [0u8; 2048];
    let mut rlen: usize = 0;

    if bytes_eq(command, b"CREATE_TOPIC") {
        // CREATE_TOPIC name [partitions]
        let s1 = skip_spaces(line, end);
        let e1 = next_token_end(line, s1);
        if s1 == e1 {
            copy_to(&mut resp, &mut rlen, b"ERR missing topic name\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }
        let topic_name = &line[s1..e1];
        if topic_name.len() > MAX_NAME_LEN {
            copy_to(&mut resp, &mut rlen, b"ERR topic name too long\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        // Optional partition count
        let s2 = skip_spaces(line, e1);
        let e2 = next_token_end(line, s2);
        let mut num_parts: usize = 2;
        if s2 < e2 {
            let n = parse_u32(&line[s2..e2]) as usize;
            if n >= 1 && n <= MAX_PARTITIONS {
                num_parts = n;
            }
        }

        // Check if already exists
        if find_topic(broker, topic_name).is_some() {
            copy_to(&mut resp, &mut rlen, b"OK topic already exists\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        // Find free slot
        let mut slot = MAX_TOPICS;
        let mut i = 0;
        while i < MAX_TOPICS {
            if !broker.topics[i].active {
                slot = i;
                break;
            }
            i += 1;
        }
        if slot == MAX_TOPICS {
            copy_to(&mut resp, &mut rlen, b"ERR max topics reached\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        let topic = &mut broker.topics[slot];
        let mut j = 0;
        while j < topic_name.len() {
            topic.name[j] = topic_name[j];
            j += 1;
        }
        topic.name_len = topic_name.len();
        topic.num_partitions = num_parts;
        topic.active = true;

        copy_to(&mut resp, &mut rlen, b"OK topic created partitions=");
        let mut nbuf = [0u8; 10];
        let n = format_u32(num_parts as u32, &mut nbuf);
        copy_to(&mut resp, &mut rlen, &nbuf[..n]);
        copy_to(&mut resp, &mut rlen, b"\n");
        unsafe { write_all(client_fd, &resp[..rlen]); }
    } else if bytes_eq(command, b"PRODUCE") {
        // PRODUCE topic key value
        let s1 = skip_spaces(line, end);
        let e1 = next_token_end(line, s1);
        let s2 = skip_spaces(line, e1);
        let e2 = next_token_end(line, s2);
        let s3 = skip_spaces(line, e2);
        let e3 = next_token_end(line, s3);

        if s1 == e1 || s2 == e2 || s3 == e3 {
            copy_to(&mut resp, &mut rlen, b"ERR usage: PRODUCE topic key value\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        let topic_name = &line[s1..e1];
        let key = &line[s2..e2];
        let value = &line[s3..e3];

        let tidx = match find_topic(broker, topic_name) {
            Some(i) => i,
            None => {
                copy_to(&mut resp, &mut rlen, b"ERR topic not found\n");
                unsafe { write_all(client_fd, &resp[..rlen]); }
                return;
            }
        };

        let num_parts = broker.topics[tidx].num_partitions;
        let hash = fnv1a_hash(key);
        let pidx = (hash % num_parts as u64) as usize;

        let partition = &mut broker.topics[tidx].partitions[pidx];
        let _slot = (partition.head + partition.next_offset as usize) % MAX_MESSAGES;
        // Use next_offset as count when buffer not yet full
        let msg_slot = if partition.next_offset < MAX_MESSAGES as u64 {
            partition.next_offset as usize
        } else {
            // Ring buffer: overwrite oldest
            let s = partition.head;
            partition.head = (partition.head + 1) % MAX_MESSAGES;
            s
        };

        let msg = &mut partition.messages[msg_slot];
        let klen = if key.len() > MAX_KEY_LEN { MAX_KEY_LEN } else { key.len() };
        let mut i = 0;
        while i < klen {
            msg.key[i] = key[i];
            i += 1;
        }
        msg.key_len = klen;

        let vlen = if value.len() > MAX_MSG_LEN { MAX_MSG_LEN } else { value.len() };
        i = 0;
        while i < vlen {
            msg.data[i] = value[i];
            i += 1;
        }
        msg.data_len = vlen;

        msg.offset = partition.next_offset;
        msg.timestamp = unsafe { libc::time(core::ptr::null_mut()) as u64 };

        partition.next_offset += 1;

        copy_to(&mut resp, &mut rlen, b"OK partition=");
        let mut nbuf = [0u8; 10];
        let n = format_u32(pidx as u32, &mut nbuf);
        copy_to(&mut resp, &mut rlen, &nbuf[..n]);
        copy_to(&mut resp, &mut rlen, b" offset=");
        let mut nbuf64 = [0u8; 20];
        let n = format_u64(msg.offset, &mut nbuf64);
        copy_to(&mut resp, &mut rlen, &nbuf64[..n]);
        copy_to(&mut resp, &mut rlen, b"\n");
        unsafe { write_all(client_fd, &resp[..rlen]); }
    } else if bytes_eq(command, b"FETCH") {
        // FETCH topic partition offset [count]
        let s1 = skip_spaces(line, end);
        let e1 = next_token_end(line, s1);
        let s2 = skip_spaces(line, e1);
        let e2 = next_token_end(line, s2);
        let s3 = skip_spaces(line, e2);
        let e3 = next_token_end(line, s3);

        if s1 == e1 || s2 == e2 || s3 == e3 {
            copy_to(&mut resp, &mut rlen, b"ERR usage: FETCH topic partition offset [count]\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        let topic_name = &line[s1..e1];
        let pidx = parse_u32(&line[s2..e2]) as usize;
        let from_offset = parse_u64(&line[s3..e3]);

        let s4 = skip_spaces(line, e3);
        let e4 = next_token_end(line, s4);
        let count: usize = if s4 < e4 {
            parse_u32(&line[s4..e4]) as usize
        } else {
            10
        };

        let tidx = match find_topic(broker, topic_name) {
            Some(i) => i,
            None => {
                copy_to(&mut resp, &mut rlen, b"ERR topic not found\n");
                unsafe { write_all(client_fd, &resp[..rlen]); }
                return;
            }
        };

        if pidx >= broker.topics[tidx].num_partitions {
            copy_to(&mut resp, &mut rlen, b"ERR invalid partition\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        let partition = &broker.topics[tidx].partitions[pidx];
        let mut sent = 0usize;

        // Find messages with offset >= from_offset
        let total = if partition.next_offset < MAX_MESSAGES as u64 {
            partition.next_offset as usize
        } else {
            MAX_MESSAGES
        };

        let mut idx = 0;
        while idx < total && sent < count {
            let msg = &partition.messages[idx];
            if msg.offset >= from_offset {
                copy_to(&mut resp, &mut rlen, b"MSG ");
                let mut nbuf64 = [0u8; 20];
                let n = format_u64(msg.offset, &mut nbuf64);
                copy_to(&mut resp, &mut rlen, &nbuf64[..n]);
                copy_to(&mut resp, &mut rlen, b" ");
                copy_to(&mut resp, &mut rlen, &msg.key[..msg.key_len]);
                copy_to(&mut resp, &mut rlen, b" ");
                copy_to(&mut resp, &mut rlen, &msg.data[..msg.data_len]);
                copy_to(&mut resp, &mut rlen, b"\n");
                sent += 1;
            }
            idx += 1;
        }
        copy_to(&mut resp, &mut rlen, b"END\n");
        unsafe { write_all(client_fd, &resp[..rlen]); }
    } else if bytes_eq(command, b"LIST_TOPICS") {
        let mut i = 0;
        while i < MAX_TOPICS {
            if broker.topics[i].active {
                copy_to(&mut resp, &mut rlen, &broker.topics[i].name[..broker.topics[i].name_len]);
                copy_to(&mut resp, &mut rlen, b" partitions=");
                let mut nbuf = [0u8; 10];
                let n = format_u32(broker.topics[i].num_partitions as u32, &mut nbuf);
                copy_to(&mut resp, &mut rlen, &nbuf[..n]);
                copy_to(&mut resp, &mut rlen, b"\n");
            }
            i += 1;
        }
        copy_to(&mut resp, &mut rlen, b"END\n");
        unsafe { write_all(client_fd, &resp[..rlen]); }
    } else if bytes_eq(command, b"JOIN_GROUP") {
        // JOIN_GROUP group topic
        let s1 = skip_spaces(line, end);
        let e1 = next_token_end(line, s1);
        let s2 = skip_spaces(line, e1);
        let e2 = next_token_end(line, s2);

        if s1 == e1 || s2 == e2 {
            copy_to(&mut resp, &mut rlen, b"ERR usage: JOIN_GROUP group topic\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        let group_name = &line[s1..e1];
        let topic_name = &line[s2..e2];

        let tidx = match find_topic(broker, topic_name) {
            Some(i) => i,
            None => {
                copy_to(&mut resp, &mut rlen, b"ERR topic not found\n");
                unsafe { write_all(client_fd, &resp[..rlen]); }
                return;
            }
        };

        // Find or create group
        let gidx = match find_group(broker, group_name) {
            Some(i) => i,
            None => {
                // Create new group
                let mut slot = MAX_GROUPS;
                let mut i = 0;
                while i < MAX_GROUPS {
                    if !broker.groups[i].active {
                        slot = i;
                        break;
                    }
                    i += 1;
                }
                if slot == MAX_GROUPS {
                    copy_to(&mut resp, &mut rlen, b"ERR max groups reached\n");
                    unsafe { write_all(client_fd, &resp[..rlen]); }
                    return;
                }
                let group = &mut broker.groups[slot];
                let nlen = if group_name.len() > MAX_NAME_LEN { MAX_NAME_LEN } else { group_name.len() };
                let mut j = 0;
                while j < nlen {
                    group.name[j] = group_name[j];
                    j += 1;
                }
                group.name_len = nlen;
                group.topic_idx = tidx;
                group.active = true;
                slot
            }
        };

        // Add member
        let mut member_slot = MAX_MEMBERS;
        let mut m = 0;
        while m < MAX_MEMBERS {
            if !broker.groups[gidx].members[m].active {
                member_slot = m;
                break;
            }
            m += 1;
        }
        if member_slot == MAX_MEMBERS {
            copy_to(&mut resp, &mut rlen, b"ERR group full\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        broker.groups[gidx].members[member_slot].client_fd = client_fd;
        broker.groups[gidx].members[member_slot].active = true;
        broker.groups[gidx].members[member_slot].num_partitions = 0;

        // Rebalance
        rebalance_group(broker, gidx);

        // Find this member and return assigned partitions
        copy_to(&mut resp, &mut rlen, b"OK partitions ");
        let member = &broker.groups[gidx].members[member_slot];
        let mut pi = 0;
        while pi < member.num_partitions {
            if pi > 0 {
                copy_to(&mut resp, &mut rlen, b",");
            }
            let mut nbuf = [0u8; 10];
            let n = format_u32(member.partitions[pi] as u32, &mut nbuf);
            copy_to(&mut resp, &mut rlen, &nbuf[..n]);
            pi += 1;
        }
        copy_to(&mut resp, &mut rlen, b"\n");
        unsafe { write_all(client_fd, &resp[..rlen]); }
    } else if bytes_eq(command, b"LEAVE_GROUP") {
        // LEAVE_GROUP group
        let s1 = skip_spaces(line, end);
        let e1 = next_token_end(line, s1);

        if s1 == e1 {
            copy_to(&mut resp, &mut rlen, b"ERR usage: LEAVE_GROUP group\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        let group_name = &line[s1..e1];

        let gidx = match find_group(broker, group_name) {
            Some(i) => i,
            None => {
                copy_to(&mut resp, &mut rlen, b"ERR group not found\n");
                unsafe { write_all(client_fd, &resp[..rlen]); }
                return;
            }
        };

        // Remove member by client_fd
        let mut m = 0;
        while m < MAX_MEMBERS {
            if broker.groups[gidx].members[m].active
                && broker.groups[gidx].members[m].client_fd == client_fd
            {
                broker.groups[gidx].members[m].active = false;
                broker.groups[gidx].members[m].num_partitions = 0;
            }
            m += 1;
        }

        rebalance_group(broker, gidx);

        copy_to(&mut resp, &mut rlen, b"OK left group\n");
        unsafe { write_all(client_fd, &resp[..rlen]); }
    } else if bytes_eq(command, b"COMMIT") {
        // COMMIT group topic partition offset
        let s1 = skip_spaces(line, end);
        let e1 = next_token_end(line, s1);
        let s2 = skip_spaces(line, e1);
        let e2 = next_token_end(line, s2);
        let s3 = skip_spaces(line, e2);
        let e3 = next_token_end(line, s3);
        let s4 = skip_spaces(line, e3);
        let e4 = next_token_end(line, s4);

        if s1 == e1 || s2 == e2 || s3 == e3 || s4 == e4 {
            copy_to(&mut resp, &mut rlen, b"ERR usage: COMMIT group topic partition offset\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        let group_name = &line[s1..e1];
        let pidx = parse_u32(&line[s3..e3]) as usize;
        let offset = parse_u64(&line[s4..e4]);

        let gidx = match find_group(broker, group_name) {
            Some(i) => i,
            None => {
                copy_to(&mut resp, &mut rlen, b"ERR group not found\n");
                unsafe { write_all(client_fd, &resp[..rlen]); }
                return;
            }
        };

        if pidx < MAX_PARTITIONS {
            broker.groups[gidx].offsets[pidx] = offset;
        }

        copy_to(&mut resp, &mut rlen, b"OK committed\n");
        unsafe { write_all(client_fd, &resp[..rlen]); }
    } else if bytes_eq(command, b"OFFSETS") {
        // OFFSETS group topic
        let s1 = skip_spaces(line, end);
        let e1 = next_token_end(line, s1);
        let s2 = skip_spaces(line, e1);
        let e2 = next_token_end(line, s2);

        if s1 == e1 || s2 == e2 {
            copy_to(&mut resp, &mut rlen, b"ERR usage: OFFSETS group topic\n");
            unsafe { write_all(client_fd, &resp[..rlen]); }
            return;
        }

        let group_name = &line[s1..e1];
        let topic_name = &line[s2..e2];

        let gidx = match find_group(broker, group_name) {
            Some(i) => i,
            None => {
                copy_to(&mut resp, &mut rlen, b"ERR group not found\n");
                unsafe { write_all(client_fd, &resp[..rlen]); }
                return;
            }
        };

        let tidx = match find_topic(broker, topic_name) {
            Some(i) => i,
            None => {
                copy_to(&mut resp, &mut rlen, b"ERR topic not found\n");
                unsafe { write_all(client_fd, &resp[..rlen]); }
                return;
            }
        };

        let num_parts = broker.topics[tidx].num_partitions;
        let mut p = 0;
        while p < num_parts {
            copy_to(&mut resp, &mut rlen, b"OFFSET ");
            let mut nbuf = [0u8; 10];
            let n = format_u32(p as u32, &mut nbuf);
            copy_to(&mut resp, &mut rlen, &nbuf[..n]);
            copy_to(&mut resp, &mut rlen, b" ");
            let mut nbuf64 = [0u8; 20];
            let n = format_u64(broker.groups[gidx].offsets[p], &mut nbuf64);
            copy_to(&mut resp, &mut rlen, &nbuf64[..n]);
            copy_to(&mut resp, &mut rlen, b"\n");
            p += 1;
        }
        copy_to(&mut resp, &mut rlen, b"END\n");
        unsafe { write_all(client_fd, &resp[..rlen]); }
    } else {
        copy_to(&mut resp, &mut rlen, b"ERR unknown command\n");
        unsafe { write_all(client_fd, &resp[..rlen]); }
    }
}

// ---------------------------------------------------------------------------
// Client disconnect: remove from groups and rebalance
// ---------------------------------------------------------------------------

fn on_client_disconnect(broker: &mut Broker, client_fd: i32) {
    let mut g = 0;
    while g < MAX_GROUPS {
        if broker.groups[g].active {
            let mut changed = false;
            let mut m = 0;
            while m < MAX_MEMBERS {
                if broker.groups[g].members[m].active
                    && broker.groups[g].members[m].client_fd == client_fd
                {
                    broker.groups[g].members[m].active = false;
                    broker.groups[g].members[m].num_partitions = 0;
                    changed = true;
                }
                m += 1;
            }
            if changed {
                rebalance_group(broker, g);
            }
        }
        g += 1;
    }
}

// ---------------------------------------------------------------------------
// Broker mode
// ---------------------------------------------------------------------------

fn run_broker() {
    unsafe {
        let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if sock < 0 {
            write_all(2, b"tiny-kafka-cluster: socket() failed\n");
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
            write_all(2, b"tiny-kafka-cluster: bind() failed\n");
            libc::exit(1);
        }

        if libc::listen(sock, 16) < 0 {
            write_all(2, b"tiny-kafka-cluster: listen() failed\n");
            libc::exit(1);
        }

        write_all(1, b"tiny-kafka-cluster broker listening on port 9094\n");

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
            write_all(2, b"tiny-kafka-cluster: mmap() failed\n");
            libc::exit(1);
        }
        let broker = &mut *(broker_ptr as *mut Broker);
        broker.listen_fd = sock;

        // Setup pollfds: [0] = listen socket, [1..MAX_CLIENTS] = clients
        let poll_size = (1 + MAX_CLIENTS) * core::mem::size_of::<libc::pollfd>();
        let poll_ptr = libc::mmap(
            core::ptr::null_mut(),
            poll_size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        if poll_ptr == libc::MAP_FAILED {
            write_all(2, b"tiny-kafka-cluster: mmap() for pollfds failed\n");
            libc::exit(1);
        }
        let pollfds = core::slice::from_raw_parts_mut(
            poll_ptr as *mut libc::pollfd,
            1 + MAX_CLIENTS,
        );

        // Initialize listen pollfd
        pollfds[0].fd = sock;
        pollfds[0].events = libc::POLLIN;
        pollfds[0].revents = 0;

        // Initialize client pollfds as inactive
        let mut i = 0;
        while i < MAX_CLIENTS {
            pollfds[1 + i].fd = -1;
            pollfds[1 + i].events = 0;
            pollfds[1 + i].revents = 0;
            i += 1;
        }

        // Read buffers for clients
        let rbuf_size = MAX_CLIENTS * 1024;
        let rbuf_ptr = libc::mmap(
            core::ptr::null_mut(),
            rbuf_size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        if rbuf_ptr == libc::MAP_FAILED {
            write_all(2, b"tiny-kafka-cluster: mmap() for rbufs failed\n");
            libc::exit(1);
        }
        let rbufs = core::slice::from_raw_parts_mut(rbuf_ptr as *mut u8, rbuf_size);
        let mut rbuf_lens = [0usize; MAX_CLIENTS];

        let nfds = (1 + MAX_CLIENTS) as libc::nfds_t;

        loop {
            let ret = libc::poll(pollfds.as_mut_ptr(), nfds, 1000);
            if ret < 0 {
                continue;
            }

            // Check listen socket for new connections
            if pollfds[0].revents & libc::POLLIN != 0 {
                let client_fd = libc::accept(sock, core::ptr::null_mut(), core::ptr::null_mut());
                if client_fd >= 0 {
                    // Find free client slot
                    let mut slot = MAX_CLIENTS;
                    let mut ci = 0;
                    while ci < MAX_CLIENTS {
                        if !broker.clients[ci].active {
                            slot = ci;
                            break;
                        }
                        ci += 1;
                    }
                    if slot < MAX_CLIENTS {
                        broker.clients[slot].fd = client_fd;
                        broker.clients[slot].active = true;
                        pollfds[1 + slot].fd = client_fd;
                        pollfds[1 + slot].events = libc::POLLIN;
                        pollfds[1 + slot].revents = 0;
                        rbuf_lens[slot] = 0;
                    } else {
                        // No room, close
                        write_all(client_fd, b"ERR server full\n");
                        libc::close(client_fd);
                    }
                }
            }

            // Check client sockets
            let mut ci = 0;
            while ci < MAX_CLIENTS {
                if !broker.clients[ci].active {
                    ci += 1;
                    continue;
                }
                let pfd = &pollfds[1 + ci];

                if pfd.revents & (libc::POLLHUP | libc::POLLERR) != 0 {
                    // Client disconnected
                    let fd = broker.clients[ci].fd;
                    on_client_disconnect(broker, fd);
                    libc::close(fd);
                    broker.clients[ci].active = false;
                    broker.clients[ci].fd = -1;
                    pollfds[1 + ci].fd = -1;
                    pollfds[1 + ci].events = 0;
                    rbuf_lens[ci] = 0;
                    ci += 1;
                    continue;
                }

                if pfd.revents & libc::POLLIN != 0 {
                    let fd = broker.clients[ci].fd;
                    let buf_offset = ci * 1024;
                    let buf_len = rbuf_lens[ci];

                    if buf_len < 1024 {
                        let n = libc::read(
                            fd,
                            rbufs.as_mut_ptr().add(buf_offset + buf_len) as *mut libc::c_void,
                            1024 - buf_len,
                        );
                        if n <= 0 {
                            // Client disconnected
                            on_client_disconnect(broker, fd);
                            libc::close(fd);
                            broker.clients[ci].active = false;
                            broker.clients[ci].fd = -1;
                            pollfds[1 + ci].fd = -1;
                            pollfds[1 + ci].events = 0;
                            rbuf_lens[ci] = 0;
                            ci += 1;
                            continue;
                        }
                        rbuf_lens[ci] += n as usize;
                    }

                    // Process complete lines
                    loop {
                        let buf_start = ci * 1024;
                        let current_len = rbuf_lens[ci];
                        let line_buf = &rbufs[buf_start..buf_start + current_len];

                        match find_line_end(line_buf, current_len) {
                            Some(nl_pos) => {
                                handle_command(broker, fd, line_buf, nl_pos);

                                // Shift remaining data
                                let remaining = current_len - (nl_pos + 1);
                                let mut j = 0;
                                while j < remaining {
                                    rbufs[buf_start + j] = rbufs[buf_start + nl_pos + 1 + j];
                                    j += 1;
                                }
                                rbuf_lens[ci] = remaining;
                            }
                            None => break,
                        }
                    }
                }
                ci += 1;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TCP connect helper
// ---------------------------------------------------------------------------

fn parse_ipv4(host: &[u8]) -> u32 {
    // Parse "A.B.C.D" into network byte order u32
    let mut octets = [0u8; 4];
    let mut octet_idx = 0;
    let mut val: u32 = 0;
    let mut i = 0;
    while i < host.len() && octet_idx < 4 {
        if host[i] == b'.' {
            octets[octet_idx] = val as u8;
            octet_idx += 1;
            val = 0;
        } else if host[i] >= b'0' && host[i] <= b'9' {
            val = val * 10 + (host[i] - b'0') as u32;
        }
        i += 1;
    }
    if octet_idx < 4 {
        octets[octet_idx] = val as u8;
    }
    // Network byte order (little-endian on x86)
    (octets[0] as u32)
        | ((octets[1] as u32) << 8)
        | ((octets[2] as u32) << 16)
        | ((octets[3] as u32) << 24)
}

unsafe fn tcp_connect(host: &[u8], port: u16) -> i32 {
    let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
    if sock < 0 {
        return -1;
    }

    let s_addr = parse_ipv4(host);

    let addr = libc::sockaddr_in {
        sin_family: libc::AF_INET as u16,
        sin_port: port.to_be(),
        sin_addr: libc::in_addr { s_addr },
        sin_zero: [0; 8],
    };

    if libc::connect(
        sock,
        &addr as *const _ as *const libc::sockaddr,
        core::mem::size_of::<libc::sockaddr_in>() as u32,
    ) < 0
    {
        libc::close(sock);
        return -1;
    }

    sock
}

// ---------------------------------------------------------------------------
// Read a line from socket
// ---------------------------------------------------------------------------

unsafe fn read_line(fd: i32, buf: &mut [u8]) -> isize {
    let mut total: usize = 0;
    while total < buf.len() {
        let n = libc::read(fd, buf.as_mut_ptr().add(total) as *mut libc::c_void, 1);
        if n <= 0 {
            return if total > 0 { total as isize } else { n };
        }
        if buf[total] == b'\n' {
            return total as isize;
        }
        total += 1;
    }
    total as isize
}

// ---------------------------------------------------------------------------
// Producer mode
// ---------------------------------------------------------------------------

fn run_producer(host: &[u8], port: u16, topic: &[u8]) {
    unsafe {
        let sock = tcp_connect(host, port);
        if sock < 0 {
            write_all(2, b"tiny-kafka-cluster producer: connect failed\n");
            libc::exit(1);
        }

        write_all(1, b"tiny-kafka-cluster producer: connected\n");

        // Auto-create topic with 2 partitions
        let mut cmd = [0u8; 256];
        let mut clen: usize = 0;
        copy_to(&mut cmd, &mut clen, b"CREATE_TOPIC ");
        copy_to(&mut cmd, &mut clen, topic);
        copy_to(&mut cmd, &mut clen, b" 2\n");
        write_all(sock, &cmd[..clen]);

        // Read response
        let mut rbuf = [0u8; 256];
        let _n = read_line(sock, &mut rbuf);

        write_all(1, b"tiny-kafka-cluster producer: topic ready\n");

        // Produce messages in a loop
        let mut counter: u32 = 0;
        let keys: [&[u8]; 4] = [b"key-0", b"key-1", b"key-2", b"key-3"];

        loop {
            let mut msg = [0u8; 256];
            let mut mlen: usize = 0;
            copy_to(&mut msg, &mut mlen, b"PRODUCE ");
            copy_to(&mut msg, &mut mlen, topic);
            copy_to(&mut msg, &mut mlen, b" ");
            copy_to(&mut msg, &mut mlen, keys[(counter % 4) as usize]);
            copy_to(&mut msg, &mut mlen, b" value-");
            let mut nbuf = [0u8; 10];
            let n = format_u32(counter, &mut nbuf);
            copy_to(&mut msg, &mut mlen, &nbuf[..n]);
            copy_to(&mut msg, &mut mlen, b"\n");
            write_all(sock, &msg[..mlen]);

            // Read response
            let rn = read_line(sock, &mut rbuf);
            if rn <= 0 {
                write_all(2, b"tiny-kafka-cluster producer: connection lost\n");
                libc::exit(1);
            }

            // Print response
            write_all(1, b"produced: ");
            write_all(1, &rbuf[..rn as usize]);
            write_all(1, b"\n");

            counter += 1;

            // Sleep 1 second
            let ts = libc::timespec {
                tv_sec: 1,
                tv_nsec: 0,
            };
            libc::nanosleep(&ts, core::ptr::null_mut());
        }
    }
}

// ---------------------------------------------------------------------------
// Consumer mode
// ---------------------------------------------------------------------------

fn run_consumer(host: &[u8], port: u16, group: &[u8], topic: &[u8]) {
    unsafe {
        let sock = tcp_connect(host, port);
        if sock < 0 {
            write_all(2, b"tiny-kafka-cluster consumer: connect failed\n");
            libc::exit(1);
        }

        write_all(1, b"tiny-kafka-cluster consumer: connected\n");

        // Join consumer group
        let mut cmd = [0u8; 256];
        let mut clen: usize = 0;
        copy_to(&mut cmd, &mut clen, b"JOIN_GROUP ");
        copy_to(&mut cmd, &mut clen, group);
        copy_to(&mut cmd, &mut clen, b" ");
        copy_to(&mut cmd, &mut clen, topic);
        copy_to(&mut cmd, &mut clen, b"\n");
        write_all(sock, &cmd[..clen]);

        // Read response: OK partitions 0,2
        let mut rbuf = [0u8; 256];
        let rn = read_line(sock, &mut rbuf);
        if rn <= 0 {
            write_all(2, b"tiny-kafka-cluster consumer: join failed\n");
            libc::exit(1);
        }

        // Parse assigned partitions
        let resp = &rbuf[..rn as usize];
        write_all(1, b"joined: ");
        write_all(1, resp);
        write_all(1, b"\n");

        // Find "partitions " in response and parse comma-separated numbers
        let mut assigned_partitions = [0usize; MAX_PARTITIONS];
        let mut num_assigned: usize = 0;

        // Look for "partitions " prefix
        let prefix = b"OK partitions ";
        let mut found = false;
        if resp.len() >= prefix.len() {
            let mut match_ok = true;
            let mut pi = 0;
            while pi < prefix.len() {
                if resp[pi] != prefix[pi] {
                    match_ok = false;
                    break;
                }
                pi += 1;
            }
            if match_ok {
                found = true;
                let mut pos = prefix.len();
                while pos < resp.len() && num_assigned < MAX_PARTITIONS {
                    let mut val: usize = 0;
                    let mut got_digit = false;
                    while pos < resp.len() && resp[pos] >= b'0' && resp[pos] <= b'9' {
                        val = val * 10 + (resp[pos] - b'0') as usize;
                        pos += 1;
                        got_digit = true;
                    }
                    if got_digit {
                        assigned_partitions[num_assigned] = val;
                        num_assigned += 1;
                    }
                    if pos < resp.len() && resp[pos] == b',' {
                        pos += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        if !found || num_assigned == 0 {
            write_all(2, b"tiny-kafka-cluster consumer: no partitions assigned\n");
            libc::exit(1);
        }

        // Track offsets per assigned partition
        let mut offsets = [0u64; MAX_PARTITIONS];

        // Poll loop
        loop {
            let mut pi = 0;
            while pi < num_assigned {
                let part = assigned_partitions[pi];

                // FETCH topic partition offset 10
                let mut fcmd = [0u8; 256];
                let mut flen: usize = 0;
                copy_to(&mut fcmd, &mut flen, b"FETCH ");
                copy_to(&mut fcmd, &mut flen, topic);
                copy_to(&mut fcmd, &mut flen, b" ");
                let mut nbuf = [0u8; 10];
                let n = format_u32(part as u32, &mut nbuf);
                copy_to(&mut fcmd, &mut flen, &nbuf[..n]);
                copy_to(&mut fcmd, &mut flen, b" ");
                let mut nbuf64 = [0u8; 20];
                let n = format_u64(offsets[pi], &mut nbuf64);
                copy_to(&mut fcmd, &mut flen, &nbuf64[..n]);
                copy_to(&mut fcmd, &mut flen, b" 10\n");
                write_all(sock, &fcmd[..flen]);

                // Read MSG lines until END
                let mut max_offset = offsets[pi];
                loop {
                    let mut lbuf = [0u8; 512];
                    let ln = read_line(sock, &mut lbuf);
                    if ln <= 0 {
                        write_all(2, b"tiny-kafka-cluster consumer: connection lost\n");
                        libc::exit(1);
                    }
                    let line = &lbuf[..ln as usize];

                    if bytes_eq(line, b"END") {
                        break;
                    }

                    // Print MSG line
                    write_all(1, line);
                    write_all(1, b"\n");

                    // Parse offset from "MSG offset key data"
                    if line.len() > 4 && line[0] == b'M' && line[1] == b'S' && line[2] == b'G' && line[3] == b' ' {
                        let s = skip_spaces(line, 4);
                        let e = next_token_end(line, s);
                        if s < e {
                            let off = parse_u64(&line[s..e]);
                            if off + 1 > max_offset {
                                max_offset = off + 1;
                            }
                        }
                    }
                }

                // Commit offset if we made progress
                if max_offset > offsets[pi] {
                    offsets[pi] = max_offset;

                    let mut ccmd = [0u8; 256];
                    let mut cclen: usize = 0;
                    copy_to(&mut ccmd, &mut cclen, b"COMMIT ");
                    copy_to(&mut ccmd, &mut cclen, group);
                    copy_to(&mut ccmd, &mut cclen, b" ");
                    copy_to(&mut ccmd, &mut cclen, topic);
                    copy_to(&mut ccmd, &mut cclen, b" ");
                    let mut nbuf2 = [0u8; 10];
                    let n2 = format_u32(part as u32, &mut nbuf2);
                    copy_to(&mut ccmd, &mut cclen, &nbuf2[..n2]);
                    copy_to(&mut ccmd, &mut cclen, b" ");
                    let mut nbuf642 = [0u8; 20];
                    let n2 = format_u64(max_offset, &mut nbuf642);
                    copy_to(&mut ccmd, &mut cclen, &nbuf642[..n2]);
                    copy_to(&mut ccmd, &mut cclen, b"\n");
                    write_all(sock, &ccmd[..cclen]);

                    // Read commit response
                    let mut crbuf = [0u8; 128];
                    let _cn = read_line(sock, &mut crbuf);
                }

                pi += 1;
            }

            // Sleep 500ms between polls
            let ts = libc::timespec {
                tv_sec: 0,
                tv_nsec: 500_000_000,
            };
            libc::nanosleep(&ts, core::ptr::null_mut());
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[inline(never)]
fn run() {
    let args = parse_args();

    if args.count == 0 {
        // Default: broker mode
        run_broker();
        return;
    }

    let mode = arg_slice(&args, 0);

    if bytes_eq(mode, b"broker") {
        run_broker();
    } else if bytes_eq(mode, b"producer") {
        // producer host port topic
        if args.count < 4 {
            unsafe {
                write_all(2, b"usage: tiny-kafka-cluster producer <host> <port> <topic>\n");
                libc::exit(1);
            }
        }
        let host = arg_slice(&args, 1);
        let port = parse_u32(arg_slice(&args, 2)) as u16;
        let topic = arg_slice(&args, 3);
        run_producer(host, port, topic);
    } else if bytes_eq(mode, b"consumer") {
        // consumer host port group topic
        if args.count < 5 {
            unsafe {
                write_all(2, b"usage: tiny-kafka-cluster consumer <host> <port> <group> <topic>\n");
                libc::exit(1);
            }
        }
        let host = arg_slice(&args, 1);
        let port = parse_u32(arg_slice(&args, 2)) as u16;
        let group = arg_slice(&args, 3);
        let topic = arg_slice(&args, 4);
        run_consumer(host, port, group, topic);
    } else {
        unsafe {
            write_all(2, b"unknown mode (use: broker, producer, consumer)\n");
            libc::exit(1);
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
