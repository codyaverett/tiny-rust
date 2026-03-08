# 35-tiny-transformer

A minimal GPT-style transformer in ~650 lines of `no_std` Rust. Every core
building block of modern large language models is implemented from scratch --
embeddings, self-attention, feed-forward networks, layer normalization,
positional encoding, and autoregressive generation.

The weights are randomly initialized (untrained), so the output is gibberish.
The point is to show **how transformers work at the math level**, not to produce
coherent text.

## Architecture

```
Input characters (ASCII 32-127)
        |
  Token Embedding  (96 x 16 lookup table)
        +
  Sinusoidal Positional Encoding
        |
  +--[Transformer Block]--+
  |  Self-Attention (Q*K^T / sqrt(d_k))
  |  + Residual Connection
  |  + Layer Normalization
  |  Feed-Forward (Linear -> ReLU -> Linear)
  |  + Residual Connection
  |  + Layer Normalization
  +------------------------+
        |
  Unembed (16 -> 96 logits)
        |
  Argmax -> next character
```

Parameters: 1 layer, 16-dim embeddings, 32-dim FFN, 96 vocab, 32-token context window.

## New Concepts

- **Self-attention**: Q*K^T / sqrt(d_k) with causal mask -- each position can only attend to itself and earlier positions
- **Softmax with numerical stability**: subtract max before exp to prevent overflow
- **Layer normalization**: normalize activations to zero mean / unit variance, then scale and shift
- **Residual connections**: add input back to output of each sub-layer, enabling gradient flow
- **Sinusoidal positional encoding**: sin/cos functions at different frequencies encode token position
- **Feed-forward network with ReLU**: two linear layers with ReLU activation between them
- **Autoregressive generation**: predict one token at a time, feed it back as input
- **Xavier weight initialization**: scale random weights by 1/sqrt(fan_in) for stable training

## Usage

```sh
cargo build --release -p tiny-transformer
```

```sh
# Default: "hello " prompt, 64 characters
./target/release/tiny-transformer

# Custom prompt
./target/release/tiny-transformer "once upon a "

# Custom prompt and length
./target/release/tiny-transformer "the " 128
```

## Limitations

- **Untrained weights**: output is random gibberish (by design)
- **Single attention head**: real transformers use multi-head attention
- **1 layer**: GPT-2 has 12-48 layers
- **16-dim embeddings**: GPT-2 uses 768-1600
- **32-token context**: GPT-2 uses 1024
- **f32 approximations**: exp, sin, cos, sqrt are Taylor/Newton approximations
- **Argmax decoding**: no temperature sampling or top-k/top-p

## Size

~14 KB release binary -- all transformer math fits in a tiny `no_std` ELF.
