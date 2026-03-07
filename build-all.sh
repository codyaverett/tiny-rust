#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo "Building all tiny-rust variants in release mode..."
echo

# 01 is excluded from workspace to have its own profile (no panic=abort)
echo "--- Building 01-release-opts (standalone) ---"
cargo build --release --manifest-path 01-release-opts/Cargo.toml 2>&1

echo "--- Building workspace members (02, 03, 04, 09-12) ---"
cargo build --release 2>&1

echo "--- Building 05-tiny-wasm (wasm32, standalone) ---"
cargo build --release --manifest-path 05-tiny-wasm/Cargo.toml --target wasm32-unknown-unknown 2>&1
# Copy wasm next to index.html for easy serving
cp 05-tiny-wasm/target/wasm32-unknown-unknown/release/tiny_wasm.wasm 05-tiny-wasm/tiny_wasm.wasm

echo
echo "=== Binary Sizes ==="
printf "%-20s %10s %12s\n" "Variant" "Size" "Bytes"
printf "%-20s %10s %12s\n" "-------" "----" "-----"

bins=(
    "01-release-opts/target/release/release-opts"
    "target/release/panic-abort"
    "target/release/no-std"
    "target/release/raw-syscall"
    "05-tiny-wasm/tiny_wasm.wasm"
    "target/release/tiny-yes"
    "target/release/tiny-base64"
    "target/release/tiny-hash"
    "target/release/tiny-random"
)
names=(
    "01-release-opts"
    "02-panic-abort"
    "03-no-std"
    "04-raw-syscall"
    "05-tiny-wasm"
    "09-tiny-yes"
    "10-tiny-base64"
    "11-tiny-hash"
    "12-tiny-random"
)

for i in "${!bins[@]}"; do
    path="${bins[$i]}"
    name="${names[$i]}"
    if [ -f "$path" ]; then
        size=$(ls -lh "$path" | awk '{print $5}')
        bytes=$(stat --format=%s "$path")
        printf "%-20s %10s %12s\n" "$name" "$size" "$bytes"
    else
        printf "%-20s %10s\n" "$name" "NOT FOUND"
    fi
done

echo
echo "=== Section Sizes ==="
for i in "${!bins[@]}"; do
    path="${bins[$i]}"
    name="${names[$i]}"
    if [ -f "$path" ]; then
        echo "--- $name ---"
        if [[ "$path" == *.wasm ]]; then
            echo "  (wasm binary - use wasm-objdump for details)"
        else
            size "$path" 2>/dev/null || echo "  (size command failed)"
        fi
    fi
done
