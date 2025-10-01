"""
Unit tests for custom exceptions.
"""
import pytest

from opnsense_log_viewer.exceptions import (
    OPNsenseLogViewerError,
    FileOperationError,
    ParseError,
    SSHConnectionError,
    FilterError,
    ValidationError,
    MemoryError as AppMemoryError,
    MultiprocessingError
)


@pytest.mark.unit
class TestOPNsenseLogViewerError:
    """Test base exception class."""

    def test_base_exception_creation(self):
        """Test creating base exception."""
        exc = OPNsenseLogViewerError("Test error")
        assert str(exc) == "Test error"

    def test_base_exception_inheritance(self):
        """Test that base exception inherits from Exception."""
        exc = OPNsenseLogViewerError("Test")
        assert isinstance(exc, Exception)

    def test_raising_base_exception(self):
        """Test raising base exception."""
        with pytest.raises(OPNsenseLogViewerError) as exc_info:
            raise OPNsenseLogViewerError("Test error")
        assert "Test error" in str(exc_info.value)

    def test_catching_derived_exceptions(self):
        """Test that base exception catches all derived exceptions."""
        exceptions_to_test = [
            FileOperationError("File error"),
            ParseError("Parse error"),
            SSHConnectionError("SSH error"),
            FilterError("Filter error"),
            ValidationError("Validation error"),
            AppMemoryError("Memory error"),
            MultiprocessingError("MP error")
        ]

        for exc in exceptions_to_test:
            with pytest.raises(OPNsenseLogViewerError):
                raise exc


@pytest.mark.unit
class TestFileOperationError:
    """Test FileOperationError class."""

    def test_basic_error(self):
        """Test basic file operation error."""
        exc = FileOperationError("File not found")
        assert "File not found" in str(exc)

    def test_error_with_file_path(self):
        """Test error with file path."""
        exc = FileOperationError("Cannot read file", file_path="/path/to/file.log")
        assert "Cannot read file" in str(exc)
        assert "/path/to/file.log" in str(exc)
        assert exc.file_path == "/path/to/file.log"

    def test_error_with_operation(self):
        """Test error with operation."""
        exc = FileOperationError("Operation failed", operation="read")
        assert "Operation failed" in str(exc)
        assert "read" in str(exc)
        assert exc.operation == "read"

    def test_error_with_all_attributes(self):
        """Test error with all attributes."""
        original_error = IOError("Disk full")
        exc = FileOperationError(
            "Cannot write file",
            file_path="/path/to/file.log",
            operation="write",
            original_error=original_error
        )

        assert "Cannot write file" in str(exc)
        assert "/path/to/file.log" in str(exc)
        assert "write" in str(exc)
        assert exc.original_error is original_error

    def test_error_attributes_accessible(self):
        """Test that error attributes are accessible."""
        exc = FileOperationError(
            "Error",
            file_path="/test.log",
            operation="read"
        )

        assert exc.file_path == "/test.log"
        assert exc.operation == "read"
        assert exc.original_error is None


@pytest.mark.unit
class TestParseError:
    """Test ParseError class."""

    def test_basic_error(self):
        """Test basic parse error."""
        exc = ParseError("Parse failed")
        assert "Parse failed" in str(exc)

    def test_error_with_parser_type(self):
        """Test error with parser type."""
        exc = ParseError("Invalid format", parser_type="xml")
        assert "Invalid format" in str(exc)
        assert "xml" in str(exc)
        assert exc.parser_type == "xml"

    def test_error_with_line_number(self):
        """Test error with line number."""
        exc = ParseError("Invalid syntax", line_number=42)
        assert "Invalid syntax" in str(exc)
        assert "42" in str(exc)
        assert exc.line_number == 42

    def test_error_with_content(self):
        """Test error with content."""
        exc = ParseError("Cannot parse", content="bad,data,format")
        assert exc.content == "bad,data,format"

    def test_error_with_all_attributes(self):
        """Test error with all attributes."""
        original_error = ValueError("Invalid value")
        exc = ParseError(
            "Parse failed",
            content="test data",
            line_number=10,
            parser_type="log",
            original_error=original_error
        )

        assert "Parse failed" in str(exc)
        assert "log" in str(exc)
        assert "10" in str(exc)
        assert exc.content == "test data"
        assert exc.original_error is original_error


