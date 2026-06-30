"""
DuckDB-backed filter engine for very large OPNsense filter logs.

Rationale
---------
The previous SQLite "parse-once / query-many" index was correct but its one-time
build on a 14 GB file (~90 M lines) cost ~24 min, dominated by single-threaded
Python parsing and multi-column index construction. Interactive search on that
volume was therefore impossible.

DuckDB reads the raw log with its compiled, multi-threaded CSV engine and filters
it in place, with NO persistent build: on a real 14 GB file a single filter scans
in ~17-37 s (measured), well under a 2 min budget, for *every* filter type
(equality, contains, !=, numeric ranges, negation, interface, label, time range).

Design
------
- The file is read as a SINGLE ``line`` column (a separator byte that never occurs
  in the data), so DuckDB never has to tabulate the ragged CSV (IPv4, IPv6 and TCP
  lines all carry a different number of comma fields). Every field is extracted on
  demand with ``split_part(line, ',', n)``.
- The real OPNsense line is ``TS<TAB>severity<TAB>filterlog<TAB> <csv...>``. Read
  with ``delim=','`` the syslog prefix glues onto the first CSV field, so the field
  index ``i`` (0-based, as the Python parser counts them) maps to the 1-based
  ``split_part`` position ``i + 1``.
- ``src``/``dst``/ports/``proto``/``length`` sit at different positions for IPv4 vs
  IPv6, so each is a ``CASE WHEN ipversion='6' THEN <v6> ELSE <v4> END`` exactly
  mirroring ``OPNsenseLogParser._parse_fields``. ``protoname`` is derived from the
  numeric proto exactly like the parser's ``proto_map``.
- A filter materializes only the matching rows into a ``matches`` table (raw line +
  already-resolved display fields, computed in the one scan), ordered newest-first
  like the unfiltered view, with a 0-based ``rn`` for cheap range paging.
- The translation of ``FilterCondition`` / ``FilterExpression`` reproduces
  ``FilterExpression.evaluate`` exactly: left-to-right fold (no precedence),
  per-condition negation, the special ``interface`` (physical OR logical name) and
  ``__label__`` (rule description) fields, and the leading time-range check.
"""
import re
import threading
from datetime import datetime
from typing import Dict, List, Optional, Tuple

from opnsense_log_viewer.services.log_parser import LogEntry
from opnsense_log_viewer.utils.logging_config import get_logger

try:
    import duckdb
except ImportError:  # pragma: no cover - exercised only when dep is missing
    duckdb = None

logger = get_logger(__name__)

# A byte that never occurs in a text log -> the whole line becomes one CSV column.
_SEP = "\x01"


def _sp(n: int) -> str:
    """1-based split_part on the single ``line`` column (comma-separated fields)."""
    return f"split_part(line, ',', {n})"


def _case6(v6_pos: int, v4_pos: int) -> str:
    """Pick an IPv6 vs IPv4 field position by ipversion; '' for anything else.

    The parser only fills these version-dependent fields when ipversion is exactly
    '4' or '6'; a malformed ipversion leaves them unset. Falling through to the v4
    positions would wrongly surface garbage, so return '' for non-4/6.
    """
    return (f"CASE {_sp(9)} WHEN '6' THEN {_sp(v6_pos)} "
            f"WHEN '4' THEN {_sp(v4_pos)} ELSE '' END")


def _field_exprs() -> Dict[str, str]:
    """field name -> SQL expression over the single ``line`` column.

    Positions are 1-based split_part indices = (0-based parser field index + 1).
    Common fields: rid=f3, interface=f4, action=f6, dir=f7, ipversion=f8.
    IPv4: protonum=f15 length=f17 src=f18 dst=f19 srcport=f20 dstport=f21.
    IPv6: protonum=f13 length=f14 src=f15 dst=f16 srcport=f17 dstport=f18.
    """
    protonum = _case6(14, 16)  # IPv6 f13 -> sp14 ; IPv4 f15 -> sp16
    protoname = (
        f"CASE {protonum} WHEN '6' THEN 'tcp' WHEN '17' THEN 'udp' "
        f"WHEN '1' THEN 'icmp' WHEN '58' THEN 'icmpv6' WHEN '112' THEN 'carp' "
        f"ELSE {protonum} END"
    )
    # The parser only populates src/dst ports for TCP (6) and UDP (17); for any
    # other protocol those positions hold different fields, so the parser leaves
    # the port empty. Mirror that exactly, otherwise e.g. 'dstport <= 53' would
    # wrongly match an ICMP line whose raw field there happens to be numeric.
    has_ports = f"{protonum} IN ('6', '17')"
    srcport = f"CASE WHEN {has_ports} THEN {_case6(18, 21)} ELSE '' END"
    dstport = f"CASE WHEN {has_ports} THEN {_case6(19, 22)} ELSE '' END"
    return {
        "action": _sp(7),
        "interface": _sp(5),
        "dir": _sp(8),
        "ipversion": _sp(9),
        "rid": _sp(4),
        "protonum": protonum,
        "protoname": protoname,
        "length": _case6(15, 18),   # IPv6 f14 ; IPv4 f17
        "src": _case6(16, 19),      # IPv6 f15 ; IPv4 f18
        "dst": _case6(17, 20),      # IPv6 f16 ; IPv4 f19
        "srcport": srcport,         # IPv6 f17 ; IPv4 f20 (TCP/UDP only)
        "dstport": dstport,         # IPv6 f18 ; IPv4 f21 (TCP/UDP only)
    }


