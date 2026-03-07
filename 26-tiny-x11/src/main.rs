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

fn format_hex(n: u32, buf: &mut [u8; 10]) -> usize {
    // Format as 0xNNNNNNNN
    buf[0] = b'0';
    buf[1] = b'x';
    let mut i = 0;
    while i < 8 {
        let nibble = (n >> (28 - i * 4)) & 0xf;
        buf[2 + i] = if nibble < 10 {
            b'0' + nibble as u8
        } else {
            b'a' + (nibble - 10) as u8
        };
        i += 1;
    }
    10
}

// ---------------------------------------------------------------------------
// Byte-order helpers (X11 uses little-endian on x86_64)
// ---------------------------------------------------------------------------

fn put_u16_le(buf: &mut [u8], offset: usize, val: u16) {
    buf[offset] = val as u8;
    buf[offset + 1] = (val >> 8) as u8;
}

fn put_u32_le(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset] = val as u8;
    buf[offset + 1] = (val >> 8) as u8;
    buf[offset + 2] = (val >> 16) as u8;
    buf[offset + 3] = (val >> 24) as u8;
}

fn get_u16_le(buf: &[u8], offset: usize) -> u16 {
    buf[offset] as u16 | (buf[offset + 1] as u16) << 8
}

fn get_u32_le(buf: &[u8], offset: usize) -> u32 {
    buf[offset] as u32
        | (buf[offset + 1] as u32) << 8
        | (buf[offset + 2] as u32) << 16
        | (buf[offset + 3] as u32) << 24
}

// ---------------------------------------------------------------------------
// Pad to 4-byte boundary
// ---------------------------------------------------------------------------

fn pad4(n: usize) -> usize {
    (n + 3) & !3
}

// ---------------------------------------------------------------------------
// X11 connection
// ---------------------------------------------------------------------------

unsafe fn x11_connect() -> i32 {
    let fd = libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0);
    if fd < 0 {
        write_all(2, b"tiny-x11: socket() failed\n");
        libc::exit(1);
    }

    // Connect to /tmp/.X11-unix/X0
    // sockaddr_un: sa_family (2 bytes) + sun_path (108 bytes)
    let mut addr: libc::sockaddr_un = core::mem::zeroed();
    addr.sun_family = libc::AF_UNIX as u16;
    let path = b"/tmp/.X11-unix/X0";
    let mut i = 0;
    while i < path.len() {
        addr.sun_path[i] = path[i] as i8;
        i += 1;
    }

    let addr_len = 2 + path.len() + 1; // family + path + null
    if libc::connect(fd, &addr as *const _ as *const libc::sockaddr, addr_len as u32) < 0 {
        write_all(2, b"tiny-x11: connect() failed -- is X11 running?\n");
        write_all(2, b"  Try: xhost +local:\n");
        libc::exit(1);
    }

    fd
}

unsafe fn read_exact(fd: i32, buf: &mut [u8], len: usize) {
    let mut done = 0;
    while done < len {
        let n = libc::read(
            fd,
            buf.as_mut_ptr().add(done) as *mut libc::c_void,
            len - done,
        );
        if n <= 0 {
            write_all(2, b"tiny-x11: read failed\n");
            libc::exit(1);
        }
        done += n as usize;
    }
}

// ---------------------------------------------------------------------------
// Main logic
// ---------------------------------------------------------------------------

