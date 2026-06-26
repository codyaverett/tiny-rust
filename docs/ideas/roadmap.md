# tiny-rust Roadmap

What to focus on up front, and the themed series that organize the rest.
Full backlog of all 248 ideas: [catalog.md](catalog.md). Org/numbering scheme: [organization.md](organization.md).

The repo's superpower is that it makes hard things look small and fun. A first-time visitor should hit something they instantly recognize and want to copy. So the first milestone deliberately spans four domains with maximum shareability: tiny-grep (the most relatable CLI on earth), tiny-chip8 (an emulator that plays real games in your terminal), tiny-redis (real redis-cli connecting to your own server), and tiny-raytrace (a from-scratch render image people post). These prove breadth - tooling, emulation, networking, graphics - in one screenful and are all flagship (mostly five-star) yet achievable on the existing no_std + libc pattern. The focus batch then front-loads the cheap foundations that unlock whole series (tiny-ppm for graphics, tiny-json for parsing, tiny-wav for audio, tiny-bf before the harder VMs) so every later example gets faster to build, and it seeds one anchor per major domain (CLI, parsing, networking, audio, compression, AI, data structures) so no track starts cold. The eight series convert 248 loose ideas into 'build your own X' journeys - the format that makes people commit - each ordered so difficulty compounds, and each tied to the repo's existing strengths (it already has networking 20-31 and AI 35-36, so Parse/Network/AI tracks feel like continuations, not pivots). The org recommendation protects the one thing that makes this repo special - the single numbered progression - by keeping dirs flat and chronological and pushing all category/series structure into a generated README driven by a single CATALOG source of truth, with a one-time 3-digit renumber now (while it is cheap) to keep sort order honest to 999 entries.

## First milestone (build these now)

The handful that show breadth and pull people into the repo.

1. **`tiny-grep`** - Every developer on earth knows grep, so it instantly answers 'what is this repo?' A hand-built regex matcher over the established no_std + libc pattern is achievable, satisfying, and the most relatable possible entry point into the new practical-utilities arc.
2. **`tiny-chip8`** - The canonical 'first emulator' - 35 opcodes and a 64x32 screen that runs REAL downloadable games in the terminal. It is the single most shareable, screenshot-worthy build in the catalog and proves the repo is fun, not just academic. Opens the whole Emulators series.
3. **`tiny-redis`** - You point real redis-cli at your own from-scratch server and it just works - that 'whoa' moment is what makes people star and stay. Leans on the repo's proven networking strength (20-31) and anchors the data-structures track.
4. **`tiny-raytrace`** - A shaded, shadowed sphere render is the iconic 'I built this from scratch' image. It gives the catalog immediate visual breadth beyond CLI/network tooling and pulls in the graphics crowd. Pairs with tiny-ppm as its output format.

## Focus batch (first ~15, in build order)

Proposed numbers continue from the existing 40.

| # | Idea | no_std | Why it earns a spot |
|---|------|:------:|---------------------|
| 41 | `tiny-grep` | yes | Milestone. Most relatable CLI flagship; hand-rolled regex matcher; opens the Unix Toolbox series. |
| 42 | `tiny-ppm` | yes | Graphics foundation every visual demo reuses; trivially simple, immediate pixel output. Must build before raytrace. |
| 43 | `tiny-raytrace` | yes | Milestone. Beautiful, shareable graphics payoff; consumes tiny-ppm; broadens the repo past tooling. |
| 44 | `tiny-json` | yes | Parsing flagship and dependency for tiny-jq and many tools; demonstrates a from-scratch parser with no serde. |
| 45 | `tiny-redis` | yes | Milestone. redis-cli talks to your server; networking + data-structures crossover; high wow-factor. |
| 46 | `tiny-chip8` | yes | Milestone. Runs real games in the terminal; the most fun/shareable build; anchors the emulator track. |
| 47 | `tiny-curl` | yes | Fetch a URL from scratch with chunked decoding - universally relatable, extends networking lineage. |
| 48 | `tiny-calc` | yes | Teaches the Pratt parser behind every language; small, satisfying, and the on-ramp to the interpreters series. |
| 49 | `tiny-bf` | yes | Smallest Turing-complete interpreter; easy win that sets up the VM/JIT progression before harder emulators. |
| 50 | `tiny-wav` | yes | Audio foundation every DSP demo stands on; zero-dependency codec, simple and reusable. |
| 51 | `tiny-synth` | yes | Type notes, hear music - delightful payoff that opens a brand-new audio domain; consumes tiny-wav. |
| 52 | `tiny-inflate` | yes | Read real .gz files with no zlib - a jaw-drop demo that anchors the compression series. |
| 53 | `tiny-mnist` | yes | The hello-world neural net doing real digit inference; complements existing 35/36 and opens AI-from-scratch to beginners. |
| 54 | `tiny-find` | yes | Raw getdents64 tree walk; pairs with grep to start a credible coreutils suite and shows syscall depth. |
| 55 | `tiny-lru` | yes | Data-structures flagship (map + linked list, O(1) eviction); foundational primitive reused by later DB examples. |

