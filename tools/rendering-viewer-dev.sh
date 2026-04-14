#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "Building hot dylib..."
cargo build -p rendering-viewer-hot

echo "Starting hot-reload rendering viewer"
echo "Edit lib/game/src/damage_number.rs and changes auto-reload."

cargo watch \
    -w lib/game/src \
    -w lib/renderer/src \
    -w tools/rendering-viewer-hot/src \
    -s "cargo build -p rendering-viewer-hot" &
WATCH_PID=$!

trap "kill $WATCH_PID 2>/dev/null; exit" INT TERM

cargo run --bin rendering-viewer -- --grf "${1:-data/data.grf}"

kill $WATCH_PID 2>/dev/null
