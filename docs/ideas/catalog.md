# tiny-rust Idea Catalog

Every candidate program we have considered for the repo, beyond the 40 already built.
This is the master backlog so nothing gets lost. For *what to build first*, see [roadmap.md](roadmap.md).

- **248 ideas**, from a 17-domain fan-out brainstorm (253 raw), deduped and scored.
- **Tier** = build-worthiness: `must-build` ★★★★★ · `strong` ★★★★ · `nice` ★★★.
- **no_std** = plausibly buildable in the `no_std` + libc style; `no` likely wants a heap/std.
- **Builds on** = an existing example (01-40) it naturally extends.

Tier mix: **98 must-build**, 135 strong, 15 nice.

## Categories

| Category | Ideas | must-build |
|----------|------:|-----------:|
| [Coreutils & CLI Utilities](#coreutils--cli-utilities) | 16 | 6 |
| [Text Processing & Parsing](#text-processing--parsing) | 14 | 5 |
| [Networking](#networking) | 15 | 6 |
| [System & Observability](#system--observability) | 16 | 6 |
| [Crypto, Hashing & Encoding](#crypto-hashing--encoding) | 17 | 8 |
| [Math & Numerical](#math--numerical) | 15 | 2 |
| [Data Structures & Databases](#data-structures--databases) | 16 | 8 |
| [Compression & Archival](#compression--archival) | 16 | 10 |
| [AI Building Blocks](#ai-building-blocks) | 17 | 3 |
| [AI Models & Applications](#ai-models--applications) | 13 | 3 |
| [Tiny Desktop / GUI](#tiny-desktop--gui) | 14 | 4 |
| [Tiny OS & Bare Metal](#tiny-os--bare-metal) | 16 | 11 |
| [Emulators, VMs & Interpreters](#emulators-vms--interpreters) | 14 | 9 |
| [Compilers & Languages](#compilers--languages) | 11 | 3 |
| [Graphics & Image](#graphics--image) | 14 | 5 |
| [Audio & DSP](#audio--dsp) | 15 | 5 |
| [Dev Tools & Fun](#dev-tools--fun) | 9 | 4 |
| **Total** | **248** | **98** |

## Coreutils & CLI Utilities

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-find` | ★★★★★ | yes | tiny-sandbox | Walk any tree by parsing raw getdents64 directory entries yourself. |
| `tiny-grep` | ★★★★★ | yes | tiny-cat | The classic line searcher with a regex matcher you build by hand. |
| `tiny-head-tail` | ★★★★★ | yes | tiny-multicall | Two utilities, one binary, and a ring buffer that tails in a single pass. |
| `tiny-hexdump` | ★★★★★ | yes | tiny-cat | hexdump -C output from a zero-alloc streaming nibble formatter. |
| `tiny-sort` | ★★★★★ | yes | tiny-alloc | sort without libc: arena-buffered lines and an offset-based merge sort. |
| `tiny-xargs` | ★★★★★ | yes | tiny-pipe | Pipeline glue that spawns processes via raw fork/execve/waitpid. |
| `tiny-cut` | ★★★★☆ | yes | tiny-cat | Slice out columns by byte or delimiter with zero-copy writev output. |
| `tiny-du` | ★★★★☆ | yes | tiny-find | What's eating my disk, answered with block-accurate accounting and hardlink dedup. |
| `tiny-seq` | ★★★★☆ | yes | tiny-yes | Generate ranges with exact decimals and no floating-point drift. |
| `tiny-shuf` | ★★★★☆ | yes | tiny-random | Unbiased Fisher-Yates plus reservoir sampling for streaming -n. |
| `tiny-stat` | ★★★★☆ | yes | tiny-cat | Decode the stat struct and mode bits straight from statx. |
| `tiny-tee` | ★★★★☆ | yes | tiny-pipe | Tap a pipeline to many files with an optional zero-copy splice path. |
| `tiny-tr` | ★★★★☆ | yes | tiny-cat | Branch-free byte transforms driven by a 256-entry lookup table. |
| `tiny-uniq` | ★★★★☆ | yes | tiny-wc | O(1)-memory dedup that proves streaming beats buffering. |
| `tiny-nl` | ★★★☆☆ | yes | tiny-cat | Number lines with writev-joined prefixes and no reallocation. |
| `tiny-tac` | ★★★☆☆ | yes | tiny-mmap | cat in reverse via a backward scan over an mmapped file. |

## Text Processing & Parsing

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-calc` | ★★★★★ | yes | - | A calculator that teaches the Pratt parser behind every language. |
| `tiny-diff` | ★★★★★ | yes | tiny-cat | The Myers algorithm that powers git diff, built from the edit graph up. |
| `tiny-jq` | ★★★★★ | yes | tiny-json | A pocket jq: compile a path, walk the JSON tree. |
| `tiny-json` | ★★★★★ | yes | - | A from-scratch JSON parser and formatter with no serde in sight. |
| `tiny-regex` | ★★★★★ | yes | - | Linear-time regex via Thompson NFA simulation, no backtracking blowups. |
| `tiny-csv` | ★★★★☆ | yes | tiny-cat | Real CSV done right: a state machine that survives quotes and newlines. |
| `tiny-freq` | ★★★★☆ | yes | tiny-wc | A no-std hash map and top-k, taught through counting words. |
| `tiny-glob` | ★★★★☆ | yes | - | Shell-style wildcard matching as a gentle on-ramp to regex. |
| `tiny-highlight` | ★★★★☆ | yes | - | Lexing made visible: source in, ANSI colors out. |
| `tiny-ini` | ★★★★☆ | yes | - | Parse sectioned config and answer get section.key queries. |
| `tiny-md` | ★★★★☆ | yes | - | Markdown to HTML via a clean block-then-inline two-pass parser. |
| `tiny-template` | ★★★★☆ | yes | - | Mustache-style rendering with nested sections and a tag stack. |
| `tiny-url` | ★★★★☆ | yes | - | Decompose and percent-decode any URL, by the spec. |
| `tiny-wrap` | ★★★★☆ | yes | - | Reflow text to any width, with real UTF-8 width accounting. |

## Networking

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-curl` | ★★★★★ | yes | tiny-server | Fetch a URL from scratch, chunked decoding and all. |
| `tiny-dns` | ★★★★★ | yes | tiny-udp-echo | Resolve a name to its bytes, packet by packet. |
| `tiny-nc` | ★★★★★ | yes | tiny-pipe | Pump two byte streams across a socket with one poll loop. |
| `tiny-ping` | ★★★★★ | yes | raw-syscall | Craft an ICMP echo by hand and time the round trip. |
| `tiny-traceroute` | ★★★★★ | yes | tiny-portscan | Watch packets hop the internet by abusing TTL expiry. |
| `tiny-ws` | ★★★★★ | yes | tiny-server | Hand-roll the RFC 6455 handshake and frame an echo. |
| `tiny-arp` | ★★★★☆ | yes | raw-syscall | Map every IP-to-MAC on the wire with raw layer-2 frames. |
| `tiny-dhcp` | ★★★★☆ | yes | tiny-udp-echo | Run the four-packet dance that hands every device its IP. |
| `tiny-ntp` | ★★★★☆ | yes | tiny-udp-echo | Ask the internet for the time and do the epoch math. |
| `tiny-tftp` | ★★★★☆ | yes | tiny-udp-echo | Reliable file transfer built from scratch on lossy UDP. |
| `tiny-bwmeter` | ★★★☆☆ | yes | tiny-server | iperf in a hundred lines: blast bytes, report Mbit/s. |
| `tiny-knock` | ★★★☆☆ | yes | tiny-portscan | Open a port with a secret knock the firewall is listening for. |
| `tiny-mdns` | ★★★☆☆ | yes | tiny-udp-echo | Be discoverable on the LAN: the Bonjour magic, in miniature. |
| `tiny-syslog` | ★★★☆☆ | yes | tiny-udp-echo | Catch syslog datagrams and unpack the PRI bit math. |
| `tiny-whois` | ★★★☆☆ | yes | tiny-server | Query domain ownership over the oldest directory protocol still in use. |

## System & Observability

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-cgroup-run` | ★★★★★ | yes | tiny-sandbox | Cap any command's CPU and memory with a throwaway cgroup v2, like a mini runtime. |
| `tiny-init` | ★★★★★ | yes | tiny-signal | tini in 100 lines: forward signals, reap zombies, be a proper PID 1. |
| `tiny-inotify` | ★★★★★ | yes | - | The minimal inotify reference: stream file events and trigger commands on change. |
| `tiny-ss` | ★★★★★ | yes | tiny-portscan | netstat from scratch: decode /proc/net hex and pin every socket to its owning PID. |
| `tiny-strace` | ★★★★★ | yes | tiny-sandbox | Trace a process's syscalls live with raw ptrace. |
| `tiny-top` | ★★★★★ | yes | - | A top clone: /proc parsing and ANSI redraw, no TUI library. |
| `tiny-cron` | ★★★★☆ | yes | - | Crontab fields in, next-fire-time computed, job fork/exec'd on schedule. |
| `tiny-dmesg` | ★★★★☆ | yes | - | dmesg from the raw /dev/kmsg stream, priority and timestamp decoded. |
| `tiny-killtree` | ★★★★☆ | yes | - | Reconstruct the process tree from PPid and kill a PID with all its descendants. |
| `tiny-lsof` | ★★★★☆ | yes | - | See what's holding a file or port open, straight from procfs. |
| `tiny-pidwait` | ★★★★☆ | yes | tiny-signal | Race-free wait on any PID you didn't fork, via pidfd_open + poll. |
| `tiny-pmap` | ★★★★☆ | yes | tiny-mmap | Lay bare a process's address space, VMA by VMA. |
| `tiny-supervisor` | ★★★★☆ | yes | tiny-init | Restart-on-crash with backoff and a crash-loop cap, no systemd required. |
| `tiny-time` | ★★★★☆ | yes | - | /usr/bin/time from scratch: wall, CPU, and max-RSS via getrusage. |
| `tiny-vmstat` | ★★★★☆ | yes | tiny-top | Per-second CPU/memory/IO deltas computed from raw /proc counters. |
| `tiny-watch` | ★★★★☆ | yes | tiny-pipe | watch(1) that flashes exactly which bytes changed between runs. |

## Crypto, Hashing & Encoding

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-aes` | ★★★★★ | yes | xor-packer | AES-128-CTR from the GF(2^8) field up, demystifying the hardware instruction. |
| `tiny-base32` | ★★★★★ | yes | tiny-base64 | 5-bit Base32 for TOTP secrets and onion addresses, the prerequisite for tiny-totp. |
| `tiny-chacha20` | ★★★★★ | yes | xor-packer | The TLS stream cipher for AES-less hardware, built from pure add-rotate-xor. |
| `tiny-crc32` | ★★★★★ | yes | tiny-hash | Table-driven CRC32/CRC32C: the integrity check inside gzip, zip, and PNG. |
| `tiny-ed25519` | ★★★★★ | yes | tiny-x25519 | Deterministic signatures for SSH and git, no nonce-reuse footgun. |
| `tiny-hmac` | ★★★★★ | yes | tiny-sha256 | Keyed HMAC-SHA256 with constant-time verify, the root of TOTP, HKDF, and webhooks. |
| `tiny-totp` | ★★★★★ | yes | tiny-hmac | A command-line Google Authenticator: base32 secret in, 6-digit code out. |
| `tiny-x25519` | ★★★★★ | yes | tiny-random | The Montgomery ladder behind TLS, SSH, WireGuard, and Signal key agreement. |
| `tiny-base58` | ★★★★☆ | yes | tiny-base64 | Typo-resistant Bitcoin/IPFS identifiers via big-int radix-58 with checksum. |
| `tiny-hex` | ★★★★☆ | yes | tiny-multicall | xxd, urlencode, and ROT13 in one tiny multicall binary. |
| `tiny-md5` | ★★★★☆ | yes | tiny-sha256 | The broken-but-everywhere hash, built to teach Merkle-Damgard the SHA family shares. |
| `tiny-pbkdf2` | ★★★★☆ | yes | tiny-hmac | Turn a password into a key that is expensive to brute-force. |
| `tiny-poly1305` | ★★★★☆ | yes | tiny-chacha20 | Polynomial-eval MAC mod 2^130-5 that completes ChaCha20-Poly1305 AEAD. |
| `tiny-rsa` | ★★★★☆ | yes | tiny-dh | Miller-Rabin keygen to sign-and-verify: RSA's number theory made concrete. |
| `tiny-shamir` | ★★★★☆ | yes | tiny-hex | Split a secret so any K of N people can rebuild it. |
| `tiny-siphash` | ★★★★☆ | yes | tiny-hash | The keyed hash that stops your HashMap from getting DoS'd. |
| `tiny-dh` | ★★★☆☆ | yes | tiny-random | The original public-key handshake: modexp over a prime, the gateway to RSA. |

## Math & Numerical

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-bignum` | ★★★★★ | yes | - | Compute 1000! exactly with hand-rolled multi-limb arithmetic. |
| `tiny-fft` | ★★★★★ | yes | - | The algorithm that made signal processing fast, in one file. |
| `tiny-cordic` | ★★★★☆ | yes | - | Trig from nothing but shifts and adds, no libm required. |
| `tiny-factor` | ★★★★☆ | yes | tiny-sieve | Crack any 64-bit integer into primes with Pollard's rho. |
| `tiny-integrate` | ★★★★☆ | yes | tiny-root | Integrate accurately by subdividing only where it's hard. |
| `tiny-life` | ★★★★☆ | yes | tiny-random | Tiny rules, emergent gliders, live in your terminal. |
| `tiny-matrix` | ★★★★☆ | yes | - | Multiply, invert, and LU-factor matrices in a few hundred lines. |
| `tiny-pi` | ★★★★☆ | yes | tiny-bignum | Stream pi's digits forever using only integer math. |
| `tiny-primality` | ★★★★☆ | yes | - | Provably decide any 64-bit number's primality with a fixed witness set. |
| `tiny-quat` | ★★★★☆ | yes | - | Gimbal-lock-free 3D rotations with smooth SLERP blending. |
| `tiny-rational` | ★★★★☆ | yes | - | Fraction math with no rounding error, ever. |
| `tiny-root` | ★★★★☆ | yes | - | Newton for speed, bisection for safety, roots either way. |
| `tiny-rpn` | ★★★★☆ | yes | - | A stack calculator you'll actually keep in your shell. |
| `tiny-sieve` | ★★★★☆ | yes | tiny-mmap | Sieve hundreds of millions of primes through an mmap'd bitset. |
| `tiny-stats` | ★★★★☆ | yes | - | Mean, stddev, and median over an infinite stream in O(1) memory. |

## Data Structures & Databases

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-bloom` | ★★★★★ | yes | tiny-hash | Probabilistic set membership in a fraction of the memory. |
| `tiny-btree` | ★★★★★ | yes | tiny-mmap | The page-based index behind every SQL database, on disk. |
| `tiny-lru` | ★★★★★ | yes | tiny-alloc | The map-plus-linked-list trick that makes eviction O(1). |
| `tiny-lsm` | ★★★★★ | yes | tiny-kv | The write-optimized engine behind RocksDB, built end to end. |
| `tiny-redis` | ★★★★★ | yes | tiny-server | Speak RESP and let redis-cli talk to your own server. |
| `tiny-ring` | ★★★★★ | yes | tiny-mmap | Wait-free message passing with nothing but two atomic counters. |
| `tiny-search` | ★★★★★ | yes | tiny-hash | Build the inverted index that powers every search box. |
| `tiny-wal` | ★★★★★ | yes | tiny-kv | Crash-proof your state by writing it down before you mean it. |
| `tiny-columnar` | ★★★★☆ | yes | tiny-sql-db | Store by column, scan at warp speed, compress for free. |
| `tiny-graph` | ★★★★☆ | yes | tiny-alloc | Cache-friendly graphs and shortest paths in one CSR package. |
| `tiny-hll` | ★★★★☆ | yes | tiny-hash | Count billions of distinct items in a few kilobytes. |
| `tiny-pager` | ★★★★☆ | yes | tiny-mmap | The pin-and-evict page cache that sits under every database. |
| `tiny-roaring` | ★★★★☆ | yes | tiny-hll | Adaptive bitmaps that switch containers to crush sparse sets. |
| `tiny-skiplist` | ★★★★☆ | yes | tiny-alloc | An ordered map that gets balance from coin flips, not rotations. |
| `tiny-trie` | ★★★★☆ | yes | tiny-alloc | Prefix search and autocomplete via path-compressed tries. |
| `tiny-tsdb` | ★★★★☆ | yes | tiny-kv | The Facebook Gorilla trick that squeezes metrics 10x. |

## Compression & Archival

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-bwt` | ★★★★★ | yes | tiny-rle | The magical reversible reshuffle behind bzip2 and genome indexing, forward and inverse. |
| `tiny-deflate` | ★★★★★ | yes | tiny-lzss | Write gzip files that real gunzip happily unpacks. |
| `tiny-huffman` | ★★★★★ | yes | tiny-base64 | The optimal prefix code at the heart of DEFLATE and JPEG. |
| `tiny-inflate` | ★★★★★ | yes | tiny-huffman | Read real .gz files with zero dependencies on zlib. |
| `tiny-lz4` | ★★★★★ | yes | tiny-lzss | Byte-exact LZ4: the speed king of real-time compression. |
| `tiny-lzss` | ★★★★★ | yes | tiny-mmap | Slide a window, point back at repeats, shrink the stream. |
| `tiny-rans` | ★★★★★ | yes | tiny-rangecoder | The zstd-era entropy coder demystified: arithmetic-coding compression at table-lookup speed. |
| `tiny-tar` | ★★★★★ | yes | tiny-cat | Pack and unpack tarballs from 512-byte header blocks. |
| `tiny-unzip` | ★★★★★ | yes | tiny-inflate | Crack open any .zip, .jar, .docx, or .apk yourself. |
| `tiny-varint` | ★★★★★ | yes | tiny-cat | The 7-bits-at-a-time encoding behind protobuf and Wasm. |
| `tiny-delta` | ★★★★☆ | yes | tiny-varint | Subtract neighbors to make smooth data nearly vanish. |
| `tiny-lzw` | ★★★★☆ | yes | tiny-cat | The self-building dictionary that compressed every GIF. |
| `tiny-rangecoder` | ★★★★☆ | yes | tiny-huffman | Code in fractional bits and beat Huffman at its own game. |
| `tiny-rle` | ★★★★☆ | yes | tiny-cat | The friendliest first codec: collapse runs, round-trip perfectly. |
| `tiny-cpio` | ★★★☆☆ | yes | tiny-tar | Build the cpio archives the kernel unpacks at boot. |
| `tiny-snappy` | ★★★☆☆ | yes | tiny-lz4 | Google's Snappy: a second LZ format to contrast with LZ4. |

## AI Building Blocks

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-attention` | ★★★★★ | yes | tiny-transformer | The one transformer kernel, isolated to pure matrix math you can test in your head. |
| `tiny-autograd` | ★★★★★ | yes | tiny-alloc | Backprop laid bare: a whole autodiff engine you can single-step through. |
| `tiny-bpe` | ★★★★★ | yes | tiny-gpt2 | Reproduce GPT's exact text-to-token-IDs step and finally see what a token really is. |
| `tiny-adam` | ★★★★☆ | yes | - | Race Adam against plain SGD on a noisy curve and see why Adam wins. |
| `tiny-bpe-train` | ★★★★☆ | yes | tiny-bpe | Watch a tokenizer vocabulary build itself, one most-frequent merge at a time. |
| `tiny-conv2d` | ★★★★☆ | yes | tiny-mmap | One sliding-window primitive that is both a blur filter and a CNN layer. |
| `tiny-dtree` | ★★★★☆ | yes | - | A gradient-free learner that outputs human-readable if-then rules via Gini splits. |
| `tiny-kmeans` | ★★★★☆ | yes | tiny-random | Unsupervised clustering in its cleanest form: assign, average, repeat. |
| `tiny-kvcache` | ★★★★☆ | yes | tiny-attention | The cache trick that turns quadratic LLM decoding into linear streaming, measured. |
| `tiny-logreg` | ★★★★☆ | yes | tiny-adam | The smallest honest supervised learner, tying probability, loss, and gradients together. |
| `tiny-lstm` | ★★★★☆ | yes | tiny-autograd | The gated-memory cell that ruled sequence modeling before transformers, gate by gate. |
| `tiny-mlp` | ★★★★☆ | yes | tiny-autograd | A neural net that learns XOR from scratch, loss curve printing to your terminal. |
| `tiny-ngram` | ★★★★☆ | yes | - | A pre-neural language model that makes perplexity and context length tangible. |
| `tiny-quantize` | ★★★★☆ | yes | tiny-gpt2 | Shrink a weight tensor 4x to int8 and measure exactly what accuracy it costs. |
| `tiny-sampler` | ★★★★☆ | yes | tiny-random | Turn those mysterious temperature and top-p knobs into code you can read. |
| `tiny-word2vec` | ★★★★☆ | yes | tiny-autograd | Train vectors where king minus man plus woman lands near queen, from raw text. |
| `tiny-knn` | ★★★☆☆ | yes | - | The classifier with no training step: just vote with your nearest neighbors. |

## AI Models & Applications

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-llama` | ★★★★★ | yes | tiny-gpt2 | GPT-2's modern cousin: RoPE, RMSNorm, SwiGLU and GQA in one small inference loop. |
| `tiny-mnist` | ★★★★★ | yes | tiny-transformer | The hello-world neural net, running real digit inference in a framework-free no_std binary. |
| `tiny-quant` | ★★★★★ | yes | tiny-gpt2 | Shrink model weights 4x and matmul them in int8, scale math included. |
| `tiny-charrnn` | ★★★★☆ | yes | tiny-gpt2 | Sequence modeling before transformers: a tiny RNN that hallucinates text one char at a time. |
| `tiny-cnn` | ★★★★☆ | yes | tiny-mnist | A full vision pipeline, conv to pool to classify, with zero library calls. |
| `tiny-ctc` | ★★★★☆ | yes | tiny-mfcc | How variable-length audio becomes text with no alignment labels, collapsed frame by frame. |
| `tiny-diffusion` | ★★★★☆ | yes | tiny-cnn | Watch noise become an image: the DDPM reverse loop with nothing hidden. |
| `tiny-embed` | ★★★★☆ | yes | tiny-server | Text in, sentence vector out: a whole embedding microservice in a tiny no_std binary. |
| `tiny-mfcc` | ★★★★☆ | yes | tiny-fft | The exact DSP pipeline that turns raw audio into the features every ASR model eats. |
| `tiny-rag` | ★★★★☆ | yes | tiny-vecsearch | RAG laid bare: embed, retrieve, and stitch a cited prompt with zero model calls. |
| `tiny-tts` | ★★★★☆ | yes | tiny-fft | Make the speakers talk: phonemes to formants to audible WAV, no neural net. |
| `tiny-vae` | ★★★★☆ | yes | tiny-cnn | Sample a latent, decode a picture: generative modeling at its simplest. |
| `tiny-vecsearch` | ★★★★☆ | yes | tiny-objstore | The RAG retrieval engine revealed as nothing but normalized dot products and a heap. |

## Tiny Desktop / GUI

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-font` | ★★★★★ | yes | tiny-x11 | The glyph-drawing primitive every other GUI app here will reuse. |
| `tiny-png` | ★★★★★ | yes | tiny-imgview | PNG from the bytes up: inflate, CRC, and scanline filters, no image crate. |
| `tiny-term` | ★★★★★ | yes | tiny-font | pty, forked shell, and a VT100 parser, the whole terminal stack in one binary. |
| `tiny-wm` | ★★★★★ | yes | tiny-x11 | Be the window manager: redirect, reparent, and drag-resize everyone else's windows. |
| `tiny-clock` | ★★★★☆ | yes | tiny-x11 | A ticking analog clock that teaches frame timing via poll() in raw X11. |
| `tiny-edit` | ★★★★☆ | yes | tiny-font | A usable editor on a gap buffer that decodes raw X11 keycodes by hand. |
| `tiny-fb` | ★★★★☆ | yes | tiny-mmap | No X, no server: mmap /dev/fb0 and paint plasma straight to the screen. |
| `tiny-imgview` | ★★★★☆ | yes | tiny-font | Decode a pixel buffer and slam it to the screen in one PutImage call. |
| `tiny-paint` | ★★★★☆ | yes | tiny-x11 | Freehand drawing that turns raw X11 motion events into strokes you can save. |
| `tiny-screenshot` | ★★★★☆ | yes | tiny-x11 | One GetImage request turns the whole desktop into a PPM file. |
| `tiny-sysmon` | ★★★★☆ | yes | tiny-x11 | A conky-style HUD: parse /proc, graph it live, dodge the window manager. |
| `tiny-tetris` | ★★★★☆ | yes | tiny-x11 | Real Tetris in raw X11: gravity on a poll tick, collisions on a grid. |
| `tiny-vnc` | ★★★★☆ | yes | tiny-screenshot | Remote desktop from scratch: RFB on the wire, GetImage frames, injected input. |
| `tiny-xeyes` | ★★★☆☆ | yes | tiny-x11 | The beloved xeyes clone: globally track the cursor and draw arcs that watch you. |

## Tiny OS & Bare Metal

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-gdt` | ★★★★★ | no | tiny-bootsector | Escape real mode: hand-pack a GDT and far-jump your way into protected mode. |
| `tiny-idt` | ★★★★★ | no | tiny-multiboot | Catch a divide-by-zero instead of triple-faulting: build an IDT from scratch. |
| `tiny-keyboard` | ★★★★★ | no | tiny-idt | Turn raw IRQ1 scancodes into keystrokes your kernel can actually read. |
| `tiny-kheap` | ★★★★★ | no | tiny-paging | Get Box and Vec inside your kernel by writing your own GlobalAlloc. |
| `tiny-kshell` | ★★★★★ | no | tiny-keyboard | A heap-free ring-0 shell that ties VGA, keyboard, and peek/poke together. |
| `tiny-ktasks` | ★★★★★ | no | tiny-kheap | Swap stacks by hand and watch cooperative multitasking demystify itself. |
| `tiny-multiboot` | ★★★★★ | no | no-std | Your first kernel: GRUB loads it, it writes colored text to 0xb8000. |
| `tiny-paging` | ★★★★★ | no | tiny-multiboot | Make virtual memory tangible: build a 4-level page table and load CR3. |
| `tiny-pit` | ★★★★★ | no | tiny-idt | Give your kernel a heartbeat: remap the PIC and count PIT timer ticks. |
| `tiny-ring3` | ★★★★★ | no | tiny-idt | Cross the ring 0/3 boundary: drop to user mode and service a real syscall. |
| `tiny-riscv` | ★★★★★ | no | tiny-multiboot | Bare-metal without the x86 baggage: boot RISC-V and blink the UART. |
| `tiny-bootsector` | ★★★★☆ | no | global-asm | 512 bytes, an 0xAA55 magic word, and a BIOS call: a CPU's very first instructions. |
| `tiny-fat12` | ★★★★☆ | yes | tiny-cat | Walk a real cluster chain: list and extract files from a FAT12 floppy. |
| `tiny-mbr` | ★★★★☆ | yes | tiny-cat | Decode the four partition entries hiding in a disk's first 512 bytes. |
| `tiny-serial` | ★★★★☆ | no | tiny-multiboot | The kernel debugger's lifeline: port I/O to COM1 gives you println! on bare metal. |
| `tiny-vga13` | ★★★★☆ | no | tiny-multiboot | From text to pixels: flip VGA into mode 13h and draw a plasma. |

## Emulators, VMs & Interpreters

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-6502` | ★★★★★ | yes | - | Build the chip behind the NES and C64, verified against Klaus' test ROM. |
| `tiny-8080` | ★★★★★ | yes | - | Emulate x86's grandfather and prove it by passing 8080EXM. |
| `tiny-chip8` | ★★★★★ | yes | tiny-cat | The canonical first emulator: 35 opcodes and a 64x32 screen in the terminal. |
| `tiny-forth` | ★★★★★ | yes | - | A self-extending language in a few KB: build Forth with threaded code. |
| `tiny-jit` | ★★★★★ | yes | tiny-mmap | Emit raw x86-64 bytes into an RWX page and call your compiler's output. |
| `tiny-lc3` | ★★★★★ | yes | tiny-vm | Boot the textbook LC-3 CPU and run real .obj programs in a few hundred lines. |
| `tiny-lisp` | ★★★★★ | yes | tiny-alloc | The classic eval/apply: closures and cons cells from first principles. |
| `tiny-regex-vm` | ★★★★★ | yes | tiny-stackvm | Compile regex to bytecode and match in linear time, Pike-VM style. |
| `tiny-stackvm` | ★★★★★ | yes | - | The dispatch loop behind Python and the JVM, distilled to its essentials. |
| `tiny-bf` | ★★★★☆ | yes | tiny-cat | Eight commands, one tape: the smallest Turing-complete interpreter you can write. |
| `tiny-gameboy-cpu` | ★★★★☆ | yes | tiny-z80 | The CPU at the heart of the GameBoy, validated against Blargg's cpu_instrs. |
| `tiny-pratt` | ★★★★☆ | yes | - | The precedence-parsing trick real language front-ends use, in one tiny REPL. |
| `tiny-wasm-interp` | ★★★★☆ | yes | tiny-wasm | Run a real .wasm module yourself -- a pocket WebAssembly engine. |
| `tiny-z80` | ★★★★☆ | yes | tiny-8080 | Layer prefix opcode pages onto the 8080 to bring the ZX Spectrum's CPU to life. |

## Compilers & Languages

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-bf-jit` | ★★★★★ | yes | tiny-mmap | A real JIT in miniature: Brainfuck straight to native machine code you jump into. |
| `tiny-elf-emit` | ★★★★★ | yes | raw-syscall | Emit a runnable Linux executable by hand -- be your own linker. |
| `tiny-vm` | ★★★★★ | yes | - | The fetch-decode-execute heart of every bytecode VM, distilled. |
| `tiny-coroutine` | ★★★★☆ | yes | global-asm | Hand-rolled register-swapping coroutines: the machinery under async/await. |
| `tiny-elf-dump` | ★★★★☆ | yes | raw-syscall | A pocket nm/objdump that cracks open any ELF binary. |
| `tiny-gc` | ★★★★☆ | yes | tiny-alloc | Watch a tracing GC find the garbage and reclaim it, algorithm laid bare. |
| `tiny-leb128` | ★★★★☆ | yes | tiny-base64 | The varint codec hiding inside wasm, DWARF, and protobuf, on its own. |
| `tiny-peephole` | ★★★★☆ | yes | - | Local rewrite rules that visibly shrink IR -- the simplest real optimizer. |
| `tiny-wat` | ★★★★☆ | yes | tiny-wasm | Assemble human-readable WAT into a real .wasm binary. |
| `tiny-cpp` | ★★★☆☆ | yes | tiny-cat | The #define/#include/#if engine that runs before any C compiler sees your code. |
| `tiny-shunting` | ★★★☆☆ | yes | - | Dijkstra's shunting-yard: infix to RPN to answer, with just a stack. |

## Graphics & Image

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-gif` | ★★★★★ | yes | tiny-quantize | Hand-roll the variable-width LZW coder that makes every GIF on the internet possible. |
| `tiny-ppm` | ★★★★★ | yes | tiny-cat | The simplest real image format -- the pixel-output base for every graphics demo. |
| `tiny-qr` | ★★★★★ | yes | tiny-ppm | Generate a phone-scannable QR code and learn Reed-Solomon doing it. |
| `tiny-raster` | ★★★★★ | yes | tiny-ppm | Build the core of a GPU in software: edge-function triangles with a Z-buffer. |
| `tiny-raytrace` | ★★★★★ | yes | tiny-ppm | Hundreds of lines of arithmetic that turn into a shaded, shadowed render. |
| `tiny-blur` | ★★★★☆ | yes | tiny-sobel | The separable-kernel trick that makes Gaussian blur cheap enough for real time. |
| `tiny-bmp` | ★★★★☆ | yes | tiny-ppm | Round-trip a real BMP file and learn headers, endianness, and stride padding. |
| `tiny-colorconv` | ★★★★☆ | yes | tiny-ppm | A pocket calculator for color: RGB, HSV, YCbCr, and the gamma curve nobody understands. |
| `tiny-dither` | ★★★★☆ | yes | tiny-ppm | Fake a thousand shades from two colors with error diffusion. |
| `tiny-img2ascii` | ★★★★☆ | yes | tiny-cat | Squash any image into terminal art and learn perceptual luminance along the way. |
| `tiny-mandelbrot` | ★★★★☆ | yes | tiny-ppm | z = z^2 + c per pixel -- infinite beauty from a five-line loop. |
| `tiny-sobel` | ★★★★☆ | yes | tiny-ppm | Find the edges in any image with two 3x3 kernels. |
| `tiny-svg` | ★★★★☆ | yes | tiny-raster | Turn vector SVG shapes into pixels with the same scanline fill every GPU and renderer uses. |
| `tiny-barcode` | ★★★☆☆ | yes | tiny-ppm | A scannable Code 128 barcode and a gentle on-ramp to QR. |

## Audio & DSP

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-biquad` | ★★★★★ | yes | tiny-fir | Five multiplies per sample become any EQ shape: the RBJ biquad cookbook in Rust. |
| `tiny-karplus` | ★★★★★ | yes | tiny-reverb | Noise plus a tuned feedback loop equals a startlingly real plucked string. |
| `tiny-spectrogram` | ★★★★★ | yes | tiny-fft | Watch sound become an image via the short-time Fourier transform, no libraries. |
| `tiny-synth` | ★★★★★ | yes | tiny-tone | Type note names, get music: oscillators, ADSR, and polyphony in a single binary. |
| `tiny-wav` | ★★★★★ | yes | tiny-cat | The zero-dependency WAV codec that every other audio demo in the set stands on. |
| `tiny-adpcm` | ★★★★☆ | yes | tiny-wav | A real 4:1 audio codec in a state machine: adaptive prediction, four bits per sample. |
| `tiny-dtmf` | ★★★★☆ | yes | tiny-tone | Dial and decode touch-tones with Goertzel, the FFT's lean single-frequency cousin. |
| `tiny-fir` | ★★★★☆ | yes | tiny-wav | Design a filter from the sinc function up and convolve it over real audio. |
| `tiny-mp3-parse` | ★★★★☆ | yes | tiny-cat | Walk an MP3's frame headers to read its duration and bitrate without decoding a sample. |
| `tiny-pitch` | ★★★★☆ | yes | tiny-wav | A guitar tuner from scratch using the YIN algorithm, accurate down to the cent. |
| `tiny-resample` | ★★★★☆ | yes | tiny-fir | Convert 44.1k to 48k correctly, and finally feel the sampling theorem in your bones. |
| `tiny-reverb` | ★★★★☆ | yes | tiny-wav | One circular delay buffer, a whole family of effects: echo, reverb, chorus, flanger. |
| `tiny-scope` | ★★★★☆ | yes | tiny-wav | A triggered oscilloscope trace rendered in Braille right in your terminal. |
| `tiny-tone` | ★★★★☆ | yes | tiny-wav | A no_std signal bench: sines, sweeps, and the phase accumulator behind all synthesis. |
| `tiny-tracker` | ★★★★☆ | yes | tiny-synth | Amiga-era tracker music from a plain-text pattern grid, mixed channel by channel. |

## Dev Tools & Fun

| Idea | Tier | no_std | Builds on | What you get out of it |
|------|------|:------:|-----------|------------------------|
| `tiny-elf` | ★★★★★ | yes | tiny-mmap | A pocket readelf that turns the byte layout of every Linux binary into plain text. |
| `tiny-git-cat` | ★★★★★ | yes | tiny-sha256 | Read raw .git objects with a hand-rolled inflate and prove git is just zipped files. |
| `tiny-maze` | ★★★★★ | yes | tiny-random | Carve a maze with DFS, solve it with BFS, render it in box-drawing glyphs. |
| `tiny-sudoku` | ★★★★★ | yes | - | 81 chars in, solved grid out: backtracking plus MRV in a few hundred lines. |
| `tiny-disasm` | ★★★★☆ | yes | tiny-elf | Decode ModRM and REX by hand and watch raw x86-64 bytes turn into mnemonics. |
| `tiny-fuzz` | ★★★★☆ | yes | tiny-random | A dumb-but-effective fuzzer: mutate, exec, and catch the crash signal as a repro. |
| `tiny-make` | ★★★★☆ | yes | tiny-pipe | A working make in miniature: dependency DAG, mtime staleness, and fork/exec recipes. |
| `tiny-markov` | ★★★★☆ | yes | tiny-random | Feed it text, get plausible nonsense back: generative modeling in a hash map. |
| `tiny-profiler` | ★★★★☆ | yes | tiny-signal | Find hot code the way perf does: a SIGPROF timer sampling the instruction pointer. |

