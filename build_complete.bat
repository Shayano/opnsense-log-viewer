@echo off
setlocal enabledelayedexpansion
title OPNsense Log Viewer - Build Script
color 0a

echo.
echo ===============================================
echo    OPNsense Log Viewer - Single File Build
echo ===============================================
echo.

REM Environment verification
echo [1/4] Environment verification...
python --version >nul 2>&1
if errorlevel 1 (
    echo    ERROR: Python not found in PATH
    echo    Install Python or add it to PATH
    pause
    exit /b 1
)
echo    OK: Python detected

if not exist "src\opnsense_log_viewer\__main__.py" (
    echo    ERROR: src\opnsense_log_viewer\__main__.py not found
    echo    Run this script in the project folder
    pause
    exit /b 1
)
echo    OK: Source files present (src/opnsense_log_viewer/)

echo.
echo [2/4] Dependencies verification/installation...
pip install -r requirements.txt >nul 2>&1
if errorlevel 1 (
    echo    WARNING: Problem with dependencies
) else (
    echo    OK: Dependencies verified
)

echo.
echo [3/4] Cleaning previous builds...
if exist dist (
    rmdir /s /q dist >nul 2>&1
    echo    OK: dist folder removed
)
if exist build (
    rmdir /s /q build >nul 2>&1  
    echo    OK: build folder removed
)
if exist *.spec (
    del *.spec >nul 2>&1
    echo    OK: spec files removed
)

echo.
echo [4/4] Building single executable...
echo    Creating all-in-one portable executable...
echo    Using new modular source from src/opnsense_log_viewer/
python -m PyInstaller --name=OPNsense_Log_Viewer --onefile --windowed --noconfirm --clean --optimize=2 --icon=icon/icon64.ico --add-data=icon;icon --collect-data=tkinter --hidden-import=opnsense_log_viewer.components --hidden-import=opnsense_log_viewer.services --hidden-import=opnsense_log_viewer.utils --hidden-import=opnsense_log_viewer.constants --hidden-import=opnsense_log_viewer.exceptions src\opnsense_log_viewer\__main__.py >nul 2>&1
if exist "dist\OPNsense_Log_Viewer.exe" (
    echo    OK: Single file executable created
) else (
    echo    ERROR: Build failed
    pause
    exit /b 1
)

REM Calculate file size
for %%F in ("dist\OPNsense_Log_Viewer.exe") do set /a size_file=%%~zF/1024/1024

echo.
echo ===============================================
echo              BUILD COMPLETED!
echo ===============================================
echo.
echo RESULT IN 'dist' FOLDER:
echo.
echo PORTABLE EXECUTABLE:
echo    dist\OPNsense_Log_Viewer.exe
echo    Size: !size_file! MB  
echo    + Single file, no dependencies
echo    + Self-extracting executable
echo    + Ready for distribution
echo    + Slower startup (temporary extraction)
echo.
echo USAGE:
echo  - Double-click the executable
echo  - Load OPNsense log files
echo  - Optional: load XML config for interfaces
echo  - Use filters to analyze your logs
echo.

REM Test menu
echo WOULD YOU LIKE TO TEST THE APPLICATION?
echo 1. Launch the executable
echo 2. Open dist folder
echo 3. Exit
echo.
set /p choice="Your choice (1-3): "

if "%choice%"=="1" (
    echo Launching application...
    start "" "dist\OPNsense_Log_Viewer.exe"
    timeout /t 2 >nul
    echo Application launched!
) else if "%choice%"=="2" (
    echo Opening dist folder...
    start "" "%CD%\dist"
) else (
    echo Goodbye!
)

echo.
echo Press any key to close...
pause >nul
