#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "Building hot dylib..."
cargo build -p effect-viewer-hot

echo "Starting hot-reload effect viewer"

cargo watch \
    -w lib/game/src \
    -w lib/renderer/src \
    -w tools/rendering-viewer-hot/src \
    -s "cargo build -p rendering-viewer-hot" &
WATCH_PID=$!

trap "kill $WATCH_PID 2>/dev/null; exit" INT TERM

cargo run --bin effect-viewer -- --grf "${1:-data/data.grf}"

kill $WATCH_PID 2>/dev/null
