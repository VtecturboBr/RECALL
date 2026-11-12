@echo off
setlocal
cd /d "%~dp0"
title Memoria Externa - Versao 0.3
where npm >nul 2>&1
if errorlevel 1 (
  echo [ERRO] Node.js / npm nao encontrado.
  echo Instale Node.js LTS: https://nodejs.org/
  pause
  exit /b 1
)
where cargo >nul 2>&1
if errorlevel 1 (
  echo [ERRO] Rust nao encontrado.
  echo Instale Rust: https://rustup.rs/
  echo Instale tambem Microsoft C++ Build Tools.
  echo Veja instrucoes no README.md.
  pause
  exit /b 1
)
if not exist "node_modules\" (
  echo Instalando bibliotecas do projeto...
  call npm install
  if errorlevel 1 goto :failed
)
echo Iniciando Memoria Externa...
call npm run desktop
if errorlevel 1 goto :failed
exit /b 0
:failed
echo.
echo Falha na inicializacao. Confira o erro acima e o arquivo README.md.
pause
exit /b 1
