# 20-tiny-server

Minimal HTTP server using raw socket syscalls, no standard library.

## Technique

Uses the `no_std` + libc pattern from example 03. Creates a TCP socket with
`socket`/`bind`/`listen`/`accept`, serves a static HTML response with proper
HTTP/1.1 headers, and logs each request to stdout. Demonstrates networking
without std in ~14 KB.

## Usage

```sh
cargo build --release
./target/release/tiny-server
# Listening on port 9999

# In another terminal:
curl http://localhost:9999
```
