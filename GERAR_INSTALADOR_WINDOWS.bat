@echo off
setlocal
cd /d "%~dp0"
if not exist "node_modules\" (
  call npm install
  if errorlevel 1 goto :failed
)
call npm run installer
if errorlevel 1 goto :failed
echo.
echo Instalador gerado em src-tauri\target\release\bundle\nsis\
pause
exit /b 0
:failed
echo.
echo Falha ao gerar instalador. Confira README.md.
pause
exit /b 1
