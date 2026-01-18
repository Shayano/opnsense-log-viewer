use serde::{Deserialize, Serialize};

// Filter field types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FilterField {
    Timestamp,
    SourceIp,
    DestinationIp,
    SourcePort,
    DestinationPort,
    Protocol,
    Action,
    Interface,
    RuleLabel,
}

// Filter operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FilterOperator {
    // Text operators
    Equals,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
    // Numeric operators
    GreaterThan,
    LessThan,
    Between,
    // Select operators
    NotEquals,
    // Timestamp operators
    AbsoluteRange,
    Relative,
}

// Filter value (enum for type safety)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    String(String),
    Number(u32),
    Range(String, String), // For between/absoluteRange
    TimeRange(String),     // For relative (1h, 6h, 24h, 7d, 30d)
}

// Boolean logic operators
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogicOperator {
    And,
    Or,
    Not,
}

// Single filter condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterCondition {
    pub field: FilterField,
    pub operator: FilterOperator,
    pub value: FilterValue,
    pub logic: Option<LogicOperator>, // Logic to next filter (None for last filter)
}

// Query request from frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    pub filters: Vec<FilterCondition>,
    pub index_hash: String, // SHA-256 of source file (verify correct index loaded)
}

// Query result to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub entry_ids: Vec<usize>,   // Matching entry IDs
    pub total_count: usize,      // Total entries in index
    pub matched_count: usize,    // Number of matches
    pub execution_time_ms: u64,  // Query execution time
}

// Query error types
#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Index not loaded or hash mismatch")]
    IndexNotFound,

    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),

    #[error("Filter execution failed: {0}")]
    ExecutionError(String),

    #[error("No filters provided")]
    NoFilters,
}
