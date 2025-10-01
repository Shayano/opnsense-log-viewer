"""
Utility functions for resource management with error handling.
"""
import os
import sys

from opnsense_log_viewer.exceptions import FileOperationError
from opnsense_log_viewer.utils.logging_config import get_logger

logger = get_logger(__name__)


def get_resource_path(relative_path):
    """
    Get absolute path to resource, works for dev and for PyInstaller.

    Args:
        relative_path: Relative path to resource

    Returns:
        Absolute path to resource

    Raises:
        FileOperationError: If resource path cannot be determined
    """
    try:
        # PyInstaller creates a temp folder and stores path in _MEIPASS
        if hasattr(sys, '_MEIPASS'):
            base_path = sys._MEIPASS
            logger.debug(f"Using PyInstaller base path: {base_path}")
        else:
            base_path = os.path.abspath(".")
            logger.debug(f"Using development base path: {base_path}")

        resource_path = os.path.join(base_path, relative_path)

        # Validate the resource exists
        if not os.path.exists(resource_path):
            logger.warning(f"Resource not found: {resource_path}")

        return resource_path

    except Exception as e:
        logger.error(f"Error determining resource path: {e}")
        raise FileOperationError(f"Could not determine resource path for {relative_path}", original_error=e)
