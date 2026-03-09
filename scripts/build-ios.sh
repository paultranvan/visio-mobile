#!/bin/bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

TARGET="${1:-device}"

if [ "$TARGET" = "sim" ]; then
    RUST_TARGET="aarch64-apple-ios-sim"
    echo "==> Building for iOS Simulator..."
else
    RUST_TARGET="aarch64-apple-ios"
    echo "==> Building for iOS Device..."
fi

# Regenerate Swift UniFFI bindings (patches modulemap for visio_native.h)
"$REPO_ROOT/scripts/generate-bindings.sh" swift

# Only build visio-ffi — it depends on visio-video, so libvisio_ffi.a
# contains all symbols from both crates. Building visio-video separately
# would produce duplicate WebRTC symbols at link time.
cargo build --target "$RUST_TARGET" -p visio-ffi --release

# Remove dynamic libraries — iOS requires static linking and the linker
# will prefer .dylib over .a when both exist, causing DYLD load errors.
rm -f "target/$RUST_TARGET/release/"*.dylib

echo "==> Library at:"
ls -la "target/$RUST_TARGET/release/libvisio_ffi.a"
