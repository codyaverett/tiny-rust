# 32-tiny-blockchain

A minimal in-memory blockchain in `no_std` Rust with an HTTP API. Each block links to the previous via FNV-1a hash, forming an append-only chain.

## What it does

- Listens on port 7878
- Initializes with a genesis block
- Accepts new blocks via POST with request body as data
- Dumps the full chain or chain info via GET

## New concepts

- **Hash chains** -- each block links to previous via hash
- **Append-only data structure** -- immutable once added
- **Timestamps** -- via `libc::time()`
- **Multi-route HTTP API** -- first REST-style path-based routing in the series

## HTTP API

| Method | Path | Action |
|--------|------|--------|
| GET | `/` | Chain info: length + latest hash |
| GET | `/chain` | Dump all blocks as text lines |
| POST | `/block` | Add block with request body as data |

## Usage

```sh
cargo build --release
./target/release/tiny-blockchain &

curl localhost:7878/
curl -X POST -d 'hello block' localhost:7878/block
curl localhost:7878/chain
```

## Response examples

```
GET /       -> "tiny-blockchain\nblocks: 3\nlatest: a1b2c3d4e5f67890\n"
POST /block -> "added block 3\nhash=fedcba9876543210\n"
GET /chain  -> "[0] a1b2... prev=0000... data=genesis\n[1] ..."
```

## Limitations

- Max 64 blocks, 256 bytes data per block
- FNV-1a hash (not cryptographic)
- Single-threaded, one connection at a time
- In-memory only, state lost on restart
