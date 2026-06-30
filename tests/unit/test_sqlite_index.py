"""
Unit tests for the persistent SQLite log index.

The core of these tests is a *semantic cross-check*: for a battery of filter
expressions, the line numbers returned by the SQL engine must equal the line
numbers produced by a straightforward Python reference that mirrors
FilterExpression.evaluate (correct left-to-right fold), the special 'interface'
and '__label__' fields, and the leading time-range check.
"""
import os
from datetime import datetime

import pytest

from opnsense_log_viewer.services.log_parser import OPNsenseLogParser
from opnsense_log_viewer.services.log_filter import LogFilter
from opnsense_log_viewer.services.sqlite_index import SQLiteLogIndex


# --- sample data -----------------------------------------------------------
INTERFACE_MAPPING = {"em0": "LAN", "em1": "WAN", "vtnet0": "DMZ"}
LABEL_DESCRIPTIONS = {
    "hashA": "CrowdSec (IPv4) block",
    "hashB": "Allow web traffic",
}


def _line(ts, rid, iface, action, protonum, src, dst, sport, dport):
    return (
        f"{ts} opnsense filterlog: 100,200,anchor,{rid},{iface},match,{action},"
        f"in,4,0x0,,64,1,0,none,{protonum},x,60,{src},{dst},{sport},{dport},"
        f"40,S,1,0,0,"
    )


SAMPLE_LINES = [
    _line("2024-01-15T10:00:00", "hashA", "em0", "block", "6", "10.0.0.1", "8.8.8.8", "1111", "80"),
    _line("2024-01-15T10:05:00", "hashB", "em1", "pass", "17", "10.0.0.2", "1.1.1.1", "2222", "443"),
    _line("2024-01-15T10:10:00", "hashA", "vtnet0", "block", "6", "192.168.1.5", "9.9.9.9", "3333", "22"),
    _line("2024-01-15T10:15:00", "other", "em0", "pass", "1", "10.0.0.3", "8.8.4.4", "0", "0"),
    _line("2024-01-15T10:20:00", "hashB", "em1", "pass", "6", "172.16.0.9", "1.1.1.1", "4444", "8080"),
    _line("2024-01-15T10:25:00", "other", "vtnet0", "block", "17", "10.0.0.1", "208.67.222.222", "5555", "53"),
    _line("2024-01-15T10:30:00", "hashA", "em0", "block", "6", "10.0.0.50", "8.8.8.8", "6666", "443"),
    _line("2024-01-15T10:35:00", "hashB", "em1", "pass", "6", "10.0.0.2", "140.82.112.3", "7777", "443"),
]


@pytest.fixture
def log_file(tmp_path):
    p = tmp_path / "sample.log"
    p.write_text("\n".join(SAMPLE_LINES) + "\n", encoding="utf-8")
    return str(p)


@pytest.fixture
def index(tmp_path, log_file):
    cache_dir = str(tmp_path / "cache")
    idx = SQLiteLogIndex(cache_dir=cache_dir)
    idx.build(log_file, OPNsenseLogParser())
    yield idx
    idx.close()


def _reference(log_file, expression, time_range=(None, None),
               label_descriptions=None, interface_mapping=None):
    """Python reference: correct left-fold + special fields + time range."""
    parser = OPNsenseLogParser()
    parser.set_interface_mapping(interface_mapping or {})
    start, end = time_range
    out = []
    with open(log_file, "r", encoding="utf-8", errors="ignore") as f:
        for line_no, line in enumerate(f):
            entry = parser.parse_log_line(line.strip())
            if entry is None:
                continue
            rid = entry.get("rid", "")
            # FilterCondition.evaluate handles '__label__' via entry.get('__label__')
            entry.parsed_data["__label__"] = (
                (label_descriptions or {}).get(rid, ""))
            if start and entry.timestamp < start:
                continue
            if end and entry.timestamp > end:
                continue
            if expression.evaluate(entry):
                out.append(line_no)
    return out


def _assert_same(index, log_file, lf, time_range=(None, None),
                 labels=None, mapping=None):
    expected = _reference(log_file, lf.expression, time_range, labels, mapping)
    got = index.query_line_numbers(lf.expression, time_range, labels, mapping)
    assert got == expected
    assert index.count(lf.expression, time_range, labels, mapping) == len(expected)


# --- tests -----------------------------------------------------------------
def test_build_creates_cache_and_rows(index, log_file):
    assert index.conn is not None
    total = index.conn.execute("SELECT COUNT(*) FROM entries").fetchone()[0]
    assert total == len(SAMPLE_LINES)


def test_no_filter_returns_all_lines(index, log_file):
    lf = LogFilter()
    _assert_same(index, log_file, lf)
    assert index.query_line_numbers(lf.expression) == list(range(len(SAMPLE_LINES)))


def test_equals_action(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "block")
    _assert_same(index, log_file, lf)


