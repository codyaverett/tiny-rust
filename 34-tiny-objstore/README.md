# 34-tiny-objstore

A minimal content-addressable object store in `no_std` Rust with an HTTP API. Objects are identified by their FNV-1a hash, providing automatic deduplication.

## What it does

- Listens on port 7880
- Stores blobs via PUT, returns a hex hash ID
- Retrieves objects by hash ID via GET
- Automatic deduplication: same content = same ID

## New concepts

- **Content-addressable storage (CAS)** -- hash-based object IDs
- **Hash-based deduplication** -- identical content stored once
- **Hex ID parsing** -- bidirectional hex encoding/decoding
- **Blob store pattern** -- opaque binary object storage

## HTTP API

| Method | Path | Action |
|--------|------|--------|
| PUT | `/obj` | Store object, return hex hash ID |
| GET | `/obj/HEXID` | Retrieve object by hash |
| DELETE | `/obj/HEXID` | Remove object |
| GET | `/stats` | Show count/bytes/capacity |

## Usage

```sh
cargo build --release
./target/release/tiny-objstore &

curl -X PUT -d 'test data' localhost:7880/obj
# Use returned hex ID:
curl localhost:7880/obj/HEXID
curl localhost:7880/stats
```

## Response examples

```
PUT /obj (body: hello)     -> "stored\nid=a1b2c3d4e5f67890\nsize=5\n"
PUT /obj (same body)       -> "exists\nid=a1b2c3d4e5f67890\nsize=5\n"
GET /obj/a1b2c3d4e5f67890  -> raw bytes
GET /stats                 -> "tiny-objstore\nobjects: 3\nbytes: 1547\ncapacity: 64\n"
```

## Limitations

- Max 64 objects, 4096 bytes per object
- FNV-1a hash (not cryptographic, collision possible)
- Single-threaded, one connection at a time
- In-memory only, state lost on restart
