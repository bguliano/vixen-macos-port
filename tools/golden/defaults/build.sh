#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
export DOTNET_ROOT="$ROOT/target/golden/dotnet-root" DOTNET_CLI_HOME="$ROOT/target/golden/cli-home"
export DOTNET_NOLOGO=1 DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
export DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE=1 MSBuildEnableWorkloadResolver=false
export NUGET_PACKAGES="$ROOT/target/golden/packages"
cd "$ROOT"
python3 tools/golden/defaults/prepare.py
"$DOTNET_ROOT/dotnet" build tools/golden/defaults/DefaultsGolden.csproj -c Release --nologo -v:q -nodeReuse:false -p:UseSharedCompilation=false --configfile tools/golden/nuget.config
if [ "${1:-}" = "run" ]; then shift; exec "$DOTNET_ROOT/dotnet" target/golden/defaults/bin/DefaultsGolden.dll "$@"; fi
