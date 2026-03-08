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

const VOCAB_SIZE: usize = 96; // printable ASCII 32..127
const EMBED_DIM: usize = 16;
const SEQ_LEN: usize = 32;
const FF_DIM: usize = 32;
const NUM_LAYERS: usize = 1;

// ---------------------------------------------------------------------------
// Model weights
// ---------------------------------------------------------------------------

struct Model {
    embed: [[f32; EMBED_DIM]; VOCAB_SIZE],
    wq: [[f32; EMBED_DIM]; EMBED_DIM],
    wk: [[f32; EMBED_DIM]; EMBED_DIM],
    wv: [[f32; EMBED_DIM]; EMBED_DIM],
    wo: [[f32; EMBED_DIM]; EMBED_DIM],
    ff1_w: [[f32; EMBED_DIM]; FF_DIM],
    ff1_b: [f32; FF_DIM],
    ff2_w: [[f32; FF_DIM]; EMBED_DIM],
    ff2_b: [f32; EMBED_DIM],
    ln1_gamma: [f32; EMBED_DIM],
    ln1_beta: [f32; EMBED_DIM],
    ln2_gamma: [f32; EMBED_DIM],
    ln2_beta: [f32; EMBED_DIM],
    unembed: [[f32; EMBED_DIM]; VOCAB_SIZE],
}

// ---------------------------------------------------------------------------
// Scratch space for forward pass
// ---------------------------------------------------------------------------

struct Scratch {
    hidden: [[f32; EMBED_DIM]; SEQ_LEN],
    q: [[f32; EMBED_DIM]; SEQ_LEN],
    k: [[f32; EMBED_DIM]; SEQ_LEN],
    v: [[f32; EMBED_DIM]; SEQ_LEN],
    attn: [[f32; SEQ_LEN]; SEQ_LEN],
    attn_out: [[f32; EMBED_DIM]; SEQ_LEN],
    ff_hidden: [[f32; FF_DIM]; SEQ_LEN],
    residual: [[f32; EMBED_DIM]; SEQ_LEN],
    logits: [f32; VOCAB_SIZE],
    tmp: [f32; EMBED_DIM],
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

fn print(s: &[u8]) {
    unsafe {
        write_all(1, s);
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
// Argument parsing via /proc/self/cmdline
// ---------------------------------------------------------------------------

struct Args {
    prompt: [u8; 128],
    prompt_len: usize,
    num_chars: u32,
}

unsafe fn parse_args() -> Args {
    let mut result = Args {
        prompt: [0u8; 128],
        prompt_len: 6,
        num_chars: 64,
    };
    // default prompt: "hello "
    result.prompt[0] = b'h';
    result.prompt[1] = b'e';
    result.prompt[2] = b'l';
    result.prompt[3] = b'l';
    result.prompt[4] = b'o';
    result.prompt[5] = b' ';

    let mut buf = [0u8; 512];
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

    // argv[1] = prompt (optional)
    if arg_count >= 1 {
        let src = args[0];
        let len = if src.len() < 128 { src.len() } else { 128 };
        let mut j = 0;
        while j < len {
            result.prompt[j] = src[j];
            j += 1;
        }
        result.prompt_len = len;
    }

    // argv[2] = num_chars (optional)
    if arg_count >= 2 {
        result.num_chars = parse_u32_from_slice(args[1]);
        if result.num_chars == 0 {
            result.num_chars = 64;
        }
    }

    result
}

fn parse_u32_from_slice(s: &[u8]) -> u32 {
    let mut val: u32 = 0;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c < b'0' || c > b'9' {
            return 0;
        }
        val = val.wrapping_mul(10).wrapping_add((c - b'0') as u32);
        i += 1;
    }
    val
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

    /// Returns f32 in [-0.5, 0.5)
    fn next_f32(&mut self) -> f32 {
        (self.next_u64() & 0xFFFFFF) as f32 / 16777216.0 - 0.5
    }

    /// Xavier-like initialization scaled by fan_in
    fn next_scaled(&mut self, fan_in: usize) -> f32 {
        let scale = inv_sqrt_approx(fan_in as f32);
        self.next_f32() * scale
    }
}

// ---------------------------------------------------------------------------
// f32 math approximations
// ---------------------------------------------------------------------------

/// exp(x) via repeated squaring: (1 + x/256)^256
fn exp_approx(x: f32) -> f32 {
    let x = if x < -88.0 {
        -88.0
    } else if x > 88.0 {
        88.0
    } else {
        x
    };
    let mut y = 1.0 + x / 256.0;
    // 8 squarings: 256 = 2^8
    y *= y; // 2
    y *= y; // 4
    y *= y; // 8
    y *= y; // 16
    y *= y; // 32
    y *= y; // 64
    y *= y; // 128
    y *= y; // 256
    y
}

/// 1/sqrt(x) via Newton-Raphson (2 iterations)
fn inv_sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 1.0;
    }
    // initial guess
    let mut y = 1.0;
    // scale down for large values
    let mut val = x;
    while val > 4.0 {
        val *= 0.25;
        y *= 0.5;
    }
    while val < 0.25 {
        val *= 4.0;
        y *= 2.0;
    }
    // Newton iterations for 1/sqrt(val)
    let mut g = y;
    g = g * (1.5 - 0.5 * val * g * g);
    g = g * (1.5 - 0.5 * val * g * g);
    g = g * (1.5 - 0.5 * val * g * g);
    g
}

