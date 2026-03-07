#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BINARY="$ROOT_DIR/target/release/raw-syscall"
OUTPUT="$SCRIPT_DIR/raw-syscall-upx"

echo "=== Building 04-raw-syscall ==="
cd "$ROOT_DIR"
cargo build --release -p raw-syscall

ORIGINAL_SIZE=$(stat --format=%s "$BINARY" 2>/dev/null || stat -f%z "$BINARY")
echo "Original size: $ORIGINAL_SIZE bytes"

echo ""
echo "=== Compressing with UPX ==="
cp "$BINARY" "$OUTPUT"
upx --best --ultra-brute "$OUTPUT"

COMPRESSED_SIZE=$(stat --format=%s "$OUTPUT" 2>/dev/null || stat -f%z "$OUTPUT")
echo ""
echo "Compressed size: $COMPRESSED_SIZE bytes"
echo "Output: $OUTPUT"
echo ""
echo "=== Test run ==="
"$OUTPUT"
