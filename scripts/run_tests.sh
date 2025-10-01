#!/bin/bash
# OPNsense Log Viewer - Test Execution Script (Linux/Mac)
# Run all tests with coverage reporting

set -e

echo "========================================"
echo "OPNsense Log Viewer - Test Suite"
echo "========================================"
echo

# Activate virtual environment if it exists
if [ -f "venv/bin/activate" ]; then
    echo "Activating virtual environment..."
    source venv/bin/activate
fi

# Check if pytest is installed
if ! python -m pytest --version > /dev/null 2>&1; then
    echo "ERROR: pytest is not installed"
    echo "Please install test dependencies: pip install -r requirements.txt"
    exit 1
fi

echo "Running tests..."
echo

# Run pytest with coverage
python -m pytest tests/ \
    -v \
    --cov=src/opnsense_log_viewer \
    --cov-report=term-missing \
    --cov-report=html:coverage_html \
    --cov-report=xml \
    --tb=short

TEST_RESULT=$?

echo
echo "========================================"
echo "Test execution completed"
echo "========================================"
echo

if [ $TEST_RESULT -eq 0 ]; then
    echo "All tests passed successfully!"
    echo
    echo "Coverage report generated:"
    echo "- Terminal output above"
    echo "- HTML report: coverage_html/index.html"
    echo "- XML report: coverage.xml"
else
    echo "Some tests failed. Please review the output above."
fi

echo

exit $TEST_RESULT
