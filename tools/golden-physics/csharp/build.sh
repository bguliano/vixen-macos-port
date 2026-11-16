#!/bin/bash
export DOTNET_ROOT="/tmp/vixen-scratch/physics/csharp/out/dotnet-root"
export DOTNET_CLI_HOME="/tmp/vixen-scratch/physics/csharp/out/cli-home"
export DOTNET_NOLOGO=1 DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1
export DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE=1 DOTNET_SKIP_WORKLOAD_INTEGRITY_CHECK=true
export DOTNET_GENERATE_ASPNET_CERTIFICATE=false MSBuildEnableWorkloadResolver=false
export NUGET_PACKAGES="/tmp/vixen-scratch/physics/csharp/out/packages"
export NUGET_HTTP_CACHE_PATH="/tmp/vixen-scratch/physics/csharp/out/nuget-http"
export NUGET_SCRATCH="/tmp/vixen-scratch/physics/csharp/out/nuget-scratch"
export NUGET_PLUGINS_CACHE_PATH="/tmp/vixen-scratch/physics/csharp/out/nuget-plugins"
cd "/tmp/vixen-scratch/physics/csharp"
"/tmp/vixen-scratch/physics/csharp/out/dotnet-root/dotnet" build Diff.csproj -c Release --nologo -v:q -nodeReuse:false -p:UseSharedCompilation=false --configfile "/tmp/vixen-scratch/physics/csharp/nuget.config" 2>&1 | grep -E "error|Warn.*[0-9]+ Warn|Build succeeded|Elapsed" | sed -E 's/ \[.*\]$//' | sort -u | head -60
if [ "${1:-}" = "run" ]; then shift; cd "/tmp/vixen-scratch/physics/csharp/out"; exec "/tmp/vixen-scratch/physics/csharp/out/dotnet-root/dotnet" "/tmp/vixen-scratch/physics/csharp/out/bin/PhysicsDiff.dll" "$@"; fi
