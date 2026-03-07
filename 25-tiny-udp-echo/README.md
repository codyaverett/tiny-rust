# 25-tiny-udp-echo

UDP echo server with epoll-based I/O multiplexing.

## Technique

Uses the `no_std` + libc pattern from example 03. Creates a UDP socket
(`SOCK_DGRAM`) and multiplexes it with stdin using `epoll`. Echoes received
datagrams back to the sender with `recvfrom`/`sendto`. Stdin accepts
`stats` (show counters) and `quit` (shutdown) commands.

Epoll is the same mechanism that powers nginx, tokio, and other high-performance
event loops. This example demonstrates monitoring heterogeneous file descriptors
(network socket + terminal) in a single event loop.

## New syscalls

- `sendto` -- send UDP datagram to specific address
- `recvfrom` -- receive UDP datagram with sender address
- `epoll_create1` -- create epoll instance
- `epoll_ctl` -- add/modify/remove fd from epoll
- `epoll_wait` -- wait for events on monitored fds

## Usage

```sh
cargo build --release
./target/release/tiny-udp-echo
# UDP echo server listening on port 9998
# Commands on stdin: stats, quit

# In another terminal:
echo "hello" | nc -u -w1 localhost 9998
# hello

# Back in server terminal, type:
# stats
# Echoed 1 datagrams, 6 bytes total
# quit
# Shutting down.
```
