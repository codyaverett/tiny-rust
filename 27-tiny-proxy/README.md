# 27-tiny-proxy

TCP port forwarder -- the simplest possible proxy.

## Technique

Uses the `no_std` + libc pattern from example 03. Listens on a local port and
forwards all TCP traffic bidirectionally to a target address using `poll()` for
I/O multiplexing. This is a layer 4 proxy -- it forwards raw bytes without
understanding the application protocol.

## New concepts

- `poll()` for I/O multiplexing (simpler alternative to `epoll` from example 25)
- Bidirectional socket forwarding (client <-> target)
- TCP proxy / port forwarding pattern

## Usage

```sh
cargo build --release

# Start a backend (e.g., tiny-server from example 20)
./target/release/tiny-server &

# Start the proxy (port 9000 -> 127.0.0.1:8080)
./target/release/tiny-proxy
# TCP proxy listening on port 9000 -> 127.0.0.1:8080

# In another terminal:
curl http://localhost:9000/
# (response from tiny-server via the proxy)
```

## Limitations

- Hardcoded listen port (9000) and target (127.0.0.1:8080)
- Single-threaded, handles one connection at a time
- No TLS support
- 30-second idle timeout per connection
