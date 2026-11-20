@echo off
@chcp 65001 >nul
setlocal enabledelayedexpansion
title Panadiga - Inicializador do Ghidra

echo ====================================================================
echo    PANADIGA: ENGENHARIA REVERSA MATSUSHITA AM34 COM NSA GHIDRA
echo    Scripts Jython SSOT: %~dp0ghidra\
echo ====================================================================
echo.

if not exist "%USERPROFILE%\ghidra_scripts" mkdir "%USERPROFILE%\ghidra_scripts" >nul 2>&1
copy /Y "%~dp0ghidra\*.py" "%USERPROFILE%\ghidra_scripts\" >nul 2>&1
if exist "%~dp0ghidra\java\*.java" (
    copy /Y "%~dp0ghidra\java\*.java" "%USERPROFILE%\ghidra_scripts\" >nul 2>&1
)
if exist "%~dp0ghidra\java\panadiga\*.java" (
    if not exist "%USERPROFILE%\ghidra_scripts\panadiga" mkdir "%USERPROFILE%\ghidra_scripts\panadiga" >nul 2>&1
    copy /Y "%~dp0ghidra\java\panadiga\*.java" "%USERPROFILE%\ghidra_scripts\panadiga\" >nul 2>&1
)
echo [OK] Scripts Jython SSOT e Analistas Java espelhados em %USERPROFILE%\ghidra_scripts

set "GHIDRA_GPR="
if exist "C:\ghidra\projects\Panadiga_MN103S.gpr" set "GHIDRA_GPR=C:\ghidra\projects\Panadiga_MN103S.gpr"
if not defined GHIDRA_GPR if exist "C:\ghidra\projects\Panadiga_MN103S\Panadiga_MN103S.gpr" set "GHIDRA_GPR=C:\ghidra\projects\Panadiga_MN103S\Panadiga_MN103S.gpr"

set "GHIDRA_BAT="
if defined GHIDRA_HOME if exist "%GHIDRA_HOME%\ghidraRun.bat" set "GHIDRA_BAT=%GHIDRA_HOME%\ghidraRun.bat"
if not defined GHIDRA_BAT for %%D in ("C:\ghidra" "C:\tools\ghidra" "D:\ghidra" "%USERPROFILE%\ghidra" "C:\Program Files\ghidra") do (
    for /d %%F in ("%%~D\ghidra_*") do (
        if exist "%%~F\ghidraRun.bat" if not defined GHIDRA_BAT set "GHIDRA_BAT=%%~F\ghidraRun.bat"
    )
)

if defined GHIDRA_BAT (
    echo [INFO] Executando: "%GHIDRA_BAT%" "%GHIDRA_GPR%"
    if defined GHIDRA_GPR (
        call "%GHIDRA_BAT%" "%GHIDRA_GPR%"
    ) else (
        call "%GHIDRA_BAT%"
    )
    exit /b 0
)

where ghidraRun.bat >nul 2>&1
if not errorlevel 1 (
    call ghidraRun.bat "%GHIDRA_GPR%"
    exit /b 0
)

echo [AVISO] ghidraRun.bat nao encontrado no PATH nem em GHIDRA_HOME / C:\ghidra.
start "" explorer "%~dp0ghidra"
pause
