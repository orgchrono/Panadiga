@echo off
@chcp 65001 >nul
setlocal EnableExtensions

rem ============================================================================
rem Panadiga Universal Windows Launcher (Idempotent + Auto-Bootstrap + Zero-Trap)
rem ============================================================================

set "SCRIPT_DIR=%~dp0"
if "%SCRIPT_DIR:~-1%"=="\" if not "%SCRIPT_DIR:~1,2%"==":\" set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"

set "PANADIGA_ROOT="
call :locate_root "%CD%"
if not defined PANADIGA_ROOT call :locate_root "%SCRIPT_DIR%"
if not defined PANADIGA_ROOT (
    echo [FAIL] Nao foi possivel localizar a raiz do repositorio Panadiga ^(Cargo.toml + scripts\build.py^).
    exit /b 1
)

if exist "%USERPROFILE%\.cargo\bin" call :prepend_path "%USERPROFILE%\.cargo\bin"
set "PYTHONUTF8=1"
if not defined NUMBER_OF_PROCESSORS set "NUMBER_OF_PROCESSORS=4"
if not defined CARGO_BUILD_JOBS set "CARGO_BUILD_JOBS=%NUMBER_OF_PROCESSORS%"
if not defined CMAKE_BUILD_PARALLEL_LEVEL set "CMAKE_BUILD_PARALLEL_LEVEL=%NUMBER_OF_PROCESSORS%"

set "PY_EXE="
set "PY_PREFIX="
call :find_python
if not defined PY_EXE (
    if not defined PANADIGA_NO_BOOTSTRAP (
        where winget >nul 2>&1
        if not errorlevel 1 (
            echo [BOOTSTRAP] INFO: Python 3.10+ nao encontrado; instalando via winget...
            winget install -e --id Python.Python.3.12 --silent --accept-package-agreements --accept-source-agreements
            call :find_python
        )
    )
)
if not defined PY_EXE (
    echo [FAIL] Python 3.10+ nao encontrado neste sistema.
    echo        Instale com: winget install Python.Python.3.12
    echo        Ou defina PANADIGA_PYTHON apontando para python.exe.
    exit /b 1
)

pushd "%PANADIGA_ROOT%" >nul
"%PY_EXE%" %PY_PREFIX% "%PANADIGA_ROOT%\scripts\build.py" %*
set "EXIT_CODE=%ERRORLEVEL%"
popd >nul
if not "%EXIT_CODE%"=="0" if "%PANADIGA_PAUSE_ON_ERROR%"=="1" pause
exit /b %EXIT_CODE%

rem ----------------------------------------------------------------------------
:prepend_path
set "_ADD_DIR=%~1"
if not defined _ADD_DIR exit /b 0
set "_CLEAN_PATH=%PATH:"=%"
setlocal EnableDelayedExpansion
set "_FOUND=0"
for %%P in ("%_CLEAN_PATH:;=";"%") do (
    if /I "%%~P"=="!_ADD_DIR!" set "_FOUND=1"
)
if "!_FOUND!"=="0" (
    endlocal & set "PATH=%_ADD_DIR%;%_CLEAN_PATH%"
) else (
    endlocal
)
exit /b 0

rem ----------------------------------------------------------------------------
:locate_root
set "_PROBE=%~1"
if not defined _PROBE exit /b 1
if "%_PROBE:~-1%"=="\" if not "%_PROBE:~1,2%"==":\" set "_PROBE=%_PROBE:~0,-1%"
:_walk_up
if exist "%_PROBE%\Cargo.toml" if exist "%_PROBE%\scripts\build.py" (
    set "PANADIGA_ROOT=%_PROBE%"
    exit /b 0
)
for %%I in ("%_PROBE%\..") do set "_PARENT=%%~fI"
if /I "%_PARENT%"=="%_PROBE%" exit /b 1
set "_PROBE=%_PARENT%"
goto :_walk_up

rem ----------------------------------------------------------------------------
:find_python
if defined PANADIGA_PYTHON (
    call :check_python_cmd "%PANADIGA_PYTHON%" "" && exit /b 0
)
if exist "%PANADIGA_ROOT%\.venv\Scripts\python.exe" (
    call :check_python_cmd "%PANADIGA_ROOT%\.venv\Scripts\python.exe" "" && exit /b 0
)
if exist "%PANADIGA_ROOT%\venv\Scripts\python.exe" (
    call :check_python_cmd "%PANADIGA_ROOT%\venv\Scripts\python.exe" "" && exit /b 0
)
for /f "delims=" %%P in ('where py 2^>nul') do (
    call :check_python_cmd "%%~fP" "-3" && exit /b 0
)
for /f "delims=" %%P in ('where python 2^>nul') do (
    call :check_python_cmd "%%~fP" "" && exit /b 0
)
for /f "delims=" %%P in ('where python3 2^>nul') do (
    call :check_python_cmd "%%~fP" "" && exit /b 0
)
if exist "%ProgramFiles%\PyManager\python.exe" (
    call :check_python_cmd "%ProgramFiles%\PyManager\python.exe" "" && exit /b 0
)
for /d %%D in ("%LOCALAPPDATA%\Programs\Python\Python3*") do (
    if exist "%%~fD\python.exe" call :check_python_cmd "%%~fD\python.exe" "" && exit /b 0
)
for /d %%D in ("%ProgramFiles%\Python3*") do (
    if exist "%%~fD\python.exe" call :check_python_cmd "%%~fD\python.exe" "" && exit /b 0
)
exit /b 1

:check_python_cmd
set "_CAND_EXE=%~1"
set "_CAND_ARG=%~2"
if not exist "%_CAND_EXE%" exit /b 1
echo "%_CAND_EXE%" | findstr /I /C:"\WindowsApps\" >nul && exit /b 1
"%_CAND_EXE%" %_CAND_ARG% -c "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)" >nul 2>&1
if errorlevel 1 exit /b 1
set "PY_EXE=%_CAND_EXE%"
set "PY_PREFIX=%_CAND_ARG%"
exit /b 0
