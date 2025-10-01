"""
Custom exceptions for OPNsense Log Viewer.

This module defines a hierarchy of custom exceptions for better error handling
and debugging throughout the application.
"""


class OPNsenseLogViewerError(Exception):
    """
    Base exception for all OPNsense Log Viewer errors.

    All custom exceptions in this application should inherit from this class.
    This allows for catching all application-specific exceptions in one handler.
    """
    pass


class FileOperationError(OPNsenseLogViewerError):
    """
    Exception raised for file operation failures.

    This includes errors during:
    - File reading/writing
    - File access permissions
    - File not found
    - Directory operations

    Attributes:
        file_path: Path to the file that caused the error
        operation: Type of operation that failed (read, write, open, etc.)
        original_error: The underlying exception that was caught
    """

    def __init__(self, message, file_path=None, operation=None, original_error=None):
        self.file_path = file_path
        self.operation = operation
        self.original_error = original_error

        full_message = message
        if file_path:
            full_message += f" (File: {file_path})"
        if operation:
            full_message += f" (Operation: {operation})"

        super().__init__(full_message)


class ParseError(OPNsenseLogViewerError):
    """
    Exception raised for parsing failures.

    This includes errors during:
    - Log file parsing
    - XML configuration parsing
    - Data format validation
    - Field extraction

    Attributes:
        content: The content that failed to parse
        line_number: Line number where parsing failed (if applicable)
        parser_type: Type of parser (log, xml, etc.)
        original_error: The underlying exception that was caught
    """

    def __init__(self, message, content=None, line_number=None, parser_type=None, original_error=None):
        self.content = content
        self.line_number = line_number
        self.parser_type = parser_type
        self.original_error = original_error

        full_message = message
        if parser_type:
            full_message += f" (Parser: {parser_type})"
        if line_number:
            full_message += f" (Line: {line_number})"

        super().__init__(full_message)


class SSHConnectionError(OPNsenseLogViewerError):
    """
    Exception raised for SSH connection failures.

    This includes errors during:
    - SSH connection establishment
    - Authentication failures
    - Network timeouts
    - Command execution over SSH

    Attributes:
        hostname: The hostname/IP that connection was attempted to
        username: Username used for authentication
        error_type: Type of SSH error (auth, timeout, network, etc.)
        original_error: The underlying exception that was caught
    """

    def __init__(self, message, hostname=None, username=None, error_type=None, original_error=None):
        self.hostname = hostname
        self.username = username
        self.error_type = error_type
        self.original_error = original_error

        full_message = message
        if hostname:
            full_message += f" (Host: {hostname})"
        if username:
            full_message += f" (User: {username})"
        if error_type:
            full_message += f" (Type: {error_type})"

        super().__init__(full_message)


class FilterError(OPNsenseLogViewerError):
    """
    Exception raised for filtering operation failures.

    This includes errors during:
    - Filter condition evaluation
    - Invalid filter expressions
    - Filter application errors
    - Regex compilation errors

    Attributes:
        filter_field: The field being filtered
        filter_value: The value being filtered for
        operator: The comparison operator used
        original_error: The underlying exception that was caught
    """

    def __init__(self, message, filter_field=None, filter_value=None, operator=None, original_error=None):
        self.filter_field = filter_field
        self.filter_value = filter_value
        self.operator = operator
        self.original_error = original_error

        full_message = message
        if filter_field:
            full_message += f" (Field: {filter_field})"
        if operator:
            full_message += f" (Operator: {operator})"

        super().__init__(full_message)


class ValidationError(OPNsenseLogViewerError):
    """
    Exception raised for input validation failures.

    This includes errors during:
    - User input validation
    - Configuration validation
    - Data type validation
    - Range/constraint violations

    Attributes:
        field_name: Name of the field that failed validation
        invalid_value: The value that failed validation
        validation_rule: The validation rule that was violated
        original_error: The underlying exception that was caught
    """

    def __init__(self, message, field_name=None, invalid_value=None, validation_rule=None, original_error=None):
        self.field_name = field_name
        self.invalid_value = invalid_value
        self.validation_rule = validation_rule
        self.original_error = original_error

        full_message = message
        if field_name:
            full_message += f" (Field: {field_name})"
        if validation_rule:
            full_message += f" (Rule: {validation_rule})"

        super().__init__(full_message)


class MemoryError(OPNsenseLogViewerError):
    """
    Exception raised for memory management failures.

    This includes errors during:
    - Memory allocation
    - Cache operations
    - Large file handling
    - Resource exhaustion

    Attributes:
        operation: Operation that caused memory issues
        memory_requested: Amount of memory requested (if known)
        original_error: The underlying exception that was caught
    """

    def __init__(self, message, operation=None, memory_requested=None, original_error=None):
        self.operation = operation
        self.memory_requested = memory_requested
        self.original_error = original_error

        full_message = message
        if operation:
            full_message += f" (Operation: {operation})"
        if memory_requested:
            full_message += f" (Requested: {memory_requested})"

        super().__init__(full_message)


class MultiprocessingError(OPNsenseLogViewerError):
    """
    Exception raised for multiprocessing operation failures.

    This includes errors during:
    - Process pool creation
    - Worker process failures
    - Inter-process communication
    - Serialization errors

    Attributes:
        worker_id: ID of the worker that failed (if applicable)
        operation: Operation that failed
        original_error: The underlying exception that was caught
    """

    def __init__(self, message, worker_id=None, operation=None, original_error=None):
        self.worker_id = worker_id
        self.operation = operation
        self.original_error = original_error

        full_message = message
        if worker_id:
            full_message += f" (Worker: {worker_id})"
        if operation:
            full_message += f" (Operation: {operation})"

        super().__init__(full_message)
