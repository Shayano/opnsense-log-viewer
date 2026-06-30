"""
Unit tests for the DuckDB fast filter engine.

Core check: a *semantic cross-check* against the real parser. For a battery of
filter expressions, the set of raw lines the DuckDB engine returns must equal the
set produced by ``OPNsenseLogParser`` + ``FilterExpression.evaluate`` (the source
of truth), including IPv4 *and* IPv6 layouts, the special ``interface`` /
``__label__`` fields, the validity gate (non-filterlog lines are dropped) and the
time-range check.
"""
from collections import Counter
from datetime import datetime

import pytest

from opnsense_log_viewer.services.log_parser import OPNsenseLogParser
from opnsense_log_viewer.services.log_filter import LogFilter

pytest.importorskip("duckdb")
from opnsense_log_viewer.services.duckdb_filter import DuckDBLogFilter  # noqa: E402


INTERFACE_MAPPING = {"igc0": "WAN", "igc2": "LAN", "ovpns3": "VPN"}
LABELS = {"ridA": "CrowdSec block", "ridB": "Allow web"}


def _v4(ts, rid, iface, action, direction, protonum, src, dst, sport, dport):
    ptext = {"6": "tcp", "17": "udp", "1": "icmp"}.get(protonum, protonum)
    return (f"{ts}\tInformational\tfilterlog\t "
            f"100,,,{rid},{iface},match,{action},{direction},4,0x0,,64,1,0,none,"
            f"{protonum},{ptext},60,{src},{dst},{sport},{dport},40")


def _v6(ts, rid, iface, action, direction, protonum, src, dst, sport, dport):
    ptext = {"6": "tcp", "17": "udp", "58": "icmpv6"}.get(protonum, protonum)
    return (f"{ts}\tInformational\tfilterlog\t "
            f"101,,,{rid},{iface},match,{action},{direction},6,0x00,0x12345,64,"
            f"{ptext},{protonum},80,{src},{dst},{sport},{dport},40")


LINES = [
    _v4("2024-01-15T10:00:00", "ridA", "igc0", "block", "in", "6", "10.0.0.1", "8.8.8.8", "1111", "80"),
    _v4("2024-01-15T10:05:00", "ridB", "igc2", "pass", "out", "17", "10.0.0.2", "1.1.1.1", "2222", "443"),
    _v4("2024-01-15T10:10:00", "ridA", "ovpns3", "block", "in", "6", "192.168.1.5", "9.9.9.9", "3333", "22"),
    _v4("2024-01-15T10:15:00", "ridC", "igc0", "pass", "out", "1", "10.0.0.3", "8.8.4.4", "0", "0"),
    _v6("2024-01-15T10:20:00", "ridB", "igc2", "pass", "in", "6", "fe80::1", "2001:4860:4860::8888", "4444", "443"),
    _v6("2024-01-15T10:25:00", "ridC", "ovpns3", "block", "out", "17", "fe80::2", "2606:4700:4700::1111", "5555", "53"),
    _v4("2024-01-15T10:30:00", "ridA", "igc0", "block", "in", "6", "10.0.0.50", "8.8.8.8", "6666", "443"),
    _v6("2024-01-15T10:35:00", "ridB", "igc2", "pass", "in", "6", "fe80::3", "2001:4860:4860::8888", "7777", "443"),
    # noise that must never match (no 'filterlog' / empty action):
    "2024-01-15T10:40:00\tInformational\tsomethingelse\t not a filter line",
    "",
]


@pytest.fixture
def log_file(tmp_path):
    p = tmp_path / "real_format.log"
    p.write_text("\n".join(LINES) + "\n", encoding="utf-8")
    return str(p)


def _reference(log_file, lf, labels, mapping):
    parser = OPNsenseLogParser()
    parser.set_interface_mapping(mapping or {})
    start, end = lf.time_range_start, lf.time_range_end
    out = []
    with open(log_file, "r", encoding="utf-8", errors="ignore") as f:
        for line in f:
            entry = parser.parse_log_line(line.strip())
            if entry is None:
                continue
            entry.parsed_data["__label__"] = (labels or {}).get(entry.get("rid", ""), "")
            if start and (entry.timestamp is None or entry.timestamp < start):
                continue
            if end and (entry.timestamp is None or entry.timestamp > end):
                continue
            if lf.expression.evaluate(entry):
                out.append(line.strip())
    return out


def _engine(log_file, lf, labels, mapping):
    eng = DuckDBLogFilter(log_file)
    try:
        n = eng.build_matches(
            lf.expression, (lf.time_range_start, lf.time_range_end),
            labels or {}, mapping or {})
        rows = eng.fetch_page(0, n)
        return [(e.raw_line or "").strip() for e in rows], n
    finally:
        eng.close()


def _assert_same(log_file, lf, labels=None, mapping=None):
    expected = _reference(log_file, lf, labels, mapping)
    got, count = _engine(log_file, lf, labels, mapping)
    assert Counter(got) == Counter(expected)
    assert count == len(expected)


