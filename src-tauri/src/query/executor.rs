use super::types::*;
use crate::indexer::{BitmapIndex, InvertedIndex};
use regex::Regex;
use roaring::RoaringBitmap;
use std::collections::HashMap;
use std::time::Instant;

pub struct QueryExecutor {
    inverted_index: InvertedIndex,
    bitmap_index: BitmapIndex,
    total_entries: usize,
    regex_cache: HashMap<String, Regex>,
}

impl QueryExecutor {
    pub fn new(
        inverted_index: InvertedIndex,
        bitmap_index: BitmapIndex,
        total_entries: usize,
    ) -> Self {
        Self {
            inverted_index,
            bitmap_index,
            total_entries,
            regex_cache: HashMap::new(),
        }
    }

    /// Execute query with filters and boolean logic
    pub fn execute(&mut self, request: QueryRequest) -> Result<QueryResult, QueryError> {
        let start_time = Instant::now();

        if request.filters.is_empty() {
            return Err(QueryError::NoFilters);
        }

        // Optimize filter execution order
        let optimized_filters = self.optimize_filters(request.filters);

        // Execute filters and combine with boolean logic
        let result_bitmap = self.execute_filters(&optimized_filters)?;

        let matched_count = result_bitmap.len() as usize;
        // Cap entry_ids to avoid sending millions of IDs to the frontend (memory + IPC).
        // Frontend only uses the first 20k for get_entries_by_ids; matched_count still reflects the true total.
        const MAX_ENTRY_IDS_IN_RESULT: usize = 20_000;
        let entry_ids: Vec<usize> = result_bitmap
            .iter()
            .take(MAX_ENTRY_IDS_IN_RESULT)
            .map(|id| id as usize)
            .collect();
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(QueryResult {
            entry_ids,
            total_count: self.total_entries,
            matched_count, // True match count; entry_ids may be capped
            execution_time_ms,
        })
    }

    /// Optimize filter execution order for performance
    fn optimize_filters(&self, filters: Vec<FilterCondition>) -> Vec<FilterCondition> {
        let mut optimized = filters;

        // Sort by selectivity: bitmap filters first (most selective)
        optimized.sort_by_key(|f| match f.field {
            FilterField::Action | FilterField::Protocol | FilterField::Interface => 0,
            FilterField::SourceIp
            | FilterField::DestinationIp
            | FilterField::SourcePort
            | FilterField::DestinationPort => 1,
            FilterField::Timestamp | FilterField::RuleLabel => 2,
        });

        optimized
    }

    /// Execute all filters with boolean logic
    fn execute_filters(
        &mut self,
        filters: &[FilterCondition],
    ) -> Result<RoaringBitmap, QueryError> {
        if filters.is_empty() {
            return Err(QueryError::NoFilters);
        }

        // Start with first filter result
        let mut result_bitmap = self.execute_single_filter(&filters[0])?;

        // Apply subsequent filters with boolean logic
        for i in 1..filters.len() {
            let filter_result = self.execute_single_filter(&filters[i])?;
            let logic = filters[i - 1].logic.unwrap_or(LogicOperator::And);

            result_bitmap = match logic {
                LogicOperator::And => result_bitmap & filter_result,
                LogicOperator::Or => result_bitmap | filter_result,
                LogicOperator::Not => result_bitmap - filter_result,
            };
        }

        Ok(result_bitmap)
    }

    /// Execute a single filter condition
    fn execute_single_filter(
        &mut self,
        filter: &FilterCondition,
    ) -> Result<RoaringBitmap, QueryError> {
        match filter.field {
            // Bitmap index fields
            FilterField::Action => self.execute_bitmap_filter("action", filter),
            FilterField::Protocol => self.execute_bitmap_filter("protocol", filter),
            FilterField::Interface => self.execute_bitmap_filter("interface", filter),

            // Inverted index fields
            FilterField::SourceIp => self.execute_inverted_filter("source_ip", filter),
            FilterField::DestinationIp => self.execute_inverted_filter("dest_ip", filter),
            FilterField::SourcePort => self.execute_inverted_filter("source_port", filter),
            FilterField::DestinationPort => self.execute_inverted_filter("dest_port", filter),

            // Special handling
            FilterField::Timestamp => self.execute_timestamp_filter(filter),
            FilterField::RuleLabel => self.execute_rule_label_filter(filter),
        }
    }

