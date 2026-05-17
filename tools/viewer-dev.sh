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

echo "Starting unified viewer"
cargo run --bin viewer -- "$@"
