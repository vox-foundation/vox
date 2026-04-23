@echo off
setlocal
REM Gate a single PowerShell line: set VOX_SHELL_CHECK_PAYLOAD then run this wrapper from repo root context.
cd /d "%~dp0..\.."
if "%VOX_SHELL_CHECK_PAYLOAD%"=="" (
  echo Set VOX_SHELL_CHECK_PAYLOAD to the PowerShell source line, then run this file. >&2
  exit /b 2
)
where vox >nul 2>&1
if errorlevel 1 (
  echo vox is not on PATH. >&2
  exit /b 127
)
vox shell check --payload "%VOX_SHELL_CHECK_PAYLOAD%"
exit /b %ERRORLEVEL%