    /// Execute bitmap index filter (Action, Protocol, Interface)
    fn execute_bitmap_filter(
        &self,
        field: &str,
        filter: &FilterCondition,
    ) -> Result<RoaringBitmap, QueryError> {
        let value_str = match &filter.value {
            FilterValue::String(s) => s.clone(),
            _ => {
                return Err(QueryError::ExecutionError(
                    "Bitmap filter requires string value".into(),
                ))
            }
        };

        let bitmap = match field {
            "action" => self.bitmap_index.query_action(&value_str),
            "protocol" => self.bitmap_index.query_protocol(&value_str),
            "interface" => self.bitmap_index.query_interface(&value_str),
            _ => None,
        };

        let bitmap = bitmap.ok_or_else(|| {
            QueryError::ExecutionError(format!("No bitmap for {}={}", field, value_str))
        })?;

        match filter.operator {
            FilterOperator::Equals => Ok(bitmap.clone()),
            FilterOperator::NotEquals => {
                let all_entries =
                    (0..self.total_entries as u32).collect::<RoaringBitmap>();
                Ok(all_entries - bitmap)
            }
            _ => Err(QueryError::ExecutionError(format!(
                "Unsupported operator for bitmap field: {:?}",
                filter.operator
            ))),
        }
    }

    /// Execute inverted index filter (IPs, Ports)
    fn execute_inverted_filter(
        &mut self,
        field: &str,
        filter: &FilterCondition,
    ) -> Result<RoaringBitmap, QueryError> {
        match &filter.operator {
            FilterOperator::Equals => {
                let entry_ids = match field {
                    "source_ip" => match &filter.value {
                        FilterValue::String(ip) => self.inverted_index.query_source_ip(ip),
                        _ => {
                            return Err(QueryError::ExecutionError(
                                "IP filter requires string value".into(),
                            ))
                        }
                    },
                    "dest_ip" => match &filter.value {
                        FilterValue::String(ip) => self.inverted_index.query_dest_ip(ip),
                        _ => {
                            return Err(QueryError::ExecutionError(
                                "IP filter requires string value".into(),
                            ))
                        }
                    },
                    "source_port" => match &filter.value {
                        FilterValue::Number(port) => {
                            self.inverted_index.query_source_port(*port as u16)
                        }
                        _ => {
                            return Err(QueryError::ExecutionError(
                                "Port filter requires number value".into(),
                            ))
                        }
                    },
                    "dest_port" => match &filter.value {
                        FilterValue::Number(port) => {
                            self.inverted_index.query_dest_port(*port as u16)
                        }
                        _ => {
                            return Err(QueryError::ExecutionError(
                                "Port filter requires number value".into(),
                            ))
                        }
                    },
                    _ => None,
                };

                Ok(entry_ids
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|&id| id as u32)
                    .collect())
            }

            FilterOperator::Contains
            | FilterOperator::StartsWith
            | FilterOperator::EndsWith
            | FilterOperator::Regex => {
                let value_str = match &filter.value {
                    FilterValue::String(s) => s.clone(),
                    _ => {
                        return Err(QueryError::ExecutionError(
                            "Text operator requires string value".into(),
                        ))
                    }
                };

                let result = RoaringBitmap::new();

                // For now, return empty bitmap for text operators on inverted index
                // This would require iterating over all keys, which we'll implement if needed
                match &filter.operator {
                    FilterOperator::Regex => {
                        let _regex = self.compile_regex(&value_str)?;
                        // TODO: Implement regex matching on inverted index keys
                    }
                    _ => {
                        // TODO: Implement contains/startsWith/endsWith on inverted index keys
                    }
                }

                Ok(result)
            }

            FilterOperator::GreaterThan
            | FilterOperator::LessThan
            | FilterOperator::Between => {
                // Numeric operators only make sense for ports
                if !field.contains("port") {
                    return Err(QueryError::ExecutionError(
                        "Numeric operators only supported for port fields".into(),
                    ));
                }

                let result = RoaringBitmap::new();
                // TODO: Implement range queries on inverted index
                // This requires iterating over port keys
                Ok(result)
            }

            _ => Err(QueryError::ExecutionError(format!(
                "Unsupported operator: {:?}",
                filter.operator
            ))),
        }
    }

    /// Execute timestamp filter
    fn execute_timestamp_filter(
        &self,
        _filter: &FilterCondition,
    ) -> Result<RoaringBitmap, QueryError> {
        // TODO: Implement timestamp filtering
        // For now, return all entries (will be implemented based on timestamp index design)
        Ok((0..self.total_entries as u32).collect())
    }

    /// Execute rule label filter
    fn execute_rule_label_filter(
        &self,
        _filter: &FilterCondition,
    ) -> Result<RoaringBitmap, QueryError> {
        // TODO: Implement rule label filtering
        // For now, return all entries
        Ok((0..self.total_entries as u32).collect())
    }

    /// Compile and cache regex pattern
    fn compile_regex(&mut self, pattern: &str) -> Result<&Regex, QueryError> {
        if !self.regex_cache.contains_key(pattern) {
            let regex = Regex::new(pattern)
                .map_err(|e| QueryError::InvalidRegex(e.to_string()))?;
            self.regex_cache.insert(pattern.to_string(), regex);
        }
        Ok(self.regex_cache.get(pattern).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_executor() -> QueryExecutor {
        let inverted_index = InvertedIndex::new();
        let bitmap_index = BitmapIndex::new();
        QueryExecutor::new(inverted_index, bitmap_index, 1000)
    }

    #[test]
    fn test_query_no_filters() {
        let mut executor = create_test_executor();
        let request = QueryRequest {
            filters: vec![],
            index_hash: "test".to_string(),
        };

        let result = executor.execute(request);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QueryError::NoFilters));
    }

    #[test]
    fn test_optimize_filters_bitmap_first() {
        let executor = create_test_executor();
        let filters = vec![
            FilterCondition {
                field: FilterField::SourceIp,
                operator: FilterOperator::Equals,
                value: FilterValue::String("192.168.1.1".to_string()),
                logic: Some(LogicOperator::And),
            },
            FilterCondition {
                field: FilterField::Action,
                operator: FilterOperator::Equals,
                value: FilterValue::String("block".to_string()),
                logic: None,
            },
        ];

        let optimized = executor.optimize_filters(filters);

        // Bitmap filter (Action) should come before inverted index filter (SourceIp)
        assert!(matches!(optimized[0].field, FilterField::Action));
        assert!(matches!(optimized[1].field, FilterField::SourceIp));
    }

    #[test]
    fn test_execute_bitmap_filter_equals() {
        let executor = create_test_executor();
        let filter = FilterCondition {
            field: FilterField::Action,
            operator: FilterOperator::Equals,
            value: FilterValue::String("block".to_string()),
            logic: None,
        };

        // Should not error even if bitmap is empty
        let result = executor.execute_bitmap_filter("action", &filter);
        // Will fail with "No bitmap" error since we have empty index
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_regex_valid() {
        let mut executor = create_test_executor();
        let result = executor.compile_regex(r"^192\.168\.");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_regex_invalid() {
        let mut executor = create_test_executor();
        let result = executor.compile_regex(r"[invalid(");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QueryError::InvalidRegex(_)));
    }
}
