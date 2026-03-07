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
            libc::exit(if ret == 0 { 1 } else { 0 });
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

fn format_hex(n: u64, buf: &mut [u8; 16]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let hex = b"0123456789abcdef";
    let mut val = n;
    let mut pos = 16;
    while val > 0 {
        pos -= 1;
        buf[pos] = hex[(val & 0xF) as usize];
        val >>= 4;
    }
    let len = 16 - pos;
    let mut i = 0;
    while i < len {
        buf[i] = buf[pos + i];
        i += 1;
    }
    len
}

/// Simple bump allocator backed by a static byte array.
/// Demonstrates heap-like allocation without std or a global allocator.
struct BumpAlloc {
    heap: [u8; 4096],
    offset: usize,
}

impl BumpAlloc {
    const fn new() -> Self {
        BumpAlloc {
            heap: [0u8; 4096],
            offset: 0,
        }
    }

    /// Allocate `size` bytes with given alignment. Returns a pointer or null.
    fn alloc(&mut self, size: usize, align: usize) -> *mut u8 {
        // Align up
        let aligned = (self.offset + align - 1) & !(align - 1);
        if aligned + size > self.heap.len() {
            return core::ptr::null_mut();
        }
        let ptr = unsafe { self.heap.as_mut_ptr().add(aligned) };
        self.offset = aligned + size;
        ptr
    }

    fn used(&self) -> usize {
        self.offset
    }

    fn capacity(&self) -> usize {
        self.heap.len()
    }

    /// Reset the allocator, freeing all allocations at once.
    fn reset(&mut self) {
        self.offset = 0;
    }
}

#[inline(never)]
fn run() {
    let mut alloc = BumpAlloc::new();

    unsafe {
        write_all(1, b"=== Tiny Bump Allocator Demo ===\n\n");

        // Allocate some integers
        let a = alloc.alloc(4, 4) as *mut u32;
        let b = alloc.alloc(4, 4) as *mut u32;
        let c = alloc.alloc(8, 8) as *mut u64;

        if a.is_null() || b.is_null() || c.is_null() {
            write_all(2, b"allocation failed\n");
            libc::exit(1);
        }

        *a = 42;
        *b = 100;
        *c = 123456789;

        write_all(1, b"Allocated 3 values:\n");

        // Print a
        write_all(1, b"  u32 @ 0x");
        let mut hex_buf = [0u8; 16];
        let len = format_hex(a as u64, &mut hex_buf);
        write_all(1, &hex_buf[..len]);
        write_all(1, b" = ");
        let mut num_buf = [0u8; 20];
        let len = format_u64(*a as u64, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"\n");

        // Print b
        write_all(1, b"  u32 @ 0x");
        let len = format_hex(b as u64, &mut hex_buf);
        write_all(1, &hex_buf[..len]);
        write_all(1, b" = ");
        let len = format_u64(*b as u64, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"\n");

        // Print c
        write_all(1, b"  u64 @ 0x");
        let len = format_hex(c as u64, &mut hex_buf);
        write_all(1, &hex_buf[..len]);
        write_all(1, b" = ");
        let len = format_u64(*c, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b"\n");

        // Allocate a string buffer
        let msg = b"Hello from the bump allocator!";
        let s = alloc.alloc(msg.len(), 1);
        if s.is_null() {
            write_all(2, b"allocation failed\n");
            libc::exit(1);
        }
        core::ptr::copy_nonoverlapping(msg.as_ptr(), s, msg.len());

        write_all(1, b"  str @ 0x");
        let len = format_hex(s as u64, &mut hex_buf);
        write_all(1, &hex_buf[..len]);
        write_all(1, b" = \"");
        write_all(1, core::slice::from_raw_parts(s, msg.len()));
        write_all(1, b"\"\n\n");

        // Print stats
        write_all(1, b"Heap used: ");
        let len = format_u64(alloc.used() as u64, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" / ");
        let len = format_u64(alloc.capacity() as u64, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" bytes\n");

        // Reset and show it works
        alloc.reset();
        write_all(1, b"After reset: ");
        let len = format_u64(alloc.used() as u64, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" / ");
        let len = format_u64(alloc.capacity() as u64, &mut num_buf);
        write_all(1, &num_buf[..len]);
        write_all(1, b" bytes\n");

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
