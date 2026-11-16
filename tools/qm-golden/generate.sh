#!/bin/bash
# Regenerates tests/golden/analysis from the upstream C++.
#
#   tools/qm-golden/generate.sh
#
# The real songs are read only: the Rust side decodes them to samples under target/analysis/qm-golden/pcm, the C++ runs
# those samples, and only features and hashes reach the golden files. The samples are deleted afterwards. Without the
# media folder the existing real-music goldens are left as they are.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MEDIA="/Users/developer/Documents/Vixen 3/Media"
OUT="$ROOT/tests/golden/analysis"
WORK="$ROOT/target/analysis/qm-golden"

"$ROOT/tools/qm-golden/build.sh"

mkdir -p "$OUT/units"
"$WORK/qm-golden" --synthetic "$OUT"
"$WORK/qm-golden-units" "$OUT/units/primitives.json"

if [ ! -d "$MEDIA" ]; then
    echo "no media folder, keeping the existing real-music goldens"
    "$ROOT/tools/qm-golden/marks/run.sh"
    exit 0
fi

source "$ROOT/env.sh"
CARGO_TARGET_DIR="$ROOT/target/analysis" cargo build --release -p vixen-analysis --example dump_pcm
DUMP="$ROOT/target/analysis/release/examples/dump_pcm"

mkdir -p "$WORK/pcm"

# name, file, then any extra harness arguments.
run_song() {
    local name=$1 file=$2
    shift 2
    local pcm="$WORK/pcm/${file//[^A-Za-z0-9]/_}.f32"
    local rate
    rate=$("$DUMP" "$MEDIA/$file" "$pcm" | sed -E 's/.* rate=([0-9]+) .*/\1/')
    "$WORK/qm-golden" --raw "$pcm" --rate "$rate" --name "$name" --out "$OUT" --note "$file" --time "$@"
}

run_song real_sandstorm "Darude_Sandstorm.mp3"
run_song real_sandstorm_bpb3 "Darude_Sandstorm.mp3" --bpb 3
run_song real_sandstorm_preview "Darude_Sandstorm.mp3" --driver preview
run_song real_wizards "Wizards in Winter - Trans-Siberian Orchestra.mp3"
run_song real_wizards_constrain "Wizards in Winter - Trans-Siberian Orchestra.mp3" --alpha 0.7 --tempo 150 --constrain 1
run_song real_coldest_winter_m4a "Coldest Winter Pentatonix.m4a"
run_song real_christmas_canon_48k "Christmas Canon.mp3"
run_song real_intro_short "christmas intro 1.mp3"
run_song real_zoom_concealed_frames "Zoom.mp3"

rm -rf "$WORK/pcm"

# The C# reads the feature files written above.
"$ROOT/tools/qm-golden/marks/run.sh"