@pytest.mark.unit
class TestSSHConnectionError:
    """Test SSHConnectionError class."""

    def test_basic_error(self):
        """Test basic SSH error."""
        exc = SSHConnectionError("Connection failed")
        assert "Connection failed" in str(exc)

    def test_error_with_hostname(self):
        """Test error with hostname."""
        exc = SSHConnectionError("Cannot connect", hostname="192.168.1.1")
        assert "Cannot connect" in str(exc)
        assert "192.168.1.1" in str(exc)
        assert exc.hostname == "192.168.1.1"

    def test_error_with_username(self):
        """Test error with username."""
        exc = SSHConnectionError("Auth failed", username="root")
        assert "Auth failed" in str(exc)
        assert "root" in str(exc)
        assert exc.username == "root"

    def test_error_with_error_type(self):
        """Test error with error type."""
        exc = SSHConnectionError("Connection error", error_type="timeout")
        assert "Connection error" in str(exc)
        assert "timeout" in str(exc)
        assert exc.error_type == "timeout"

    def test_error_with_all_attributes(self):
        """Test error with all attributes."""
        import socket
        original_error = socket.timeout()
        exc = SSHConnectionError(
            "Connection timeout",
            hostname="192.168.1.1",
            username="admin",
            error_type="timeout",
            original_error=original_error
        )

        assert "Connection timeout" in str(exc)
        assert "192.168.1.1" in str(exc)
        assert "admin" in str(exc)
        assert "timeout" in str(exc)
        assert exc.original_error is original_error


@pytest.mark.unit
class TestFilterError:
    """Test FilterError class."""

    def test_basic_error(self):
        """Test basic filter error."""
        exc = FilterError("Filter failed")
        assert "Filter failed" in str(exc)

    def test_error_with_filter_field(self):
        """Test error with filter field."""
        exc = FilterError("Invalid field", filter_field="action")
        assert "Invalid field" in str(exc)
        assert "action" in str(exc)
        assert exc.filter_field == "action"

    def test_error_with_operator(self):
        """Test error with operator."""
        exc = FilterError("Invalid operator", operator="==")
        assert "Invalid operator" in str(exc)
        assert "==" in str(exc)
        assert exc.operator == "=="

    def test_error_with_filter_value(self):
        """Test error with filter value."""
        exc = FilterError("Invalid value", filter_value="test")
        assert exc.filter_value == "test"

    def test_error_with_all_attributes(self):
        """Test error with all attributes."""
        original_error = ValueError("Bad regex")
        exc = FilterError(
            "Regex compilation failed",
            filter_field="src",
            filter_value="[invalid",
            operator="regex",
            original_error=original_error
        )

        assert "Regex compilation failed" in str(exc)
        assert "src" in str(exc)
        assert "regex" in str(exc)
        assert exc.original_error is original_error


@pytest.mark.unit
class TestValidationError:
    """Test ValidationError class."""

    def test_basic_error(self):
        """Test basic validation error."""
        exc = ValidationError("Validation failed")
        assert "Validation failed" in str(exc)

    def test_error_with_field_name(self):
        """Test error with field name."""
        exc = ValidationError("Invalid input", field_name="port")
        assert "Invalid input" in str(exc)
        assert "port" in str(exc)
        assert exc.field_name == "port"

    def test_error_with_validation_rule(self):
        """Test error with validation rule."""
        exc = ValidationError("Rule violation", validation_rule="range(1-65535)")
        assert "Rule violation" in str(exc)
        assert "range(1-65535)" in str(exc)
        assert exc.validation_rule == "range(1-65535)"

    def test_error_with_invalid_value(self):
        """Test error with invalid value."""
        exc = ValidationError("Bad value", invalid_value="abc")
        assert exc.invalid_value == "abc"

    def test_error_with_all_attributes(self):
        """Test error with all attributes."""
        original_error = TypeError("Wrong type")
        exc = ValidationError(
            "Type mismatch",
            field_name="timeout",
            invalid_value="not_a_number",
            validation_rule="must be integer",
            original_error=original_error
        )

        assert "Type mismatch" in str(exc)
        assert "timeout" in str(exc)
        assert "must be integer" in str(exc)
        assert exc.original_error is original_error


