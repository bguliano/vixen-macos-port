#!/bin/bash
# Unchanged upstream filter/ballistics over the probe's decoded mono input.
# No Windows decoder, audio device, network, or upstream writes.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
export DOTNET_ROOT="$ROOT/target/golden/dotnet-root" DOTNET_CLI_HOME="$ROOT/target/golden/cli-home"
export DOTNET_NOLOGO=1 DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
export DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE=1 DOTNET_SKIP_WORKLOAD_INTEGRITY_CHECK=true
export DOTNET_GENERATE_ASPNET_CERTIFICATE=false MSBuildEnableWorkloadResolver=false
export NUGET_PACKAGES="$ROOT/target/golden/packages" NUGET_HTTP_CACHE_PATH="$ROOT/target/golden/nuget-http"
export NUGET_SCRATCH="$ROOT/target/golden/nuget-scratch" NUGET_PLUGINS_CACHE_PATH="$ROOT/target/golden/nuget-plugins"
cd "$ROOT"
"$DOTNET_ROOT/dotnet" build tools/golden/real-audio/RealAudio.csproj -c Release --nologo -v:q -nodeReuse:false -p:UseSharedCompilation=false --configfile tools/golden/nuget.config
if [ "${1:-}" = "run" ]; then shift; exec "$DOTNET_ROOT/dotnet" target/golden/real-audio/bin/RealAudio.dll "$@"; fi
