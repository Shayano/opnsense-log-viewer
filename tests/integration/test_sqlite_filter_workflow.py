"""
Integration test: VirtualLogManager filtering through the SQLite index.

Validates the full surgical path: load_file -> apply_filter_sql (SQLite) ->
filtered_indices (line numbers) -> get_entries() maps them back to LogEntry
objects via the existing chunk/display path.
"""
from opnsense_log_viewer.services.virtual_log_manager import VirtualLogManager
from opnsense_log_viewer.services.sqlite_index import SQLiteLogIndex
from opnsense_log_viewer.services.log_filter import LogFilter


def _line(ts, iface, action, protonum, src, dst, sport, dport):
    return (
        f"{ts} opnsense filterlog: 100,200,anchor,r1,{iface},match,{action},in,4,"
        f"0x0,,64,1,0,none,{protonum},x,60,{src},{dst},{sport},{dport},40,S,1,0,0,"
    )


LINES = [
    _line("2024-01-15T10:00:00", "em0", "block", "6", "10.0.0.1", "8.8.8.8", "1", "80"),
    _line("2024-01-15T10:01:00", "em1", "pass", "17", "10.0.0.2", "1.1.1.1", "2", "443"),
    _line("2024-01-15T10:02:00", "em0", "block", "6", "10.0.0.3", "9.9.9.9", "3", "22"),
    _line("2024-01-15T10:03:00", "em1", "pass", "6", "10.0.0.4", "1.1.1.1", "4", "8080"),
    _line("2024-01-15T10:04:00", "em0", "block", "1", "10.0.0.5", "8.8.4.4", "0", "0"),
]


def _make_vlm(tmp_path):
    log = tmp_path / "logs.log"
    log.write_text("\n".join(LINES) + "\n", encoding="utf-8")
    vlm = VirtualLogManager(chunk_size=2, cache_size=10)
    vlm.load_file(str(log))
    # Isolate the on-disk index cache to the test's tmp dir.
    vlm.sqlite_index = SQLiteLogIndex(cache_dir=str(tmp_path / "cache"))
    return vlm, str(log)


def test_filter_sql_sets_line_numbers_and_displays_entries(tmp_path):
    vlm, _ = _make_vlm(tmp_path)
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "block")

    vlm.apply_filter_sql(lf)

    assert vlm.is_filtered is True
    assert vlm.filtered_indices == [0, 2, 4]
    assert vlm.get_total_entries() == 3

    entries = vlm.get_entries(0, 100)
    assert len(entries) == 3
    assert all(e["action"] == "block" for e in entries)
    assert [e["src"] for e in entries] == ["10.0.0.1", "10.0.0.3", "10.0.0.5"]


def test_clear_filter_restores_full_view(tmp_path):
    vlm, _ = _make_vlm(tmp_path)
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "block")
    vlm.apply_filter_sql(lf)
    assert vlm.get_total_entries() == 3

    vlm.clear_filter()
    assert vlm.is_filtered is False
    assert vlm.get_total_entries() == len(LINES)


def test_interface_logical_name_filter(tmp_path):
    vlm, _ = _make_vlm(tmp_path)
    vlm.log_parser.set_interface_mapping({"em0": "LAN", "em1": "WAN"})
    lf = LogFilter()
    lf.add_filter_condition("interface", "==", "WAN")

    vlm.apply_filter_sql(lf)
    # em1 rows are line 1 and 3.
    assert vlm.filtered_indices == [1, 3]
    entries = vlm.get_entries(0, 100)
    assert all(e["interface"] == "em1" for e in entries)


def test_second_filter_does_not_rebuild_index(tmp_path):
    vlm, _ = _make_vlm(tmp_path)
    lf1 = LogFilter()
    lf1.add_filter_condition("action", "==", "block")
    vlm.apply_filter_sql(lf1)
    db_path = vlm.sqlite_index.db_path

    lf2 = LogFilter()
    lf2.add_filter_condition("protoname", "==", "udp")
    vlm.apply_filter_sql(lf2)
    # Same opened database, just a new query.
    assert vlm.sqlite_index.db_path == db_path
    assert vlm.filtered_indices == [1]
