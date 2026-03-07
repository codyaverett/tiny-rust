# 16-tiny-multicall

BusyBox-style multi-call binary that dispatches based on `argv[0]`.

## Technique

Uses the `no_std` + libc pattern from example 03. Reads `argv[0]` from
`/proc/self/cmdline` to determine which applet to run. A single ~14 KB binary
provides multiple utilities via symlinks.

Available applets: `yes`, `true`, `false`, `echo`.

## Usage

```sh
cargo build --release

# Run directly to see available applets
./target/release/tiny-multicall

# Create symlinks and use
ln -s $(pwd)/target/release/tiny-multicall /tmp/yes
ln -s $(pwd)/target/release/tiny-multicall /tmp/echo
/tmp/yes | head -3
/tmp/echo hello world
```
