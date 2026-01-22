//! FilterAST to SQL Conversion
//!
//! Story 6.4: Converts FilterCondition AST to parameterized SQL queries.
//!
//! ## Design
//!
//! This module translates the frontend's `FilterCondition` structures into
//! parameterized SQL WHERE clauses. All values are bound as parameters to
//! prevent SQL injection.
//!
//! ## Example
//!
//! ```ignore
//! let filters = vec![
//!     FilterCondition { field: SourceIp, operator: Equals, value: "192.168.1.1".into(), logic: Some(And) },
//!     FilterCondition { field: Action, operator: Equals, value: "block".into(), logic: None },
//! ];
//! let sql_query = FilterToSql::convert(&filters)?;
//! // sql_query.where_clause = "source_ip = ? AND action = ?"
//! // sql_query.params = ["192.168.1.1", "block"]
//! ```

use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use rusqlite::types::Value;

use crate::query::types::{
    FilterCondition, FilterField, FilterOperator, FilterValue, LogicOperator, QueryError,
};

/// Represents a converted SQL query with parameterized values
#[derive(Debug, Clone)]
pub struct SqlQuery {
    /// WHERE clause without the "WHERE" keyword (e.g., "source_ip = ? AND action = ?")
    pub where_clause: String,
    /// Parameters in order they appear in the WHERE clause
    pub params: Vec<Value>,
}

impl SqlQuery {
    /// Create an empty query (matches all rows)
    pub fn empty() -> Self {
        Self {
            where_clause: String::from("1=1"),
            params: Vec::new(),
        }
    }
}

/// Converts FilterCondition AST to parameterized SQL
pub struct FilterToSql;

impl FilterToSql {
    /// Convert a slice of FilterConditions to a parameterized SQL query
    ///
    /// # Arguments
    /// * `filters` - Slice of filter conditions from the frontend
    ///
    /// # Returns
    /// * `Ok(SqlQuery)` - WHERE clause and parameters
    /// * `Err(QueryError)` - If conversion fails (e.g., invalid regex)
    pub fn convert(filters: &[FilterCondition]) -> Result<SqlQuery, QueryError> {
        if filters.is_empty() {
            return Ok(SqlQuery::empty());
        }

        let mut where_parts: Vec<String> = Vec::new();
        let mut all_params: Vec<Value> = Vec::new();

        for (i, filter) in filters.iter().enumerate() {
            let (clause, params) = Self::convert_single(filter)?;

            // Add logic operator before clause (except for first filter)
            if i > 0 {
                // Logic operator is on the PREVIOUS filter
                if let Some(ref prev_filter) = filters.get(i - 1) {
                    let logic = prev_filter.logic.unwrap_or(LogicOperator::And);
                    let logic_str = match logic {
                        LogicOperator::And => " AND ",
                        LogicOperator::Or => " OR ",
                        LogicOperator::Not => " AND NOT ",
                    };
                    where_parts.push(logic_str.to_string());
                }
            }

            where_parts.push(clause);
            all_params.extend(params);
        }

        Ok(SqlQuery {
            where_clause: where_parts.join(""),
            params: all_params,
        })
    }

