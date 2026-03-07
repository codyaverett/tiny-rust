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

// SHA-256 constants: first 32 bits of the fractional parts of the cube roots
// of the first 64 primes
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
    0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
    0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
    0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
    0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
    0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
    0xc67178f2,
];

// Initial hash values: first 32 bits of the fractional parts of the square
// roots of the first 8 primes
const H_INIT: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
    0x5be0cd19,
];

struct Sha256 {
    state: [u32; 8],
    buf: [u8; 64],
    buf_len: usize,
    total_len: u64,
}

impl Sha256 {
    fn new() -> Self {
        Sha256 {
            state: H_INIT,
            buf: [0u8; 64],
            buf_len: 0,
            total_len: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.total_len += data.len() as u64;
        let mut offset = 0;

        // Fill partial buffer
        if self.buf_len > 0 {
            let space = 64 - self.buf_len;
            let to_copy = if data.len() < space { data.len() } else { space };
            let mut i = 0;
            while i < to_copy {
                self.buf[self.buf_len + i] = data[i];
                i += 1;
            }
            self.buf_len += to_copy;
            offset += to_copy;
            if self.buf_len == 64 {
                let block = self.buf;
                self.compress(&block);
                self.buf_len = 0;
            }
        }

        // Process full blocks
        while offset + 64 <= data.len() {
            let mut block = [0u8; 64];
            let mut i = 0;
            while i < 64 {
                block[i] = data[offset + i];
                i += 1;
            }
            self.compress(&block);
            offset += 64;
        }

        // Store remainder
        while offset < data.len() {
            self.buf[self.buf_len] = data[offset];
            self.buf_len += 1;
            offset += 1;
        }
    }

    fn finalize(&mut self) -> [u8; 32] {
        let bit_len = self.total_len * 8;

        // Padding: append 1 bit
        self.buf[self.buf_len] = 0x80;
        self.buf_len += 1;

        // If not enough room for 8-byte length, pad and compress
        if self.buf_len > 56 {
            while self.buf_len < 64 {
                self.buf[self.buf_len] = 0;
                self.buf_len += 1;
            }
            let block = self.buf;
            self.compress(&block);
            self.buf_len = 0;
        }

        // Pad to 56 bytes
        while self.buf_len < 56 {
            self.buf[self.buf_len] = 0;
            self.buf_len += 1;
        }

        // Append length in bits as big-endian u64
        self.buf[56] = (bit_len >> 56) as u8;
        self.buf[57] = (bit_len >> 48) as u8;
        self.buf[58] = (bit_len >> 40) as u8;
        self.buf[59] = (bit_len >> 32) as u8;
        self.buf[60] = (bit_len >> 24) as u8;
        self.buf[61] = (bit_len >> 16) as u8;
        self.buf[62] = (bit_len >> 8) as u8;
        self.buf[63] = bit_len as u8;

        let block = self.buf;
        self.compress(&block);

        // Produce final hash
        let mut result = [0u8; 32];
        let mut i = 0;
        while i < 8 {
            result[i * 4] = (self.state[i] >> 24) as u8;
            result[i * 4 + 1] = (self.state[i] >> 16) as u8;
            result[i * 4 + 2] = (self.state[i] >> 8) as u8;
            result[i * 4 + 3] = self.state[i] as u8;
            i += 1;
        }
        result
    }

    fn compress(&mut self, block: &[u8; 64]) {
        // Prepare message schedule
        let mut w = [0u32; 64];
        let mut i = 0;
        while i < 16 {
            w[i] = (block[i * 4] as u32) << 24
                | (block[i * 4 + 1] as u32) << 16
                | (block[i * 4 + 2] as u32) << 8
                | (block[i * 4 + 3] as u32);
            i += 1;
        }
        while i < 64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
            i += 1;
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        i = 0;
        while i < 64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
            i += 1;
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

#[inline(never)]
fn run() {
    unsafe {
        let mut sha = Sha256::new();
        let mut buf = [0u8; 4096];

        loop {
            let n = libc::read(0, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n <= 0 {
                break;
            }
            sha.update(&buf[..n as usize]);
        }

        let hash = sha.finalize();

        // Format as 64-char hex string
        let mut hex_out = [0u8; 66]; // 64 hex + newline + spare
        let mut i = 0;
        while i < 32 {
            hex_out[i * 2] = HEX[(hash[i] >> 4) as usize];
            hex_out[i * 2 + 1] = HEX[(hash[i] & 0x0f) as usize];
            i += 1;
        }
        hex_out[64] = b' ';
        hex_out[65] = b'-';
        write_all(1, &hex_out);
        write_all(1, b"\n");
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
