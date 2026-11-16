#!/bin/bash
# Compiles the upstream QM Bar and Beat Tracker sources, unmodified, into the golden harness.
#
#   tools/qm-golden/build.sh
#
# Output goes to target/analysis/qm-golden. Nothing here is part of the Rust build.
#
# -ffp-contract=off keeps a*b+c as two rounded operations. Clang on arm64 would otherwise fuse them into one FMA
# instruction. The shipped Windows build (MSVC x64, /fp:precise, no /arch:AVX2) does not fuse, and neither does Rust.
# qm-golden-contract is the same harness with clang's default, kept to measure how much that choice matters.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HERE="$ROOT/tools/qm-golden"
QM="$ROOT/upstream/Vixen/src/Vixen.Modules/Analysis/QMLibrary/QMFiles"
OUT="$ROOT/target/analysis/qm-golden"

mkdir -p "$OUT/obj" "$OUT/obj-contract"

# Same defines and include directories as QMLibrary.vcxproj.
FLAGS=(-O2 -D_USE_MATH_DEFINES -Dkiss_fft_scalar=double
    -I "$QM/vamp-plugin-sdk-2.10" -I "$QM/ext/kissfft" -I "$QM/ext/kissfft/tools" -I "$QM")

# Everything QMLibrary.vcxproj compiles from QMFiles.
REACHABLE_CXX=(
    BarBeatTrack.cpp
    dsp/onsets/DetectionFunction.cpp
    dsp/phasevocoder/PhaseVocoder.cpp
    dsp/rateconversion/Decimator.cpp
    dsp/tempotracking/DownBeat.cpp
    dsp/tempotracking/TempoTrackV2.cpp
    dsp/transforms/FFT.cpp
    maths/MathUtilities.cpp
    vamp-plugin-sdk-2.10/src/vamp-sdk/RealTime.cpp
)
UNREACHED_CXX=(
    dsp/onsets/PeakPicking.cpp
    dsp/signalconditioning/DFProcess.cpp
    dsp/signalconditioning/Filter.cpp
    dsp/signalconditioning/FiltFilt.cpp
)
C_SOURCES=(
    ext/kissfft/kiss_fft.c
    ext/kissfft/tools/kiss_fftr.c
)

object_for() {
    echo "$1/${2//\//_}.o"
}

compile_set() {
    local objdir=$1 contract=$2 src
    for src in "${REACHABLE_CXX[@]}" "${UNREACHED_CXX[@]}"; do
        clang++ -std=c++17 "-ffp-contract=$contract" -w "${FLAGS[@]}" -c "$QM/$src" -o "$(object_for "$objdir" "$src")"
    done
    for src in "${C_SOURCES[@]}"; do
        clang "-ffp-contract=$contract" -w "${FLAGS[@]}" -c "$QM/$src" -o "$(object_for "$objdir" "$src")"
    done
    clang++ -std=c++17 "-ffp-contract=$contract" -Wall -Wno-unused-function "${FLAGS[@]}" \
        -c "$HERE/qm_golden.cpp" -o "$objdir/qm_golden.o"
}

compile_set "$OUT/obj" off
clang++ "$OUT"/obj/*.o -o "$OUT/qm-golden"

clang++ -std=c++17 -ffp-contract=off -w "${FLAGS[@]}" -c "$HERE/qm_golden_units.cpp" -o "$OUT/qm_golden_units.o"
UPSTREAM_OBJECTS=()
for src in "${REACHABLE_CXX[@]}" "${C_SOURCES[@]}"; do
    UPSTREAM_OBJECTS+=("$(object_for "$OUT/obj" "$src")")
done
clang++ "$OUT/qm_golden_units.o" "${UPSTREAM_OBJECTS[@]}" -o "$OUT/qm-golden-units"

compile_set "$OUT/obj-contract" on
clang++ "$OUT"/obj-contract/*.o -o "$OUT/qm-golden-contract"

# Linking succeeds only if the tracker never reaches these sources, which is why the Rust crate leaves them out.
clang++ "$OUT/obj/qm_golden.o" "${UPSTREAM_OBJECTS[@]}" -o "$OUT/qm-golden-reachable-check"
echo "reachability: the tracker links without PeakPicking, DFProcess, Filter and FiltFilt"

echo "built $OUT/qm-golden"
