# 18-tiny-signal

Signal handler demo using `sigaction` for SIGINT and SIGUSR1.

## Technique

Uses the `no_std` + libc pattern from example 03. Installs signal handlers via
`libc::sigaction`, uses `AtomicBool` for async-signal-safe flag communication,
and sleeps with `pause()` between signals. Demonstrates graceful shutdown on
Ctrl+C.

## Usage

```sh
cargo build --release
./target/release/tiny-signal

# In another terminal:
kill -USR1 <pid>   # prints a counter
kill -INT <pid>    # or Ctrl+C to exit gracefully
```
