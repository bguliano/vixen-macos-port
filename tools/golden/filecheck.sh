#!/bin/bash
# Puts the Rust port's saved files through the original Vixen's loader and saver, and compares the results.
#
#   tools/golden/filecheck.sh
#
# Everything happens on copies under target/filecheck. The compiled original comes from target/golden/bin, without the extras
# assembly, whose copies of effect data classes would clash in the serializer's known types.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="$ROOT/target/filecheck"
"$ROOT/tools/golden/build.sh"
export DOTNET_ROOT="$ROOT/target/golden/dotnet-root"
export DOTNET_CLI_HOME="$ROOT/target/golden/cli-home" DOTNET_NOLOGO=1 DOTNET_CLI_TELEMETRY_OPTOUT=1
export NUGET_PACKAGES="$ROOT/target/golden/packages" MSBuildEnableWorkloadResolver=false
"$DOTNET_ROOT/dotnet" build "$ROOT/tools/golden/filecheck/Module.Effect.FileCheckExtras.csproj" -c Release --nologo -v:q -nodeReuse:false \
  -p:UseSharedCompilation=false --configfile "$ROOT/tools/golden/nuget.config"

rm -rf "$OUT/bin" "$OUT/original" "$OUT/port" "$OUT/out-original" "$OUT/out-port"
mkdir -p "$OUT"
rsync -a --exclude 'Module.Effect.GoldenExtras.*' "$ROOT/target/golden/bin/" "$OUT/bin/"
cp "$ROOT/target/golden/filecheck-extras/Module.Effect.FileCheckExtras.dll" "$OUT/bin/"
rsync -a --exclude Media "$ROOT/target/show-copy/" "$OUT/original/"
mkdir -p "$OUT/original/Media"
rsync -a "$OUT/original/" "$OUT/port/"

# The compiled original has no migrator. Going from version 8 to 9 only touches SetPosition and Nutcracker effects, which these files don't have.
if grep -lE 'SetPositionData|NutcrackerData' "$OUT/original/Sequence/"*.tim; then
  echo "Some sequences need a real migration, so version 8 can't simply be relabelled."
  exit 1
fi
sed -i '' '2s/<TimedSequenceData version="8"/<TimedSequenceData version="9"/' "$OUT/original/Sequence/"*.tim

source "$ROOT/env.sh" >/dev/null 2>&1
CARGO_TARGET_DIR="$ROOT/target/integration-cargo" cargo run --release -q -p vixen-session --example resave_show -- "$OUT/port" 2>"$OUT/resave.log" || { tail -30 "$OUT/resave.log"; exit 1; }
# What the port builds itself, as opposed to reloads: every effect the compiled original has, at its defaults and with every value
# changed, plus marks and layers. The effects named here are the ones it lacks.
rm -rf "$OUT/new" "$OUT/out-new"
mkdir -p "$OUT/new"
CARGO_TARGET_DIR="$ROOT/target/integration-cargo" cargo run --release -q -p vixen-session --example write_every_effect -- \
  "$OUT/port" "$OUT/new/every-effect.tim" "Count Down" "Fixture" "Fixture Strobe" "Frost" "Glediator" "Gobo" "Line Dance" "LipSync" \
  "Liquid" "Morph" "Picture" "Prism" "Set Position" "Set Zoom" "Shapes" "Snowflakes" "Spin Color Wheel" "State" "Text" "Video" 2>"$OUT/write.log" | tail -1 || { tail -30 "$OUT/write.log"; exit 1; }
CARGO_TARGET_DIR="$ROOT/target/integration-cargo" cargo run --release -q -p vixen-session --example reload_sequence -- \
  "$OUT/port" "$OUT/new/every-effect.tim" "$OUT/new-reloaded.tim" 2>"$OUT/reload.log" || { tail -30 "$OUT/reload.log"; exit 1; }

# The original rewrites these, so keep what the port wrote.
rm -rf "$OUT/port-raw"
mkdir -p "$OUT/port-raw"
cp "$OUT/port/SystemData/SystemConfig.xml" "$OUT/port/SystemData/ModuleStore.xml" "$OUT/port-raw/"
cp -R "$OUT/port/Sequence" "$OUT/port-raw/Sequence"

cd "$ROOT"
set +e
for which in original port; do
  "$DOTNET_ROOT/dotnet" "$OUT/bin/VixenGolden.dll" --profile "$OUT/$which" --resave "$OUT/$which/Sequence" "$OUT/out-$which" > "$OUT/$which.log" 2>&1
  echo "$which: loader exit $?, $(grep '^unresolved assemblies' "$OUT/$which.log")"
done
"$DOTNET_ROOT/dotnet" "$OUT/bin/VixenGolden.dll" --profile "$OUT/port" --resave "$OUT/new" "$OUT/out-new" > "$OUT/new.log" 2>&1
echo "effects the port built: loader exit $?"
python3 "$ROOT/tools/golden/compare_files.py" "$OUT"