/// sin(x) via Taylor series (5 terms), x in radians
fn sin_approx(x: f32) -> f32 {
    // Reduce to [-pi, pi]
    let pi = 3.14159265;
    let two_pi = 6.2831853;
    let mut t = x;
    while t > pi {
        t -= two_pi;
    }
    while t < -pi {
        t += two_pi;
    }
    let t2 = t * t;
    let t3 = t2 * t;
    let t5 = t3 * t2;
    let t7 = t5 * t2;
    let t9 = t7 * t2;
    t - t3 / 6.0 + t5 / 120.0 - t7 / 5040.0 + t9 / 362880.0
}

/// cos(x) via Taylor series (5 terms)
fn cos_approx(x: f32) -> f32 {
    let pi = 3.14159265;
    let two_pi = 6.2831853;
    let mut t = x;
    while t > pi {
        t -= two_pi;
    }
    while t < -pi {
        t += two_pi;
    }
    let t2 = t * t;
    let t4 = t2 * t2;
    let t6 = t4 * t2;
    let t8 = t6 * t2;
    1.0 - t2 / 2.0 + t4 / 24.0 - t6 / 720.0 + t8 / 40320.0
}

// ---------------------------------------------------------------------------
// Vector / matrix operations
// ---------------------------------------------------------------------------

fn dot_product(a: &[f32], b: &[f32], len: usize) -> f32 {
    let mut sum = 0.0f32;
    let mut i = 0;
    while i < len {
        sum += a[i] * b[i];
        i += 1;
    }
    sum
}

/// out[i] = sum_j(mat[i][j] * vec[j]) — mat is [out_dim][in_dim]
fn mat_vec_mul<const ROWS: usize, const COLS: usize>(
    mat: &[[f32; COLS]; ROWS],
    vec: &[f32; COLS],
    out: &mut [f32],
) {
    let mut i = 0;
    while i < ROWS {
        out[i] = dot_product(&mat[i], vec, COLS);
        i += 1;
    }
}

fn vec_add(dst: &mut [f32], src: &[f32], len: usize) {
    let mut i = 0;
    while i < len {
        dst[i] += src[i];
        i += 1;
    }
}

