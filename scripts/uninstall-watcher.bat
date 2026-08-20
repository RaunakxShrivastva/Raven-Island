@echo off
REM Raven Smart Watcher - Uninstaller

echo ========================================
echo   Raven Smart Watcher - Remove Auto-Start
echo ========================================
echo.

set TASK_NAME=Raven_Doc_Watcher

echo Removing scheduled task...
echo.

schtasks /Delete /TN "%TASK_NAME%" /F

if %ERRORLEVEL% EQU 0 (
    echo [SUCCESS] Task removed.
    echo The watcher will no longer start automatically.
) else (
    echo [INFO] Task was not found or already removed.
)

echo.
pause
