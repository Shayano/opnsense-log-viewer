"""
Unit tests for the raw (unfiltered) view: the lazy SequentialRawView reader
and the VirtualLogManager dispatch around it.

These cover the pre-cache phase deterministically (no DuckDB engine involved):
file-order pages, the moving entry frontier, sparse-offset reuse, and the
manager's totals/exactness reporting.
"""
import pytest

from opnsense_log_viewer.services.log_parser import OPNsenseLogParser
from opnsense_log_viewer.services.virtual_log_manager import (
    SequentialRawView,
    VirtualLogManager,
)


def _v4(i, action="block"):
    return (f"2024-02-01T{8 + i // 3600:02d}:{(i // 60) % 60:02d}:{i % 60:02d}"
            f"\tInformational\tfilterlog\t "
            f"100,,,rid{i},igc0,match,{action},in,4,0x0,,64,1,0,none,"
            f"6,tcp,60,10.0.0.{i % 250},8.8.8.8,1000,{80 + i % 3},40")


NOISE = "2024-02-01T08:00:00\tInformational\tsomethingelse\t not a filter line"


def _write_log(path, valid_count, with_noise=False):
    lines = []
    for i in range(valid_count):
        lines.append(_v4(i))
        if with_noise and i % 3 == 0:
            lines.append(NOISE)
            lines.append("")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return [l for l in lines if "filterlog" in l]


@pytest.fixture
def parser():
    return OPNsenseLogParser()


class TestSequentialRawView:
    def test_pages_in_file_order_skipping_noise(self, tmp_path, parser):
        p = tmp_path / "log.log"
        expected = _write_log(p, 20, with_noise=True)
        view = SequentialRawView(str(p), parser, stride=5)

        got = [e.raw_line for e in view.get_entries(0, 100)]
        assert got == expected
        assert view.is_complete
        assert view.known_entries == 20

    def test_lazy_frontier_grows_with_navigation(self, tmp_path, parser):
        p = tmp_path / "log.log"
        _write_log(p, 250)
        view = SequentialRawView(str(p), parser, stride=50)

        first = view.get_entries(0, 10)
        assert len(first) == 10
        assert view.known_entries == 10
        assert not view.is_complete

        # A deep access extends the scan and serves the right slice.
        deep = view.get_entries(200, 30)
        assert [e.raw_line for e in deep] == [_v4(i) for i in range(200, 230)]
        assert view.known_entries >= 230
        assert not view.is_complete

        # Revisiting an early page is stable.
        again = view.get_entries(0, 10)
        assert [e.raw_line for e in again] == [e.raw_line for e in first]

    def test_offsets_make_revisits_consistent(self, tmp_path, parser):
        p = tmp_path / "log.log"
        _write_log(p, 120, with_noise=True)
        view = SequentialRawView(str(p), parser, stride=10)

        reference = [e.raw_line for e in view.get_entries(0, 120)]
        for start in (0, 15, 37, 90, 110):
            page = [e.raw_line for e in view.get_entries(start, 10)]
            assert page == reference[start:start + 10]

    def test_read_past_end_returns_empty_and_completes(self, tmp_path, parser):
        p = tmp_path / "log.log"
        _write_log(p, 7)
        view = SequentialRawView(str(p), parser, stride=5)

        assert view.get_entries(50, 10) == []
        assert view.is_complete
        assert view.known_entries == 7

    def test_empty_file(self, tmp_path, parser):
        p = tmp_path / "empty.log"
        p.write_text("", encoding="utf-8")
        view = SequentialRawView(str(p), parser)

        assert view.get_entries(0, 10) == []
        assert view.is_complete
        assert view.known_entries == 0


class TestManagerRawViewDispatch:
    @pytest.fixture
    def vlm_no_engine(self, tmp_path, monkeypatch):
        """A manager whose DuckDB engine is unavailable: the raw view must
        carry the whole session on its own."""
        from opnsense_log_viewer.services.duckdb_filter import DuckDBLogFilter
        monkeypatch.setattr(DuckDBLogFilter, "is_available", staticmethod(lambda: False))
        return VirtualLogManager(duckdb_cache_dir=str(tmp_path / "pq_cache"))

    def test_load_and_browse_without_engine(self, tmp_path, vlm_no_engine):
        p = tmp_path / "log.log"
        _write_log(p, 30)
        vlm_no_engine.load_file(str(p))
        assert vlm_no_engine.duckdb_engine is None

        page = vlm_no_engine.get_entries(0, 10)
        assert [e.raw_line for e in page] == [_v4(i) for i in range(10)]
        assert vlm_no_engine.total_entries >= 10

        # Reading to the end makes the total exact.
        assert vlm_no_engine.get_entries(0, 100)
        assert vlm_no_engine.total_is_exact
        assert vlm_no_engine.get_total_entries() == 30

    def test_total_inexact_while_frontier_open(self, tmp_path, vlm_no_engine):
        p = tmp_path / "log.log"
        _write_log(p, 2500)
        vlm_no_engine.load_file(str(p))

        vlm_no_engine.get_entries(0, 10)
        assert not vlm_no_engine.total_is_exact
        assert vlm_no_engine.total_entries < 2500

        assert vlm_no_engine.get_entries(2499, 10)
        vlm_no_engine.get_entries(2500, 10)
        assert vlm_no_engine.total_is_exact
        assert vlm_no_engine.total_entries == 2500

    def test_load_missing_file_raises(self, vlm_no_engine, tmp_path):
        with pytest.raises(OSError):
            vlm_no_engine.load_file(str(tmp_path / "does_not_exist.log"))
