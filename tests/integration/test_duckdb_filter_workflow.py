"""
Integration test: VirtualLogManager driving the DuckDB filter engine end to end,
exactly as the GUI does (apply_filter_duckdb -> get_total_entries / get_entries).
"""
import pytest

from opnsense_log_viewer.services.log_filter import LogFilter
from opnsense_log_viewer.services.virtual_log_manager import VirtualLogManager

pytest.importorskip("duckdb")


def _v4(ts, rid, iface, action, protonum, src, dst, sport, dport):
    ptext = {"6": "tcp", "17": "udp", "1": "icmp"}.get(protonum, protonum)
    return (f"{ts}\tInformational\tfilterlog\t "
            f"100,,,{rid},{iface},match,{action},in,4,0x0,,64,1,0,none,"
            f"{protonum},{ptext},60,{src},{dst},{sport},{dport},40")


LINES = [
    _v4("2024-02-01T08:00:00", "r1", "igc0", "block", "6", "10.0.0.1", "8.8.8.8", "1000", "80"),
    _v4("2024-02-01T08:01:00", "r2", "igc1", "pass", "17", "10.0.0.2", "1.1.1.1", "1001", "53"),
    _v4("2024-02-01T08:02:00", "r1", "igc0", "block", "6", "10.0.0.3", "8.8.8.8", "1002", "443"),
    _v4("2024-02-01T08:03:00", "r3", "igc1", "pass", "6", "10.0.0.4", "9.9.9.9", "1003", "443"),
]


@pytest.fixture
def loaded_vlm(tmp_path):
    p = tmp_path / "wf.log"
    p.write_text("\n".join(LINES) + "\n", encoding="utf-8")
    vlm = VirtualLogManager()
    vlm.load_file(str(p))
    yield vlm
    if vlm.duckdb_engine is not None:
        vlm.duckdb_engine.close()


def test_apply_filter_duckdb_sets_count_and_entries(loaded_vlm):
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "block")
    loaded_vlm.apply_filter_duckdb(lf)

    assert loaded_vlm.is_filtered is True
    assert loaded_vlm.duckdb_filtered is True
    assert loaded_vlm.get_total_entries() == 2

    entries = loaded_vlm.get_entries(0, 10)
    assert len(entries) == 2
    assert all(e.get("action") == "block" for e in entries)
    assert {e.get("dst") for e in entries} == {"8.8.8.8"}
    # newest-first
    assert entries[0].timestamp >= entries[1].timestamp


def test_filter_on_dst(loaded_vlm):
    lf = LogFilter()
    lf.add_filter_condition("dst", "==", "8.8.8.8")
    loaded_vlm.apply_filter_duckdb(lf)
    assert loaded_vlm.get_total_entries() == 2


def test_numeric_and_proto_filter(loaded_vlm):
    lf = LogFilter()
    lf.add_filter_condition("dstport", "==", "443")
    lf.add_filter_condition("protoname", "==", "tcp", logic_operator="AND")
    loaded_vlm.apply_filter_duckdb(lf)
    assert loaded_vlm.get_total_entries() == 2


def test_clear_filter_resets_duckdb(loaded_vlm):
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "block")
    loaded_vlm.apply_filter_duckdb(lf)
    assert loaded_vlm.duckdb_filtered is True

    loaded_vlm.clear_filter()
    assert loaded_vlm.duckdb_filtered is False
    assert loaded_vlm.is_filtered is False
    assert loaded_vlm.get_total_entries() == loaded_vlm.total_entries


def test_pagination_through_get_entries(loaded_vlm):
    lf = LogFilter()  # no conditions -> all valid entries
    loaded_vlm.apply_filter_duckdb(lf)
    assert loaded_vlm.get_total_entries() == 4

    first_two = loaded_vlm.get_entries(0, 2)
    next_two = loaded_vlm.get_entries(2, 2)
    assert len(first_two) == 2
    assert len(next_two) == 2
    raws = {e.raw_line for e in first_two} | {e.raw_line for e in next_two}
    assert len(raws) == 4
