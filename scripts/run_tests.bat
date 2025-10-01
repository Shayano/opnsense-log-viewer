@echo off
REM OPNsense Log Viewer - Test Execution Script (Windows)
REM Run all tests with coverage reporting

echo ========================================
echo OPNsense Log Viewer - Test Suite
echo ========================================
echo.

REM Activate virtual environment if it exists
if exist venv\Scripts\activate.bat (
    echo Activating virtual environment...
    call venv\Scripts\activate.bat
)

REM Check if pytest is installed
python -m pytest --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: pytest is not installed
    echo Please install test dependencies: pip install -r requirements.txt
    pause
    exit /b 1
)

echo Running tests...
echo.

REM Run pytest with coverage
python -m pytest tests\ ^
    -v ^
    --cov=src\opnsense_log_viewer ^
    --cov-report=term-missing ^
    --cov-report=html:coverage_html ^
    --cov-report=xml ^
    --tb=short

set TEST_RESULT=%ERRORLEVEL%

echo.
echo ========================================
echo Test execution completed
echo ========================================
echo.

if %TEST_RESULT% equ 0 (
    echo All tests passed successfully!
    echo.
    echo Coverage report generated:
    echo - Terminal output above
    echo - HTML report: coverage_html\index.html
    echo - XML report: coverage.xml
) else (
    echo Some tests failed. Please review the output above.
)

echo.
pause
exit /b %TEST_RESULT%
