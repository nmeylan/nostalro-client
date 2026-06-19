#!/bin/bash
set -e
cd "$(dirname "$0")/.."

# Unified viewer: map + character + effect preview.
# Usage:
#   tools/viewer-dev.sh                              # prontera, default GRF
#   tools/viewer-dev.sh --map geffen                 # different map
#   tools/viewer-dev.sh --grf path/to/data.grf       # explicit GRF
#   tools/viewer-dev.sh --map prontera --effect 42   # spawn an effect at startup
#
# Controls (also printed with --help):
#   B               cycle background: RSW map -> ground proxy -> clear
#   Right drag      orbit camera
#   Scroll          zoom (also +/-)
#   C               reset camera on character
#   Space           pause animation + effects
#   Arrow keys      action/direction
#   Q/W S h/H E r/R weapon / sex / head / headgear / shield
#   N/P             cycle effect preset, F to replay

# The `viewer` binary links effects statically (no cdylib backend), so hot
# reload here means cargo-watch rebuilds and restarts the viewer on any source
# change under the watched crates — including lib/effects/src.
echo "Starting unified viewer (auto-rebuild on source change)"
echo "Edit anything under lib/effects/src, lib/game/src, lib/renderer/src — the viewer restarts."

cargo watch \
    -w lib/effects/src \
    -w lib/game/src \
    -w lib/renderer/src \
    -w lib/formats/src \
    -w lib/ui-core/src \
    -w tools/src \
    -x "run --bin viewer -- $*"