## Learning series

Curated tracks to present the catalog to new eyes. Each is an ordered path where every step builds on the last.

### The Unix Toolbox

Rebuild the shell you use every day from raw syscalls - directory walking, line searching, sorting, process glue - then graduate to the tools that watch the system itself.

`tiny-find` -> `tiny-grep` -> `tiny-cut` -> `tiny-sort` -> `tiny-xargs` -> `tiny-strace` -> `tiny-top`

### Parse Anything

Go from byte streams to structured meaning: tokenizers, state machines, recursive-descent and Pratt parsing, ending with a pocket jq and a linear-time regex engine.

`tiny-json` -> `tiny-csv` -> `tiny-calc` -> `tiny-jq` -> `tiny-regex` -> `tiny-md`

### Build Your Own Language

Climb the ladder of language implementation - from an 8-instruction interpreter to a stack VM, a real Lisp, and finally a JIT that emits native x86 you jump into.

`tiny-bf` -> `tiny-stackvm` -> `tiny-forth` -> `tiny-lisp` -> `tiny-jit` -> `tiny-bf-jit` -> `tiny-elf-emit`

### Emulate the Classics

Bring dead hardware back to life and prove it with real test ROMs - the CHIP-8, the 6502 behind the NES, and the chips powering the GameBoy and ZX Spectrum.

`tiny-chip8` -> `tiny-6502` -> `tiny-8080` -> `tiny-z80` -> `tiny-gameboy-cpu`

### Compression, Step by Step

Build up to real gzip one codec at a time: run-length, Huffman, sliding windows, then inflate/deflate that interoperate with gunzip, and finally pack and crack tar and zip archives.

`tiny-rle` -> `tiny-huffman` -> `tiny-lzss` -> `tiny-inflate` -> `tiny-deflate` -> `tiny-tar` -> `tiny-unzip`

### Pixels From Scratch

Write your own pixels with zero image crates - from the simplest format to fractals, a software raytracer and rasterizer, real PNG decoding, and painting straight to the framebuffer.

`tiny-ppm` -> `tiny-mandelbrot` -> `tiny-raytrace` -> `tiny-png` -> `tiny-raster` -> `tiny-font` -> `tiny-fb`

### AI From Scratch

Assemble a transformer from first principles: an autodiff engine, an MLP that learns XOR, real MNIST inference, the attention kernel, a true GPT tokenizer, and a modern LLaMA-style loop - tying directly into the repo's existing GPT-2 build.

`tiny-autograd` -> `tiny-mlp` -> `tiny-mnist` -> `tiny-attention` -> `tiny-bpe` -> `tiny-llama`

### Build a Tiny OS

The most aspirational track: boot from GRUB, draw to VGA, set up the GDT/IDT, take timer and keyboard interrupts, get a heap, and end with a heap-free ring-0 shell - bare metal, no std, no kernel underneath you.

`tiny-multiboot` -> `tiny-vga13` -> `tiny-gdt` -> `tiny-idt` -> `tiny-pit` -> `tiny-keyboard` -> `tiny-kheap` -> `tiny-kshell`

