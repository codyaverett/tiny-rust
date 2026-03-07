#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo "Building all tiny-rust variants in release mode..."
echo

# 01 is excluded from workspace to have its own profile (no panic=abort)
echo "--- Building 01-release-opts (standalone) ---"
cargo build --release --manifest-path 01-release-opts/Cargo.toml 2>&1

echo "--- Building workspace members (02, 03, 04) ---"
cargo build --release 2>&1

echo
echo "=== Binary Sizes ==="
printf "%-20s %10s %12s\n" "Variant" "Size" "Bytes"
printf "%-20s %10s %12s\n" "-------" "----" "-----"

bins=(
    "01-release-opts/target/release/release-opts"
    "target/release/panic-abort"
    "target/release/no-std"
    "target/release/raw-syscall"
)
names=(
    "01-release-opts"
    "02-panic-abort"
    "03-no-std"
    "04-raw-syscall"
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
        size "$path" 2>/dev/null || echo "  (size command failed)"
    fi
done
