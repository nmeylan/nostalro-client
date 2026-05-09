#!/bin/bash
# Hot-reload development script for UI component examples.
#
# Usage:
#   ./dev.sh <example_name> [--grf <path>]
#
# Examples:
#   ./dev.sh inventory
#   ./dev.sh npc_shop --grf data/data.grf
#   ./dev.sh chat
#
# Available examples:
#   inventory, npc_shop, login, chat, npc_dialog, confirm_dialog,
#   server_list, equipment, system_menu, char_select
#
# Categories (show multiple windows at once):
#   game     — inventory, npc_shop, npc_dialog, equipment, system_menu, confirm_dialog, chat
#   account  — login, server_list, char_select
#
# This script runs two processes:
#   1. The hot_reload host binary (keeps window open)
#   2. cargo-watch rebuilding the dylib on file changes

set -e

EXAMPLE="${1:?Usage: ./dev.sh <example_name> [--grf path]}"
shift

cd "$(dirname "$0")/../.."

echo "Building hot dylib..."
cargo build -p ragnarok-ui-component-hot

echo "Starting hot-reload for: $EXAMPLE"
echo "Edit files in lib/ui-component/src/ and they will auto-reload."
echo ""

# Start cargo-watch in background to rebuild dylib on changes
cargo watch \
	--no-vcs-ignores \
	-w lib/ui-component/src \
	-w lib/ui-component-hot/src \
	-w lib/ui-core/src \
	-s "cargo build -p ragnarok-ui-component-hot" &
WATCH_PID=$!

trap "kill $WATCH_PID 2>/dev/null; exit" INT TERM

# Run the host example
cargo run --example hot_reload -p ragnarok-ui-component -- --example "$EXAMPLE" "$@"

kill $WATCH_PID 2>/dev/null
