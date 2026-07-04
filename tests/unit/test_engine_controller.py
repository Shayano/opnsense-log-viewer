"""
Unit tests for the engine lifecycle controller and its cancellation token.
"""
import threading

import pytest

from opnsense_log_viewer.services.engine_controller import (
    CancellationToken,
    EngineController,
)

pytest.importorskip("duckdb")


LINE = ("2024-01-15T10:00:00\tInformational\tfilterlog\t "
        "100,,,ridA,igc0,match,block,in,4,0x0,,64,1,0,none,"
        "6,tcp,60,10.0.0.1,8.8.8.8,1111,80,40")


@pytest.fixture
def log_file(tmp_path):
    p = tmp_path / "ctl.log"
    p.write_text((LINE + "\n") * 5, encoding="utf-8")
    return str(p)


@pytest.fixture
def cache_dir(tmp_path):
    return str(tmp_path / "pq_cache")


class TestCancellationToken:
    def test_starts_clean_and_flips_once(self):
        token = CancellationToken()
        assert not token.cancelled
        token.cancel()
        assert token.cancelled

    def test_wait_returns_immediately_when_cancelled(self):
        token = CancellationToken()
        token.cancel()
        assert token.wait(10) is True

    def test_wait_wakes_early_on_cancel(self):
        token = CancellationToken()
        threading.Timer(0.05, token.cancel).start()
        assert token.wait(10) is True

    def test_wait_times_out_uncancelled(self):
        token = CancellationToken()
        assert token.wait(0.01) is False


class TestEngineController:
    def test_no_engine_is_unavailable(self, cache_dir):
        ctl = EngineController(cache_dir=cache_dir)
        assert ctl.engine is None
        assert ctl.wait_cache_ready() == 'unavailable'
        ctl.close()  # idempotent on nothing

    def test_ensure_creates_once(self, log_file, cache_dir):
        ctl = EngineController(cache_dir=cache_dir)
        try:
            eng1 = ctl.ensure(log_file)
            eng2 = ctl.ensure(log_file)
            assert eng1 is not None
            assert eng1 is eng2
            assert ctl.engine is eng1
        finally:
            ctl.close()

    def test_open_replaces_and_closes_previous(self, log_file, cache_dir):
        ctl = EngineController(cache_dir=cache_dir)
        try:
            eng1 = ctl.open(log_file)
            eng2 = ctl.open(log_file)
            assert eng2 is not eng1
            assert eng1.is_closed
            assert not eng2.is_closed
        finally:
            ctl.close()

    def test_wait_until_ready_reports_progress(self, log_file, cache_dir):
        ctl = EngineController(cache_dir=cache_dir)
        try:
            eng = ctl.open(log_file)
            state = ctl.wait_cache_ready(poll_interval=0.05)
            assert state == 'ready'
            assert eng.cache_ready
        finally:
            ctl.close()

    def test_precancelled_token_wins_over_ready(self, log_file, cache_dir):
        ctl = EngineController(cache_dir=cache_dir)
        try:
            ctl.open(log_file)
            token = CancellationToken()
            token.cancel()
            assert ctl.wait_cache_ready(token=token) == 'cancelled'
        finally:
            ctl.close()

    def test_wait_on_closed_engine_is_closed(self, log_file, cache_dir):
        ctl = EngineController(cache_dir=cache_dir)
        eng = ctl.open(log_file)
        ctl.close()
        # The stale operation pinned the old engine: it must see 'closed',
        # not silently wait on (or create) a new one.
        assert ctl.wait_cache_ready(engine=eng) == 'closed'

    def test_wait_reports_failed_build(self, log_file, tmp_path):
        # cache_dir pointing at an existing FILE makes the conversion fail.
        bogus = tmp_path / "not_a_dir"
        bogus.write_text("x", encoding="utf-8")
        ctl = EngineController(cache_dir=str(bogus))
        try:
            ctl.open(log_file)
            state = ctl.wait_cache_ready(poll_interval=0.05)
            assert state == 'failed'
        finally:
            ctl.close()

    def test_close_is_idempotent(self, log_file, cache_dir):
        ctl = EngineController(cache_dir=cache_dir)
        ctl.open(log_file)
        ctl.close()
        ctl.close()
        assert ctl.engine is None
