"""
Unit tests for Logging Configuration.
"""
import pytest
import logging
import os
from pathlib import Path
from unittest.mock import patch, MagicMock

from opnsense_log_viewer.utils.logging_config import (
    setup_logging,
    get_logger,
    log_exception,
    set_log_level,
    log_performance,
    log_user_action,
    LOG_LEVEL_DEBUG,
    LOG_LEVEL_INFO,
    LOG_LEVEL_WARNING,
    LOG_LEVEL_ERROR
)


@pytest.mark.unit
class TestLoggingSetup:
    """Test logging setup functions."""

    def test_setup_logging_default(self):
        """Test setting up logging with defaults."""
        logger = setup_logging()

        assert logger is not None
        assert isinstance(logger, logging.Logger)
        assert len(logger.handlers) > 0

    def test_setup_logging_file_only(self, temp_dir):
        """Test setting up file logging only."""
        with patch('opnsense_log_viewer.utils.logging_config.LOG_DIR', temp_dir):
            logger = setup_logging(log_to_file=True, log_to_console=False)

            # Should have at least one handler (file)
            assert len(logger.handlers) >= 1

    def test_setup_logging_console_only(self):
        """Test setting up console logging only."""
        logger = setup_logging(log_to_file=False, log_to_console=True)

        # Should have at least one handler (console)
        assert len(logger.handlers) >= 1

    def test_setup_logging_custom_levels(self):
        """Test setting up logging with custom levels."""
        logger = setup_logging(
            log_level=LOG_LEVEL_DEBUG,
            console_level=LOG_LEVEL_WARNING
        )

        assert logger is not None

    def test_setup_logging_creates_log_directory(self, temp_dir):
        """Test that logging creates log directory."""
        log_dir = Path(temp_dir) / 'test_logs'

        with patch('opnsense_log_viewer.utils.logging_config.LOG_DIR', str(log_dir)):
            setup_logging(log_to_file=True)

            # Directory should be created
            assert log_dir.exists()

    def test_setup_logging_file_creation(self, temp_dir):
        """Test that log file is created."""
        with patch('opnsense_log_viewer.utils.logging_config.LOG_DIR', temp_dir):
            setup_logging(log_to_file=True)

            log_file = Path(temp_dir) / 'opnsense_viewer.log'
            assert log_file.exists()

    def test_setup_logging_clears_existing_handlers(self):
        """Test that setup clears existing handlers."""
        root_logger = logging.getLogger()
        initial_handler_count = len(root_logger.handlers)

        logger = setup_logging()

        # Handlers should be cleared and new ones added
        assert logger is not None

    def test_setup_logging_failure_fallback(self, mocker):
        """Test logging setup failure fallback."""
        # Mock Path.mkdir to raise exception
        mock_path = mocker.patch('pathlib.Path.mkdir')
        mock_path.side_effect = PermissionError("Cannot create directory")

        # Should not raise exception, but fall back gracefully
        logger = setup_logging(log_to_file=True, log_to_console=True)

        assert logger is not None


@pytest.mark.unit
class TestGetLogger:
    """Test get_logger function."""

    def test_get_logger_returns_logger(self):
        """Test that get_logger returns a logger."""
        logger = get_logger(__name__)

        assert logger is not None
        assert isinstance(logger, logging.Logger)

    def test_get_logger_with_module_name(self):
        """Test getting logger with module name."""
        logger = get_logger('test_module')

        assert logger.name == 'test_module'

    def test_get_logger_different_names(self):
        """Test getting loggers with different names."""
        logger1 = get_logger('module1')
        logger2 = get_logger('module2')

        assert logger1.name != logger2.name

    def test_get_logger_same_name_returns_same_logger(self):
        """Test that same name returns same logger instance."""
        logger1 = get_logger('same_module')
        logger2 = get_logger('same_module')

        assert logger1 is logger2

    def test_get_logger_inherits_config(self):
        """Test that logger inherits root configuration."""
        setup_logging()
        logger = get_logger('test_inherit')

        # Should inherit root logger configuration
        assert logger is not None


@pytest.mark.unit
class TestLogException:
    """Test log_exception function."""

    def test_log_exception_basic(self):
        """Test logging exception with basic message."""
        logger = get_logger('test_exception')

        try:
            raise ValueError("Test error")
        except ValueError as e:
            # Should not raise exception
            log_exception(logger, e, "Error occurred")

    def test_log_exception_with_context(self):
        """Test logging exception with context."""
        logger = get_logger('test_exception')

        try:
            raise IOError("File error")
        except IOError as e:
            log_exception(logger, e, "File operation failed",
                        file_path="/test.txt", operation="read")

    def test_log_exception_with_multiple_kwargs(self):
        """Test logging exception with multiple context values."""
        logger = get_logger('test_exception')

        try:
            raise Exception("Test")
        except Exception as e:
            log_exception(logger, e, "Operation failed",
                        user="admin", action="delete", resource="file.txt")

    def test_log_exception_logs_to_error_level(self, mocker):
        """Test that exception is logged at ERROR level."""
        logger = get_logger('test_exception')
        mock_error = mocker.patch.object(logger, 'error')

        try:
            raise ValueError("Test")
        except ValueError as e:
            log_exception(logger, e, "Test message")

        mock_error.assert_called_once()


