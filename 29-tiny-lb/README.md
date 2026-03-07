# 29-tiny-lb

Round-robin HTTP load balancer with backend failover.

## Technique

Uses the `no_std` + libc pattern from example 03. Accepts HTTP requests and
distributes them across multiple backends in round-robin order. If a backend
is unreachable, it tries the next one. Returns 502 if all backends are down.

## New concepts

- Round-robin scheduling across multiple backends
- Backend failover on connection failure
- Load balancer pattern (layer 7, HTTP-aware)

## Usage

```sh
cargo build --release

# Start some backends (e.g., multiple tiny-server instances)
./target/release/tiny-server &  # default port 8081
# (start more on 8082, 8083)

# Start the load balancer
./target/release/tiny-lb
# Round-robin LB on port 9001 -> backends :8081 :8082 :8083

# Send requests
curl http://localhost:9001/
# [#1] -> :8081  GET / HTTP/1.1
curl http://localhost:9001/
# [#2] -> :8082  GET / HTTP/1.1
curl http://localhost:9001/
# [#3] -> :8083  GET / HTTP/1.1
```

## Limitations

- Hardcoded backends (127.0.0.1:8081-8083)
- Single-threaded, one request at a time
- No health checks between requests
- No connection pooling or keep-alive
