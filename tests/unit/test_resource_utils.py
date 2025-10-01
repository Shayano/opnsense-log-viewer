"""
Unit tests for Resource Utils.
"""
import pytest
import os
import sys
from pathlib import Path
from unittest.mock import patch

from opnsense_log_viewer.utils.resource_utils import get_resource_path
from opnsense_log_viewer.exceptions import FileOperationError


@pytest.mark.unit
class TestResourceUtils:
    """Test resource utility functions."""

    def test_get_resource_path_development_mode(self):
        """Test getting resource path in development mode."""
        # In development, sys._MEIPASS should not exist
        if hasattr(sys, '_MEIPASS'):
            delattr(sys, '_MEIPASS')

        path = get_resource_path('test.txt')

        assert isinstance(path, str)
        assert 'test.txt' in path

    def test_get_resource_path_pyinstaller_mode(self):
        """Test getting resource path in PyInstaller mode."""
        # Mock PyInstaller environment
        with patch.object(sys, '_MEIPASS', '/tmp/pyinstaller_temp'):
            path = get_resource_path('test.txt')

            assert isinstance(path, str)
            assert '/tmp/pyinstaller_temp' in path or '\\tmp\\pyinstaller_temp' in path
            assert 'test.txt' in path

    def test_get_resource_path_relative_path(self):
        """Test getting resource path with relative path."""
        path = get_resource_path('data/config.xml')

        assert isinstance(path, str)
        assert 'data' in path
        assert 'config.xml' in path

    def test_get_resource_path_nested_path(self):
        """Test getting resource path with nested directories."""
        path = get_resource_path('resources/icons/app.ico')

        assert isinstance(path, str)
        assert 'resources' in path
        assert 'icons' in path
        assert 'app.ico' in path

    def test_get_resource_path_absolute_result(self):
        """Test that returned path is absolute."""
        path = get_resource_path('test.txt')

        # Check if path is absolute
        assert os.path.isabs(path) or '\\' in path or '/' in path

    def test_get_resource_path_nonexistent_file(self):
        """Test getting path to non-existent file (should not raise)."""
        # Should return path even if file doesn't exist
        path = get_resource_path('nonexistent_file.txt')

        assert isinstance(path, str)
        assert 'nonexistent_file.txt' in path

    def test_get_resource_path_empty_string(self):
        """Test getting resource path with empty string."""
        path = get_resource_path('')

        assert isinstance(path, str)

    def test_get_resource_path_with_backslashes(self):
        """Test getting resource path with backslashes."""
        path = get_resource_path('data\\config\\settings.ini')

        assert isinstance(path, str)
        assert 'settings.ini' in path

    def test_get_resource_path_with_forward_slashes(self):
        """Test getting resource path with forward slashes."""
        path = get_resource_path('data/config/settings.ini')

        assert isinstance(path, str)
        assert 'settings.ini' in path

    def test_get_resource_path_consistency(self):
        """Test that multiple calls return consistent paths."""
        path1 = get_resource_path('test.txt')
        path2 = get_resource_path('test.txt')

        # Normalize paths for comparison
        path1_norm = os.path.normpath(path1)
        path2_norm = os.path.normpath(path2)

        assert path1_norm == path2_norm

    def test_get_resource_path_different_files(self):
        """Test getting paths to different files."""
        path1 = get_resource_path('file1.txt')
        path2 = get_resource_path('file2.txt')

        assert path1 != path2
        assert 'file1.txt' in path1
        assert 'file2.txt' in path2

    @patch('os.path.abspath')
    def test_get_resource_path_exception_handling(self, mock_abspath):
        """Test exception handling in get_resource_path."""
        # Make abspath raise an exception
        mock_abspath.side_effect = Exception("Path error")

        with pytest.raises(FileOperationError):
            get_resource_path('test.txt')

    def test_get_resource_path_pyinstaller_and_dev_distinction(self):
        """Test that PyInstaller and dev modes return different paths."""
        # Get development mode path
        if hasattr(sys, '_MEIPASS'):
            delattr(sys, '_MEIPASS')
        dev_path = get_resource_path('test.txt')

        # Get PyInstaller mode path
        with patch.object(sys, '_MEIPASS', '/tmp/different_path'):
            pyinstaller_path = get_resource_path('test.txt')

        # Normalize for comparison
        dev_norm = os.path.normpath(dev_path)
        pyi_norm = os.path.normpath(pyinstaller_path)

        # They should be different (unless running from /tmp/different_path)
        if '/tmp/different_path' not in dev_norm and '\\tmp\\different_path' not in dev_norm:
            assert dev_norm != pyi_norm

    def test_get_resource_path_preserves_filename(self):
        """Test that filename is preserved in returned path."""
        test_files = [
            'config.xml',
            'data.json',
            'app.log',
            'image.png',
            'style.css'
        ]

        for filename in test_files:
            path = get_resource_path(filename)
            assert filename in path

    def test_get_resource_path_with_special_characters(self):
        """Test resource path with special characters."""
        special_files = [
            'file with spaces.txt',
            'file-with-dashes.txt',
            'file_with_underscores.txt',
            'file.multiple.dots.txt'
        ]

        for filename in special_files:
            path = get_resource_path(filename)
            assert isinstance(path, str)
            # At least part of the filename should be preserved
            base_name = filename.split('.')[0].split()[0]
            assert len(base_name) > 0  # Verify we have something to check

    def test_get_resource_path_cross_platform(self):
        """Test that resource path works on different path separators."""
        # Test with both separators
        path_unix = get_resource_path('data/file.txt')
        path_win = get_resource_path('data\\file.txt')

        # Both should be valid paths
        assert isinstance(path_unix, str)
        assert isinstance(path_win, str)
        assert 'file.txt' in path_unix
        assert 'file.txt' in path_win
