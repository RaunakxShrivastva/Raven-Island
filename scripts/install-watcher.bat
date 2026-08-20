@echo off
REM Raven Smart Watcher - Windows Task Scheduler Installer
REM This sets up the watcher to start automatically when you log in

echo ========================================
echo   Raven Smart Watcher - Auto-Start Setup
echo ========================================
echo.

cd /d "%~dp0.."
set SCRIPT_PATH=%CD%\scripts\smart-watcher.js
set TASK_NAME=Raven_Doc_Watcher

echo Installing Task Scheduler job...
echo.

REM Create the scheduled task
schtasks /Create /TN "%TASK_NAME%" /TR "powershell -WindowStyle Hidden -Command \"node '%SCRIPT_PATH%'\"" /SC ONLOGON /RU "%USERNAME%" /RL LIMITED /F

if %ERRORLEVEL% EQU 0 (
    echo.
    echo [SUCCESS] Task installed successfully!
    echo.
    echo The watcher will now start automatically when you log in.
    echo.
    echo Resource usage:
    echo   - Idle: ~15 MB RAM, 0%% CPU
    echo   - Active: Updates only when files change
    echo.
    echo To manually start now: node scripts\smart-watcher.js
    echo To uninstall: run scripts\uninstall-watcher.bat
    echo.
) else (
    echo.
    echo [ERROR] Failed to install task.
    echo Try running this script as Administrator.
    echo.
)

pause
