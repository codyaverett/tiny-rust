# 30-tiny-lb-sticky

IP-hash sticky session load balancer with failover.

## Technique

Uses the `no_std` + libc pattern from example 03. Hashes the client IP address
with FNV-1a (same algorithm as example 11) to deterministically select a backend.
The same client always reaches the same backend -- essential for session-based
apps. If the primary backend is down, falls over to the next one.

## New concepts

- IP-hash based backend selection (sticky sessions)
- FNV-1a hash reuse for deterministic routing
- Failover with affinity preservation
- Contrast with stateless round-robin (example 29)

## Usage

```sh
cargo build --release

# Start some backends
# (on ports 8081, 8082, 8083)

# Start the sticky load balancer
./target/release/tiny-lb-sticky
# IP-hash sticky LB on port 9002 -> backends :8081 :8082 :8083

# Same client always hits same backend
curl http://localhost:9002/
# [#1] 127.0.0.1 -> :8082  GET / HTTP/1.1
curl http://localhost:9002/
# [#2] 127.0.0.1 -> :8082  GET / HTTP/1.1  (same backend!)
```

## Limitations

- Hardcoded backends (127.0.0.1:8081-8083)
- Single-threaded, one request at a time
- Sticky only by IP (not by cookie or header)
- No health checks between requests
