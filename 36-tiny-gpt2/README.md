# 36-tiny-gpt2

Full GPT-2 Small (124M parameter) inference engine in no_std Rust.

Unlike project 35 (random weights, 1 layer, 1 head), this loads real pre-trained
weights from HuggingFace and produces coherent English text.

## Architecture

- **Model**: GPT-2 Small — 12 layers, 12 heads, 768 embedding dim, 50257 vocab
- **Weights**: ~497 MB external file (f32 little-endian), loaded zero-copy via mmap
- **Tokenizer**: BPE with ~50K vocab, byte-level encoding
- **Inference**: Single-token autoregressive with KV cache
- **Activation**: GELU approximation via tanh
- **Sampling**: Temperature scaling + top-k (k=40) + softmax sampling
- **Binary**: ~14 KB (no_std + libc pattern)

## Prerequisites

```sh
pip install transformers torch numpy
```

## Usage

```sh
# 1. Export weights and tokenizer from HuggingFace
python3 convert.py

# 2. Build
cargo build --release

# 3. Run
./target/release/tiny-gpt2 gpt2.bin gpt2_tokenizer.bin "Once upon a time" 128 0.8
```

### Arguments

| # | Argument | Default | Description |
|---|----------|---------|-------------|
| 1 | weights path | (required) | Path to `gpt2.bin` |
| 2 | tokenizer path | (required) | Path to `gpt2_tokenizer.bin` |
| 3 | prompt | `<\|endoftext\|>` | Text prompt to start generation |
| 4 | n_tokens | 128 | Number of tokens to generate |
| 5 | temperature | 0.8 | Sampling temperature (0 = greedy argmax) |

## Limitations

- Math uses f32 approximations (exp via repeated squaring, tanh, inv_sqrt via Newton-Raphson)
- BPE encoding uses linear search (slow for first encode, but prompt is typically short)
- Maximum context length: 1024 tokens (GPT-2 limit)
- No batching — single sequence inference only
- Inference is CPU-only, single-threaded

## File Formats

### gpt2.bin (~497 MB)
- 16-byte header: magic `0x47505432`, n_layers, n_heads, embed_dim (u32 each)
- Token embeddings `[50257 x 768]`
- Position embeddings `[1024 x 768]`
- 12x layer weights (ln1, c_attn QKV, c_proj, ln2, fc, proj — all transposed to row-major)
- Final layer norm weight + bias

### gpt2_tokenizer.bin (~2-3 MB)
- 12-byte header: magic `0x544F4B4E`, vocab_size, n_merges (u32 each)
- Vocabulary: per token `u16 length + bytes`
- Merges: sorted by priority as `(id_a: u32, id_b: u32, result: u32)`

## Size

~14 KB binary. All model weights are external files loaded at runtime via mmap.
