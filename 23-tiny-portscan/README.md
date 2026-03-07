# 23-tiny-portscan

TCP connect scanner with non-blocking sockets.

## Technique

Uses the `no_std` + libc pattern from example 03. For each port in the range,
creates a non-blocking TCP socket with `fcntl(O_NONBLOCK)`, initiates `connect()`,
waits with `poll()` for completion (200ms timeout), then checks `getsockopt(SO_ERROR)`
to determine if the port is open. Parses IP addresses manually.

Pairs with example 20 (tiny-server): start tiny-server, then scan to find port 9999 open.

## New syscalls

- `connect` -- initiate TCP connection
- `fcntl` -- set O_NONBLOCK flag
- `poll` -- wait for socket events with timeout
- `getsockopt` -- check SO_ERROR after non-blocking connect

## Usage

```sh
cargo build --release
./target/release/tiny-portscan 127.0.0.1 9990 10000
# Scanning 127.0.0.1 ports 9990-10000 (timeout 200ms)
#   OPEN  9999
# Scan complete: 1 open port(s)
```
