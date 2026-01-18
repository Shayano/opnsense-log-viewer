pub mod executor;
pub mod types;

// Re-export public API
pub use executor::QueryExecutor;
pub use types::{
    FilterCondition, FilterField, FilterOperator, FilterValue, LogicOperator, QueryError,
    QueryRequest, QueryResult,
};
