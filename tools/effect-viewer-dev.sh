#!/bin/bash
set -e
cd "$(dirname "$0")/.."

echo "Building hot dylib..."
cargo build -p effect-viewer-hot

echo "Starting hot-reload effect viewer"
echo "Edit tools/effect-viewer-hot/src/lib.rs (picker, overlay) or anything under"
echo "lib/renderer/src/effect/ and lib/effects/src/ — changes auto-reload."

cargo watch \
    -w lib/renderer/src \
    -w lib/ui-core/src \
    -w lib/effects/src \
    -w lib/game/src \
    -w lib/formats/src \
    -w tools/effect-viewer-hot/src \
    -s "cargo build -p effect-viewer-hot" &
WATCH_PID=$!

trap "kill $WATCH_PID 2>/dev/null; exit" INT TERM

cargo run --bin effect-viewer -- --grf "${1:-data/data.grf}"

kill $WATCH_PID 2>/dev/null