    /// Convert a single FilterCondition to SQL clause and parameters
    fn convert_single(filter: &FilterCondition) -> Result<(String, Vec<Value>), QueryError> {
        let column = Self::field_to_column(&filter.field);

        match &filter.operator {
            FilterOperator::Equals => {
                let value = Self::value_to_sql(&filter.value)?;
                Ok((format!("{} = ?", column), vec![value]))
            }

            FilterOperator::NotEquals => {
                let value = Self::value_to_sql(&filter.value)?;
                Ok((format!("{} != ?", column), vec![value]))
            }

            FilterOperator::Contains => {
                let value = Self::value_to_like_pattern(&filter.value, "%", "%")?;
                Ok((format!("{} LIKE ? ESCAPE '\\'", column), vec![value]))
            }

            FilterOperator::StartsWith => {
                let value = Self::value_to_like_pattern(&filter.value, "", "%")?;
                Ok((format!("{} LIKE ? ESCAPE '\\'", column), vec![value]))
            }

            FilterOperator::EndsWith => {
                let value = Self::value_to_like_pattern(&filter.value, "%", "")?;
                Ok((format!("{} LIKE ? ESCAPE '\\'", column), vec![value]))
            }

            FilterOperator::Regex => {
                Self::validate_regex(&filter.value)?;
                let value = Self::value_to_sql(&filter.value)?;
                Ok((format!("{} REGEXP ?", column), vec![value]))
            }

            FilterOperator::GreaterThan => {
                let value = Self::value_to_sql(&filter.value)?;
                Ok((format!("{} > ?", column), vec![value]))
            }

            FilterOperator::LessThan => {
                let value = Self::value_to_sql(&filter.value)?;
                Ok((format!("{} < ?", column), vec![value]))
            }

            FilterOperator::Between => {
                let (start, end) = Self::value_to_range(&filter.value)?;
                Ok((format!("{} BETWEEN ? AND ?", column), vec![start, end]))
            }

            FilterOperator::AbsoluteRange => {
                let (start, end) = Self::value_to_range(&filter.value)?;
                Ok((format!("{} BETWEEN ? AND ?", column), vec![start, end]))
            }

            FilterOperator::Relative => {
                let cutoff = Self::relative_to_timestamp(&filter.value)?;
                Ok((format!("{} >= ?", column), vec![cutoff]))
            }
        }
    }

