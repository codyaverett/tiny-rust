# 39-tiny-kafka-pubsub

A minimal Kafka-style publish/subscribe message broker in `no_std` Rust. Single binary, two modes: publisher (broker) and subscriber (client).

## Build

```sh
cargo build --release
```

## Usage

Start the publisher (broker):

```sh
./target/release/tiny-kafka-pubsub
# or explicitly:
./target/release/tiny-kafka-pubsub pub
```

Start a subscriber in another terminal:

```sh
./target/release/tiny-kafka-pubsub sub
```

Publish messages via any TCP client:

```sh
# Connect and publish
echo -e "PUBLISH logs hello world" | nc localhost 9093

# Subscribe with topic filter
echo -e "SUBSCRIBE logs" | nc -q -1 localhost 9093

# Publish to a filtered topic
echo -e "PUBLISH logs.error disk full" | nc localhost 9093
echo -e "PUBLISH metrics.cpu 42" | nc localhost 9093
```

## Protocol

| Command | Description |
|---------|-------------|
| `SUBSCRIBE [filter]\n` | Subscribe to messages (optional topic prefix filter) |
| `PUBLISH topic message\n` | Publish message to a topic |

### Responses

| Response | Meaning |
|----------|---------|
| `OK connected\n` | Connection accepted |
| `OK subscribed\n` | Subscription registered |
| `OK published N\n` | Message delivered to N subscribers |
| `MSG topic 0 message\n` | Delivered message (sent to subscribers) |

## Architecture

- **Brokerless fan-out** -- single publisher process acts as the broker
- **poll()-based I/O** -- non-blocking multiplexed socket handling
- **Prefix-match filtering** -- subscribers can filter by topic prefix (e.g., `logs` matches `logs.error`, `logs.info`)
- **No persistence** -- messages are delivered in real-time only, no consumer groups or offsets
- **Stack-allocated** -- all state lives on the stack, max 16 concurrent subscribers
- **Port 9093** -- listens on localhost

## Limitations

- Max 16 concurrent subscribers
- Topic filters max 32 bytes (prefix match only)
- No message persistence or replay
- No authentication or encryption
- Single-threaded, no consumer groups
