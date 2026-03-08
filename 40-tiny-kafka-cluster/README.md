# 40-tiny-kafka-cluster

A minimal Kafka-like message broker with consumer groups, key-based partitioning, and poll-based concurrency in `no_std` Rust.

## Build

```sh
cargo build --release
```

## Usage

### Broker (default mode)

```sh
./target/release/tiny-kafka-cluster
# or explicitly:
./target/release/tiny-kafka-cluster broker
```

Listens on port 9094.

### Producer

```sh
./target/release/tiny-kafka-cluster producer 127.0.0.1 9094 my-topic
```

Auto-creates the topic with 2 partitions, then produces messages with cycling keys (key-0 through key-3) at 1 message/second.

### Consumer

```sh
./target/release/tiny-kafka-cluster consumer 127.0.0.1 9094 my-group my-topic
```

Joins the consumer group, gets assigned partitions, fetches and commits offsets in a loop (500ms poll interval).

### Multi-consumer example

```sh
# Terminal 1: broker
./target/release/tiny-kafka-cluster

# Terminal 2: producer
./target/release/tiny-kafka-cluster producer 127.0.0.1 9094 events

# Terminal 3: consumer A
./target/release/tiny-kafka-cluster consumer 127.0.0.1 9094 workers events

# Terminal 4: consumer B (partitions rebalance automatically)
./target/release/tiny-kafka-cluster consumer 127.0.0.1 9094 workers events
```

### Manual protocol via netcat

```sh
echo "CREATE_TOPIC orders 4" | nc 127.0.0.1 9094
echo "PRODUCE orders user-42 placed-order" | nc 127.0.0.1 9094
echo "FETCH orders 0 0 10" | nc 127.0.0.1 9094
echo "LIST_TOPICS" | nc 127.0.0.1 9094
echo "JOIN_GROUP payments orders" | nc 127.0.0.1 9094
echo "OFFSETS payments orders" | nc 127.0.0.1 9094
```

## Protocol reference

| Command | Description | Response |
|---------|-------------|----------|
| `CREATE_TOPIC name [partitions]` | Create topic (1-4 partitions, default 2) | `OK topic created partitions=N` |
| `PRODUCE topic key value` | Produce message with key-based routing | `OK partition=P offset=O` |
| `FETCH topic partition offset [count]` | Fetch messages from partition | `MSG offset key data\n`... `END` |
| `LIST_TOPICS` | List all topics | `name partitions=N\n`... `END` |
| `JOIN_GROUP group topic` | Join consumer group | `OK partitions 0,2` |
| `LEAVE_GROUP group` | Leave consumer group | `OK left group` |
| `COMMIT group topic partition offset` | Commit consumer offset | `OK committed` |
| `OFFSETS group topic` | Get committed offsets | `OFFSET P O\n`... `END` |

## Architecture

- **poll-based concurrency** -- single-threaded event loop with `poll()` for up to 16 concurrent clients
- **FNV-1a key partitioning** -- deterministic partition assignment based on message key hash
- **consumer group coordination** -- round-robin partition rebalancing on member join/leave/disconnect
- **offset tracking** -- per-partition committed offsets stored per consumer group
- **ring buffer storage** -- 64 messages per partition with wrap-around
- **mmap allocation** -- all data structures allocated via anonymous mmap

## Comparison

| Feature | 38-tiny-kafka-broker | 39-tiny-kafka-pubsub | 40-tiny-kafka-cluster |
|---------|---------------------|---------------------|----------------------|
| Concurrency | blocking accept | poll (publisher) | poll (broker) |
| Partitions | no | no | 1-4 per topic |
| Key routing | no | no | FNV-1a hash |
| Consumer groups | no | no | yes, with rebalance |
| Offset commits | no | no | yes |
| Modes | broker only | publisher + subscriber | broker + producer + consumer |
