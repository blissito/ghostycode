@echo off
setlocal enabledelayedexpansion
:: GhostyCode Windows installer
:: Copies ghosty.exe, ghosty-tui.exe, and ghosty.bat to %USERPROFILE%\bin (single binary, no ghosty-tui.exe)

set "BIN_DIR=%USERPROFILE%\bin"
set "SCRIPT_DIR=%~dp0"

if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"

echo Installing ghosty to %BIN_DIR%...

copy /Y "%SCRIPT_DIR%ghosty.exe" "%BIN_DIR%\ghosty.exe" >nul
if %ERRORLEVEL% neq 0 (
    echo ERROR: Failed to copy ghosty.exe
    exit /b 1
)

copy /Y "%SCRIPT_DIR%ghosty-tui.exe" "%BIN_DIR%\ghosty-tui.exe" >nul
if %ERRORLEVEL% neq 0 (
    echo ERROR: Failed to copy ghosty-tui.exe
    exit /b 1
)

copy /Y "%SCRIPT_DIR%ghosty.bat" "%BIN_DIR%\ghosty.bat" >nul
if %ERRORLEVEL% neq 0 (
    echo ERROR: Failed to copy ghosty.bat
    exit /b 1
)

echo.
echo Done. Commands installed to %BIN_DIR%.
echo.
echo Add %BIN_DIR% to your PATH:
echo   1. Open Start, search "environment variables"
echo   2. Click "Environment Variables..."
echo   3. Under "User variables", select "Path" and click "Edit"
echo   4. Click "New" and add: %BIN_DIR%
echo   5. Click OK, then restart your terminal
echo.
echo Or run this in an admin PowerShell:
echo   [Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path', 'User') + ';%BIN_DIR%', 'User')
echo.
echo Then run: ghosty
echo Double-click ghosty.bat (not ghosty.exe) to open Windows Terminal when it is installed.
