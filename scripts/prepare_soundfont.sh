#!/usr/bin/env bash
# Stage original MIDI Evil exports; validated engine save owns runtime installation.
set -euo pipefail
GAME_DIR="$(cd "$(dirname "$0")/.." && pwd)"
exec python3 "$GAME_DIR/music-source/build.py" "$@"
