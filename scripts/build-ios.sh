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

cargo build --target "$RUST_TARGET" -p visio-ffi -p visio-video --release

# Remove dynamic libraries — iOS requires static linking and the linker
# will prefer .dylib over .a when both exist, causing DYLD load errors.
rm -f "target/$RUST_TARGET/release/"*.dylib

# Merge both static libs into one to avoid duplicate WebRTC symbols
libtool -static -o "target/$RUST_TARGET/release/libvisio.a" \
  "target/$RUST_TARGET/release/libvisio_ffi.a" \
  "target/$RUST_TARGET/release/libvisio_video.a"

echo "==> Merged library at:"
ls -la "target/$RUST_TARGET/release/libvisio.a"

echo ""
echo "To integrate with Xcode:"
echo "  1. Add libvisio_ffi.a and libvisio_video.a to Link Binary With Libraries"
echo "  2. Set Library Search Path to: \$(PROJECT_DIR)/../../target/$RUST_TARGET/release"
echo "  3. Add Other Linker Flags: -lvisio_ffi -lvisio_video"
echo "  4. Add bridging header pointing to ios/VisioMobile/Generated/visioFFI.h"
