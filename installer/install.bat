@echo off
REM Zyra Installer Launcher
REM Double-click this file or run from command prompt

echo.
echo ========================================
echo    Zyra Programming Language Installer
echo ========================================
echo.

REM Check for PowerShell execution policy
powershell -Command "Get-ExecutionPolicy" >nul 2>&1
if errorlevel 1 (
    echo Error: PowerShell not available
    pause
    exit /b 1
)

REM Run the installer
powershell -ExecutionPolicy Bypass -File "%~dp0install.ps1" %*

pause
