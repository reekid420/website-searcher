@echo off
setlocal enabledelayedexpansion

:: Check for --gui or -g flag
set LAUNCH_GUI=0
set REMAINING_ARGS=
set BINARY_NAME=website-searcher.exe

:parse_args
if "%~1"=="" goto done_parsing
if /i "%~1"=="--gui" (
    set LAUNCH_GUI=1
    shift
    goto parse_args
)
if /i "%~1"=="-g" (
    set LAUNCH_GUI=1
    shift
    goto parse_args
)
set REMAINING_ARGS=!REMAINING_ARGS! %1
shift
goto parse_args

:done_parsing
if %LAUNCH_GUI%==1 (
    set BINARY_NAME=website-searcher-gui.exe
)

set DIR=%~dp0
set CANDIDATES=
set CANDIDATES=%CANDIDATES% "%DIR%%BINARY_NAME%"
set CANDIDATES=%CANDIDATES% "%DIR%..\bin\%BINARY_NAME%"
set CANDIDATES=%CANDIDATES% "%DIR%..\%BINARY_NAME%"
set CANDIDATES=%CANDIDATES% "%ProgramFiles%\website-searcher\bin\%BINARY_NAME%"

for %%F in (%CANDIDATES%) do (
  if exist %%~fF (
    "%%~fF" %REMAINING_ARGS%
    exit /b %errorlevel%
  )
)

echo ws.cmd: could not find %BINARY_NAME% next to this script or under Program Files.
exit /b 1
