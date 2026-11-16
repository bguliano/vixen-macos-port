#!/bin/bash
# Builds and runs QmMarksGolden with the read-only .NET SDK that tools/golden also uses, in its own private root.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
SDK="/Users/developer/Documents/Codex/2026-09-13/is-the/work/dotnet"
OUT="$ROOT/target/analysis/qm-golden/cs"
FARM="$OUT/dotnet-root"
if [ ! -x "$FARM/dotnet" ]; then
  mkdir -p "$FARM/metadata"
  cp "$SDK/dotnet" "$FARM/dotnet"
  for x in host shared sdk packs sdk-manifests templates; do ln -sfn "$SDK/$x" "$FARM/$x"; done
fi
export DOTNET_ROOT="$FARM"
export DOTNET_CLI_HOME="$OUT/cli-home"
export DOTNET_NOLOGO=1 DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
export DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE=1 DOTNET_SKIP_WORKLOAD_INTEGRITY_CHECK=true
export DOTNET_GENERATE_ASPNET_CERTIFICATE=false MSBuildEnableWorkloadResolver=false
export NUGET_PACKAGES="$OUT/packages"
export NUGET_HTTP_CACHE_PATH="$OUT/nuget-http"
export NUGET_SCRATCH="$OUT/nuget-scratch"
export NUGET_PLUGINS_CACHE_PATH="$OUT/nuget-plugins"
mkdir -p "$DOTNET_CLI_HOME" "$NUGET_PACKAGES" "$OUT/empty-feed"
cd "$ROOT/tools/qm-golden/marks"
"$FARM/dotnet" build QmMarksGolden.csproj -c Release --nologo -v:q -nodeReuse:false -p:UseSharedCompilation=false --configfile nuget.config
cd "$ROOT"
exec "$FARM/dotnet" "$OUT/bin/QmMarksGolden.dll" "$ROOT/tests/golden/analysis"