    /// Map FilterField to SQLite column name
    fn field_to_column(field: &FilterField) -> &'static str {
        match field {
            FilterField::Timestamp => "timestamp",
            FilterField::SourceIp => "source_ip",
            FilterField::DestinationIp => "dest_ip",
            FilterField::SourcePort => "source_port",
            FilterField::DestinationPort => "dest_port",
            FilterField::Protocol => "protocol",
            FilterField::Action => "action",
            FilterField::Interface => "interface",
            FilterField::RuleLabel => "rule_id",
        }
    }

    /// Convert FilterValue to rusqlite Value
    fn value_to_sql(value: &FilterValue) -> Result<Value, QueryError> {
        match value {
            FilterValue::String(s) => Ok(Value::Text(s.clone())),
            FilterValue::Number(n) => Ok(Value::Integer(*n as i64)),
            FilterValue::Range(_, _) => Err(QueryError::ExecutionError(
                "Range value used where single value expected".to_string(),
            )),
            FilterValue::TimeRange(_) => Err(QueryError::ExecutionError(
                "TimeRange value used where single value expected".to_string(),
            )),
        }
    }

    /// Convert FilterValue to LIKE pattern with prefix/suffix
    fn value_to_like_pattern(
        value: &FilterValue,
        prefix: &str,
        suffix: &str,
    ) -> Result<Value, QueryError> {
        match value {
            FilterValue::String(s) => {
                // Escape SQL LIKE special characters in the value
                let escaped = s.replace('%', "\\%").replace('_', "\\_");
                Ok(Value::Text(format!("{}{}{}", prefix, escaped, suffix)))
            }
            FilterValue::Number(n) => {
                Ok(Value::Text(format!("{}{}{}", prefix, n, suffix)))
            }
            _ => Err(QueryError::ExecutionError(
                "LIKE pattern requires string or number value".to_string(),
            )),
        }
    }

    /// Extract range values from FilterValue
    fn value_to_range(value: &FilterValue) -> Result<(Value, Value), QueryError> {
        match value {
            FilterValue::Range(start, end) => {
                Ok((Value::Text(start.clone()), Value::Text(end.clone())))
            }
            _ => Err(QueryError::ExecutionError(
                "Range operator requires Range value".to_string(),
            )),
        }
    }

    /// Convert relative time filter (1h, 6h, 24h, 7d, 30d) to absolute timestamp
    fn relative_to_timestamp(value: &FilterValue) -> Result<Value, QueryError> {
        let time_str = match value {
            FilterValue::TimeRange(s) => s,
            FilterValue::String(s) => s,
            _ => {
                return Err(QueryError::ExecutionError(
                    "Relative time requires string value (e.g., '1h', '24h', '7d')".to_string(),
                ));
            }
        };

        let duration = Self::parse_relative_duration(time_str)?;
        let cutoff: DateTime<Utc> = Utc::now() - duration;
        Ok(Value::Text(cutoff.to_rfc3339()))
    }

    /// Parse relative time string to Duration
    fn parse_relative_duration(s: &str) -> Result<Duration, QueryError> {
        let s = s.trim().to_lowercase();

        // Try to parse formats like "1h", "24h", "7d", "30d"
        if let Some(hours_str) = s.strip_suffix('h') {
            let hours: i64 = hours_str.parse().map_err(|_| {
                QueryError::ExecutionError(format!("Invalid hours value: {}", hours_str))
            })?;
            return Ok(Duration::hours(hours));
        }

        if let Some(days_str) = s.strip_suffix('d') {
            let days: i64 = days_str.parse().map_err(|_| {
                QueryError::ExecutionError(format!("Invalid days value: {}", days_str))
            })?;
            return Ok(Duration::days(days));
        }

        if let Some(mins_str) = s.strip_suffix('m') {
            let mins: i64 = mins_str.parse().map_err(|_| {
                QueryError::ExecutionError(format!("Invalid minutes value: {}", mins_str))
            })?;
            return Ok(Duration::minutes(mins));
        }

        Err(QueryError::ExecutionError(format!(
            "Invalid relative time format: '{}'. Expected formats: 1h, 24h, 7d, 30d",
            s
        )))
    }

    /// Validate regex pattern before use in SQL
    fn validate_regex(value: &FilterValue) -> Result<(), QueryError> {
        let pattern = match value {
            FilterValue::String(s) => s,
            _ => {
                return Err(QueryError::InvalidRegex(
                    "Regex operator requires string value".to_string(),
                ));
            }
        };

        Regex::new(pattern).map_err(|e| QueryError::InvalidRegex(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a simple filter
    fn make_filter(
        field: FilterField,
        op: FilterOperator,
        value: FilterValue,
        logic: Option<LogicOperator>,
    ) -> FilterCondition {
        FilterCondition {
            field,
            operator: op,
            value,
            logic,
        }
    }

    #[test]
    fn test_empty_filters() {
        let result = FilterToSql::convert(&[]).unwrap();
        assert_eq!(result.where_clause, "1=1");
        assert!(result.params.is_empty());
    }

    #[test]
    fn test_equals_string() {
        let filters = vec![make_filter(
            FilterField::SourceIp,
            FilterOperator::Equals,
            FilterValue::String("192.168.1.1".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "source_ip = ?");
        assert_eq!(result.params.len(), 1);
        assert_eq!(result.params[0], Value::Text("192.168.1.1".to_string()));
    }

    #[test]
    fn test_equals_number() {
        let filters = vec![make_filter(
            FilterField::SourcePort,
            FilterOperator::Equals,
            FilterValue::Number(443),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "source_port = ?");
        assert_eq!(result.params.len(), 1);
        assert_eq!(result.params[0], Value::Integer(443));
    }

    #[test]
    fn test_not_equals() {
        let filters = vec![make_filter(
            FilterField::Action,
            FilterOperator::NotEquals,
            FilterValue::String("block".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "action != ?");
        assert_eq!(result.params[0], Value::Text("block".to_string()));
    }

    #[test]
    fn test_contains() {
        let filters = vec![make_filter(
            FilterField::SourceIp,
            FilterOperator::Contains,
            FilterValue::String("168".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "source_ip LIKE ? ESCAPE '\\'");
        assert_eq!(result.params[0], Value::Text("%168%".to_string()));
    }

    #[test]
    fn test_starts_with() {
        let filters = vec![make_filter(
            FilterField::SourceIp,
            FilterOperator::StartsWith,
            FilterValue::String("192.168".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "source_ip LIKE ? ESCAPE '\\'");
        assert_eq!(result.params[0], Value::Text("192.168%".to_string()));
    }

    #[test]
    fn test_ends_with() {
        let filters = vec![make_filter(
            FilterField::SourceIp,
            FilterOperator::EndsWith,
            FilterValue::String(".1".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "source_ip LIKE ? ESCAPE '\\'");
        assert_eq!(result.params[0], Value::Text("%.1".to_string()));
    }

    #[test]
    fn test_like_escaping() {
        // Special LIKE characters should be escaped
        let filters = vec![make_filter(
            FilterField::SourceIp,
            FilterOperator::Contains,
            FilterValue::String("test%value_here".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(
            result.params[0],
            Value::Text("%test\\%value\\_here%".to_string())
        );
    }

    #[test]
    fn test_greater_than() {
        let filters = vec![make_filter(
            FilterField::SourcePort,
            FilterOperator::GreaterThan,
            FilterValue::Number(1024),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "source_port > ?");
        assert_eq!(result.params[0], Value::Integer(1024));
    }

    #[test]
    fn test_less_than() {
        let filters = vec![make_filter(
            FilterField::DestinationPort,
            FilterOperator::LessThan,
            FilterValue::Number(1024),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "dest_port < ?");
        assert_eq!(result.params[0], Value::Integer(1024));
    }

    #[test]
    fn test_between() {
        let filters = vec![make_filter(
            FilterField::Timestamp,
            FilterOperator::Between,
            FilterValue::Range(
                "2026-01-01T00:00:00Z".to_string(),
                "2026-01-31T23:59:59Z".to_string(),
            ),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "timestamp BETWEEN ? AND ?");
        assert_eq!(result.params.len(), 2);
        assert_eq!(
            result.params[0],
            Value::Text("2026-01-01T00:00:00Z".to_string())
        );
        assert_eq!(
            result.params[1],
            Value::Text("2026-01-31T23:59:59Z".to_string())
        );
    }

    #[test]
    fn test_regex_valid() {
        let filters = vec![make_filter(
            FilterField::SourceIp,
            FilterOperator::Regex,
            FilterValue::String(r"192\.168\.\d+\.\d+".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "source_ip REGEXP ?");
        assert_eq!(
            result.params[0],
            Value::Text(r"192\.168\.\d+\.\d+".to_string())
        );
    }

    #[test]
    fn test_regex_invalid() {
        let filters = vec![make_filter(
            FilterField::SourceIp,
            FilterOperator::Regex,
            FilterValue::String(r"[invalid".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QueryError::InvalidRegex(_)));
    }

    #[test]
    fn test_and_logic() {
        let filters = vec![
            make_filter(
                FilterField::SourceIp,
                FilterOperator::Equals,
                FilterValue::String("192.168.1.1".to_string()),
                Some(LogicOperator::And),
            ),
            make_filter(
                FilterField::Action,
                FilterOperator::Equals,
                FilterValue::String("block".to_string()),
                None,
            ),
        ];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "source_ip = ? AND action = ?");
        assert_eq!(result.params.len(), 2);
    }

    #[test]
    fn test_or_logic() {
        let filters = vec![
            make_filter(
                FilterField::Action,
                FilterOperator::Equals,
                FilterValue::String("pass".to_string()),
                Some(LogicOperator::Or),
            ),
            make_filter(
                FilterField::Action,
                FilterOperator::Equals,
                FilterValue::String("block".to_string()),
                None,
            ),
        ];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "action = ? OR action = ?");
        assert_eq!(result.params.len(), 2);
    }

    #[test]
    fn test_not_logic() {
        let filters = vec![
            make_filter(
                FilterField::Action,
                FilterOperator::Equals,
                FilterValue::String("pass".to_string()),
                Some(LogicOperator::Not),
            ),
            make_filter(
                FilterField::Protocol,
                FilterOperator::Equals,
                FilterValue::String("ICMP".to_string()),
                None,
            ),
        ];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "action = ? AND NOT protocol = ?");
        assert_eq!(result.params.len(), 2);
    }

    #[test]
    fn test_complex_query() {
        // (source_ip = X AND action = Y) OR protocol = Z
        let filters = vec![
            make_filter(
                FilterField::SourceIp,
                FilterOperator::Equals,
                FilterValue::String("192.168.1.1".to_string()),
                Some(LogicOperator::And),
            ),
            make_filter(
                FilterField::Action,
                FilterOperator::Equals,
                FilterValue::String("block".to_string()),
                Some(LogicOperator::Or),
            ),
            make_filter(
                FilterField::Protocol,
                FilterOperator::Equals,
                FilterValue::String("TCP".to_string()),
                None,
            ),
        ];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(
            result.where_clause,
            "source_ip = ? AND action = ? OR protocol = ?"
        );
        assert_eq!(result.params.len(), 3);
    }

    #[test]
    fn test_all_fields() {
        // Test that all FilterField variants map to correct columns
        let fields_and_columns = [
            (FilterField::Timestamp, "timestamp"),
            (FilterField::SourceIp, "source_ip"),
            (FilterField::DestinationIp, "dest_ip"),
            (FilterField::SourcePort, "source_port"),
            (FilterField::DestinationPort, "dest_port"),
            (FilterField::Protocol, "protocol"),
            (FilterField::Action, "action"),
            (FilterField::Interface, "interface"),
            (FilterField::RuleLabel, "rule_id"),
        ];

        for (field, expected_col) in fields_and_columns {
            let filters = vec![make_filter(
                field,
                FilterOperator::Equals,
                FilterValue::String("test".to_string()),
                None,
            )];

            let result = FilterToSql::convert(&filters).unwrap();
            assert!(
                result.where_clause.starts_with(expected_col),
                "Field {:?} should map to column {}",
                filters[0].field,
                expected_col
            );
        }
    }

    #[test]
    fn test_relative_time_hours() {
        let filters = vec![make_filter(
            FilterField::Timestamp,
            FilterOperator::Relative,
            FilterValue::TimeRange("24h".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "timestamp >= ?");
        assert_eq!(result.params.len(), 1);
        // The timestamp should be approximately 24 hours ago
        if let Value::Text(ts) = &result.params[0] {
            assert!(ts.contains("T"), "Should be ISO timestamp");
        } else {
            panic!("Expected Text value");
        }
    }

    #[test]
    fn test_relative_time_days() {
        let filters = vec![make_filter(
            FilterField::Timestamp,
            FilterOperator::Relative,
            FilterValue::TimeRange("7d".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "timestamp >= ?");
    }

    #[test]
    fn test_relative_time_minutes() {
        let filters = vec![make_filter(
            FilterField::Timestamp,
            FilterOperator::Relative,
            FilterValue::TimeRange("30m".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "timestamp >= ?");
    }

    #[test]
    fn test_relative_time_invalid() {
        let filters = vec![make_filter(
            FilterField::Timestamp,
            FilterOperator::Relative,
            FilterValue::TimeRange("invalid".to_string()),
            None,
        )];

        let result = FilterToSql::convert(&filters);
        assert!(result.is_err());
    }

    #[test]
    fn test_absolute_range() {
        let filters = vec![make_filter(
            FilterField::Timestamp,
            FilterOperator::AbsoluteRange,
            FilterValue::Range(
                "2026-01-01T00:00:00Z".to_string(),
                "2026-01-31T23:59:59Z".to_string(),
            ),
            None,
        )];

        let result = FilterToSql::convert(&filters).unwrap();
        assert_eq!(result.where_clause, "timestamp BETWEEN ? AND ?");
    }
}
