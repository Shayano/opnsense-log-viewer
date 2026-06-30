"""
Persistent SQLite index for OPNsense logs.

Parses each log line exactly once into an on-disk SQLite database, then answers
filter queries with SQL instead of re-parsing the whole file on every filter.
The database is cached on disk and keyed by a quick file fingerprint, so
re-opening the same file in a later session reuses the index instantly.

Design notes
------------
- The index stores one row per *parseable* line, keyed by ``line_no`` (the 0-based
  line index in the source file). That is exactly the contract the rest of the app
  already uses for ``VirtualLogManager.filtered_indices``: a sorted list of line
  numbers that the display/export path maps back to entries via ``get_chunk``.
- Only the *physical* interface and the rule-id hash (``rid``) are stored. The
  ``interface`` filter (which matches both the physical name and its logical
  display name) and the ``__label__`` filter (which matches a rule description)
  are resolved in Python at query time against the *current* interface mapping /
  rule-label mapping, so the index never needs rebuilding when those change.
- The generated SQL reproduces ``FilterExpression.evaluate`` exactly, including
  its left-to-right fold (no operator precedence) and per-condition negation.
"""
import os
import sys
import re
import sqlite3
import hashlib
import tempfile
from functools import lru_cache
from typing import Dict, List, Optional, Tuple

from opnsense_log_viewer.utils.logging_config import get_logger

logger = get_logger(__name__)

SCHEMA_VERSION = 1

# Columns physically stored in the index. Everything the filter UI can target
# maps onto one of these (interface/label get special resolution, see below).
_STORED_COLUMNS = (
    "action", "interface", "dir", "ipversion", "protonum", "protoname",
    "src", "dst", "srcport", "dstport", "length", "rid",
)

# Filter field name -> stored column. 'interface' and '__label__' are special.
_FIELD_TO_COLUMN = {
    "action": "action",
    "interface": "interface",
    "dir": "dir",
    "ipversion": "ipversion",
    "protonum": "protonum",
    "protoname": "protoname",
    "src": "src",
    "dst": "dst",
    "srcport": "srcport",
    "dstport": "dstport",
    "length": "length",
}

# Text columns that get a case-insensitive index (fast equality lookups).
_INDEXED_COLUMNS = ("action", "interface", "protoname", "src", "dst",
                    "srcport", "dstport", "rid")

_INSERT_BATCH = 50_000


def _sql_quote(value: str) -> str:
    """Return a safe single-quoted SQL string literal."""
    return "'" + str(value).replace("'", "''") + "'"


