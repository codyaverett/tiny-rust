# 22-tiny-pipe

Fork a child process and communicate via anonymous pipe.

## Technique

Uses the `no_std` + libc pattern from example 03. Creates an anonymous pipe
with `pipe()`, forks a child with `fork()`, child writes messages through the
pipe, parent reads and displays them, then waits for child exit with `waitpid()`.
Demonstrates WIFEXITED/WEXITSTATUS bit manipulation for exit status decoding.

## New syscalls

- `fork` -- create child process
- `pipe` -- create anonymous pipe (read/write fd pair)
- `waitpid` -- wait for child process and get exit status

## Usage

```sh
cargo build --release
./target/release/tiny-pipe
# [parent pid=12345] forked child pid=12346
# [parent] reading from pipe:
# [child  pid=12346] sending 3 messages through pipe
#   > Hello from the child process!
# Pipes are a Unix IPC mechanism.
# This is the last message. Goodbye!
# [parent] child exited with status 0
```
