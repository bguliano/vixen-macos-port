@echo off
setlocal
cd /d "%~dp0"
echo Vixen Windows diagnostics v3.2 - independent timed stages; inventory first.
"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Run-Capture.ps1"
echo.
echo Copy the displayed results ZIP or folder back, even if optional probes were skipped.
echo No Vixen application was launched and nothing was installed.
pause