#[inline(never)]
fn run() {
    unsafe {
        let fd = x11_connect();

        // --- Connection setup (little-endian, protocol 11.0, no auth) ---
        let mut setup_req = [0u8; 12];
        setup_req[0] = b'l'; // little-endian
        put_u16_le(&mut setup_req, 2, 11); // major version
        put_u16_le(&mut setup_req, 4, 0); // minor version
        // auth name/data lengths = 0 (bytes 6-11 stay zero)
        write_all(fd, &setup_req);

        // Read first 8 bytes of response to check status and get length
        let mut resp_header = [0u8; 8];
        read_exact(fd, &mut resp_header, 8);

        if resp_header[0] != 1 {
            write_all(2, b"tiny-x11: connection refused by X server\n");
            write_all(2, b"  Try: xhost +local:\n");
            libc::exit(1);
        }

        // Additional data length in 4-byte units
        let extra_len = get_u16_le(&resp_header, 6) as usize * 4;
        let mut setup_data = [0u8; 4096];
        read_exact(fd, &mut setup_data, extra_len);

        // Parse setup response
        // Bytes 0-3 of setup_data:
        //   0-3: release number
        //   4-7: resource_id_base
        //   8-11: resource_id_mask
        let resource_id_base = get_u32_le(&setup_data, 4);
        let vendor_length = get_u16_le(&setup_data, 16) as usize;
        let num_pixmap_formats = setup_data[21] as usize;

        // Screen info starts after fixed header (32 bytes from setup_data start)
        // + padded vendor string + pixmap formats
        let screen_offset = 32 + pad4(vendor_length) + (num_pixmap_formats * 8);

        // Extract from first screen structure
        let root_window = get_u32_le(&setup_data, screen_offset);
        let white_pixel = get_u32_le(&setup_data, screen_offset + 8);
        let root_visual = get_u32_le(&setup_data, screen_offset + 32);
        let root_depth = setup_data[screen_offset + 38];

        let mut hex_buf = [0u8; 10];

        write_all(1, b"Connected to X11 display :0\n");
        write_all(1, b"Root window: ");
        let len = format_hex(root_window, &mut hex_buf);
        write_all(1, &hex_buf[..len]);
        write_all(1, b"\n");

        // --- InternAtom: WM_PROTOCOLS ---
        let wm_protocols_name = b"WM_PROTOCOLS";
        let mut intern1 = [0u8; 24]; // 8 + pad4(12) = 20, round up to be safe
        intern1[0] = 16; // InternAtom opcode
        intern1[1] = 0; // only_if_exists = false
        let req_len1 = (8 + pad4(wm_protocols_name.len())) / 4;
        put_u16_le(&mut intern1, 2, req_len1 as u16);
        put_u16_le(&mut intern1, 4, wm_protocols_name.len() as u16);
        let mut j = 0;
        while j < wm_protocols_name.len() {
            intern1[8 + j] = wm_protocols_name[j];
            j += 1;
        }
        write_all(fd, &intern1[..req_len1 * 4]);

        // --- InternAtom: WM_DELETE_WINDOW ---
        let wm_delete_name = b"WM_DELETE_WINDOW";
        let mut intern2 = [0u8; 28]; // 8 + pad4(16) = 24
        intern2[0] = 16; // InternAtom opcode
        intern2[1] = 0;
        let req_len2 = (8 + pad4(wm_delete_name.len())) / 4;
        put_u16_le(&mut intern2, 2, req_len2 as u16);
        put_u16_le(&mut intern2, 4, wm_delete_name.len() as u16);
        j = 0;
        while j < wm_delete_name.len() {
            intern2[8 + j] = wm_delete_name[j];
            j += 1;
        }
        write_all(fd, &intern2[..req_len2 * 4]);

        // Read InternAtom replies (32 bytes each)
        let mut reply1 = [0u8; 32];
        read_exact(fd, &mut reply1, 32);
        let wm_protocols_atom = get_u32_le(&reply1, 8);

        let mut reply2 = [0u8; 32];
        read_exact(fd, &mut reply2, 32);
        let wm_delete_atom = get_u32_le(&reply2, 8);

        // --- CreateWindow ---
        let wid = resource_id_base; // our window id
        let width: u16 = 400;
        let height: u16 = 300;

        // CreateWindow request: opcode 1
        // Fixed part: 32 bytes + 4 bytes per value in value_mask
        // value_mask: BackPixel (0x00000002) | EventMask (0x00000800) = 0x00000802
        let mut cw = [0u8; 40]; // 32 + 2 values * 4 = 40
        cw[0] = 1; // CreateWindow opcode
        cw[1] = root_depth; // depth
        put_u16_le(&mut cw, 2, 10); // request length in 4-byte units (40/4)
        put_u32_le(&mut cw, 4, wid); // window id
        put_u32_le(&mut cw, 8, root_window); // parent
        put_u16_le(&mut cw, 12, 100); // x
        put_u16_le(&mut cw, 14, 100); // y
        put_u16_le(&mut cw, 16, width);
        put_u16_le(&mut cw, 18, height);
        put_u16_le(&mut cw, 20, 0); // border width
        put_u16_le(&mut cw, 22, 1); // class: InputOutput
        put_u32_le(&mut cw, 24, root_visual); // visual
        put_u32_le(&mut cw, 28, 0x00000802); // value_mask: BackPixel | EventMask

        // Values (in mask bit order):
        put_u32_le(&mut cw, 32, white_pixel); // BackPixel
        // EventMask: KeyPress (0x01) | Exposure (0x8000) | StructureNotify (0x20000)
        put_u32_le(&mut cw, 36, 0x00028001);
        write_all(fd, &cw);

        let mut num_buf = [0u8; 10];
        write_all(1, b"Window created: ");
        let len = format_hex(wid, &mut hex_buf);
        write_all(1, &hex_buf[..len]);
        write_all(1, b" (");
        let len = format_u32(width as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"x");
        let len = format_u32(height as u32, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b")\n");

        // --- ChangeProperty: WM_NAME = "tiny-x11" ---
        let title = b"tiny-x11";
        let mut cp_name = [0u8; 36]; // 24 + pad4(8) = 32, round up
        cp_name[0] = 18; // ChangeProperty opcode
        cp_name[1] = 0; // Replace
        let cp_name_len = (24 + pad4(title.len())) / 4;
        put_u16_le(&mut cp_name, 2, cp_name_len as u16);
        put_u32_le(&mut cp_name, 4, wid); // window
        put_u32_le(&mut cp_name, 8, 39); // property: WM_NAME (predefined atom 39)
        put_u32_le(&mut cp_name, 12, 31); // type: STRING (predefined atom 31)
        cp_name[16] = 8; // format: 8-bit
        put_u32_le(&mut cp_name, 20, title.len() as u32); // data length
        j = 0;
        while j < title.len() {
            cp_name[24 + j] = title[j];
            j += 1;
        }
        write_all(fd, &cp_name[..cp_name_len * 4]);

        // --- ChangeProperty: WM_PROTOCOLS = [WM_DELETE_WINDOW] ---
        let mut cp_proto = [0u8; 32]; // 24 + 4 = 28, padded
        cp_proto[0] = 18; // ChangeProperty
        cp_proto[1] = 0; // Replace
        put_u16_le(&mut cp_proto, 2, 7); // request length: (24+4)/4 = 7
        put_u32_le(&mut cp_proto, 4, wid); // window
        put_u32_le(&mut cp_proto, 8, wm_protocols_atom); // property
        put_u32_le(&mut cp_proto, 12, 4); // type: ATOM (predefined atom 4)
        cp_proto[16] = 32; // format: 32-bit
        put_u32_le(&mut cp_proto, 20, 1); // 1 atom
        put_u32_le(&mut cp_proto, 24, wm_delete_atom); // WM_DELETE_WINDOW
        write_all(fd, &cp_proto[..28]);

        // --- MapWindow ---
        let mut map = [0u8; 8];
        map[0] = 8; // MapWindow opcode
        put_u16_le(&mut map, 2, 2); // length: 2 words
        put_u32_le(&mut map, 4, wid);
        write_all(fd, &map);

        write_all(1, b"Window mapped\n");

        // --- Event loop ---
        let mut event = [0u8; 32];
        loop {
            read_exact(fd, &mut event, 32);
            let event_code = event[0] & 0x7f; // mask out send_event bit

            if event_code == 12 {
                // Expose
                write_all(1, b"[event] Expose\n");
            } else if event_code == 2 {
                // KeyPress
                write_all(1, b"Key pressed, exiting\n");
                break;
            } else if event_code == 33 {
                // ClientMessage: type=WM_PROTOCOLS at offset 8, data[0] at offset 12
                let data_atom = get_u32_le(&event, 12);
                if data_atom == wm_delete_atom {
                    write_all(1, b"Close button pressed, exiting\n");
                    break;
                }
            }
            // Ignore other events (ConfigureNotify, MapNotify, etc.)
        }

        libc::close(fd);
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