@pytest.mark.unit
class TestMemoryError:
    """Test custom MemoryError class."""

    def test_basic_error(self):
        """Test basic memory error."""
        exc = AppMemoryError("Out of memory")
        assert "Out of memory" in str(exc)

    def test_error_with_operation(self):
        """Test error with operation."""
        exc = AppMemoryError("Memory allocation failed", operation="cache")
        assert "Memory allocation failed" in str(exc)
        assert "cache" in str(exc)
        assert exc.operation == "cache"

    def test_error_with_memory_requested(self):
        """Test error with memory requested."""
        exc = AppMemoryError("Cannot allocate", memory_requested="500MB")
        assert "Cannot allocate" in str(exc)
        assert "500MB" in str(exc)
        assert exc.memory_requested == "500MB"

    def test_error_with_all_attributes(self):
        """Test error with all attributes."""
        original_error = MemoryError("System out of memory")
        exc = AppMemoryError(
            "Cache allocation failed",
            operation="chunk_cache",
            memory_requested="1GB",
            original_error=original_error
        )

        assert "Cache allocation failed" in str(exc)
        assert "chunk_cache" in str(exc)
        assert "1GB" in str(exc)
        assert exc.original_error is original_error


@pytest.mark.unit
class TestMultiprocessingError:
    """Test MultiprocessingError class."""

    def test_basic_error(self):
        """Test basic multiprocessing error."""
        exc = MultiprocessingError("Worker failed")
        assert "Worker failed" in str(exc)

    def test_error_with_worker_id(self):
        """Test error with worker ID."""
        exc = MultiprocessingError("Worker crashed", worker_id="worker-3")
        assert "Worker crashed" in str(exc)
        assert "worker-3" in str(exc)
        assert exc.worker_id == "worker-3"

    def test_error_with_operation(self):
        """Test error with operation."""
        exc = MultiprocessingError("Operation failed", operation="filter")
        assert "Operation failed" in str(exc)
        assert "filter" in str(exc)
        assert exc.operation == "filter"

    def test_error_with_all_attributes(self):
        """Test error with all attributes."""
        original_error = RuntimeError("Process terminated")
        exc = MultiprocessingError(
            "Worker process failed",
            worker_id="worker-1",
            operation="parallel_filter",
            original_error=original_error
        )

        assert "Worker process failed" in str(exc)
        assert "worker-1" in str(exc)
        assert "parallel_filter" in str(exc)
        assert exc.original_error is original_error


@pytest.mark.unit
class TestExceptionContextPreservation:
    """Test that exception context is preserved."""

    def test_original_error_preserved(self):
        """Test that original error is preserved in wrapper."""
        original = IOError("Disk error")
        wrapped = FileOperationError("Cannot read", original_error=original)

        assert wrapped.original_error is original
        assert isinstance(wrapped.original_error, IOError)

    def test_exception_chaining(self):
        """Test exception chaining."""
        try:
            try:
                raise IOError("Original error")
            except IOError as e:
                raise FileOperationError("Wrapped error", original_error=e)
        except FileOperationError as wrapped:
            assert wrapped.original_error is not None
            assert isinstance(wrapped.original_error, IOError)

    def test_multiple_attributes_preserved(self):
        """Test that all attributes are preserved."""
        exc = ParseError(
            "Parse failed",
            content="test data",
            line_number=42,
            parser_type="xml"
        )

        # All attributes should be accessible
        assert exc.content == "test data"
        assert exc.line_number == 42
        assert exc.parser_type == "xml"

    def test_exception_message_formatting(self):
        """Test exception message includes context."""
        exc = FileOperationError(
            "Cannot read",
            file_path="/test.log",
            operation="read"
        )

        message = str(exc)
        assert "Cannot read" in message
        assert "/test.log" in message
        assert "read" in message
