# 33-tiny-kv

A minimal in-memory key-value store in `no_std` Rust with an HTTP API. Uses a hash table with open addressing and linear probing.

## What it does

- Listens on port 7879
- Stores key-value pairs via PUT, retrieves via GET, removes via DELETE
- Uses FNV-1a hashing with linear probing for collision resolution
- Tombstone-based deletion preserves probe chains

## New concepts

- **Hash table with open addressing** -- linear probing for collision resolution
- **Tombstone-based deletion** -- preserves lookup chains
- **Load factor management** -- rejects inserts above 75% capacity
- **REST CRUD on parameterized paths** -- GET/PUT/DELETE `/key/NAME`

## HTTP API

| Method | Path | Action |
|--------|------|--------|
| GET | `/key/NAME` | Retrieve value |
| PUT | `/key/NAME` | Store value (body = value) |
| DELETE | `/key/NAME` | Remove key |
| GET | `/stats` | Show count/capacity |

## Usage

```sh
cargo build --release
./target/release/tiny-kv &

curl -X PUT -d 'world' localhost:7879/key/hello
curl localhost:7879/key/hello
curl -X DELETE localhost:7879/key/hello
curl localhost:7879/stats
```

## Limitations

- Max 256 entries, 64-byte keys, 256-byte values
- Single-threaded, one connection at a time
- In-memory only, state lost on restart
