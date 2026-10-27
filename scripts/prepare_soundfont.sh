#!/usr/bin/env bash
# Build Chicken Coop's bank, assets/soundfonts/chicken_coop.sf2, and the LICENSE beside it,
# through the engine's scripts/prepare_game_soundfont.sh: the FluidR3 GM presets the William
# Tell finale plays. The subset is committed; this script is how anyone rebuilds it byte for
# byte.
#
# Usage: scripts/prepare_soundfont.sh [--bank path/to/FluidR3_GM.sf2]
#   The engine's script documents the fetch, the sha256 check and the 4 MiB cap.
#
# The presets, as bank:program: trumpet (the galop's tune, the first chicken's hop, a nest's
# fanfare, the round's cadence), piccolo (the counter-voice, the second chicken's hop, the
# jam), trombone (the pah, a chicken's squawk), tuba (the oom), timpani (the gallop). No
# horns and no strings: any of them beside the piccolo and the timpani is over 4 MiB.
set -euo pipefail

GAME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENGINE_DIR="$(cd "$GAME_DIR/../../insiculous_2d" && pwd)"
OUTPUT_DIR="$GAME_DIR/assets/soundfonts"

exec "$ENGINE_DIR/scripts/prepare_game_soundfont.sh" "$@" --max-mib 4 "$OUTPUT_DIR" chicken_coop 0:56 0:72 0:57 0:58 0:47