# Timestamp = first tab-separated piece of the raw line.
_TS_TEXT = "split_part(line, chr(9), 1)"
_TS_TS = f"try_cast({_TS_TEXT} AS TIMESTAMP)"

# Columns materialized into the matches table (besides the raw line and rn).
_MATCH_FIELDS = ("rid", "interface", "action", "dir", "ipversion",
                 "src", "dst", "srcport", "dstport", "protonum", "protoname", "length")


def _sql_str(value: str) -> str:
    """Single-quoted SQL string literal (with quote doubling)."""
    return "'" + str(value).replace("'", "''") + "'"


def _py_match(condition, field_value) -> bool:
    """Mirror of FilterCondition._check_value_match for in-Python resolution.

    Used to resolve the special 'interface' (logical names) and '__label__'
    (rule descriptions) fields against small in-memory mappings, without scanning.
    """
    cs = condition.case_sensitive
    if not cs and isinstance(field_value, str):
        field_value = field_value.lower()
        comp = condition.value.lower()
    else:
        comp = condition.value
    op = condition.operator
    try:
        if op == "==":
            return str(field_value) == comp
        if op == "!=":
            return str(field_value) != comp
        if op == "contains":
            return comp in str(field_value)
        if op == "startswith":
            return str(field_value).startswith(comp)
        if op == "endswith":
            return str(field_value).endswith(comp)
        if op == "regex":
            flags = 0 if cs else re.IGNORECASE
            return bool(re.compile(condition.value, flags).search(str(field_value)))
        if op in (">", "<", ">=", "<="):
            a, b = float(field_value), float(comp)
            return {">": a > b, "<": a < b, ">=": a >= b, "<=": a <= b}[op]
    except Exception:
        return False
    return False


def _scalar_sql(expr: str, op: str, value: str, case_sensitive: bool) -> Tuple[str, list]:
    """SQL for one condition on a single field expression, matching _check_value_match."""
    if op in (">", "<", ">=", "<="):
        # coalesce(..., FALSE) so a non-numeric/empty operand yields FALSE, not NULL,
        # matching Python's _numeric_compare (float() raises -> False). Critically this
        # also makes a negated numeric condition behave like Python: NOT(FALSE)=TRUE,
        # whereas NOT(NULL)=NULL would wrongly drop the row.
        return (f"coalesce(try_cast({expr} AS DOUBLE) {op} try_cast(? AS DOUBLE), FALSE)",
                [value])
    if op == "regex":
        if case_sensitive:
            return (f"regexp_matches({expr}, ?)", [value])
        return (f"regexp_matches({expr}, ?, 'i')", [value])

    if case_sensitive:
        lhs = expr
        rhs = "?"
    else:
        lhs = f"lower({expr})"
        rhs = "lower(?)"

    if op == "==":
        return (f"({lhs} = {rhs})", [value])
    if op == "!=":
        return (f"({lhs} <> {rhs})", [value])
    if op == "contains":
        return (f"contains({lhs}, {rhs})", [value])
    if op == "startswith":
        return (f"starts_with({lhs}, {rhs})", [value])
    if op == "endswith":
        return (f"ends_with({lhs}, {rhs})", [value])
    # Unknown operator: match nothing (FilterCondition returns False).
    return ("FALSE", [])