def test_equals_is_case_insensitive_by_default(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "BLOCK")
    _assert_same(index, log_file, lf)
    assert len(index.query_line_numbers(lf.expression)) == 4


def test_not_equals(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("protoname", "!=", "tcp")
    _assert_same(index, log_file, lf)


def test_contains_src(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("src", "contains", "10.0.0")
    _assert_same(index, log_file, lf)


def test_startswith_and_endswith(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("dst", "startswith", "8.8")
    _assert_same(index, log_file, lf)

    lf2 = LogFilter()
    lf2.add_filter_condition("dstport", "endswith", "43")
    _assert_same(index, log_file, lf2)


def test_regex(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("src", "regex", r"^10\.0\.0\.\d+$")
    _assert_same(index, log_file, lf)


def test_like_special_chars_are_escaped(index, log_file):
    # '%' must be treated literally, not as a LIKE wildcard.
    lf = LogFilter()
    lf.add_filter_condition("src", "contains", "0%0")
    _assert_same(index, log_file, lf)
    assert index.query_line_numbers(lf.expression) == []


def test_interface_matches_physical_and_logical(index, log_file):
    # 'LAN' is the logical name for physical em0.
    lf = LogFilter()
    lf.add_filter_condition("interface", "==", "LAN")
    _assert_same(index, log_file, lf, mapping=INTERFACE_MAPPING)

    lf2 = LogFilter()
    lf2.add_filter_condition("interface", "==", "em0")
    _assert_same(index, log_file, lf2, mapping=INTERFACE_MAPPING)


def test_label_filter(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("__label__", "contains", "CrowdSec")
    _assert_same(index, log_file, lf, labels=LABEL_DESCRIPTIONS)


def test_label_filter_empty_description(index, log_file):
    # rid 'other' has no description -> label '' ; '== ""' should match those.
    lf = LogFilter()
    lf.add_filter_condition("__label__", "==", "")
    _assert_same(index, log_file, lf, labels=LABEL_DESCRIPTIONS)


def test_and_combination(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "block")
    lf.add_filter_condition("protoname", "==", "tcp", logic_operator="AND")
    _assert_same(index, log_file, lf)


def test_or_combination(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("protoname", "==", "udp")
    lf.add_filter_condition("action", "==", "pass", logic_operator="OR")
    _assert_same(index, log_file, lf)


def test_negation(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "block", negate=True)
    _assert_same(index, log_file, lf)


def test_mixed_and_or_left_fold(index, log_file):
    # ((action==block OR action==pass) AND protoname==tcp) by left fold.
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "block")
    lf.add_filter_condition("action", "==", "pass", logic_operator="OR")
    lf.add_filter_condition("protoname", "==", "tcp", logic_operator="AND")
    _assert_same(index, log_file, lf)


def test_time_range(index, log_file):
    lf = LogFilter()
    start = datetime(2024, 1, 15, 10, 10, 0)
    end = datetime(2024, 1, 15, 10, 25, 0)
    _assert_same(index, log_file, lf, time_range=(start, end))


def test_time_range_with_filter(index, log_file):
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "block")
    start = datetime(2024, 1, 15, 10, 9, 0)
    end = datetime(2024, 1, 15, 10, 31, 0)
    _assert_same(index, log_file, lf, time_range=(start, end))


def test_cache_is_reused(tmp_path, log_file):
    cache_dir = str(tmp_path / "cache")
    idx1 = SQLiteLogIndex(cache_dir=cache_dir)
    idx1.build(log_file, OPNsenseLogParser())
    db_path = idx1.db_path
    idx1.close()
    assert db_path and os.path.exists(db_path)

    idx2 = SQLiteLogIndex(cache_dir=cache_dir)
    key = idx2.fingerprint(log_file)
    assert idx2._is_valid_cache(idx2._db_path_for(key))
    # Building again reuses the same database file.
    idx2.build(log_file, OPNsenseLogParser())
    assert idx2.db_path == db_path
    lf = LogFilter()
    lf.add_filter_condition("action", "==", "block")
    assert idx2.query_line_numbers(lf.expression) == [0, 2, 5, 6]
    idx2.close()


def test_rebuild_on_content_change(tmp_path):
    cache_dir = str(tmp_path / "cache")
    log = tmp_path / "changing.log"
    log.write_text(SAMPLE_LINES[0] + "\n", encoding="utf-8")
    idx = SQLiteLogIndex(cache_dir=cache_dir)
    idx.build(str(log), OPNsenseLogParser())
    assert idx.conn.execute("SELECT COUNT(*) FROM entries").fetchone()[0] == 1
    idx.close()

    # Change the file: fingerprint must change and the index rebuild.
    log.write_text("\n".join(SAMPLE_LINES[:3]) + "\n", encoding="utf-8")
    idx2 = SQLiteLogIndex(cache_dir=cache_dir)
    idx2.build(str(log), OPNsenseLogParser())
    assert idx2.conn.execute("SELECT COUNT(*) FROM entries").fetchone()[0] == 3
    idx2.close()
