#!/usr/bin/env python3
"""
Convert GPT-2 small (124M) weights and tokenizer to binary format for tiny-gpt2.

Usage:
    python3 convert.py [output_dir]

Produces:
    gpt2.bin           (~497 MB) - model weights in f32 little-endian
    gpt2_tokenizer.bin (~2-3 MB) - BPE vocabulary and merge table

Requires: pip install transformers torch
"""

import struct
import sys
import os
import numpy as np

def main():
    from transformers import GPT2LMHeadModel, GPT2Tokenizer

    out_dir = sys.argv[1] if len(sys.argv) > 1 else "."

    print("Loading GPT-2 small from HuggingFace...")
    model = GPT2LMHeadModel.from_pretrained("gpt2")
    tokenizer = GPT2Tokenizer.from_pretrained("gpt2")
    sd = model.state_dict()

    # Architecture constants
    n_layers = 12
    n_heads = 12
    embed_dim = 768

    # -------------------------------------------------------------------------
    # Export weights to gpt2.bin
    # -------------------------------------------------------------------------
    weights_path = os.path.join(out_dir, "gpt2.bin")
    print(f"Writing weights to {weights_path}...")

    with open(weights_path, "wb") as f:
        # Header: magic, n_layers, n_heads, embed_dim (4 x u32 = 16 bytes)
        f.write(struct.pack("<I", 0x47505432))  # "GPT2"
        f.write(struct.pack("<I", n_layers))
        f.write(struct.pack("<I", n_heads))
        f.write(struct.pack("<I", embed_dim))

        def write_tensor(name, expected_shape=None, transpose=False):
            t = sd[name].float().numpy()
            if expected_shape is not None:
                assert t.shape == expected_shape, f"{name}: expected {expected_shape}, got {t.shape}"
            if transpose:
                t = t.T
            t = np.ascontiguousarray(t)
            f.write(t.tobytes())
            print(f"  {name}: {t.shape} {'(transposed)' if transpose else ''}")

        # Token embeddings: [50257, 768]
        write_tensor("transformer.wte.weight", (50257, 768))

        # Position embeddings: [1024, 768]
        write_tensor("transformer.wpe.weight", (1024, 768))

        # 12 transformer layers
        for layer in range(n_layers):
            prefix = f"transformer.h.{layer}"

            # Layer norm 1: weight [768], bias [768]
            write_tensor(f"{prefix}.ln_1.weight", (768,))
            write_tensor(f"{prefix}.ln_1.bias", (768,))

            # c_attn (combined QKV): weight [768, 2304] -> transpose to [2304, 768]
            # bias [2304]
            write_tensor(f"{prefix}.attn.c_attn.weight", (768, 2304), transpose=True)
            write_tensor(f"{prefix}.attn.c_attn.bias", (2304,))

            # c_proj (attention output): weight [768, 768] -> transpose to [768, 768]
            # bias [768]
            write_tensor(f"{prefix}.attn.c_proj.weight", (768, 768), transpose=True)
            write_tensor(f"{prefix}.attn.c_proj.bias", (768,))

            # Layer norm 2: weight [768], bias [768]
            write_tensor(f"{prefix}.ln_2.weight", (768,))
            write_tensor(f"{prefix}.ln_2.bias", (768,))

            # MLP fc (first linear): weight [768, 3072] -> transpose to [3072, 768]
            # bias [3072]
            write_tensor(f"{prefix}.mlp.c_fc.weight", (768, 3072), transpose=True)
            write_tensor(f"{prefix}.mlp.c_fc.bias", (3072,))

            # MLP proj (second linear): weight [3072, 768] -> transpose to [768, 3072]
            # bias [768]
            write_tensor(f"{prefix}.mlp.c_proj.weight", (3072, 768), transpose=True)
            write_tensor(f"{prefix}.mlp.c_proj.bias", (768,))

        # Final layer norm
        write_tensor("transformer.ln_f.weight", (768,))
        write_tensor("transformer.ln_f.bias", (768,))

    file_size = os.path.getsize(weights_path)
    print(f"Weights file: {file_size:,} bytes ({file_size / 1024 / 1024:.1f} MB)")

    # -------------------------------------------------------------------------
    # Export tokenizer to gpt2_tokenizer.bin
    # -------------------------------------------------------------------------
    tok_path = os.path.join(out_dir, "gpt2_tokenizer.bin")
    print(f"\nWriting tokenizer to {tok_path}...")

    # Get vocabulary: id -> bytes
    encoder = tokenizer.encoder  # str -> id
    decoder = {v: k for k, v in encoder.items()}  # id -> str
    vocab_size = len(encoder)

    # Get BPE merges
    bpe_ranks = tokenizer.bpe_ranks  # (str, str) -> rank
    merges = sorted(bpe_ranks.items(), key=lambda x: x[1])

    with open(tok_path, "wb") as f:
        # Header: magic, vocab_size, n_merges (3 x u32 = 12 bytes)
        f.write(struct.pack("<I", 0x544F4B4E))  # "TOKE"
        f.write(struct.pack("<I", vocab_size))
        f.write(struct.pack("<I", len(merges)))

        # Vocabulary entries: for each token id 0..vocab_size-1
        # write u16 length + raw bytes
        for token_id in range(vocab_size):
            token_str = decoder.get(token_id, "")
            # GPT-2 uses a byte-level BPE, decode the token string to bytes
            token_bytes = token_str.encode("utf-8")
            f.write(struct.pack("<H", len(token_bytes)))
            f.write(token_bytes)

        # Merges: each merge is (token_a_str, token_b_str) -> result
        # We store as (id_a: u32, id_b: u32, result_id: u32)
        for (a_str, b_str), _rank in merges:
            merged_str = a_str + b_str
            a_id = encoder.get(a_str, 0)
            b_id = encoder.get(b_str, 0)
            merged_id = encoder.get(merged_str, 0)
            f.write(struct.pack("<III", a_id, b_id, merged_id))

    file_size = os.path.getsize(tok_path)
    print(f"Tokenizer file: {file_size:,} bytes ({file_size / 1024:.1f} KB)")
    print(f"  Vocabulary size: {vocab_size}")
    print(f"  Number of merges: {len(merges)}")
    print("\nDone! Run with:")
    print(f"  ./target/release/tiny-gpt2 {weights_path} {tok_path} \"Once upon a time\" 128 0.8")


if __name__ == "__main__":
    main()
