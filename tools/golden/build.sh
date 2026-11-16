#!/bin/bash
# Builds and runs the golden generator with the read-only .NET SDK. All output goes to target/golden.
#
#   tools/golden/build.sh             build only
#   tools/golden/build.sh run [args]  build, then run (arguments pass through to the generator)
#
# `dotnet build` writes scratch files into <dotnet root>/metadata, so it must not run from the SDK folder
# itself. target/golden/dotnet-root is a private root: its own copy of the 140 KB muxer and its own metadata
# folder, with symlinks to the SDK's host, shared, sdk, packs, sdk-manifests and templates folders.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SDK="/Users/developer/Documents/Codex/2026-09-13/is-the/work/dotnet"
FARM="$ROOT/target/golden/dotnet-root"
if [ ! -x "$FARM/dotnet" ]; then
  mkdir -p "$FARM/metadata"
  cp "$SDK/dotnet" "$FARM/dotnet"
  for x in host shared sdk packs sdk-manifests templates; do ln -sfn "$SDK/$x" "$FARM/$x"; done
fi
export DOTNET_ROOT="$FARM"
export DOTNET_CLI_HOME="$ROOT/target/golden/cli-home"
export DOTNET_NOLOGO=1 DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
export DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE=1 DOTNET_SKIP_WORKLOAD_INTEGRITY_CHECK=true
export DOTNET_GENERATE_ASPNET_CERTIFICATE=false MSBuildEnableWorkloadResolver=false
export NUGET_PACKAGES="$ROOT/target/golden/packages"
export NUGET_HTTP_CACHE_PATH="$ROOT/target/golden/nuget-http"
export NUGET_SCRATCH="$ROOT/target/golden/nuget-scratch"
export NUGET_PLUGINS_CACHE_PATH="$ROOT/target/golden/nuget-plugins"
mkdir -p "$DOTNET_CLI_HOME" "$NUGET_PACKAGES" "$ROOT/target/golden/empty-feed"
python3 "$ROOT/tools/golden/helpers/prepare.py"
cd "$ROOT/tools/golden"
"$FARM/dotnet" build Golden.csproj -c Release --nologo -v:q -nodeReuse:false -p:UseSharedCompilation=false --configfile "$ROOT/tools/golden/nuget.config"
"$FARM/dotnet" build extras/Module.Effect.GoldenExtras.csproj -c Release --nologo -v:q -nodeReuse:false -p:UseSharedCompilation=false --configfile "$ROOT/tools/golden/nuget.config"
if [ "${1:-}" = "run" ]; then
  shift
  cd "$ROOT"
  exec "$FARM/dotnet" "$ROOT/target/golden/bin/VixenGolden.dll" "$@"
fi
