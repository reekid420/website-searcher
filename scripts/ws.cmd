@echo off
setlocal
set DIR=%~dp0
set CANDIDATES=
set CANDIDATES=%CANDIDATES% "%DIR%website-searcher.exe"
set CANDIDATES=%CANDIDATES% "%DIR%..\bin\website-searcher.exe"
set CANDIDATES=%CANDIDATES% "%DIR%..\website-searcher.exe"
set CANDIDATES=%CANDIDATES% "%ProgramFiles%\website-searcher\bin\website-searcher.exe"

for %%F in (%CANDIDATES%) do (
  if exist %%~fF (
    "%%~fF" %*
    exit /b %errorlevel%
  )
)

echo ws.cmd: could not find website-searcher.exe next to this script or under Program Files.
exit /b 1

