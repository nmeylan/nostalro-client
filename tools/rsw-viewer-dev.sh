#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "Building hot dylib..."
cargo build -p rsw-viewer-hot

echo "Starting hot-reload RSW viewer"
echo "Edit tools/rsw-viewer-hot/src/lib.rs (camera, overlays, info panel) and changes auto-reload."

cargo watch \
    -w lib/renderer/src \
    -w lib/ui-core/src \
    -w tools/rsw-viewer-hot/src \
    -s "cargo build -p rsw-viewer-hot" &
WATCH_PID=$!

trap "kill $WATCH_PID 2>/dev/null; exit" INT TERM

cargo run --bin rsw-viewer -- --grf "${1:-data/data.grf}" ${2:+--map "$2"}

kill $WATCH_PID 2>/dev/null
