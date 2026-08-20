@echo off
REM Raven Smart Watcher - Start Immediately

cd /d "%~dp0.."
echo Starting Raven Smart Watcher...
echo.
echo Press Ctrl+C to stop
echo.

node scripts\smart-watcher.js