def _lf(*conditions, time_range=None):
    lf = LogFilter()
    for c in conditions:
        lf.add_filter_condition(*c)
    if time_range:
        lf.set_time_range(*time_range)
    return lf


# (field, op, value[, logic, negate, case_sensitive])
CASES = {
    "eq_action": [("action", "==", "block")],
    "ne_action": [("action", "!=", "block")],
    "eq_dst_v4": [("dst", "==", "8.8.8.8")],
    "eq_dst_v6": [("dst", "==", "2001:4860:4860::8888")],
    "contains_src": [("src", "contains", "10.0.0")],
    "contains_src_v6": [("src", "contains", "fe80")],
    "startswith_dst": [("dst", "startswith", "8.8")],
    "endswith_dstport": [("dstport", "endswith", "43")],
    "regex_src": [("src", "regex", r"^10\.0\.0\.\d+$")],
    "numeric_gt_port": [("dstport", ">", "100")],
    "numeric_le_port": [("dstport", "<=", "53")],
    "proto_tcp": [("protoname", "==", "tcp")],
    "proto_udp": [("protoname", "==", "udp")],
    "proto_icmp": [("protoname", "==", "icmp")],
    "iface_logical": [("interface", "==", "WAN")],
    "iface_physical": [("interface", "==", "igc0")],
    "iface_contains": [("interface", "contains", "LA")],
    "and_combo": [("action", "==", "block"), ("protoname", "==", "tcp", "AND")],
    "or_combo": [("protoname", "==", "udp"), ("action", "==", "pass", "OR")],
    "negate": [("action", "==", "block", "AND", True)],
    # Negated numeric on a gated (port-less) field: the ICMP line has srcport ''
    # -> Python NOT(float('') fails -> False) = True (kept); DuckDB must coalesce the
    # NULL cast to FALSE so NOT() keeps it too, instead of NOT(NULL)=NULL dropping it.
    "negate_numeric_gated": [("srcport", ">=", "1024", "AND", True)],
    "negate_numeric_nonnumeric_value": [("dstport", ">", "abc", "AND", True)],
    "mixed_fold": [("action", "==", "block"), ("action", "==", "pass", "OR"),
                   ("protoname", "==", "tcp", "AND")],
    "case_sensitive_eq": [("action", "==", "BLOCK", "AND", False, True)],
    "case_insensitive_eq": [("action", "==", "BLOCK")],
}


@pytest.mark.parametrize("name", list(CASES))
def test_engine_matches_reference(log_file, name):
    _assert_same(log_file, _lf(*CASES[name]), mapping=INTERFACE_MAPPING)


def test_no_conditions_returns_all_valid_entries(log_file):
    # Empty expression -> every real filterlog entry (the noise/blank line excluded).
    _assert_same(log_file, _lf(), mapping=INTERFACE_MAPPING)
    _, count = _engine(log_file, _lf(), {}, INTERFACE_MAPPING)
    assert count == 8


def test_time_range(log_file):
    lf = _lf(time_range=(datetime(2024, 1, 15, 10, 10, 0),
                         datetime(2024, 1, 15, 10, 25, 0)))
    _assert_same(log_file, lf, mapping=INTERFACE_MAPPING)


def test_time_range_with_condition(log_file):
    lf = _lf(("action", "==", "block"),
             time_range=(datetime(2024, 1, 15, 10, 9, 0),
                         datetime(2024, 1, 15, 10, 31, 0)))
    _assert_same(log_file, lf, mapping=INTERFACE_MAPPING)


def test_label_equals(log_file):
    _assert_same(log_file, _lf(("__label__", "==", "CrowdSec block")),
                 labels=LABELS, mapping=INTERFACE_MAPPING)


def test_label_contains(log_file):
    _assert_same(log_file, _lf(("__label__", "contains", "block")),
                 labels=LABELS, mapping=INTERFACE_MAPPING)


def test_label_empty_matches_undescribed_rids(log_file):
    _assert_same(log_file, _lf(("__label__", "==", "")),
                 labels=LABELS, mapping=INTERFACE_MAPPING)


def test_label_not_equals(log_file):
    _assert_same(log_file, _lf(("__label__", "!=", "CrowdSec block")),
                 labels=LABELS, mapping=INTERFACE_MAPPING)


def test_newest_first_ordering(log_file):
    eng = DuckDBLogFilter(log_file)
    try:
        n = eng.build_matches(LogFilter().expression)
        rows = eng.fetch_page(0, n)
        ts = [e.timestamp for e in rows if e.timestamp]
        assert ts == sorted(ts, reverse=True)
    finally:
        eng.close()


def test_pagination_is_stable_and_disjoint(log_file):
    eng = DuckDBLogFilter(log_file)
    try:
        n = eng.build_matches(LogFilter().expression)
        page1 = [e.raw_line for e in eng.fetch_page(0, 3)]
        page2 = [e.raw_line for e in eng.fetch_page(3, 3)]
        allrows = [e.raw_line for e in eng.fetch_page(0, n)]
        assert page1 == allrows[0:3]
        assert page2 == allrows[3:6]
        assert len(set(page1) & set(page2)) == 0 or len(page1) < 3
    finally:
        eng.close()
