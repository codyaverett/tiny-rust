# 38-tiny-kafka-broker

A minimal Kafka-like message broker in `no_std` Rust with a text-based TCP protocol. Supports topics with partitions, consumer groups, and ring-buffer message storage.

## What it does

- Listens on port 9092
- Text-based protocol (one command per connection, via netcat)
- Topics with 1-4 partitions, round-robin message distribution
- Consumer groups with offset tracking
- Ring-buffer storage (64 messages per partition)
- All storage via anonymous mmap

## New concepts

- **Ring-buffer partitions** -- fixed-size circular buffer per partition, old messages evicted when full
- **Round-robin producer** -- messages distributed across partitions in order
- **Consumer group offsets** -- per-group, per-partition offset tracking with manual commit
- **Poll-based consumption** -- consumer groups poll for next unread message across partitions

## Protocol reference

| Command | Description |
|---------|-------------|
| `CREATE_TOPIC name [partitions]` | Create topic with 1-4 partitions (default 1) |
| `PRODUCE topic msg` | Append message (round-robin partition) |
| `CONSUME topic partition offset [count]` | Read messages from partition at offset |
| `LIST_TOPICS` | List all active topics |
| `STATS` | Show broker statistics |
| `SUBSCRIBE group topic` | Subscribe consumer group to topic |
| `POLL group topic` | Poll next message for consumer group |
| `COMMIT group topic partition offset` | Commit consumer offset |

## Response format

- Success: `OK ...data...\n`
- Error: `ERR reason\n`
- CONSUME: `MSG offset data\n` per message, then `END\n`
- LIST_TOPICS: `TOPIC name partitions\n` per topic, then `END\n`

## Usage

```sh
cargo build --release
./target/release/tiny-kafka-broker &

# Create a topic with 2 partitions
echo "CREATE_TOPIC events 2" | nc localhost 9092

# Produce messages
echo "PRODUCE events hello-world" | nc localhost 9092
echo "PRODUCE events second-message" | nc localhost 9092

# Consume from partition 0, offset 0, count 10
echo "CONSUME events 0 0 10" | nc localhost 9092

# List topics
echo "LIST_TOPICS" | nc localhost 9092

# Broker stats
echo "STATS" | nc localhost 9092

# Consumer groups
echo "SUBSCRIBE mygroup events" | nc localhost 9092
echo "POLL mygroup events" | nc localhost 9092
echo "COMMIT mygroup events 0 1" | nc localhost 9092
```

## Limitations

- Max 8 topics, 4 partitions per topic, 64 messages per partition
- Messages max 256 bytes, names max 32 bytes
- Single-threaded, one connection at a time
- In-memory only, state lost on restart
- Ring buffer evicts oldest messages when full
