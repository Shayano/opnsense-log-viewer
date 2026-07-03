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

Parquet cache
-------------
Direct scan makes EVERY filter re-read the whole file (~17-37 s on 14 GB). A
one-time background conversion therefore writes the valid rows (raw line, typed
timestamp, resolved fields) to a ZSTD Parquet cache; once it is in place every
filter reads the cache instead of the raw file and drops to ~1-4 s (measured on
the same 14 GB / 72 M-row file, cache 1.6 GB built in ~84 s). Until the cache is
ready (or if the conversion fails) filters keep using the direct scan, so the
cache is a pure accelerator, never a prerequisite. Cache files are keyed by
(absolute path, size, mtime) so any file change invalidates them, are written
atomically (tmp + os.replace), and old ones are pruned by age and total size.
"""
import hashlib
import os
import re
import tempfile
import threading
import time
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

# ---- Parquet cache tuning ----------------------------------------------------
# Default row groups (122880 rows) keep zone-map/bloom-filter pruning granular;
# DICTIONARY_SIZE_LIMIT is raised so high-cardinality columns (src/dst IPs) stay
# dictionary-encoded and therefore get bloom filters (DuckDB only writes bloom
# filters for dictionary-encoded columns), which makes equality filters on a
# value that is absent from a row group nearly free.
_PARQUET_OPTS = ("FORMAT PARQUET, COMPRESSION ZSTD, "
                 "ROW_GROUP_SIZE 122880, DICTIONARY_SIZE_LIMIT 100000")
# The conversion streams, but cap its memory so building the cache never starves
# the UI process on smaller machines.
_CACHE_BUILD_MEMORY_LIMIT = "4GB"
_CACHE_MAX_TOTAL_BYTES = 20 * 1024 ** 3   # prune oldest caches beyond ~20 GB total
_CACHE_MAX_AGE_S = 14 * 86400             # prune caches not used for 14 days
_CACHE_TMP_MAX_AGE_S = 86400              # leftover tmp from a dead build


def _default_cache_dir() -> str:
    base = os.environ.get("LOCALAPPDATA") or tempfile.gettempdir()
    return os.path.join(base, "OPNsenseLogViewer", "parquet_cache")


# Bump when the parquet layout changes (columns, encoding of fields...): old
# caches then simply miss and get rebuilt, instead of breaking build_matches.
_CACHE_SCHEMA_VERSION = 1


def _canonical_path(path: str) -> str:
    """One canonical form per file, so C:\\Logs, c:\\logs and a mapped drive of
    the same share hit the same cache entry (realpath resolves links and mapped
    forms; normcase folds case and separators on Windows)."""
    try:
        return os.path.normcase(os.path.realpath(path))
    except OSError:
        return os.path.normcase(os.path.abspath(path))


def _file_cache_key(path: str, st: os.stat_result) -> str:
    """Cache key tied to the file identity: any size/mtime change invalidates."""
    raw = (f"v{_CACHE_SCHEMA_VERSION}|{_canonical_path(path)}"
           f"|{st.st_size}|{st.st_mtime_ns}")
    return hashlib.sha1(raw.encode("utf-8", "replace")).hexdigest()


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

    def __init__(self, file_path: str, threads: Optional[int] = None,
                 cache_dir: Optional[str] = None):
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
        # Which source the LAST build_matches actually used (the status bar must
        # report the source of the rows on screen, not the current cache state).
        self.last_used_cache = False
        self._closed = False

        # Parquet cache state. cache_ready flips to True either right here (a
        # previous run already converted this exact file) or when the background
        # conversion finishes; build_matches picks its source per call.
        self.cache_dir = cache_dir if cache_dir else _default_cache_dir()
        self.parquet_path: Optional[str] = None
        self.cache_ready = False
        self._cache_failed = False
        self._cache_thread: Optional[threading.Thread] = None
        self._cache_con = None
        self._cache_state_lock = threading.Lock()
        # Signature of the source at open time: if the file changes afterwards
        # (a still-growing log on a share), the frozen cache no longer matches
        # what the live direct scan would return, so _use_cache re-checks it.
        self._src_sig: Optional[Tuple[int, int]] = None
        try:
            st = os.stat(file_path)
            self._src_sig = (st.st_size, st.st_mtime_ns)
            key = _file_cache_key(file_path, st)
        except OSError:
            key = None
        if key:
            self.parquet_path = os.path.join(self.cache_dir, key + ".parquet")
            if os.path.exists(self.parquet_path):
                if self._validate_cache_file():
                    self.cache_ready = True
                    try:
                        # Refresh mtime so age-based pruning tracks last use.
                        os.utime(self.parquet_path)
                    except OSError:
                        pass
                else:
                    self._try_remove(self.parquet_path)

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
    # ``exprs`` maps field name -> SQL expression and differs by source: raw scan
    # uses split_part over the single ``line`` column, the Parquet cache uses its
    # materialized columns directly.
    def _condition_sql(self, condition, label_descriptions, interface_mapping,
                       exprs: Dict[str, str]) -> Tuple[str, list]:
        field = condition.field
        if field == "interface":
            return self._interface_sql(condition, interface_mapping, exprs)
        if field == "__label__":
            return self._label_sql(condition, label_descriptions, exprs)
        expr = exprs.get(field)
        if expr is None:
            # Unknown field -> entry.get(field, '') == '' (matches the parser).
            expr = "''"
        return _scalar_sql(expr, condition.operator, condition.value,
                           condition.case_sensitive)

    def _interface_sql(self, condition, interface_mapping,
                       exprs: Dict[str, str]) -> Tuple[str, list]:
        """A row matches if the condition matches the physical name OR its logical
        display name. The physical match is direct SQL; the logical match is the
        set of physical names whose mapped display matches (resolved in Python,
        no scan needed)."""
        expr = exprs["interface"]
        base_sql, base_params = _scalar_sql(
            expr, condition.operator, condition.value, condition.case_sensitive)
        phys = [p for p, disp in (interface_mapping or {}).items()
                if _py_match(condition, disp)]
        if phys:
            inlist = ", ".join(_sql_str(p) for p in phys)
            return (f"({base_sql} OR ({expr} IN ({inlist})))", base_params)
        return (base_sql, base_params)

    def _label_sql(self, condition, label_descriptions,
                   exprs: Dict[str, str]) -> Tuple[str, list]:
        """The rule label of a row is label_descriptions.get(rid, ''). Select the
        rids whose resolved description matches, plus (if the empty label matches)
        every rid that has no description."""
        expr = exprs["rid"]
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

    def _expression_sql(self, expression, label_descriptions, interface_mapping,
                        exprs: Dict[str, str]) -> Tuple[str, list]:
        conditions = getattr(expression, "conditions", [])
        operators = getattr(expression, "operators", [])
        negations = getattr(expression, "negations", [])
        if not conditions:
            return "", []

        params: List = []
        frag0, p0 = self._condition_sql(
            conditions[0], label_descriptions, interface_mapping, exprs)
        params.extend(p0)
        if negations and negations[0]:
            frag0 = f"(NOT ({frag0}))"
        combined = f"({frag0})"

        for i in range(1, len(conditions)):
            frag, p = self._condition_sql(
                conditions[i], label_descriptions, interface_mapping, exprs)
            params.extend(p)
            if i < len(negations) and negations[i]:
                frag = f"(NOT ({frag}))"
            op = operators[i - 1] if (i - 1) < len(operators) else "AND"
            sql_op = "OR" if str(op).upper() == "OR" else "AND"
            combined = f"({combined} {sql_op} ({frag}))"
        return combined, params

    def _build_where(self, expression, time_range, label_descriptions,
                     interface_mapping, exprs: Dict[str, str],
                     ts_expr: str) -> Tuple[str, list]:
        clauses: List[str] = []
        params: List = []
        start, end = time_range
        if start is not None:
            clauses.append(f"({ts_expr} >= ?)")
            params.append(start)
        if end is not None:
            clauses.append(f"({ts_expr} <= ?)")
            params.append(end)
        expr_sql, expr_params = self._expression_sql(
            expression, label_descriptions, interface_mapping, exprs)
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
        use_cache = self._use_cache()
        self.last_used_cache = use_cache
        if use_cache:
            # Read the Parquet cache: fields are real columns (qualified with the
            # ``src`` alias so the ``ts`` output alias cannot shadow the typed
            # ``src.ts`` inside the window ORDER BY).
            exprs = {f: f'src."{f}"' for f in _MATCH_FIELDS}
            ts_expr = "src.ts"
        else:
            exprs = self._exprs
            ts_expr = _TS_TS

        where, params = self._build_where(
            expression, time_range,
            label_descriptions or {}, interface_mapping or {},
            exprs, ts_expr)

        order = "DESC" if order_desc else "ASC"
        if use_cache:
            # The validity gate was applied when the cache was written. Ordering
            # is by timestamp only, like the direct scan (equal-timestamp ties
            # are arbitrary on both paths, a documented limitation).
            full_where = where if where else "TRUE"
            rel = f"read_parquet('{self.parquet_path.replace(chr(39), chr(39) * 2)}') AS src"
            resolved = ", ".join(f'{exprs[f]} AS "{f}"' for f in _MATCH_FIELDS)
            select = (
                f"SELECT src.line AS line, split_part(src.line, chr(9), 1) AS ts, "
                f"{resolved}, (row_number() OVER (ORDER BY src.ts {order}) - 1) AS rn "
                f"FROM {rel}"
                f" WHERE {full_where}"
            )
        else:
            # Validity gate mirroring OPNsenseLogParser.parse_log_line: a row is a
            # real filterlog entry only if the line contains 'filterlog' and the
            # action field is non-empty. Without this, non-filterlog / blank lines
            # would spuriously match negative conditions (e.g. action != 'block').
            base = f"(contains(line, 'filterlog') AND {self._exprs['action']} <> '')"
            full_where = base if not where else f"{base} AND ({where})"
            resolved = ", ".join(
                f'{self._exprs[f]} AS "{f}"' for f in _MATCH_FIELDS)
            select = (
                f"SELECT line, {_TS_TEXT} AS ts, {resolved}, "
                f"(row_number() OVER (ORDER BY {_TS_TS} {order}) - 1) AS rn "
                f"FROM {self._rel}"
                f" WHERE {full_where}"
            )
        sql = "CREATE OR REPLACE TABLE matches AS " + select
        try:
            with self._lock:
                self.con.execute(sql, params)
                self.match_count = self.con.execute(
                    "SELECT count(*) FROM matches").fetchone()[0]
        except Exception as exc:
            if not use_cache:
                raise
            # The cache became unreadable mid-session (deleted by hand, pruned
            # by another instance, disk trouble...): drop it and answer through
            # the direct scan instead of failing over to the slow legacy path.
            logger.warning("Parquet cache unreadable, retrying with direct "
                           "scan: %s", exc)
            self.cache_ready = False
            self._cache_failed = True
            if self.parquet_path:
                self._try_remove(self.parquet_path)
            return self.build_matches(expression, time_range,
                                      label_descriptions, interface_mapping,
                                      order_desc)
        logger.info("DuckDB filter materialized %s matches (%s)",
                    self.match_count,
                    "parquet cache" if use_cache else "direct scan")
        return self.match_count

    # ----- Parquet cache ------------------------------------------------------
    def _use_cache(self) -> bool:
        if not (self.cache_ready and self.parquet_path
                and os.path.exists(self.parquet_path)):
            return False
        # The cache is a frozen snapshot; the direct scan reads the live file.
        # If the source changed since the key was computed, serving the cache
        # would silently return different results than the direct scan did a
        # moment earlier, so drop back to the direct scan for good.
        try:
            st = os.stat(self.file_path)
        except OSError:
            return False
        if (st.st_size, st.st_mtime_ns) != self._src_sig:
            logger.warning("Source file changed since open; parquet cache "
                           "disabled for this session")
            self.cache_ready = False
            self._cache_failed = True
            return False
        return True

    def start_cache_build(self, on_status=None) -> None:
        """Start the one-time Parquet conversion in a background thread.

        Filters keep working on the direct-scan path while it runs; once the
        cache is in place every subsequent filter reads it instead of the raw
        file. No-op if the cache already exists, already failed, or is building.
        ``on_status`` (optional) receives short human-readable progress strings
        and is called from the background thread.
        """
        if self.parquet_path is None or self._cache_failed or self._closed:
            return
        if self.cache_ready:
            if on_status:
                on_status("Filter cache ready: filters are near-instant")
            return
        with self._cache_state_lock:
            if self._cache_thread is not None and self._cache_thread.is_alive():
                return
            self._cache_thread = threading.Thread(
                target=self._cache_worker, args=(on_status,), daemon=True)
            self._cache_thread.start()

    def _cache_worker(self, on_status) -> None:
        # No status chatter once the engine is closed (file switched, app gone).
        try:
            if on_status and not self._closed:
                on_status("Optimizing filter cache in background "
                          "(one-time; filtering stays available meanwhile)...")
            self.build_cache_sync()
            if self.cache_ready and on_status and not self._closed:
                on_status("Filter cache ready: filters are now near-instant")
        except Exception as exc:
            self._cache_failed = True
            logger.warning("Parquet cache build failed (direct scan kept): %s", exc)
            if on_status and not self._closed:
                on_status("Filter cache unavailable; keeping direct scan")

    def build_cache_sync(self) -> None:
        """Run the conversion in the calling thread (worker thread and tests).

        Writes to a tmp file then renames atomically, so a killed process can
        never leave a half-written cache at the final path. Uses its own DuckDB
        connection: the main connection stays free for filters meanwhile.
        """
        if self.parquet_path is None or self._use_cache() or self._closed:
            return
        os.makedirs(self.cache_dir, exist_ok=True)
        self._cleanup_cache_dir()
        tmp_path = f"{self.parquet_path}.{os.getpid()}.tmp"
        con = duckdb.connect()
        self._cache_con = con
        try:
            try:
                con.execute(f"PRAGMA memory_limit='{_CACHE_BUILD_MEMORY_LIMIT}'")
                con.execute(
                    "SET temp_directory="
                    + _sql_str(os.path.join(self.cache_dir, "duckdb_tmp")))
            except Exception:
                pass
            # Narrow the close()-vs-build race: interrupt() only aborts a RUNNING
            # query, so re-check the flag just before starting the conversion.
            if self._closed:
                return
            con.execute(self._convert_sql(tmp_path))
            # Publish only if the source is still exactly what the cache key
            # describes: a file that grew DURING the conversion would otherwise
            # publish a snapshot diverging from the live direct scan.
            try:
                st = os.stat(self.file_path)
                unchanged = (st.st_size, st.st_mtime_ns) == self._src_sig
            except OSError:
                unchanged = False
            if not unchanged:
                self._cache_failed = True
                logger.warning("Source changed during cache conversion; "
                               "cache discarded, direct scan kept")
                return
            os.replace(tmp_path, self.parquet_path)
            self.cache_ready = True
            logger.info("Parquet filter cache ready: %s", self.parquet_path)
        finally:
            self._cache_con = None
            try:
                con.close()
            except Exception:
                pass
            if os.path.exists(tmp_path):
                self._try_remove(tmp_path)

    def _convert_sql(self, dest_path: str) -> str:
        """One streaming pass: valid rows only, raw line + typed ts + resolved
        fields. Deliberately NO row_number()/line_no column: a windowed column
        serializes the parquet write and was measured 5x slower end to end.
        """
        field_cols = ", ".join(
            f'{expr} AS "{name}"' for name, expr in self._exprs.items())
        dest = dest_path.replace("'", "''")
        return (
            f"COPY (SELECT line, {_TS_TS} AS ts, {field_cols} "
            f"FROM {self._rel} "
            f"WHERE contains(line, 'filterlog') AND {self._exprs['action']} <> '') "
            f"TO '{dest}' ({_PARQUET_OPTS})"
        )

    def _validate_cache_file(self) -> bool:
        """A quick footer/metadata read; False for truncated/corrupt files."""
        try:
            con = duckdb.connect()
            try:
                p = self.parquet_path.replace("'", "''")
                con.execute(f"SELECT num_rows FROM parquet_file_metadata('{p}')").fetchone()
                return True
            finally:
                con.close()
        except Exception:
            logger.warning("Discarding unreadable parquet cache: %s", self.parquet_path)
            return False

    def _cleanup_cache_dir(self) -> None:
        """Prune leftover tmp files, caches unused for too long, then the oldest
        caches beyond the total-size cap. Never touches the current file's cache."""
        try:
            names = os.listdir(self.cache_dir)
        except OSError:
            return
        now = time.time()
        parquets = []
        for name in names:
            path = os.path.join(self.cache_dir, name)
            if path == self.parquet_path or not os.path.isfile(path):
                continue
            try:
                st = os.stat(path)
            except OSError:
                continue
            if name.endswith(".tmp"):
                if now - st.st_mtime > _CACHE_TMP_MAX_AGE_S:
                    self._try_remove(path)
                continue
            if not name.endswith(".parquet"):
                continue
            if now - st.st_mtime > _CACHE_MAX_AGE_S:
                self._try_remove(path)
            else:
                parquets.append((st.st_mtime, st.st_size, path))
        parquets.sort()  # oldest first
        total = sum(size for _, size, _ in parquets)
        while parquets and total > _CACHE_MAX_TOTAL_BYTES:
            _, size, path = parquets.pop(0)
            self._try_remove(path)
            total -= size
        # Spill files DuckDB leaves behind when the process is killed mid-
        # conversion (the daemon thread never runs its finally). Same age gate
        # as the tmp files so a live build of another instance is untouched.
        spill_dir = os.path.join(self.cache_dir, "duckdb_tmp")
        try:
            for name in os.listdir(spill_dir):
                path = os.path.join(spill_dir, name)
                try:
                    if (os.path.isfile(path)
                            and now - os.stat(path).st_mtime > _CACHE_TMP_MAX_AGE_S):
                        self._try_remove(path)
                except OSError:
                    continue
        except OSError:
            pass

    @staticmethod
    def _try_remove(path: str) -> None:
        try:
            os.remove(path)
        except OSError:
            pass

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
        # Abort a cache conversion still running on its own connection, so a
        # file switch or app exit does not keep churning through gigabytes.
        self._closed = True
        cache_con = self._cache_con
        if cache_con is not None:
            try:
                cache_con.interrupt()
            except Exception:
                pass
        # Hold the lock so we never close the connection out from under a build/fetch
        # running on another thread.
        with self._lock:
            try:
                if self.con is not None:
                    self.con.close()
            except Exception:
                pass
            self.con = None