class DuckDBLogFilter:
    """Filter a large OPNsense log with DuckDB; no persistent index is built."""

    def __init__(self, file_path: str, threads: Optional[int] = None):
        if duckdb is None:
            raise ImportError(
                "duckdb is required for the fast filter engine "
                "(pip install duckdb)")
        self.file_path = file_path
        self.con = duckdb.connect()
        if threads:
            try:
                self.con.execute(f"PRAGMA threads={int(threads)}")
            except Exception:
                pass
        self._exprs = _field_exprs()
        self._rel = self._build_rel(file_path)
        self._lock = threading.Lock()
        self._interface_mapping: Dict[str, str] = {}
        self.match_count = 0

    @staticmethod
    def is_available() -> bool:
        return duckdb is not None

    def _build_rel(self, path: str) -> str:
        p = path.replace("'", "''")
        return (
            f"read_csv('{p}', sep='{_SEP}', header=false, quote='', "
            f"columns={{'line': 'VARCHAR'}}, strict_mode=false, ignore_errors=true)"
        )

    # ----- WHERE translation -------------------------------------------------
    def _condition_sql(self, condition, label_descriptions, interface_mapping
                       ) -> Tuple[str, list]:
        field = condition.field
        if field == "interface":
            return self._interface_sql(condition, interface_mapping)
        if field == "__label__":
            return self._label_sql(condition, label_descriptions)
        expr = self._exprs.get(field)
        if expr is None:
            # Unknown field -> entry.get(field, '') == '' (matches the parser).
            expr = "''"
        return _scalar_sql(expr, condition.operator, condition.value,
                           condition.case_sensitive)

    def _interface_sql(self, condition, interface_mapping) -> Tuple[str, list]:
        """A row matches if the condition matches the physical name OR its logical
        display name. The physical match is direct SQL; the logical match is the
        set of physical names whose mapped display matches (resolved in Python,
        no scan needed)."""
        expr = self._exprs["interface"]
        base_sql, base_params = _scalar_sql(
            expr, condition.operator, condition.value, condition.case_sensitive)
        phys = [p for p, disp in (interface_mapping or {}).items()
                if _py_match(condition, disp)]
        if phys:
            inlist = ", ".join(_sql_str(p) for p in phys)
            return (f"({base_sql} OR ({expr} IN ({inlist})))", base_params)
        return (base_sql, base_params)

    def _label_sql(self, condition, label_descriptions) -> Tuple[str, list]:
        """The rule label of a row is label_descriptions.get(rid, ''). Select the
        rids whose resolved description matches, plus (if the empty label matches)
        every rid that has no description."""
        expr = self._exprs["rid"]
        ld = label_descriptions or {}
        matching = [rid for rid, desc in ld.items() if _py_match(condition, desc)]
        empty_matches = _py_match(condition, "")
        parts = []
        if matching:
            parts.append(f"{expr} IN ({', '.join(_sql_str(r) for r in matching)})")
        if empty_matches:
            if ld:
                allrids = ", ".join(_sql_str(r) for r in ld.keys())
                parts.append(f"{expr} NOT IN ({allrids})")
            else:
                parts.append("TRUE")
        if not parts:
            return ("FALSE", [])
        return ("(" + " OR ".join(parts) + ")", [])

    def _expression_sql(self, expression, label_descriptions, interface_mapping
                        ) -> Tuple[str, list]:
        conditions = getattr(expression, "conditions", [])
        operators = getattr(expression, "operators", [])
        negations = getattr(expression, "negations", [])
        if not conditions:
            return "", []

        params: List = []
        frag0, p0 = self._condition_sql(
            conditions[0], label_descriptions, interface_mapping)
        params.extend(p0)
        if negations and negations[0]:
            frag0 = f"(NOT ({frag0}))"
        combined = f"({frag0})"

        for i in range(1, len(conditions)):
            frag, p = self._condition_sql(
                conditions[i], label_descriptions, interface_mapping)
            params.extend(p)
            if i < len(negations) and negations[i]:
                frag = f"(NOT ({frag}))"
            op = operators[i - 1] if (i - 1) < len(operators) else "AND"
            sql_op = "OR" if str(op).upper() == "OR" else "AND"
            combined = f"({combined} {sql_op} ({frag}))"
        return combined, params

    def _build_where(self, expression, time_range, label_descriptions,
                     interface_mapping) -> Tuple[str, list]:
        clauses: List[str] = []
        params: List = []
        start, end = time_range
        if start is not None:
            clauses.append(f"({_TS_TS} >= ?)")
            params.append(start)
        if end is not None:
            clauses.append(f"({_TS_TS} <= ?)")
            params.append(end)
        expr_sql, expr_params = self._expression_sql(
            expression, label_descriptions, interface_mapping)
        if expr_sql:
            clauses.append(expr_sql)
            params.extend(expr_params)
        return " AND ".join(clauses), params

    # ----- build / query -----------------------------------------------------
    def build_matches(self, expression, time_range=(None, None),
                      label_descriptions: Optional[Dict[str, str]] = None,
                      interface_mapping: Optional[Dict[str, str]] = None,
                      order_desc: bool = True) -> int:
        """Scan the file once and materialize matching rows into ``matches``.

        Returns the number of matches. Subsequent ``fetch_page`` calls are instant.
        """
        self._interface_mapping = dict(interface_mapping or {})
        where, params = self._build_where(
            expression, time_range,
            label_descriptions or {}, interface_mapping or {})

        # Validity gate mirroring OPNsenseLogParser.parse_log_line: a row is a real
        # filterlog entry only if the line contains 'filterlog' and the action field
        # is non-empty. Without this, non-filterlog / blank lines would spuriously
        # match negative conditions (e.g. action != 'block').
        base = f"(contains(line, 'filterlog') AND {self._exprs['action']} <> '')"
        full_where = base if not where else f"{base} AND ({where})"

        order = "DESC" if order_desc else "ASC"
        resolved = ", ".join(
            f"{self._exprs[f]} AS {f}" for f in _MATCH_FIELDS)
        select = (
            f"SELECT line, {_TS_TEXT} AS ts, {resolved}, "
            f"(row_number() OVER (ORDER BY {_TS_TS} {order}) - 1) AS rn "
            f"FROM {self._rel}"
            f" WHERE {full_where}"
        )
        sql = "CREATE OR REPLACE TABLE matches AS " + select
        with self._lock:
            self.con.execute(sql, params)
            self.match_count = self.con.execute(
                "SELECT count(*) FROM matches").fetchone()[0]
        logger.info("DuckDB filter materialized %s matches", self.match_count)
        return self.match_count

    def fetch_page(self, start: int, count: int,
                   interface_mapping: Optional[Dict[str, str]] = None
                   ) -> List[LogEntry]:
        """Return up to ``count`` LogEntry objects starting at offset ``start``."""
        if count <= 0 or start < 0:
            return []
        mapping = interface_mapping if interface_mapping is not None else self._interface_mapping
        cols = "line, ts, " + ", ".join(_MATCH_FIELDS)
        sql = (f"SELECT {cols} FROM matches "
               f"WHERE rn >= ? AND rn < ? ORDER BY rn")
        with self._lock:
            rows = self.con.execute(sql, [start, start + count]).fetchall()
        return [self._row_to_entry(r, mapping) for r in rows]

    def _row_to_entry(self, row, interface_mapping) -> LogEntry:
        # row = (line, ts, rid, interface, action, dir, ipversion,
        #        src, dst, srcport, dstport, protonum, protoname, length)
        line, ts = row[0], row[1]
        (rid, interface, action, direction, ipversion, src, dst,
         srcport, dstport, protonum, protoname, length) = row[2:14]
        entry = LogEntry(line if line is not None else "")
        display = interface_mapping.get(interface, interface) if interface else interface
        entry.parsed_data = {
            "rid": rid or "",
            "interface": interface or "",
            "interface_display": display or "",
            "action": action or "",
            "dir": direction or "",
            "ipversion": ipversion or "",
            "src": src or "",
            "dst": dst or "",
            "srcport": srcport or "",
            "dstport": dstport or "",
            "protonum": protonum or "",
            "protoname": protoname or "",
            "length": length or "",
            "__timestamp__": ts or "",
            "__host__": "opnsense",
        }
        entry.timestamp = self._parse_ts(ts)
        entry.host = "opnsense"
        return entry

    @staticmethod
    def _parse_ts(ts: Optional[str]) -> Optional[datetime]:
        if not ts:
            return None
        try:
            if "T" in ts:
                return datetime.fromisoformat(ts.replace("T", " "))
            return datetime.strptime(ts, "%b %d %H:%M:%S")
        except Exception:
            return None

    def close(self) -> None:
        # Hold the lock so we never close the connection out from under a build/fetch
        # running on another thread.
        with self._lock:
            try:
                if self.con is not None:
                    self.con.close()
            except Exception:
                pass
            self.con = None
