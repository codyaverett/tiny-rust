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
// Constants — GPT-2 Small (124M parameters)
// ---------------------------------------------------------------------------

const VOCAB_SIZE: usize = 50257;
const EMBED_DIM: usize = 768;
const NUM_LAYERS: usize = 12;
const NUM_HEADS: usize = 12;
const HEAD_DIM: usize = EMBED_DIM / NUM_HEADS; // 64
const FF_DIM: usize = 4 * EMBED_DIM; // 3072
const MAX_SEQ_LEN: usize = 1024;

// Weight file magic numbers
const WEIGHT_MAGIC: u32 = 0x47505432; // "GPT2"
const TOKEN_MAGIC: u32 = 0x544F4B4E; // "TOKE"

// ---------------------------------------------------------------------------
// Syscall helpers
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

fn print(s: &[u8]) {
    unsafe {
        write_all(1, s);
    }
}

fn eprint(s: &[u8]) {
    unsafe {
        write_all(2, s);
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

fn print_u32(n: u32) {
    let mut buf = [0u8; 10];
    let len = format_u32(n, &mut buf);
    print(&buf[..len]);
}

unsafe fn open_file(path: &[u8]) -> i32 {
    // path must be null-terminated
    libc::open(path.as_ptr() as *const libc::c_char, libc::O_RDONLY)
}

unsafe fn file_size(fd: i32) -> usize {
    let off = libc::lseek(fd, 0, libc::SEEK_END);
    libc::lseek(fd, 0, libc::SEEK_SET);
    off as usize
}

unsafe fn mmap_file(fd: i32, size: usize) -> *const u8 {
    let ptr = libc::mmap(
        core::ptr::null_mut(),
        size,
        libc::PROT_READ,
        libc::MAP_PRIVATE,
        fd,
        0,
    );
    if ptr == libc::MAP_FAILED {
        core::ptr::null()
    } else {
        ptr as *const u8
    }
}

unsafe fn mmap_anon(size: usize) -> *mut u8 {
    let ptr = libc::mmap(
        core::ptr::null_mut(),
        size,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
        -1,
        0,
    );
    if ptr == libc::MAP_FAILED {
        core::ptr::null_mut()
    } else {
        ptr as *mut u8
    }
}

// ---------------------------------------------------------------------------
// Math approximations
// ---------------------------------------------------------------------------

/// exp(x) via repeated squaring: (1 + x/1024)^1024  (10 squarings)
fn exp_approx(x: f32) -> f32 {
    let x = if x < -88.0 {
        -88.0
    } else if x > 88.0 {
        88.0
    } else {
        x
    };
    let mut y = 1.0 + x / 1024.0;
    y *= y; // 2
    y *= y; // 4
    y *= y; // 8
    y *= y; // 16
    y *= y; // 32
    y *= y; // 64
    y *= y; // 128
    y *= y; // 256
    y *= y; // 512
    y *= y; // 1024
    if y < 0.0 { 0.0 } else { y }
}

/// tanh(x) = (exp(2x) - 1) / (exp(2x) + 1)
fn tanh_approx(x: f32) -> f32 {
    if x > 10.0 {
        return 1.0;
    }
    if x < -10.0 {
        return -1.0;
    }
    let e2x = exp_approx(2.0 * x);
    (e2x - 1.0) / (e2x + 1.0)
}

/// GELU: 0.5 * x * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
fn gelu(x: f32) -> f32 {
    let c = 0.7978845608; // sqrt(2/pi)
    let inner = c * (x + 0.044715 * x * x * x);
    0.5 * x * (1.0 + tanh_approx(inner))
}

/// 1/sqrt(x) via Newton-Raphson
fn inv_sqrt(x: f32) -> f32 {
    if x <= 0.0 {
        return 1.0;
    }
    let mut val = x;
    let mut y = 1.0f32;
    while val > 4.0 {
        val *= 0.25;
        y *= 0.5;
    }
    while val < 0.25 {
        val *= 4.0;
        y *= 2.0;
    }
    let mut g = y;
    g = g * (1.5 - 0.5 * val * g * g);
    g = g * (1.5 - 0.5 * val * g * g);
    g = g * (1.5 - 0.5 * val * g * g);
    g
}

fn softmax(arr: &mut [f32], len: usize) {
    let mut max = arr[0];
    let mut i = 1;
    while i < len {
        if arr[i] > max {
            max = arr[i];
        }
        i += 1;
    }
    let mut sum = 0.0f32;
    i = 0;
    while i < len {
        arr[i] = exp_approx(arr[i] - max);
        sum += arr[i];
        i += 1;
    }
    if sum > 0.0 {
        let inv = 1.0 / sum;
        i = 0;
        while i < len {
            arr[i] *= inv;
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Argument parsing via /proc/self/cmdline
// ---------------------------------------------------------------------------

struct Args {
    weights_path: [u8; 256],
    weights_path_len: usize,
    tokenizer_path: [u8; 256],
    tokenizer_path_len: usize,
    prompt: [u8; 512],
    prompt_len: usize,
    n_tokens: u32,
    temperature: f32,
}

fn parse_u32_from_slice(s: &[u8]) -> u32 {
    let mut val: u32 = 0;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c < b'0' || c > b'9' {
            return val;
        }
        val = val.wrapping_mul(10).wrapping_add((c - b'0') as u32);
        i += 1;
    }
    val
}

/// Parse "X.Y" as f32 (simple: integer part + single decimal digit)
fn parse_f32_from_slice(s: &[u8]) -> f32 {
    let mut int_part: u32 = 0;
    let mut frac_part: u32 = 0;
    let mut frac_digits: u32 = 0;
    let mut in_frac = false;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == b'.' {
            in_frac = true;
        } else if c >= b'0' && c <= b'9' {
            if in_frac {
                frac_part = frac_part * 10 + (c - b'0') as u32;
                frac_digits += 1;
            } else {
                int_part = int_part * 10 + (c - b'0') as u32;
            }
        }
        i += 1;
    }
    let mut result = int_part as f32;
    if frac_digits > 0 {
        let mut divisor = 1u32;
        let mut d = 0;
        while d < frac_digits {
            divisor *= 10;
            d += 1;
        }
        result += frac_part as f32 / divisor as f32;
    }
    result
}

unsafe fn parse_args() -> Args {
    let mut result = Args {
        weights_path: [0u8; 256],
        weights_path_len: 0,
        tokenizer_path: [0u8; 256],
        tokenizer_path_len: 0,
        prompt: [0u8; 512],
        prompt_len: 0,
        n_tokens: 128,
        temperature: 0.8,
    };

    let mut buf = [0u8; 2048];
    let fd = libc::open(
        b"/proc/self/cmdline\0".as_ptr() as *const libc::c_char,
        libc::O_RDONLY,
    );
    if fd < 0 {
        return result;
    }
    let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
    libc::close(fd);
    if n <= 0 {
        return result;
    }

    let data = &buf[..n as usize];
    let mut args: [&[u8]; 8] = [&[]; 8];
    let mut arg_count = 0;

    // skip argv[0]
    let mut i = 0;
    while i < data.len() && data[i] != 0 {
        i += 1;
    }
    i += 1;

    while i < data.len() && arg_count < 8 {
        let start = i;
        while i < data.len() && data[i] != 0 {
            i += 1;
        }
        if i > start {
            args[arg_count] = &data[start..i];
            arg_count += 1;
        }
        i += 1;
    }

    // argv[1] = weights path (required)
    if arg_count >= 1 {
        let src = args[0];
        let len = if src.len() < 255 { src.len() } else { 255 };
        let mut j = 0;
        while j < len {
            result.weights_path[j] = src[j];
            j += 1;
        }
        result.weights_path[len] = 0; // null terminate
        result.weights_path_len = len;
    }

    // argv[2] = tokenizer path (required)
    if arg_count >= 2 {
        let src = args[1];
        let len = if src.len() < 255 { src.len() } else { 255 };
        let mut j = 0;
        while j < len {
            result.tokenizer_path[j] = src[j];
            j += 1;
        }
        result.tokenizer_path[len] = 0;
        result.tokenizer_path_len = len;
    }

    // argv[3] = prompt (optional)
    if arg_count >= 3 {
        let src = args[2];
        let len = if src.len() < 512 { src.len() } else { 512 };
        let mut j = 0;
        while j < len {
            result.prompt[j] = src[j];
            j += 1;
        }
        result.prompt_len = len;
    }

    // argv[4] = n_tokens (optional)
    if arg_count >= 4 {
        let v = parse_u32_from_slice(args[3]);
        if v > 0 {
            result.n_tokens = v;
        }
    }

    // argv[5] = temperature (optional)
    if arg_count >= 5 {
        let v = parse_f32_from_slice(args[4]);
        if v > 0.0 {
            result.temperature = v;
        }
    }

    result
}

// ---------------------------------------------------------------------------
// PRNG (xorshift64)
// ---------------------------------------------------------------------------

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 { 0x12345678_9abcdef0 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u64() & 0xFFFFFF) as f32 / 16777216.0
    }
}

// ---------------------------------------------------------------------------
// Weight layout — pointers into mmap'd weight file
// ---------------------------------------------------------------------------

struct LayerWeights {
    ln1_weight: *const f32,  // [768]
    ln1_bias: *const f32,    // [768]
    c_attn_weight: *const f32, // [2304, 768] (QKV combined, row-major)
    c_attn_bias: *const f32,   // [2304]
    c_proj_weight: *const f32, // [768, 768]
    c_proj_bias: *const f32,   // [768]
    ln2_weight: *const f32,  // [768]
    ln2_bias: *const f32,    // [768]
    fc_weight: *const f32,   // [3072, 768]
    fc_bias: *const f32,     // [3072]
    proj_weight: *const f32, // [768, 3072]
    proj_bias: *const f32,   // [768]
}

struct ModelWeights {
    wte: *const f32,      // [50257, 768] token embeddings
    wpe: *const f32,      // [1024, 768] position embeddings
    layers: [LayerWeights; NUM_LAYERS],
    ln_f_weight: *const f32, // [768]
    ln_f_bias: *const f32,   // [768]
}

unsafe fn load_weights(data: *const u8) -> ModelWeights {
    let header = data as *const u32;
    let magic = *header;
    if magic != WEIGHT_MAGIC {
        eprint(b"Error: invalid weight file magic\n");
        libc::exit(1);
    }

    let mut offset: usize = 16; // skip 16-byte header

    let mut f32_ptr = |count: usize| -> *const f32 {
        let ptr = data.add(offset) as *const f32;
        offset += count * 4;
        ptr
    };

    let wte = f32_ptr(VOCAB_SIZE * EMBED_DIM);
    let wpe = f32_ptr(MAX_SEQ_LEN * EMBED_DIM);

    const EMPTY_LAYER: LayerWeights = LayerWeights {
        ln1_weight: core::ptr::null(),
        ln1_bias: core::ptr::null(),
        c_attn_weight: core::ptr::null(),
        c_attn_bias: core::ptr::null(),
        c_proj_weight: core::ptr::null(),
        c_proj_bias: core::ptr::null(),
        ln2_weight: core::ptr::null(),
        ln2_bias: core::ptr::null(),
        fc_weight: core::ptr::null(),
        fc_bias: core::ptr::null(),
        proj_weight: core::ptr::null(),
        proj_bias: core::ptr::null(),
    };
    let mut layers = [EMPTY_LAYER; NUM_LAYERS];

    let mut l = 0;
    while l < NUM_LAYERS {
        layers[l] = LayerWeights {
            ln1_weight: f32_ptr(EMBED_DIM),
            ln1_bias: f32_ptr(EMBED_DIM),
            c_attn_weight: f32_ptr(3 * EMBED_DIM * EMBED_DIM),
            c_attn_bias: f32_ptr(3 * EMBED_DIM),
            c_proj_weight: f32_ptr(EMBED_DIM * EMBED_DIM),
            c_proj_bias: f32_ptr(EMBED_DIM),
            ln2_weight: f32_ptr(EMBED_DIM),
            ln2_bias: f32_ptr(EMBED_DIM),
            fc_weight: f32_ptr(FF_DIM * EMBED_DIM),
            fc_bias: f32_ptr(FF_DIM),
            proj_weight: f32_ptr(EMBED_DIM * FF_DIM),
            proj_bias: f32_ptr(EMBED_DIM),
        };
        l += 1;
    }

    let ln_f_weight = f32_ptr(EMBED_DIM);
    let ln_f_bias = f32_ptr(EMBED_DIM);

    ModelWeights {
        wte,
        wpe,
        layers,
        ln_f_weight,
        ln_f_bias,
    }
}

// ---------------------------------------------------------------------------
// Tokenizer — BPE encode and decode
// ---------------------------------------------------------------------------

struct Tokenizer {
    // Vocab: array of (offset, length) into vocab_data
    vocab_offsets: *const u32, // [vocab_size] offset into vocab_data
    vocab_lengths: *const u16, // [vocab_size] byte length
    vocab_data: *const u8,     // concatenated token bytes
    vocab_size: u32,

    // Merges: sorted by priority (index = priority)
    merge_a: *const u32,       // [n_merges]
    merge_b: *const u32,       // [n_merges]
    merge_result: *const u32,  // [n_merges]
    n_merges: u32,
}

/// Scratch for tokenizer: we parse the binary tokenizer file into these arrays
struct TokenizerScratch {
    vocab_offsets: [u32; VOCAB_SIZE],
    vocab_lengths: [u16; VOCAB_SIZE],
    vocab_data: [u8; 512 * 1024], // 512KB for all token strings
    vocab_data_len: u32,
    merge_a: [u32; 64000],
    merge_b: [u32; 64000],
    merge_result: [u32; 64000],
}

unsafe fn load_tokenizer(data: *const u8, _size: usize, scratch: &mut TokenizerScratch) -> Tokenizer {
    let header = data as *const u32;
    let magic = *header;
    if magic != TOKEN_MAGIC {
        eprint(b"Error: invalid tokenizer file magic\n");
        libc::exit(1);
    }
    let vocab_size = *header.add(1);
    let n_merges = *header.add(2);

    // Read vocabulary
    let mut offset: usize = 12; // past header
    let mut data_offset: u32 = 0;

    let mut i: u32 = 0;
    while i < vocab_size && (i as usize) < VOCAB_SIZE {
        let len = *(data.add(offset) as *const u16);
        offset += 2;

        scratch.vocab_offsets[i as usize] = data_offset;
        scratch.vocab_lengths[i as usize] = len;

        let mut j = 0u16;
        while j < len {
            if (data_offset as usize) < scratch.vocab_data.len() {
                scratch.vocab_data[data_offset as usize] = *data.add(offset + j as usize);
                data_offset += 1;
            }
            j += 1;
        }
        offset += len as usize;
        i += 1;
    }
    scratch.vocab_data_len = data_offset;

    // Read merges
    let merge_data = data.add(offset) as *const u32;
    let mut m: u32 = 0;
    while m < n_merges && (m as usize) < 64000 {
        scratch.merge_a[m as usize] = *merge_data.add(m as usize * 3);
        scratch.merge_b[m as usize] = *merge_data.add(m as usize * 3 + 1);
        scratch.merge_result[m as usize] = *merge_data.add(m as usize * 3 + 2);
        m += 1;
    }

    Tokenizer {
        vocab_offsets: scratch.vocab_offsets.as_ptr(),
        vocab_lengths: scratch.vocab_lengths.as_ptr(),
        vocab_data: scratch.vocab_data.as_ptr(),
        vocab_size,
        merge_a: scratch.merge_a.as_ptr(),
        merge_b: scratch.merge_b.as_ptr(),
        merge_result: scratch.merge_result.as_ptr(),
        n_merges,
    }
}

/// Get token bytes for a given token id
unsafe fn token_bytes<'a>(tok: &Tokenizer, id: u32) -> &'a [u8] {
    if id >= tok.vocab_size {
        return &[];
    }
    let off = *tok.vocab_offsets.add(id as usize) as usize;
    let len = *tok.vocab_lengths.add(id as usize) as usize;
    core::slice::from_raw_parts(tok.vocab_data.add(off), len)
}

/// Find token id for exact byte match. Returns vocab_size if not found.
unsafe fn find_token(tok: &Tokenizer, bytes: &[u8]) -> u32 {
    let mut i: u32 = 0;
    while i < tok.vocab_size {
        let tb = token_bytes(tok, i);
        if tb.len() == bytes.len() {
            let mut eq = true;
            let mut j = 0;
            while j < tb.len() {
                if tb[j] != bytes[j] {
                    eq = false;
                    break;
                }
                j += 1;
            }
            if eq {
                return i;
            }
        }
        i += 1;
    }
    tok.vocab_size
}

/// BPE encode: split text into bytes, then iteratively merge
unsafe fn bpe_encode(tok: &Tokenizer, text: &[u8], out_tokens: &mut [u32], max_tokens: usize) -> usize {
    if text.is_empty() {
        return 0;
    }

    // Start with byte-level tokens
    // GPT-2 uses a byte-to-unicode mapping. Each byte 0-255 maps to a specific unicode char.
    // The token for byte b is the single-char string of the mapped unicode char.
    // We need to find the token id for each individual byte.
    let mut tokens_buf = [0u32; 2048];
    let mut n_tok = 0usize;

    // For each byte, find its single-byte token
    let mut i = 0;
    while i < text.len() && n_tok < 2048 {
        let byte_slice = &text[i..i + 1];
        let id = find_token(tok, byte_slice);
        if id < tok.vocab_size {
            tokens_buf[n_tok] = id;
        } else {
            // Try the GPT-2 byte encoding: bytes 0-255 are mapped to specific chars
            // For printable ASCII, the token is just the character itself
            // For non-printable, GPT-2 maps them to higher unicode chars
            // As a fallback, use the byte value directly if we can find it
            tokens_buf[n_tok] = 0; // unknown token
        }
        n_tok += 1;
        i += 1;
    }

    // Iterative BPE merging: find the highest-priority merge and apply it
    let mut changed = true;
    while changed {
        changed = false;
        let mut best_merge: u32 = tok.n_merges; // invalid = no merge found
        let mut best_pos: usize = 0;

        // Find the merge with lowest index (highest priority)
        let mut pos = 0;
        while pos + 1 < n_tok {
            let a = tokens_buf[pos];
            let b = tokens_buf[pos + 1];
            // Linear search through merges for this pair
            let mut m: u32 = 0;
            while m < tok.n_merges && m < best_merge {
                if *tok.merge_a.add(m as usize) == a && *tok.merge_b.add(m as usize) == b {
                    best_merge = m;
                    best_pos = pos;
                    break;
                }
                m += 1;
            }
            pos += 1;
        }

        if best_merge < tok.n_merges {
            // Apply the merge: replace tokens_buf[best_pos] and [best_pos+1] with result
            let result = *tok.merge_result.add(best_merge as usize);
            tokens_buf[best_pos] = result;
            // Shift remaining tokens left
            let mut j = best_pos + 1;
            while j + 1 < n_tok {
                tokens_buf[j] = tokens_buf[j + 1];
                j += 1;
            }
            n_tok -= 1;
            changed = true;
        }
    }

    // Copy to output
    let copy_len = if n_tok < max_tokens { n_tok } else { max_tokens };
    let mut i = 0;
    while i < copy_len {
        out_tokens[i] = tokens_buf[i];
        i += 1;
    }
    copy_len
}

/// Decode token to stdout
unsafe fn decode_token(tok: &Tokenizer, id: u32) {
    let bytes = token_bytes(tok, id);
    if !bytes.is_empty() {
        print(bytes);
    }
}

// ---------------------------------------------------------------------------
// Vector / matrix operations
// ---------------------------------------------------------------------------

unsafe fn dot(a: *const f32, b: *const f32, len: usize) -> f32 {
    let mut sum = 0.0f32;
    let mut i = 0;
    while i < len {
        sum += *a.add(i) * *b.add(i);
        i += 1;
    }
    sum
}

/// out[i] = dot(mat[i*cols..], vec, cols) for i in 0..rows
unsafe fn mat_vec_mul(mat: *const f32, vec: *const f32, out: *mut f32, rows: usize, cols: usize) {
    let mut i = 0;
    while i < rows {
        *out.add(i) = dot(mat.add(i * cols), vec, cols);
        i += 1;
    }
}

unsafe fn vec_add(dst: *mut f32, src: *const f32, len: usize) {
    let mut i = 0;
    while i < len {
        *dst.add(i) += *src.add(i);
        i += 1;
    }
}

unsafe fn vec_copy(dst: *mut f32, src: *const f32, len: usize) {
    let mut i = 0;
    while i < len {
        *dst.add(i) = *src.add(i);
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Layer normalization
// ---------------------------------------------------------------------------

unsafe fn layer_norm(x: *mut f32, weight: *const f32, bias: *const f32) {
    let mut mean = 0.0f32;
    let mut i = 0;
    while i < EMBED_DIM {
        mean += *x.add(i);
        i += 1;
    }
    mean /= EMBED_DIM as f32;

    let mut var = 0.0f32;
    i = 0;
    while i < EMBED_DIM {
        let d = *x.add(i) - mean;
        var += d * d;
        i += 1;
    }
    var /= EMBED_DIM as f32;

    let inv_std = inv_sqrt(var + 1e-5);

    i = 0;
    while i < EMBED_DIM {
        *x.add(i) = (*x.add(i) - mean) * inv_std * *weight.add(i) + *bias.add(i);
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Scratch memory layout
// ---------------------------------------------------------------------------

struct Scratch {
    // Per-token hidden state
    hidden: *mut f32,        // [EMBED_DIM]
    // Temporary buffers
    qkv: *mut f32,           // [3 * EMBED_DIM]
    attn_out: *mut f32,      // [EMBED_DIM]
    ff_hidden: *mut f32,     // [FF_DIM]
    tmp: *mut f32,           // [EMBED_DIM]
    attn_scores: *mut f32,   // [MAX_SEQ_LEN] for one head
    logits: *mut f32,        // [VOCAB_SIZE]
    // KV cache: [NUM_LAYERS][MAX_SEQ_LEN][EMBED_DIM] for K and V each
    kv_cache_k: *mut f32,
    kv_cache_v: *mut f32,
}

const SCRATCH_HIDDEN: usize = EMBED_DIM * 4;          // hidden state
const SCRATCH_QKV: usize = 3 * EMBED_DIM * 4;
const SCRATCH_ATTN_OUT: usize = EMBED_DIM * 4;
const SCRATCH_FF: usize = FF_DIM * 4;
const SCRATCH_TMP: usize = EMBED_DIM * 4;
const SCRATCH_ATTN_SCORES: usize = MAX_SEQ_LEN * 4;
const SCRATCH_LOGITS: usize = VOCAB_SIZE * 4;
const SCRATCH_KV_ONE: usize = NUM_LAYERS * MAX_SEQ_LEN * EMBED_DIM * 4;
const SCRATCH_TOTAL: usize = SCRATCH_HIDDEN + SCRATCH_QKV + SCRATCH_ATTN_OUT
    + SCRATCH_FF + SCRATCH_TMP + SCRATCH_ATTN_SCORES + SCRATCH_LOGITS
    + SCRATCH_KV_ONE * 2;

unsafe fn init_scratch(base: *mut u8) -> Scratch {
    let mut off = 0usize;
    let mut alloc = |size: usize| -> *mut f32 {
        let ptr = base.add(off) as *mut f32;
        off += size;
        ptr
    };

    Scratch {
        hidden: alloc(SCRATCH_HIDDEN),
        qkv: alloc(SCRATCH_QKV),
        attn_out: alloc(SCRATCH_ATTN_OUT),
        ff_hidden: alloc(SCRATCH_FF),
        tmp: alloc(SCRATCH_TMP),
        attn_scores: alloc(SCRATCH_ATTN_SCORES),
        logits: alloc(SCRATCH_LOGITS),
        kv_cache_k: alloc(SCRATCH_KV_ONE),
        kv_cache_v: alloc(SCRATCH_KV_ONE),
    }
}

// ---------------------------------------------------------------------------
// Transformer block: LN1 -> Attn -> Residual -> LN2 -> FFN -> Residual
// ---------------------------------------------------------------------------

unsafe fn transformer_block(
    layer: &LayerWeights,
    scratch: &Scratch,
    layer_idx: usize,
    pos: usize,
) {
    // Save residual
    vec_copy(scratch.attn_out, scratch.hidden, EMBED_DIM); // use attn_out as temp residual store

    // Layer norm 1
    layer_norm(scratch.hidden, layer.ln1_weight, layer.ln1_bias);

    // Self-attention (overwrites attn_out internally, but we saved residual)
    // We need residual, so save it first in tmp
    vec_copy(scratch.tmp, scratch.attn_out, EMBED_DIM);

    // Attention adds its output to hidden. But GPT-2 is:
    // x = x + attn(ln1(x))
    // So we need: hidden = residual, then run attn which does hidden += attn_output
    // Actually let's restructure:

    // After LN1, hidden has the normalized input. Attention needs this.
    // We want: new_hidden = residual + attn(ln1_output)
    // So: save residual, compute attn output onto hidden (starting from ln1 output),
    //     then hidden = residual + attn_result

    // Re-do: hidden currently = ln1(original_hidden)
    // We saved original_hidden in tmp
    // Run attention which computes attn output and adds it to hidden
    // But attention does: hidden += proj(attn_weighted_sum)
    // So after attention: hidden = ln1(x) + proj(attn(...))
    // We want: hidden = x + proj(attn(ln1(x)))
    // Fix: zero out hidden's attn contribution start from residual

    // Let me restructure more cleanly:
    // 1. residual = hidden (save in tmp)
    // 2. hidden = layer_norm(hidden)
    // 3. Compute attention, store result in attn_out
    // 4. hidden = residual + attn_out

    // Step 1-2 already done. Now redo attention to not modify hidden directly:
    // Actually, let me just restructure the code:

    // Save the LN1 output for attention
    // scratch.attn_out was overwritten by attention, so use a different approach

    // Simplest: compute attn into attn_out, then hidden = residual + attn_out
    // But our attention function modifies hidden. Let me fix.

    // OK, let me just inline the logic properly:

    // At this point:
    // scratch.tmp = original hidden (residual)
    // scratch.hidden = ln1(original hidden)

    // Compute QKV from hidden (which has ln1 output)
    mat_vec_mul(layer.c_attn_weight, scratch.hidden, scratch.qkv, 3 * EMBED_DIM, EMBED_DIM);
    vec_add(scratch.qkv, layer.c_attn_bias, 3 * EMBED_DIM);

    let q = scratch.qkv;
    let k = scratch.qkv.add(EMBED_DIM);
    let v = scratch.qkv.add(2 * EMBED_DIM);

    // Store K, V in cache
    let kv_layer_offset = layer_idx * MAX_SEQ_LEN * EMBED_DIM;
    let kv_pos_offset = kv_layer_offset + pos * EMBED_DIM;
    vec_copy(scratch.kv_cache_k.add(kv_pos_offset), k, EMBED_DIM);
    vec_copy(scratch.kv_cache_v.add(kv_pos_offset), v, EMBED_DIM);

    // Zero attn_out
    let mut d = 0;
    while d < EMBED_DIM {
        *scratch.attn_out.add(d) = 0.0;
        d += 1;
    }

    // Multi-head attention
    let scale = inv_sqrt(HEAD_DIM as f32);
    let mut h = 0;
    while h < NUM_HEADS {
        let head_off = h * HEAD_DIM;
        let mut j = 0;
        while j <= pos {
            let cached_k = scratch.kv_cache_k.add(kv_layer_offset + j * EMBED_DIM + head_off);
            *scratch.attn_scores.add(j) = dot(q.add(head_off), cached_k, HEAD_DIM) * scale;
            j += 1;
        }
        softmax(
            core::slice::from_raw_parts_mut(scratch.attn_scores, pos + 1),
            pos + 1,
        );
        j = 0;
        while j <= pos {
            let w = *scratch.attn_scores.add(j);
            if w > 1e-8 {
                let cached_v = scratch.kv_cache_v.add(kv_layer_offset + j * EMBED_DIM + head_off);
                let mut dd = 0;
                while dd < HEAD_DIM {
                    *scratch.attn_out.add(head_off + dd) += w * *cached_v.add(dd);
                    dd += 1;
                }
            }
            j += 1;
        }
        h += 1;
    }

    // Output projection: hidden = c_proj_weight * attn_out + c_proj_bias
    mat_vec_mul(layer.c_proj_weight, scratch.attn_out, scratch.hidden, EMBED_DIM, EMBED_DIM);
    vec_add(scratch.hidden, layer.c_proj_bias, EMBED_DIM);

    // Residual: hidden = residual + attn_proj_output
    vec_add(scratch.hidden, scratch.tmp, EMBED_DIM);

    // --- FFN ---
    // Save residual
    vec_copy(scratch.tmp, scratch.hidden, EMBED_DIM);

    // Layer norm 2
    layer_norm(scratch.hidden, layer.ln2_weight, layer.ln2_bias);

    // FFN: ff_hidden = gelu(fc_weight * hidden + fc_bias)
    mat_vec_mul(layer.fc_weight, scratch.hidden, scratch.ff_hidden, FF_DIM, EMBED_DIM);
    vec_add(scratch.ff_hidden, layer.fc_bias, FF_DIM);
    let mut i = 0;
    while i < FF_DIM {
        *scratch.ff_hidden.add(i) = gelu(*scratch.ff_hidden.add(i));
        i += 1;
    }

    // proj: hidden = proj_weight * ff_hidden + proj_bias
    mat_vec_mul(layer.proj_weight, scratch.ff_hidden, scratch.hidden, EMBED_DIM, FF_DIM);
    vec_add(scratch.hidden, layer.proj_bias, EMBED_DIM);

    // Residual
    vec_add(scratch.hidden, scratch.tmp, EMBED_DIM);
}

// ---------------------------------------------------------------------------
// Forward pass (single token at position pos)
// ---------------------------------------------------------------------------

unsafe fn forward(
    weights: &ModelWeights,
    scratch: &Scratch,
    token: u32,
    pos: usize,
) {
    // Embed: hidden = wte[token] + wpe[pos]
    vec_copy(scratch.hidden, weights.wte.add(token as usize * EMBED_DIM), EMBED_DIM);
    vec_add(scratch.hidden, weights.wpe.add(pos * EMBED_DIM), EMBED_DIM);

    // 12 transformer blocks
    let mut l = 0;
    while l < NUM_LAYERS {
        transformer_block(&weights.layers[l], scratch, l, pos);
        l += 1;
    }

    // Final layer norm
    layer_norm(scratch.hidden, weights.ln_f_weight, weights.ln_f_bias);

    // Logits: logits[v] = dot(hidden, wte[v]) — weight tying
    let mut v = 0;
    while v < VOCAB_SIZE {
        *scratch.logits.add(v) = dot(scratch.hidden, weights.wte.add(v * EMBED_DIM), EMBED_DIM);
        v += 1;
    }
}

// ---------------------------------------------------------------------------
// Sampling
// ---------------------------------------------------------------------------

unsafe fn sample_token(logits: *mut f32, temperature: f32, rng: &mut Rng) -> u32 {
    if temperature < 0.01 {
        // Argmax (greedy)
        let mut best = 0u32;
        let mut best_val = *logits;
        let mut i = 1u32;
        while i < VOCAB_SIZE as u32 {
            if *logits.add(i as usize) > best_val {
                best_val = *logits.add(i as usize);
                best = i;
            }
            i += 1;
        }
        return best;
    }

    // Apply temperature
    let inv_temp = 1.0 / temperature;
    let mut i = 0;
    while i < VOCAB_SIZE {
        *logits.add(i) *= inv_temp;
        i += 1;
    }

    // Top-k sampling (k=40)
    let k = 40usize;
    let mut top_k_ids = [0u32; 40];
    let mut top_k_vals = [0.0f32; 40];

    // Initialize with first k
    i = 0;
    while i < k {
        top_k_ids[i] = i as u32;
        top_k_vals[i] = *logits.add(i);
        i += 1;
    }

    // Find minimum in top-k
    let mut min_idx = 0;
    let mut min_val = top_k_vals[0];
    i = 1;
    while i < k {
        if top_k_vals[i] < min_val {
            min_val = top_k_vals[i];
            min_idx = i;
        }
        i += 1;
    }

    // Scan remaining, replace minimum if larger
    i = k;
    while i < VOCAB_SIZE {
        let v = *logits.add(i);
        if v > min_val {
            top_k_ids[min_idx] = i as u32;
            top_k_vals[min_idx] = v;
            // Find new minimum
            min_val = top_k_vals[0];
            min_idx = 0;
            let mut j = 1;
            while j < k {
                if top_k_vals[j] < min_val {
                    min_val = top_k_vals[j];
                    min_idx = j;
                }
                j += 1;
            }
        }
        i += 1;
    }

    // Softmax over top-k values
    softmax(&mut top_k_vals, k);

    // Sample from distribution
    let r = rng.next_f32();
    let mut cumsum = 0.0f32;
    i = 0;
    while i < k {
        cumsum += top_k_vals[i];
        if r < cumsum {
            return top_k_ids[i];
        }
        i += 1;
    }
    top_k_ids[k - 1]
}

// ---------------------------------------------------------------------------
// Generation loop
// ---------------------------------------------------------------------------

unsafe fn generate(
    weights: &ModelWeights,
    scratch: &Scratch,
    tok: &Tokenizer,
    prompt_tokens: &[u32],
    n_prompt: usize,
    n_generate: u32,
    temperature: f32,
) {
    let mut rng = Rng::new(42);

    // Process prompt tokens (prefill)
    let mut pos = 0;
    while pos < n_prompt {
        forward(weights, scratch, prompt_tokens[pos], pos);
        // Print decoded token
        decode_token(tok, prompt_tokens[pos]);
        pos += 1;
    }

    // Autoregressive generation
    let mut generated = 0u32;
    while generated < n_generate {
        if pos >= MAX_SEQ_LEN {
            break;
        }

        let next_token = sample_token(scratch.logits, temperature, &mut rng);
        decode_token(tok, next_token);

        // Feed the new token
        forward(weights, scratch, next_token, pos);
        pos += 1;
        generated += 1;
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn run() {
    unsafe {
        let args = parse_args();

        if args.weights_path_len == 0 || args.tokenizer_path_len == 0 {
            eprint(b"Usage: tiny-gpt2 <weights.bin> <tokenizer.bin> [prompt] [n_tokens] [temp]\n");
            libc::exit(1);
        }

        // Open and mmap weight file
        let wfd = open_file(&args.weights_path);
        if wfd < 0 {
            eprint(b"Error: cannot open weight file\n");
            libc::exit(1);
        }
        let wsize = file_size(wfd);
        let wdata = mmap_file(wfd, wsize);
        libc::close(wfd);
        if wdata.is_null() {
            eprint(b"Error: cannot mmap weight file\n");
            libc::exit(1);
        }

        // Open and mmap tokenizer file
        let tfd = open_file(&args.tokenizer_path);
        if tfd < 0 {
            eprint(b"Error: cannot open tokenizer file\n");
            libc::exit(1);
        }
        let tsize = file_size(tfd);
        let tdata = mmap_file(tfd, tsize);
        libc::close(tfd);
        if tdata.is_null() {
            eprint(b"Error: cannot mmap tokenizer file\n");
            libc::exit(1);
        }

        // Load weights
        let weights = load_weights(wdata);

        // Allocate tokenizer scratch via mmap
        let tok_scratch_size = core::mem::size_of::<TokenizerScratch>();
        let tok_scratch_ptr = mmap_anon(tok_scratch_size) as *mut TokenizerScratch;
        if tok_scratch_ptr.is_null() {
            eprint(b"Error: cannot allocate tokenizer scratch\n");
            libc::exit(1);
        }
        let tok_scratch = &mut *tok_scratch_ptr;

        // Load tokenizer
        let tokenizer = load_tokenizer(tdata, tsize, tok_scratch);

        // Allocate inference scratch space
        let scratch_ptr = mmap_anon(SCRATCH_TOTAL);
        if scratch_ptr.is_null() {
            eprint(b"Error: cannot allocate scratch memory\n");
            libc::exit(1);
        }
        let scratch = init_scratch(scratch_ptr);

        // Print banner
        print(b"tiny-gpt2: GPT-2 Small (124M) inference engine\n");
        print(b"Architecture: 12 layers, 12 heads, 768 dim, 50257 vocab\n");
        print(b"Temperature: ");
        // Print temperature as X.X
        let temp_int = args.temperature as u32;
        let temp_frac = ((args.temperature - temp_int as f32) * 10.0) as u32;
        print_u32(temp_int);
        print(b".");
        print_u32(temp_frac);
        print(b", Tokens to generate: ");
        print_u32(args.n_tokens);
        print(b"\n\n");

        // Encode prompt
        let mut prompt_tokens = [0u32; 2048];
        let n_prompt = if args.prompt_len > 0 {
            bpe_encode(
                &tokenizer,
                &args.prompt[..args.prompt_len],
                &mut prompt_tokens,
                2048,
            )
        } else {
            // Default: BOS token or just use token 0
            prompt_tokens[0] = 50256; // <|endoftext|> as start token
            1
        };

        print(b"Prompt tokens: ");
        print_u32(n_prompt as u32);
        print(b"\n---\n");

        // Generate
        generate(
            &weights,
            &scratch,
            &tokenizer,
            &prompt_tokens,
            n_prompt,
            args.n_tokens,
            args.temperature,
        );

        print(b"\n---\nDone.\n");
        libc::exit(0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::asm!(
        "and rsp, -16",
        "call {run}",
        run = sym run,
        options(noreturn),
    );
}
