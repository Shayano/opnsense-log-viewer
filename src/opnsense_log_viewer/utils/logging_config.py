"""
Centralized logging configuration for OPNsense Log Viewer.

This module provides a consistent logging setup across the entire application
with file rotation, multiple log levels, and structured formatting.
"""
import logging
import os
import sys
from logging.handlers import RotatingFileHandler
from pathlib import Path


# Log levels
LOG_LEVEL_DEBUG = logging.DEBUG
LOG_LEVEL_INFO = logging.INFO
LOG_LEVEL_WARNING = logging.WARNING
LOG_LEVEL_ERROR = logging.ERROR
LOG_LEVEL_CRITICAL = logging.CRITICAL

# Default log format
LOG_FORMAT = '%(asctime)s - %(name)s - %(levelname)s - %(module)s:%(funcName)s:%(lineno)d - %(message)s'
DATE_FORMAT = '%Y-%m-%d %H:%M:%S'

# Console format (simpler for user-facing messages)
CONSOLE_FORMAT = '%(levelname)s - %(message)s'

# Log file settings
LOG_DIR = 'logs'
LOG_FILE = 'opnsense_viewer.log'
MAX_LOG_SIZE = 10 * 1024 * 1024  # 10 MB
BACKUP_COUNT = 5  # Keep 5 backup files


def setup_logging(log_level=LOG_LEVEL_INFO, console_level=LOG_LEVEL_ERROR, log_to_file=True, log_to_console=True):
    """
    Configure the logging system for the application.

    This function sets up:
    - File handler with rotation (logs/opnsense_viewer.log)
    - Console handler for errors and above
    - Structured formatting with timestamps and context
    - Multiple log levels for different verbosity

    Args:
        log_level: Minimum level for file logging (default: INFO)
        console_level: Minimum level for console logging (default: ERROR)
        log_to_file: Enable file logging (default: True)
        log_to_console: Enable console logging (default: True)

    Returns:
        logging.Logger: Configured root logger

    Raises:
        FileOperationError: If log directory cannot be created
    """
    # Get root logger
    logger = logging.getLogger()
    logger.setLevel(LOG_LEVEL_DEBUG)  # Capture everything, handlers will filter

    # Clear any existing handlers
    logger.handlers.clear()

    # File handler with rotation
    if log_to_file:
        try:
            # Create log directory if it doesn't exist
            log_dir_path = Path(LOG_DIR)
            log_dir_path.mkdir(parents=True, exist_ok=True)

            log_file_path = log_dir_path / LOG_FILE

            # Create rotating file handler
            file_handler = RotatingFileHandler(
                log_file_path,
                maxBytes=MAX_LOG_SIZE,
                backupCount=BACKUP_COUNT,
                encoding='utf-8'
            )
            file_handler.setLevel(log_level)

            # Set format
            file_formatter = logging.Formatter(LOG_FORMAT, datefmt=DATE_FORMAT)
            file_handler.setFormatter(file_formatter)

            logger.addHandler(file_handler)

        except Exception as e:
            # If file logging fails, at least log to console
            print(f"Warning: Could not set up file logging: {e}", file=sys.stderr)

    # Console handler (only errors and above by default)
    if log_to_console:
        console_handler = logging.StreamHandler(sys.stdout)
        console_handler.setLevel(console_level)

        # Simpler format for console
        console_formatter = logging.Formatter(CONSOLE_FORMAT)
        console_handler.setFormatter(console_formatter)

        logger.addHandler(console_handler)

    # Log the initialization
    logger.info("="*80)
    logger.info("OPNsense Log Viewer - Logging System Initialized")
    logger.info(f"Log Level (File): {logging.getLevelName(log_level)}")
    logger.info(f"Log Level (Console): {logging.getLevelName(console_level)}")
    logger.info(f"Log File: {LOG_DIR}/{LOG_FILE}")
    logger.info("="*80)

    return logger


def get_logger(name):
    """
    Get a logger instance for a specific module.

    Args:
        name: Name of the module (typically __name__)

    Returns:
        logging.Logger: Configured logger for the module

    Example:
        logger = get_logger(__name__)
        logger.info("Processing started")
    """
    return logging.getLogger(name)


def log_exception(logger, exc, message="An error occurred", **kwargs):
    """
    Log an exception with full stack trace and context.

    Args:
        logger: Logger instance to use
        exc: Exception object to log
        message: Custom message to prefix the error
        **kwargs: Additional context to log (key-value pairs)

    Example:
        try:
            risky_operation()
        except Exception as e:
            log_exception(logger, e, "Failed to process file", file_path="/path/to/file")
    """
    context = " | ".join(f"{k}={v}" for k, v in kwargs.items())
    full_message = f"{message}"
    if context:
        full_message += f" | Context: {context}"

    logger.error(full_message, exc_info=True)


def set_log_level(level, handler_type='all'):
    """
    Change the logging level at runtime.

    Args:
        level: New log level (logging.DEBUG, INFO, WARNING, ERROR, CRITICAL)
        handler_type: Which handlers to update ('file', 'console', or 'all')

    Example:
        set_log_level(logging.DEBUG, 'file')  # Enable debug logging to file
    """
    logger = logging.getLogger()

    if handler_type == 'all':
        logger.setLevel(level)
        for handler in logger.handlers:
            handler.setLevel(level)
    else:
        for handler in logger.handlers:
            if handler_type == 'file' and isinstance(handler, RotatingFileHandler):
                handler.setLevel(level)
            elif handler_type == 'console' and isinstance(handler, logging.StreamHandler) and not isinstance(handler, RotatingFileHandler):
                handler.setLevel(level)


def log_performance(logger, operation, duration, **kwargs):
    """
    Log performance metrics for operations.

    Args:
        logger: Logger instance to use
        operation: Name of the operation
        duration: Duration in seconds
        **kwargs: Additional metrics (rows_processed, memory_used, etc.)

    Example:
        start = time.time()
        process_logs()
        log_performance(logger, "process_logs", time.time() - start, rows_processed=1000)
    """
    metrics = " | ".join(f"{k}={v}" for k, v in kwargs.items())
    message = f"Performance: {operation} completed in {duration:.2f}s"
    if metrics:
        message += f" | Metrics: {metrics}"

    logger.info(message)


def log_user_action(logger, action, **kwargs):
    """
    Log user actions for audit trail.

    Args:
        logger: Logger instance to use
        action: Description of user action
        **kwargs: Additional context

    Example:
        log_user_action(logger, "File opened", file_path="/path/to/file", user="admin")
    """
    context = " | ".join(f"{k}={v}" for k, v in kwargs.items())
    message = f"User Action: {action}"
    if context:
        message += f" | Context: {context}"

    logger.info(message)


# Initialize logging on module import (with default settings)
# Applications can call setup_logging() again with custom settings
try:
    setup_logging()
except Exception as e:
    # Fallback to basic logging if setup fails
    logging.basicConfig(level=logging.INFO)
    logging.error(f"Failed to initialize logging system: {e}")
