@echo off
REM Raven Documentation Agent Starter
REM Starts the auto-documentation watcher in the background

echo Starting Raven Documentation Agent...
echo The agent will monitor your Raven project and auto-update documentation.
echo Press Ctrl+C to stop.
echo.

cd /d "%~dp0.."
node scripts\watch-docs.js
