pub mod executor;
pub mod types;
pub mod filter_to_sql;
pub mod sqlite_executor;

// Re-export public API
pub use executor::QueryExecutor;
pub use types::{
    FilterCondition, FilterField, FilterOperator, FilterValue, LogicOperator, QueryError,
    QueryRequest, QueryResult,
};
pub use filter_to_sql::{FilterToSql, SqlQuery};
pub use sqlite_executor::{SqliteQueryExecutor, SqliteLogEntry, MAX_ENTRY_IDS_IN_RESULT};
