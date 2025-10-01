"""
Utility functions for file operations with error handling.
"""
import os

from opnsense_log_viewer.exceptions import FileOperationError
from opnsense_log_viewer.utils.logging_config import get_logger, log_exception

logger = get_logger(__name__)


def read_file_tail(file_path, num_lines):
    """
    Read last N lines from file efficiently.

    Args:
        file_path: Path to file
        num_lines: Number of lines to read from end

    Returns:
        List of last N lines

    Raises:
        FileOperationError: If file cannot be read
    """
    # Validate file exists
    if not os.path.exists(file_path):
        error_msg = f"File not found: {file_path}"
        logger.error(error_msg)
        raise FileOperationError(error_msg, file_path=file_path, operation="read")

    if not os.access(file_path, os.R_OK):
        error_msg = f"File is not readable: {file_path}"
        logger.error(error_msg)
        raise FileOperationError(error_msg, file_path=file_path, operation="read")

    logger.debug(f"Reading last {num_lines} lines from {file_path}")

    try:
        with open(file_path, 'rb') as f:
            # Go to end of file
            f.seek(0, 2)  # Seek to end
            file_size = f.tell()

            # Read chunks from end until we have enough lines
            lines = []
            chunk_size = 8192  # 8KB chunks
            pos = file_size

            while len(lines) < num_lines and pos > 0:
                # Calculate chunk position
                chunk_start = max(0, pos - chunk_size)
                chunk_size_actual = pos - chunk_start

                # Read chunk
                f.seek(chunk_start)
                chunk = f.read(chunk_size_actual).decode('utf-8', errors='ignore')

                # Split into lines and prepend to our list
                chunk_lines = chunk.split('\n')
                lines = chunk_lines + lines

                pos = chunk_start

            # Return last num_lines
            return lines[-num_lines:] if len(lines) > num_lines else lines

    except PermissionError as e:
        error_msg = f"Permission denied reading file: {file_path}"
        logger.error(error_msg)
        raise FileOperationError(error_msg, file_path=file_path, operation="read", original_error=e)
    except IOError as e:
        error_msg = f"I/O error reading file: {file_path}"
        log_exception(logger, e, error_msg, file_path=file_path)
        raise FileOperationError(error_msg, file_path=file_path, operation="read", original_error=e)
    except Exception as e:
        error_msg = f"Unexpected error reading file tail: {file_path}"
        log_exception(logger, e, error_msg, file_path=file_path)
        raise FileOperationError(error_msg, file_path=file_path, operation="read", original_error=e)
