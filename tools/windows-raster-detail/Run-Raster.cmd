@echo off
setlocal
cd /d "%~dp0"
echo Vixen raster detail diagnostic - 80 bounded cases, no installation.
"%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Watch-Raster.ps1"
echo.
echo Copy the displayed results ZIP or folder back, even if some cases failed.
echo No Vixen application was launched and nothing was installed.
pause
