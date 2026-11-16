#!/bin/bash
# Offline, read-only upstream oracle; all outputs and .NET state are private under target/golden.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
FARM="$ROOT/target/golden/dotnet-root"
if [ ! -x "$FARM/dotnet" ]; then "$ROOT/tools/golden/build.sh"; fi
export DOTNET_ROOT="$FARM" DOTNET_CLI_HOME="$ROOT/target/golden/cli-home"
export DOTNET_NOLOGO=1 DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
export DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE=1 DOTNET_SKIP_WORKLOAD_INTEGRITY_CHECK=true
export DOTNET_GENERATE_ASPNET_CERTIFICATE=false MSBuildEnableWorkloadResolver=false
export NUGET_PACKAGES="$ROOT/target/golden/packages" NUGET_HTTP_CACHE_PATH="$ROOT/target/golden/nuget-http"
export NUGET_SCRATCH="$ROOT/target/golden/nuget-scratch" NUGET_PLUGINS_CACHE_PATH="$ROOT/target/golden/nuget-plugins"
cd "$ROOT"
"$FARM/dotnet" build tools/golden/media/MediaGolden.csproj -c Release --nologo -v:q -nodeReuse:false -p:UseSharedCompilation=false --configfile tools/golden/nuget.config
if [ "${1:-}" = "run" ]; then shift; exec "$FARM/dotnet" target/golden/media/bin/MediaGolden.dll "$@"; fi