def _escape_like(value: str) -> str:
    r"""Escape % _ and \ for use in a LIKE pattern with ESCAPE '\'."""
    return (value.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_"))


@lru_cache(maxsize=512)
def _compiled(pattern: str):
    return re.compile(pattern)


def _regexp(pattern: Optional[str], value: Optional[str]) -> int:
    """SQLite REGEXP implementation backed by Python ``re.search``."""
    if pattern is None or value is None:
        return 0
    try:
        return 1 if _compiled(pattern).search(str(value)) else 0
    except re.error:
        return 0


def default_cache_dir() -> str:
    """Return a stable, per-user cache directory for index databases."""
    if sys.platform.startswith("win"):
        base = os.environ.get("LOCALAPPDATA") or tempfile.gettempdir()
    elif sys.platform == "darwin":
        base = os.path.join(os.path.expanduser("~"), "Library", "Caches")
    else:
        base = os.environ.get("XDG_CACHE_HOME") or os.path.join(
            os.path.expanduser("~"), ".cache")
    path = os.path.join(base, "opnsense-log-viewer", "index-cache")
    os.makedirs(path, exist_ok=True)
    return path


class SQLiteLogIndex:
    """Build and query a persistent, disk-backed SQLite index of a log file."""

    def __init__(self, cache_dir: Optional[str] = None):
        self.cache_dir = cache_dir or default_cache_dir()
        os.makedirs(self.cache_dir, exist_ok=True)
        self.conn: Optional[sqlite3.Connection] = None
        self.db_path: Optional[str] = None
        self.source_file: Optional[str] = None
        self._distinct_interfaces: Optional[List[str]] = None
        self._distinct_rids: Optional[List[str]] = None

    # ----- fingerprinting / cache management -------------------------------
    @staticmethod
    def fingerprint(file_path: str) -> str:
        """Quick fingerprint: size + mtime + first and last 64 KB of the file."""
        st = os.stat(file_path)
        h = hashlib.sha256()
        h.update(os.path.abspath(file_path).encode("utf-8", "ignore"))
        h.update(str(st.st_size).encode())
        h.update(str(st.st_mtime_ns).encode())
        chunk = 64 * 1024
        with open(file_path, "rb") as f:
            h.update(f.read(chunk))
            if st.st_size > chunk:
                f.seek(max(0, st.st_size - chunk))
                h.update(f.read(chunk))
        return h.hexdigest()[:32]

    def _db_path_for(self, key: str) -> str:
        return os.path.join(self.cache_dir, f"{key}.sqlite")

    def _is_valid_cache(self, path: str) -> bool:
        if not os.path.exists(path):
            return False
        try:
            conn = sqlite3.connect(path)
            try:
                row = conn.execute(
                    "SELECT value FROM meta WHERE key='schema_version'").fetchone()
                done = conn.execute(
                    "SELECT value FROM meta WHERE key='complete'").fetchone()
            finally:
                conn.close()
            return bool(row) and row[0] == str(SCHEMA_VERSION) and bool(done) and done[0] == "1"
        except sqlite3.Error:
            return False

    # ----- build -----------------------------------------------------------
    def build(self, file_path: str, parser, progress_callback=None,
              force: bool = False) -> None:
        """Ensure an index for ``file_path`` exists and open a connection to it.

        Reuses a valid cached database when present; otherwise parses the file
        once with ``parser.parse_log_line`` and populates a fresh database.
        """
        key = self.fingerprint(file_path)
        db_path = self._db_path_for(key)

        if not force and self._is_valid_cache(db_path):
            if progress_callback:
                progress_callback("Reusing cached index...")
            self._open(db_path, file_path)
            return

        self._build_db(file_path, db_path, parser, progress_callback)
        self._open(db_path, file_path)

    def _build_db(self, file_path: str, db_path: str, parser,
                  progress_callback=None) -> None:
        tmp_path = db_path + ".tmp"
        if os.path.exists(tmp_path):
            try:
                os.remove(tmp_path)
            except OSError:
                pass

        if progress_callback:
            progress_callback("Building index (parsing file once)...")

        conn = sqlite3.connect(tmp_path)
        try:
            conn.execute("PRAGMA journal_mode=OFF")
            conn.execute("PRAGMA synchronous=OFF")
            conn.execute("PRAGMA temp_store=MEMORY")
            conn.execute(
                "CREATE TABLE entries ("
                "line_no INTEGER PRIMARY KEY, ts_epoch REAL, "
                + ", ".join(f"{c} TEXT" for c in _STORED_COLUMNS) + ")"
            )
            conn.execute("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)")

            insert_sql = (
                "INSERT INTO entries (line_no, ts_epoch, "
                + ", ".join(_STORED_COLUMNS) + ") VALUES ("
                + ", ".join(["?"] * (len(_STORED_COLUMNS) + 2)) + ")"
            )

            batch: List[tuple] = []
            line_count = 0
            row_count = 0
            with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
                for line_no, line in enumerate(f):
                    line_count += 1
                    entry = parser.parse_log_line(line.strip())
                    if entry is None:
                        continue
                    data = entry.parsed_data
                    ts = entry.timestamp.timestamp() if entry.timestamp else None
                    batch.append((
                        line_no, ts,
                        data.get("action", ""), data.get("interface", ""),
                        data.get("dir", ""), data.get("ipversion", ""),
                        data.get("protonum", ""), data.get("protoname", ""),
                        data.get("src", ""), data.get("dst", ""),
                        data.get("srcport", ""), data.get("dstport", ""),
                        data.get("length", ""), data.get("rid", ""),
                    ))
                    row_count += 1
                    if len(batch) >= _INSERT_BATCH:
                        conn.executemany(insert_sql, batch)
                        batch.clear()
                        if progress_callback:
                            progress_callback(f"Indexing: {row_count:,} entries...")
            if batch:
                conn.executemany(insert_sql, batch)

            if progress_callback:
                progress_callback("Building index lookups...")
            for col in _INDEXED_COLUMNS:
                conn.execute(
                    f"CREATE INDEX idx_{col} ON entries({col} COLLATE NOCASE)")
            conn.execute("CREATE INDEX idx_ts ON entries(ts_epoch)")

            conn.executemany(
                "INSERT INTO meta (key, value) VALUES (?, ?)",
                [("schema_version", str(SCHEMA_VERSION)),
                 ("line_count", str(line_count)),
                 ("row_count", str(row_count)),
                 ("source", os.path.abspath(file_path)),
                 ("complete", "1")],
            )
            conn.commit()
        finally:
            conn.close()

        os.replace(tmp_path, db_path)
        if progress_callback:
            progress_callback(f"Index ready ({row_count:,} entries)")

    def _open(self, db_path: str, file_path: str) -> None:
        self.close()
        self.conn = sqlite3.connect(db_path, check_same_thread=False)
        self.conn.create_function("regexp", 2, _regexp, deterministic=True)
        self.db_path = db_path
        self.source_file = file_path
        self._distinct_interfaces = None
        self._distinct_rids = None

    # ----- query -----------------------------------------------------------
    def _distinct(self, column: str) -> List[str]:
        rows = self.conn.execute(
            f"SELECT DISTINCT {column} FROM entries").fetchall()
        return [r[0] if r[0] is not None else "" for r in rows]

    def query_line_numbers(self, expression, time_range=(None, None),
                           label_descriptions: Optional[Dict[str, str]] = None,
                           interface_mapping: Optional[Dict[str, str]] = None
                           ) -> List[int]:
        """Return the sorted line numbers matching ``expression`` + time range.

        Mirrors ``FilterExpression.evaluate`` (left-to-right fold, per-condition
        NOT) plus the optimized filter's leading time-range check.
        """
        if self.conn is None:
            raise RuntimeError("Index is not built/opened")

        where, params = self._build_where(
            expression, time_range,
            label_descriptions or {}, interface_mapping or {})
        sql = "SELECT line_no FROM entries"
        if where:
            sql += " WHERE " + where
        sql += " ORDER BY line_no"
        cur = self.conn.execute(sql, params)
        return [row[0] for row in cur.fetchall()]

    def count(self, expression, time_range=(None, None),
              label_descriptions=None, interface_mapping=None) -> int:
        """Return only the number of matching rows (no id materialization)."""
        if self.conn is None:
            raise RuntimeError("Index is not built/opened")
        where, params = self._build_where(
            expression, time_range,
            label_descriptions or {}, interface_mapping or {})
        sql = "SELECT COUNT(*) FROM entries"
        if where:
            sql += " WHERE " + where
        return int(self.conn.execute(sql, params).fetchone()[0])

    def _build_where(self, expression, time_range, label_descriptions,
                     interface_mapping) -> Tuple[str, list]:
        clauses: List[str] = []
        params: List = []

        start, end = time_range
        if start is not None:
            clauses.append("(ts_epoch IS NOT NULL AND ts_epoch >= ?)")
            params.append(start.timestamp())
        if end is not None:
            clauses.append("(ts_epoch IS NOT NULL AND ts_epoch <= ?)")
            params.append(end.timestamp())

        expr_sql, expr_params = self._expression_sql(
            expression, label_descriptions, interface_mapping)
        if expr_sql:
            clauses.append(expr_sql)
            params.extend(expr_params)

        return " AND ".join(clauses), params

    def _expression_sql(self, expression, label_descriptions,
                        interface_mapping) -> Tuple[str, list]:
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
            # Left fold to match FilterExpression.evaluate (no precedence).
            combined = f"({combined} {sql_op} ({frag}))"

        return combined, params

    def _condition_sql(self, condition, label_descriptions,
                       interface_mapping) -> Tuple[str, list]:
        field = condition.field
        if field == "interface":
            return self._interface_condition(condition, interface_mapping), []
        if field == "__label__":
            return self._label_condition(condition, label_descriptions), []

        column = _FIELD_TO_COLUMN.get(field)
        if column is None:
            # Unknown field: nothing can match it (matches entry.get(field,'')).
            return self._scalar_condition("''", condition)
        return self._scalar_condition(column, condition, is_column=True)

    def _scalar_condition(self, column_expr: str, condition,
                          is_column: bool = False) -> Tuple[str, list]:
        """SQL for a condition on a single column, matching _check_value_match."""
        op = condition.operator
        value = condition.value
        case_sensitive = condition.case_sensitive

        if op in (">", "<", ">=", "<="):
            return f"(CAST({column_expr} AS REAL) {op} ?)", [float_or_nan(value)]

        if op == "regex":
            pattern = value if case_sensitive else f"(?i){value}"
            return f"({column_expr} REGEXP ?)", [pattern]

        if not case_sensitive:
            if op == "==":
                return f"({column_expr} = ? COLLATE NOCASE)", [value]
            if op == "!=":
                return f"({column_expr} <> ? COLLATE NOCASE)", [value]
            if op == "contains":
                return (f"({column_expr} LIKE ? ESCAPE '\\')",
                        [f"%{_escape_like(value)}%"])
            if op == "startswith":
                return (f"({column_expr} LIKE ? ESCAPE '\\')",
                        [f"{_escape_like(value)}%"])
            if op == "endswith":
                return (f"({column_expr} LIKE ? ESCAPE '\\')",
                        [f"%{_escape_like(value)}"])
        else:
            # Case-sensitive string ops via anchored, escaped regex (rare path).
            esc = re.escape(value)
            if op == "==":
                return f"({column_expr} REGEXP ?)", [f"^{esc}$"]
            if op == "!=":
                return f"(NOT ({column_expr} REGEXP ?))", [f"^{esc}$"]
            if op == "contains":
                return f"({column_expr} REGEXP ?)", [esc]
            if op == "startswith":
                return f"({column_expr} REGEXP ?)", [f"^{esc}"]
            if op == "endswith":
                return f"({column_expr} REGEXP ?)", [f"{esc}$"]

        # Unknown operator: match nothing (FilterCondition returns False).
        return "(0)", []

    def _in_clause(self, column: str, values) -> str:
        values = list(values)
        if not values:
            return "(0)"
        joined = ", ".join(_sql_quote(v) for v in values)
        return f"({column} IN ({joined}))"

    def _interface_condition(self, condition, interface_mapping) -> str:
        """Resolve an 'interface' filter to a physical-interface IN clause.

        Matches FilterCondition.evaluate for 'interface': a row matches when the
        condition matches either the physical name or its logical display name.
        """
        if self._distinct_interfaces is None:
            self._distinct_interfaces = self._distinct("interface")
        matching = []
        for phys in self._distinct_interfaces:
            display = interface_mapping.get(phys, phys) if interface_mapping else phys
            if _check_value_match(condition, phys) or _check_value_match(condition, display):
                matching.append(phys)
        return self._in_clause("interface", matching)

    def _label_condition(self, condition, label_descriptions) -> str:
        """Resolve a '__label__' filter to a rid IN clause.

        The rule label of a row is ``label_descriptions.get(rid, '')``; we select
        the rids whose resolved description matches the condition.
        """
        if self._distinct_rids is None:
            self._distinct_rids = self._distinct("rid")
        matching = []
        for rid in self._distinct_rids:
            label = label_descriptions.get(rid, "") if label_descriptions else ""
            if _check_value_match(condition, label):
                matching.append(rid)
        return self._in_clause("rid", matching)

    def close(self) -> None:
        if self.conn is not None:
            try:
                self.conn.close()
            except sqlite3.Error:
                pass
            self.conn = None


def float_or_nan(value):
    try:
        return float(value)
    except (TypeError, ValueError):
        return float("nan")


def _check_value_match(condition, field_value) -> bool:
    """Python reference matcher, identical to FilterCondition._check_value_match.

    Used to resolve the special 'interface' and '__label__' fields against
    in-Python data (interface display names, rule descriptions).
    """
    case_sensitive = condition.case_sensitive
    if not case_sensitive and isinstance(field_value, str):
        field_value = field_value.lower()
        comparison_value = condition.value.lower()
    else:
        comparison_value = condition.value
    try:
        op = condition.operator
        if op == "==":
            return str(field_value) == comparison_value
        if op == "!=":
            return str(field_value) != comparison_value
        if op == "contains":
            return comparison_value in str(field_value)
        if op == "startswith":
            return str(field_value).startswith(comparison_value)
        if op == "endswith":
            return str(field_value).endswith(comparison_value)
        if op == "regex":
            flags = 0 if case_sensitive else re.IGNORECASE
            return bool(re.compile(condition.value, flags).search(str(field_value)))
        if op in (">", "<", ">=", "<="):
            try:
                a, b = float(field_value), float(comparison_value)
            except (TypeError, ValueError):
                return False
            return {">": a > b, "<": a < b, ">=": a >= b, "<=": a <= b}[op]
    except Exception:
        return False
    return False
