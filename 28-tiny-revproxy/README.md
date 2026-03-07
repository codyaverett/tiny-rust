# 28-tiny-revproxy

HTTP reverse proxy with request logging and header injection.

## Technique

Uses the `no_std` + libc pattern from example 03. Listens for HTTP requests,
parses headers to extract the request line and Content-Length, connects to a
backend server, and forwards the request with an added `X-Forwarded-For` header.
Relays the backend response to the client. This is a layer 7 proxy -- it
understands HTTP structure.

## New concepts

- HTTP request header parsing (method, path, Content-Length)
- Header injection (X-Forwarded-For)
- Layer 7 reverse proxy pattern (contrast with layer 4 in example 27)
- Proper 502 Bad Gateway error responses

## Usage

```sh
cargo build --release

# Start a backend (e.g., tiny-server from example 20)
./target/release/tiny-server &

# Start the reverse proxy (port 8888 -> 127.0.0.1:8080)
./target/release/tiny-revproxy
# HTTP reverse proxy listening on port 8888 -> 127.0.0.1:8080

# In another terminal:
curl http://localhost:8888/
# (response from tiny-server, proxied with X-Forwarded-For)

# Server log shows:
# [#1] 127.0.0.1 GET / HTTP/1.1
```

## Limitations

- Hardcoded listen port (8888) and backend (127.0.0.1:8080)
- Single-threaded, handles one request at a time
- No TLS, no chunked transfer encoding
- Max 8KB request headers
- Case-insensitive Content-Length match only