fn vec_copy(dst: &mut [f32], src: &[f32], len: usize) {
    let mut i = 0;
    while i < len {
        dst[i] = src[i];
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Softmax (numerically stable)
// ---------------------------------------------------------------------------

fn softmax(arr: &mut [f32], len: usize) {
    // Find max for numerical stability
    let mut max = arr[0];
    let mut i = 1;
    while i < len {
        if arr[i] > max {
            max = arr[i];
        }
        i += 1;
    }
    // exp and sum
    let mut sum = 0.0f32;
    i = 0;
    while i < len {
        arr[i] = exp_approx(arr[i] - max);
        sum += arr[i];
        i += 1;
    }
    // normalize
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
// Layer normalization
// ---------------------------------------------------------------------------

fn layer_norm(x: &mut [f32], gamma: &[f32], beta: &[f32]) {
    // Mean
    let mut mean = 0.0f32;
    let mut i = 0;
    while i < EMBED_DIM {
        mean += x[i];
        i += 1;
    }
    mean /= EMBED_DIM as f32;

    // Variance
    let mut var = 0.0f32;
    i = 0;
    while i < EMBED_DIM {
        let d = x[i] - mean;
        var += d * d;
        i += 1;
    }
    var /= EMBED_DIM as f32;

    let inv_std = inv_sqrt_approx(var + 1e-5);

    i = 0;
    while i < EMBED_DIM {
        x[i] = (x[i] - mean) * inv_std * gamma[i] + beta[i];
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Embedding + sinusoidal positional encoding
// ---------------------------------------------------------------------------

fn embed_token(model: &Model, ch: u8, pos: usize, out: &mut [f32; EMBED_DIM]) {
    let idx = if ch >= 32 && ch < 128 {
        (ch - 32) as usize
    } else {
        0
    };

    // Token embedding
    let mut i = 0;
    while i < EMBED_DIM {
        out[i] = model.embed[idx][i];
        i += 1;
    }

    // Sinusoidal positional encoding: PE(pos, 2i) = sin(pos / 10000^(2i/d))
    //                                  PE(pos, 2i+1) = cos(pos / 10000^(2i/d))
    i = 0;
    while i < EMBED_DIM / 2 {
        // 10000^(2i/d) approximated via exp
        let freq = exp_approx(-((2 * i) as f32) / EMBED_DIM as f32 * 9.21034); // ln(10000) ~ 9.21
        let angle = pos as f32 * freq;
        out[2 * i] += sin_approx(angle);
        out[2 * i + 1] += cos_approx(angle);
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Self-attention (single head)
// ---------------------------------------------------------------------------

fn self_attention(model: &Model, scratch: &mut Scratch, seq_len: usize) {
    // Project Q, K, V for each position
    let mut t = 0;
    while t < seq_len {
        mat_vec_mul::<EMBED_DIM, EMBED_DIM>(
            &model.wq,
            &scratch.hidden[t],
            &mut scratch.q[t],
        );
        mat_vec_mul::<EMBED_DIM, EMBED_DIM>(
            &model.wk,
            &scratch.hidden[t],
            &mut scratch.k[t],
        );
        mat_vec_mul::<EMBED_DIM, EMBED_DIM>(
            &model.wv,
            &scratch.hidden[t],
            &mut scratch.v[t],
        );
        t += 1;
    }

    // Scaled dot-product attention: attn[i][j] = Q[i] . K[j] / sqrt(d_k)
    // With causal mask: future positions get -1e9
    let scale = inv_sqrt_approx(EMBED_DIM as f32); // 1/sqrt(d_k)

    let mut i = 0;
    while i < seq_len {
        let mut j = 0;
        while j < seq_len {
            if j <= i {
                // causal: can attend to current and past
                scratch.attn[i][j] =
                    dot_product(&scratch.q[i], &scratch.k[j], EMBED_DIM) * scale;
            } else {
                // mask future positions
                scratch.attn[i][j] = -1e9;
            }
            j += 1;
        }
        // Softmax over the row
        softmax(&mut scratch.attn[i], seq_len);
        i += 1;
    }

    // Weighted sum of values: attn_out[i] = sum_j attn[i][j] * V[j]
    i = 0;
    while i < seq_len {
        let mut d = 0;
        while d < EMBED_DIM {
            scratch.attn_out[i][d] = 0.0;
            d += 1;
        }
        let mut j = 0;
        while j < seq_len {
            let w = scratch.attn[i][j];
            if w > 1e-8 {
                d = 0;
                while d < EMBED_DIM {
                    scratch.attn_out[i][d] += w * scratch.v[j][d];
                    d += 1;
                }
            }
            j += 1;
        }
        i += 1;
    }

    // Output projection
    i = 0;
    while i < seq_len {
        mat_vec_mul::<EMBED_DIM, EMBED_DIM>(
            &model.wo,
            &scratch.attn_out[i],
            &mut scratch.tmp,
        );
        vec_copy(&mut scratch.attn_out[i], &scratch.tmp, EMBED_DIM);
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Feed-forward network: Linear -> ReLU -> Linear
// ---------------------------------------------------------------------------

fn feed_forward(model: &Model, scratch: &mut Scratch, seq_len: usize) {
    let mut t = 0;
    while t < seq_len {
        // First linear: hidden -> ff_dim
        mat_vec_mul::<FF_DIM, EMBED_DIM>(
            &model.ff1_w,
            &scratch.hidden[t],
            &mut scratch.ff_hidden[t],
        );
        // Add bias and ReLU
        let mut i = 0;
        while i < FF_DIM {
            scratch.ff_hidden[t][i] += model.ff1_b[i];
            if scratch.ff_hidden[t][i] < 0.0 {
                scratch.ff_hidden[t][i] = 0.0;
            }
            i += 1;
        }
        // Second linear: ff_dim -> embed_dim
        mat_vec_mul::<EMBED_DIM, FF_DIM>(
            &model.ff2_w,
            &scratch.ff_hidden[t],
            &mut scratch.tmp,
        );
        // Add bias
        i = 0;
        while i < EMBED_DIM {
            scratch.tmp[i] += model.ff2_b[i];
            i += 1;
        }
        vec_copy(&mut scratch.hidden[t], &scratch.tmp, EMBED_DIM);
        t += 1;
    }
}

// ---------------------------------------------------------------------------
// Transformer block: Attention + residual + LN + FFN + residual + LN
// ---------------------------------------------------------------------------

fn transformer_block(model: &Model, scratch: &mut Scratch, seq_len: usize) {
    // Save residual
    let mut t = 0;
    while t < seq_len {
        vec_copy(&mut scratch.residual[t], &scratch.hidden[t], EMBED_DIM);
        t += 1;
    }

    // Self-attention
    self_attention(model, scratch, seq_len);

    // Add residual + layer norm 1
    t = 0;
    while t < seq_len {
        vec_add(&mut scratch.attn_out[t], &scratch.residual[t], EMBED_DIM);
        vec_copy(&mut scratch.hidden[t], &scratch.attn_out[t], EMBED_DIM);
        layer_norm(&mut scratch.hidden[t], &model.ln1_gamma, &model.ln1_beta);
        t += 1;
    }

    // Save residual for FFN
    t = 0;
    while t < seq_len {
        vec_copy(&mut scratch.residual[t], &scratch.hidden[t], EMBED_DIM);
        t += 1;
    }

    // Feed-forward
    feed_forward(model, scratch, seq_len);

    // Add residual + layer norm 2
    t = 0;
    while t < seq_len {
        vec_add(&mut scratch.hidden[t], &scratch.residual[t], EMBED_DIM);
        layer_norm(&mut scratch.hidden[t], &model.ln2_gamma, &model.ln2_beta);
        t += 1;
    }
}

// ---------------------------------------------------------------------------
// Forward pass: embed -> positional -> transformer -> unembed
// ---------------------------------------------------------------------------

fn forward(
    model: &Model,
    scratch: &mut Scratch,
    tokens: &[u8],
    seq_len: usize,
) {
    // Embed tokens with positional encoding
    let mut t = 0;
    while t < seq_len {
        embed_token(model, tokens[t], t, &mut scratch.hidden[t]);
        t += 1;
    }

    // Run transformer block(s)
    let mut _layer = 0;
    while _layer < NUM_LAYERS {
        transformer_block(model, scratch, seq_len);
        _layer += 1;
    }

    // Compute output logits from the last position via unembed
    mat_vec_mul::<VOCAB_SIZE, EMBED_DIM>(
        &model.unembed,
        &scratch.hidden[seq_len - 1],
        &mut scratch.logits,
    );
}

// ---------------------------------------------------------------------------
// Token generation (argmax)
// ---------------------------------------------------------------------------

fn argmax(logits: &[f32; VOCAB_SIZE]) -> u8 {
    let mut best_idx = 0usize;
    let mut best_val = logits[0];
    let mut i = 1;
    while i < VOCAB_SIZE {
        if logits[i] > best_val {
            best_val = logits[i];
            best_idx = i;
        }
        i += 1;
    }
    (best_idx as u8) + 32 // map back to ASCII
}

// ---------------------------------------------------------------------------
// Weight initialization (Xavier-like)
// ---------------------------------------------------------------------------

fn init_weights(model: &mut Model, rng: &mut Rng) {
    // Embedding
    let mut i = 0;
    while i < VOCAB_SIZE {
        let mut j = 0;
        while j < EMBED_DIM {
            model.embed[i][j] = rng.next_scaled(EMBED_DIM);
            j += 1;
        }
        i += 1;
    }

    // Attention projections
    init_matrix::<EMBED_DIM, EMBED_DIM>(&mut model.wq, rng);
    init_matrix::<EMBED_DIM, EMBED_DIM>(&mut model.wk, rng);
    init_matrix::<EMBED_DIM, EMBED_DIM>(&mut model.wv, rng);
    init_matrix::<EMBED_DIM, EMBED_DIM>(&mut model.wo, rng);

    // FFN
    init_matrix::<FF_DIM, EMBED_DIM>(&mut model.ff1_w, rng);
    init_bias::<FF_DIM>(&mut model.ff1_b);
    init_matrix::<EMBED_DIM, FF_DIM>(&mut model.ff2_w, rng);
    init_bias::<EMBED_DIM>(&mut model.ff2_b);

    // Layer norm: gamma=1, beta=0
    init_ln_params(&mut model.ln1_gamma, &mut model.ln1_beta);
    init_ln_params(&mut model.ln2_gamma, &mut model.ln2_beta);

    // Unembed
    i = 0;
    while i < VOCAB_SIZE {
        let mut j = 0;
        while j < EMBED_DIM {
            model.unembed[i][j] = rng.next_scaled(EMBED_DIM);
            j += 1;
        }
        i += 1;
    }
}

fn init_matrix<const ROWS: usize, const COLS: usize>(
    mat: &mut [[f32; COLS]; ROWS],
    rng: &mut Rng,
) {
    let mut i = 0;
    while i < ROWS {
        let mut j = 0;
        while j < COLS {
            mat[i][j] = rng.next_scaled(COLS);
            j += 1;
        }
        i += 1;
    }
}

fn init_bias<const N: usize>(b: &mut [f32; N]) {
    let mut i = 0;
    while i < N {
        b[i] = 0.0;
        i += 1;
    }
}

fn init_ln_params(gamma: &mut [f32; EMBED_DIM], beta: &mut [f32; EMBED_DIM]) {
    let mut i = 0;
    while i < EMBED_DIM {
        gamma[i] = 1.0;
        beta[i] = 0.0;
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn run() {
    unsafe {
        let args = parse_args();

        // Allocate model and scratch via mmap (too large for stack)
        let model_ptr = libc::mmap(
            core::ptr::null_mut(),
            core::mem::size_of::<Model>(),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut Model;
        if model_ptr == libc::MAP_FAILED as *mut Model {
            libc::exit(1);
        }
        let scratch_ptr = libc::mmap(
            core::ptr::null_mut(),
            core::mem::size_of::<Scratch>(),
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut Scratch;
        if scratch_ptr == libc::MAP_FAILED as *mut Scratch {
            libc::exit(1);
        }
        let model = &mut *model_ptr;
        let scratch = &mut *scratch_ptr;

        // Initialize weights with PRNG
        let mut rng = Rng::new(42);
        init_weights(model, &mut rng);

        // Print banner
        print(b"tiny-transformer: GPT-style text generation (untrained)\n");
        print(b"Architecture: ");
        let mut nbuf = [0u8; 10];
        let n = format_u32(NUM_LAYERS as u32, &mut nbuf);
        print(&nbuf[..n]);
        print(b" layer, ");
        let n = format_u32(EMBED_DIM as u32, &mut nbuf);
        print(&nbuf[..n]);
        print(b"d embed, ");
        let n = format_u32(FF_DIM as u32, &mut nbuf);
        print(&nbuf[..n]);
        print(b"d FFN, ");
        let n = format_u32(VOCAB_SIZE as u32, &mut nbuf);
        print(&nbuf[..n]);
        print(b" vocab (ASCII 32-127)\n");
        print(b"Weights: random (untrained) -- output is gibberish by design\n\n");

        // Fill initial context from prompt
        let mut tokens = [b' '; SEQ_LEN];
        let prompt_len = if args.prompt_len > SEQ_LEN {
            SEQ_LEN
        } else {
            args.prompt_len
        };
        let mut i = 0;
        while i < prompt_len {
            tokens[i] = args.prompt[i];
            i += 1;
        }

        // Print prompt
        print(b"Prompt: \"");
        print(&args.prompt[..args.prompt_len]);
        print(b"\"\nOutput: ");
        print(&tokens[..prompt_len]);

        // Autoregressive generation loop
        let mut ctx_len = prompt_len;
        let mut generated = 0u32;

        while generated < args.num_chars {
            let run_len = if ctx_len > 0 { ctx_len } else { 1 };

            // Forward pass
            forward(model, scratch, &tokens, run_len);

            // Get next token via argmax
            let next = argmax(&scratch.logits);

            // Print the generated character
            let ch = [next];
            print(&ch);

            // Append to context or slide window
            if ctx_len < SEQ_LEN {
                tokens[ctx_len] = next;
                ctx_len += 1;
            } else {
                // Slide window: shift left by 1
                let mut j = 0;
                while j < SEQ_LEN - 1 {
                    tokens[j] = tokens[j + 1];
                    j += 1;
                }
                tokens[SEQ_LEN - 1] = next;
            }

            generated += 1;
        }

        print(b"\n\n");
        print(b"Generated ");
        let n = format_u32(generated, &mut nbuf);
        print(&nbuf[..n]);
        print(b" characters.\n");

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
