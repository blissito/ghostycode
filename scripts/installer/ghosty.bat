@echo off
set NO_ANIMATIONS=1
where wt >nul 2>nul
if "%ERRORLEVEL%"=="0" (
    wt --title Ghosty cmd /k "%~dp0ghosty.exe"
) else (
    "%~dp0ghosty.exe"
)
