# Project Classes

How the examples are categorized. With 40 built and 200+ planned, two
independent tags keep things navigable without nesting the directory tree.

Directories stay **flat and numbered** -- that is the repo's identity and keeps
`ls` order equal to build order. Categories live in metadata
(`[package.metadata.tiny]` in each `Cargo.toml`), not in folders.

## Two axes

Every example has exactly one **class** (what kind of thing it is -- for
browsing) and one **build** contract (how it compiles and runs -- for the
workspace and CI).

### Axis 1 -- class (navigation)

| Class | What defines it | Run contract |
|-------|-----------------|--------------|
| `size-lab` | Minimization *techniques*, not apps -- the original "how small can it get" story | `ls -la`, compare bytes |
| `userland-tools` | stdin/argv filters and tools that do one thing and exit | shell pipe |
| `network-services` | Sockets, long-running daemons, message brokers | `nc` / `curl`, run as daemon |
| `data-storage` | Stateful engines (KV, object, SQL, chains) with a CLI or network face | a client talks to it |
| `system-process` | `/proc`, signals, ptrace, memory, process control | run against the live system |
| `compute-ai` | Numerically heavy: tensors, models, inference | feed input, read output |
| `graphics-media` | Pixels and audio: image/audio codecs, renderers, GUIs | view PPM / play WAV / X11 |
| `emulators-languages` | Interpreters and VMs: CPU emulators, language runtimes | load a ROM or program |
| `bare-metal` | Freestanding, no libc, boots under an emulator | QEMU |

Class is just a browsing label -- cheap to reassign, no mechanical effect.

### Axis 2 -- build (toolchain contract)

This is the axis that actually structures the workspace. Most examples share one
contract; the rest are deliberate carve-outs.

| Build | Toolchain / target | In default `cargo build`? |
|-------|--------------------|---------------------------|
| `hosted` | stable, `no_std` + libc, host Linux x86_64 | yes -- `default-members` |
| `bare-metal` | custom target JSON, no libc, run in QEMU | no -- see [BARE-METAL.md](BARE-METAL.md) |
| `wasm` | `--target wasm32-unknown-unknown` | no |
| `nightly` | nightly, `-Z build-std` | no |
| `size-lab` | standalone release profile or post-build tooling | no -- built independently |

The heavy machinery (workspace membership, CI, prerequisites) keys off `build`,
not `class`. ~95% of examples are `hosted`, so plain `cargo build --release`
stays fast and never tries to QEMU-build a kernel on your host. The carve-outs
match the pattern already used for examples 05 (wasm) and 06/07 (nightly).

## Metadata block

Each example's `Cargo.toml` carries:

```toml
[package.metadata.tiny]
class  = "userland-tools"     # axis 1, navigation
build  = "hosted"             # axis 2, toolchain contract
domain = "coreutils"          # fine-grained tag (matches the catalog categories)
series = "the-unix-toolbox"   # optional learning path, omitted when none
```

A future `gen-readme` script reads these blocks to regenerate the README catalog
tables and keep status, sizes, and grouping honest.

## How the README presents it

Three views of the same set:

1. **The Size Journey (01-08)** -- the original incremental shrink story, kept
   linear and up front.
2. **Catalog table** -- grouped by `class`, with status emoji (idea / planned /
   built).
3. **Learning series** -- themed `domain` tracks for newcomers
   (see [ideas/roadmap.md](ideas/roadmap.md)).

## Related docs

- [ideas/catalog.md](ideas/catalog.md) -- full backlog of candidate examples
- [ideas/roadmap.md](ideas/roadmap.md) -- focus order and learning series
- [ideas/organization.md](ideas/organization.md) -- numbering, status tracking, scaling to 100+
