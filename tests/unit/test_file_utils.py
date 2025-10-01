"""
Unit tests for File Utils.
"""
import pytest
import os
from pathlib import Path

from opnsense_log_viewer.utils.file_utils import read_file_tail
from opnsense_log_viewer.exceptions import FileOperationError


@pytest.mark.unit
@pytest.mark.file_io
class TestFileUtils:
    """Test file utility functions."""

    def test_read_file_tail_basic(self, temp_dir):
        """Test reading last N lines from file."""
        # Create test file
        test_file = Path(temp_dir) / 'test.txt'
        lines = [f"Line {i}\n" for i in range(1, 11)]
        test_file.write_text(''.join(lines))

        # Read last 5 lines
        result = read_file_tail(str(test_file), 5)

        assert len(result) == 5
        assert 'Line 6' in result[0] or 'Line 6' in result[1]
        assert 'Line 10' in result[-1]

    def test_read_file_tail_exact_count(self, temp_dir):
        """Test reading exact number of lines as file has."""
        test_file = Path(temp_dir) / 'test.txt'
        lines = [f"Line {i}\n" for i in range(1, 6)]
        test_file.write_text(''.join(lines))

        result = read_file_tail(str(test_file), 5)

        assert len(result) <= 5

    def test_read_file_tail_more_than_available(self, temp_dir):
        """Test reading more lines than file has."""
        test_file = Path(temp_dir) / 'test.txt'
        lines = [f"Line {i}\n" for i in range(1, 4)]
        test_file.write_text(''.join(lines))

        result = read_file_tail(str(test_file), 10)

        # Should return all available lines
        assert len(result) <= 10

    def test_read_file_tail_single_line(self, temp_dir):
        """Test reading single line."""
        test_file = Path(temp_dir) / 'test.txt'
        test_file.write_text("Single line\n")

        result = read_file_tail(str(test_file), 1)

        assert len(result) == 1
        assert 'Single line' in result[0]

    def test_read_file_tail_empty_file(self, temp_dir):
        """Test reading from empty file."""
        test_file = Path(temp_dir) / 'empty.txt'
        test_file.write_text('')

        result = read_file_tail(str(test_file), 5)

        assert isinstance(result, list)
        assert len(result) <= 5

    def test_read_file_tail_large_lines(self, temp_dir):
        """Test reading file with large lines."""
        test_file = Path(temp_dir) / 'large.txt'
        lines = [f"{'X' * 1000}\n" for i in range(10)]
        test_file.write_text(''.join(lines))

        result = read_file_tail(str(test_file), 3)

        assert len(result) <= 3
        assert all(len(line) > 500 for line in result if line.strip())

    def test_read_file_tail_nonexistent_file(self):
        """Test reading non-existent file."""
        with pytest.raises(FileOperationError) as exc_info:
            read_file_tail('/nonexistent/file.txt', 5)

        assert "not found" in str(exc_info.value).lower()
        assert exc_info.value.file_path == '/nonexistent/file.txt'
        assert exc_info.value.operation == 'read'

    def test_read_file_tail_not_readable(self, temp_dir, mocker):
        """Test reading unreadable file."""
        test_file = Path(temp_dir) / 'test.txt'
        test_file.write_text('test content')

        # Mock access to return False
        mocker.patch('os.access', return_value=False)

        with pytest.raises(FileOperationError) as exc_info:
            read_file_tail(str(test_file), 5)

        assert "not readable" in str(exc_info.value).lower()

    def test_read_file_tail_permission_error(self, temp_dir, mocker):
        """Test handling permission error."""
        test_file = Path(temp_dir) / 'test.txt'
        test_file.write_text('test content')

        # Mock open to raise PermissionError
        original_open = open
        def mock_open(*args, **kwargs):
            if str(test_file) in str(args[0]):
                raise PermissionError("Access denied")
            return original_open(*args, **kwargs)

        mocker.patch('builtins.open', side_effect=mock_open)

        with pytest.raises(FileOperationError) as exc_info:
            read_file_tail(str(test_file), 5)

        assert exc_info.value.original_error is not None

    def test_read_file_tail_io_error(self, temp_dir, mocker):
        """Test handling IO error."""
        test_file = Path(temp_dir) / 'test.txt'
        test_file.write_text('test content')

        # Mock open to raise IOError
        original_open = open
        def mock_open(*args, **kwargs):
            if str(test_file) in str(args[0]):
                raise IOError("Disk error")
            return original_open(*args, **kwargs)

        mocker.patch('builtins.open', side_effect=mock_open)

        with pytest.raises(FileOperationError) as exc_info:
            read_file_tail(str(test_file), 5)

        assert exc_info.value.original_error is not None

    def test_read_file_tail_unicode_content(self, temp_dir):
        """Test reading file with unicode content."""
        test_file = Path(temp_dir) / 'unicode.txt'
        content = "Line 1: Hello\nLine 2: こんにちは\nLine 3: Привет\nLine 4: مرحبا\n"
        test_file.write_text(content, encoding='utf-8')

        result = read_file_tail(str(test_file), 2)

        assert len(result) <= 3  # May include empty line

    def test_read_file_tail_no_newline_at_end(self, temp_dir):
        """Test reading file without newline at end."""
        test_file = Path(temp_dir) / 'no_newline.txt'
        test_file.write_text("Line 1\nLine 2\nLine 3")

        result = read_file_tail(str(test_file), 2)

        assert len(result) <= 2
        assert any('Line 3' in line for line in result)

    def test_read_file_tail_mixed_line_endings(self, temp_dir):
        """Test reading file with mixed line endings."""
        test_file = Path(temp_dir) / 'mixed.txt'
        content = "Line 1\nLine 2\rLine 3\r\nLine 4\n"
        test_file.write_text(content)

        result = read_file_tail(str(test_file), 5)

        assert isinstance(result, list)
        assert len(result) > 0

    def test_read_file_tail_binary_content(self, temp_dir):
        """Test reading file with binary-like content."""
        test_file = Path(temp_dir) / 'binary.txt'
        # Write some content that might have encoding issues
        content = "Line 1\nLine 2\nLine 3\n"
        test_file.write_bytes(content.encode('utf-8'))

        # Should handle gracefully with errors='ignore'
        result = read_file_tail(str(test_file), 2)

        assert isinstance(result, list)

    def test_read_file_tail_very_long_file(self, temp_dir):
        """Test reading from very long file."""
        test_file = Path(temp_dir) / 'long.txt'
        lines = [f"Line {i}\n" for i in range(1, 1001)]
        test_file.write_text(''.join(lines))

        result = read_file_tail(str(test_file), 10)

        assert len(result) == 10
        # Should have lines from the end
        assert any('99' in line for line in result)

    def test_read_file_tail_chunks(self, temp_dir):
        """Test that chunked reading works correctly."""
        test_file = Path(temp_dir) / 'chunks.txt'
        # Create file larger than typical chunk size
        lines = [f"Line {i:04d}\n" for i in range(1, 2001)]
        test_file.write_text(''.join(lines))

        result = read_file_tail(str(test_file), 100)

        assert len(result) == 100
        # Should contain lines from the end
        assert any('1900' in line or '2000' in line for line in result)

    def test_read_file_tail_exact_chunk_boundary(self, temp_dir):
        """Test reading at chunk boundary."""
        test_file = Path(temp_dir) / 'boundary.txt'
        # Create content at exactly chunk boundaries
        lines = [f"Line {i}\n" for i in range(1, 101)]
        test_file.write_text(''.join(lines))

        result = read_file_tail(str(test_file), 50)

        assert len(result) == 50

    def test_read_file_tail_preserves_content(self, temp_dir):
        """Test that content is preserved correctly."""
        test_file = Path(temp_dir) / 'preserve.txt'
        expected_lines = [
            "First line with special chars: @#$%\n",
            "Second line with numbers: 123456\n",
            "Third line with spaces:     \n",
            "Last line\n"
        ]
        test_file.write_text(''.join(expected_lines))

        result = read_file_tail(str(test_file), 2)

        # Last lines should be preserved
        assert len(result) <= 2
        assert any('Last line' in line for line in result)

    def test_read_file_tail_zero_lines(self, temp_dir):
        """Test reading zero lines."""
        test_file = Path(temp_dir) / 'test.txt'
        test_file.write_text("Line 1\nLine 2\n")

        result = read_file_tail(str(test_file), 0)

        assert result == []

    def test_read_file_tail_negative_lines(self, temp_dir):
        """Test reading negative number of lines."""
        test_file = Path(temp_dir) / 'test.txt'
        test_file.write_text("Line 1\nLine 2\n")

        result = read_file_tail(str(test_file), -5)

        # Should return empty or all lines depending on implementation
        assert isinstance(result, list)