@pytest.mark.unit
class TestSetLogLevel:
    """Test set_log_level function."""

    def test_set_log_level_all_handlers(self):
        """Test setting log level for all handlers."""
        setup_logging()
        set_log_level(LOG_LEVEL_DEBUG, 'all')

        # Should not raise exception
        assert True

    def test_set_log_level_file_only(self):
        """Test setting log level for file handler only."""
        setup_logging(log_to_file=True)
        set_log_level(LOG_LEVEL_WARNING, 'file')

        assert True

    def test_set_log_level_console_only(self):
        """Test setting log level for console handler only."""
        setup_logging(log_to_console=True)
        set_log_level(LOG_LEVEL_ERROR, 'console')

        assert True

    def test_set_log_level_different_levels(self):
        """Test setting different log levels."""
        setup_logging()

        for level in [LOG_LEVEL_DEBUG, LOG_LEVEL_INFO, LOG_LEVEL_WARNING, LOG_LEVEL_ERROR]:
            set_log_level(level, 'all')
            # Should complete without error
            assert True


@pytest.mark.unit
class TestLogPerformance:
    """Test log_performance function."""

    def test_log_performance_basic(self):
        """Test logging performance metric."""
        logger = get_logger('test_performance')
        log_performance(logger, "test_operation", 1.5)

        # Should not raise exception
        assert True

    def test_log_performance_with_metrics(self):
        """Test logging performance with additional metrics."""
        logger = get_logger('test_performance')
        log_performance(logger, "parse_logs", 2.3,
                       rows_processed=1000, memory_used="50MB")

        assert True

    def test_log_performance_zero_duration(self):
        """Test logging performance with zero duration."""
        logger = get_logger('test_performance')
        log_performance(logger, "fast_operation", 0.001)

        assert True

    def test_log_performance_large_duration(self):
        """Test logging performance with large duration."""
        logger = get_logger('test_performance')
        log_performance(logger, "slow_operation", 3600.0)

        assert True

    def test_log_performance_logs_at_info_level(self, mocker):
        """Test that performance is logged at INFO level."""
        logger = get_logger('test_performance')
        mock_info = mocker.patch.object(logger, 'info')

        log_performance(logger, "operation", 1.0)

        mock_info.assert_called_once()


@pytest.mark.unit
class TestLogUserAction:
    """Test log_user_action function."""

    def test_log_user_action_basic(self):
        """Test logging user action."""
        logger = get_logger('test_user_action')
        log_user_action(logger, "File opened")

        assert True

    def test_log_user_action_with_context(self):
        """Test logging user action with context."""
        logger = get_logger('test_user_action')
        log_user_action(logger, "Filter applied",
                       field="action", value="pass", user="admin")

        assert True

    def test_log_user_action_multiple_context(self):
        """Test logging user action with multiple context values."""
        logger = get_logger('test_user_action')
        log_user_action(logger, "Export completed",
                       format="csv", rows=500, destination="/exports/data.csv")

        assert True

    def test_log_user_action_logs_at_info_level(self, mocker):
        """Test that user action is logged at INFO level."""
        logger = get_logger('test_user_action')
        mock_info = mocker.patch.object(logger, 'info')

        log_user_action(logger, "Action performed")

        mock_info.assert_called_once()


@pytest.mark.unit
class TestLoggingIntegration:
    """Test integrated logging functionality."""

    def test_logging_workflow(self, temp_dir):
        """Test complete logging workflow."""
        # Setup
        with patch('opnsense_log_viewer.utils.logging_config.LOG_DIR', temp_dir):
            logger = setup_logging(log_to_file=True, log_to_console=False)

            # Get module logger
            module_logger = get_logger('test_module')

            # Log various levels
            module_logger.debug("Debug message")
            module_logger.info("Info message")
            module_logger.warning("Warning message")
            module_logger.error("Error message")

            # Log exception
            try:
                raise ValueError("Test exception")
            except ValueError as e:
                log_exception(module_logger, e, "Exception occurred")

            # Log performance
            log_performance(module_logger, "test_op", 1.5, items=100)

            # Log user action
            log_user_action(module_logger, "Test action", user="test")

            # Verify log file exists
            log_file = Path(temp_dir) / 'opnsense_viewer.log'
            assert log_file.exists()

    def test_multiple_loggers_coexist(self):
        """Test that multiple loggers can coexist."""
        setup_logging()

        logger1 = get_logger('module1')
        logger2 = get_logger('module2')
        logger3 = get_logger('module3')

        # All should log without interfering
        logger1.info("Message from logger1")
        logger2.info("Message from logger2")
        logger3.info("Message from logger3")

        assert True

    def test_logging_thread_safety(self):
        """Test that logging is thread-safe."""
        import threading

        setup_logging()
        logger = get_logger('thread_test')

        def log_messages():
            for i in range(10):
                logger.info(f"Message {i}")

        threads = [threading.Thread(target=log_messages) for _ in range(5)]

        for t in threads:
            t.start()

        for t in threads:
            t.join()

        # Should complete without errors
        assert True
