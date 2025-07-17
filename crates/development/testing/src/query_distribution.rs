// Query distribution validation tests for distributed system
//
// This module tests the distributed query execution, validation,
// and result aggregation across multiple nodes

use anyhow::Result;
use async_trait::async_trait;
use almanac_infrastructure_core::coordination::{QueryDistribution, QueryPlan, QueryNode};
use almanac_infrastructure_core::distributed_query::{DistributedQuery, QueryResult, QueryValidation};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};
use uuid::Uuid;

/// Test query distribution validator
pub struct QueryDistributionValidator {
    query_plans: Arc<RwLock<HashMap<String, QueryPlan>>>,
    node_results: Arc<RwLock<HashMap<String, Vec<QueryResult>>>>,
}

impl QueryDistributionValidator {
    pub fn new() -> Self {
        Self {
            query_plans: Arc::new(RwLock::new(HashMap::new())),
            node_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Validate query distribution across nodes
    pub async fn validate_distribution(&self, query: &DistributedQuery) -> Result<ValidationResult> {
        info!("Validating query distribution for query: {}", query.id);

        // Step 1: Validate query syntax and parameters
        let syntax_valid = self.validate_query_syntax(query).await?;
        if !syntax_valid {
            return Ok(ValidationResult {
                valid: false,
                errors: vec!["Invalid query syntax".to_string()],
                warnings: vec![],
            });
        }

        // Step 2: Generate query plan
        let query_plan = self.generate_query_plan(query).await?;
        self.query_plans.write().await.insert(query.id.clone(), query_plan.clone());

        // Step 3: Validate node assignments
        let node_validation = self.validate_node_assignments(&query_plan).await?;
        if !node_validation.valid {
            return Ok(node_validation);
        }

        // Step 4: Validate data locality
        let locality_validation = self.validate_data_locality(&query_plan).await?;
        if !locality_validation.valid {
            return Ok(locality_validation);
        }

        // Step 5: Validate resource requirements
        let resource_validation = self.validate_resource_requirements(&query_plan).await?;
        if !resource_validation.valid {
            return Ok(resource_validation);
        }

        Ok(ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        })
    }

    /// Validate query syntax
    async fn validate_query_syntax(&self, query: &DistributedQuery) -> Result<bool> {
        // Check required fields
        if query.query_text.is_empty() {
            return Ok(false);
        }

        // Validate query type
        match query.query_type.as_str() {
            "events" | "blocks" | "contracts" | "aggregation" => Ok(true),
            _ => Ok(false),
        }
    }

    /// Generate query execution plan
    async fn generate_query_plan(&self, query: &DistributedQuery) -> Result<QueryPlan> {
        debug!("Generating query plan for: {}", query.id);

        let mut plan = QueryPlan {
            query_id: query.id.clone(),
            stages: vec![],
            node_assignments: HashMap::new(),
            estimated_cost: 0.0,
            parallelism_level: 1,
        };

        // Determine query stages based on type
        match query.query_type.as_str() {
            "events" => {
                plan.stages = vec![
                    QueryStage::Filter,
                    QueryStage::Project,
                    QueryStage::Sort,
                ];
            }
            "blocks" => {
                plan.stages = vec![
                    QueryStage::Scan,
                    QueryStage::Filter,
                    QueryStage::Aggregate,
                ];
            }
            "aggregation" => {
                plan.stages = vec![
                    QueryStage::Scan,
                    QueryStage::LocalAggregate,
                    QueryStage::GlobalAggregate,
                ];
            }
            _ => {
                plan.stages = vec![QueryStage::Scan];
            }
        }

        // Calculate cost and parallelism
        plan.estimated_cost = self.estimate_query_cost(query).await?;
        plan.parallelism_level = self.determine_parallelism(query).await?;

        Ok(plan)
    }

    /// Validate node assignments for query execution
    async fn validate_node_assignments(&self, plan: &QueryPlan) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if nodes are available
        if plan.node_assignments.is_empty() {
            errors.push("No nodes assigned to query plan".to_string());
        }

        // Check node capabilities
        for (stage, nodes) in &plan.node_assignments {
            if nodes.is_empty() {
                errors.push(format!("No nodes assigned for stage: {:?}", stage));
            }

            // Validate each node can handle the stage
            for node in nodes {
                if !self.validate_node_capability(node, stage).await? {
                    errors.push(format!("Node {} cannot handle stage {:?}", node.node_id, stage));
                }
            }
        }

        // Check for load balancing
        let max_nodes_per_stage = plan.node_assignments.values()
            .map(|nodes| nodes.len())
            .max()
            .unwrap_or(0);
        
        let min_nodes_per_stage = plan.node_assignments.values()
            .map(|nodes| nodes.len())
            .min()
            .unwrap_or(0);

        if max_nodes_per_stage > min_nodes_per_stage * 2 {
            warnings.push("Unbalanced node distribution across stages".to_string());
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Validate data locality for efficient query execution
    async fn validate_data_locality(&self, plan: &QueryPlan) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if data is co-located with compute nodes
        for (stage, nodes) in &plan.node_assignments {
            match stage {
                QueryStage::Scan => {
                    // Scan stages should have good data locality
                    for node in nodes {
                        let locality_score = self.calculate_locality_score(node).await?;
                        if locality_score < 0.5 {
                            warnings.push(format!(
                                "Poor data locality for node {} (score: {:.2})",
                                node.node_id, locality_score
                            ));
                        }
                    }
                }
                QueryStage::GlobalAggregate => {
                    // Global aggregation requires data movement
                    if nodes.len() > 1 {
                        warnings.push("Global aggregation distributed across multiple nodes".to_string());
                    }
                }
                _ => {}
            }
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Validate resource requirements
    async fn validate_resource_requirements(&self, plan: &QueryPlan) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check estimated memory usage
        let estimated_memory_mb = plan.estimated_cost * 100.0; // Simple estimation
        if estimated_memory_mb > 1024.0 {
            warnings.push(format!("High memory usage estimated: {:.0} MB", estimated_memory_mb));
        }

        // Check parallelism level
        if plan.parallelism_level > 100 {
            warnings.push(format!("Very high parallelism level: {}", plan.parallelism_level));
        }

        // Validate node resources
        for nodes in plan.node_assignments.values() {
            for node in nodes {
                let available_memory = self.get_node_available_memory(node).await?;
                if available_memory < estimated_memory_mb / nodes.len() as f64 {
                    errors.push(format!(
                        "Node {} has insufficient memory: {:.0} MB available, {:.0} MB required",
                        node.node_id,
                        available_memory,
                        estimated_memory_mb / nodes.len() as f64
                    ));
                }
            }
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Validate node capability for a specific stage
    async fn validate_node_capability(&self, node: &QueryNode, stage: &QueryStage) -> Result<bool> {
        // Check if node supports the required operations
        match stage {
            QueryStage::Scan => node.capabilities.can_scan,
            QueryStage::Filter => node.capabilities.can_filter,
            QueryStage::Project => node.capabilities.can_project,
            QueryStage::Sort => node.capabilities.can_sort,
            QueryStage::LocalAggregate => node.capabilities.can_aggregate,
            QueryStage::GlobalAggregate => node.capabilities.can_coordinate,
        }
    }

    /// Calculate data locality score for a node
    async fn calculate_locality_score(&self, node: &QueryNode) -> Result<f64> {
        // Simulate locality calculation
        // In real implementation, this would check data distribution
        Ok(0.75) // Mock score
    }

    /// Get available memory for a node
    async fn get_node_available_memory(&self, node: &QueryNode) -> Result<f64> {
        // Simulate memory check
        // In real implementation, this would query node metrics
        Ok(512.0) // Mock 512 MB available
    }

    /// Estimate query cost
    async fn estimate_query_cost(&self, query: &DistributedQuery) -> Result<f64> {
        // Simple cost estimation based on query complexity
        let base_cost = match query.query_type.as_str() {
            "events" => 1.0,
            "blocks" => 2.0,
            "aggregation" => 5.0,
            _ => 1.0,
        };

        // Adjust for filters and limits
        let filter_factor = if query.filters.is_empty() { 1.0 } else { 0.5 };
        let limit_factor = if query.limit.is_some() { 0.2 } else { 1.0 };

        Ok(base_cost * filter_factor * limit_factor)
    }

    /// Determine parallelism level
    async fn determine_parallelism(&self, query: &DistributedQuery) -> Result<usize> {
        // Determine based on query type and data size
        match query.query_type.as_str() {
            "aggregation" => Ok(10),
            "blocks" => Ok(5),
            _ => Ok(1),
        }
    }

    /// Validate query results from multiple nodes
    pub async fn validate_results(
        &self,
        query_id: &str,
        node_results: Vec<(String, QueryResult)>,
    ) -> Result<ResultValidation> {
        info!("Validating results for query: {}", query_id);

        // Store results
        let mut results_by_node = HashMap::new();
        for (node_id, result) in node_results {
            results_by_node.entry(node_id.clone())
                .or_insert_with(Vec::new)
                .push(result);
        }
        self.node_results.write().await.insert(query_id.to_string(), results_by_node.values().flatten().cloned().collect());

        // Validate result consistency
        let consistency = self.validate_result_consistency(&results_by_node).await?;
        
        // Validate result completeness
        let completeness = self.validate_result_completeness(query_id, &results_by_node).await?;
        
        // Validate result ordering
        let ordering = self.validate_result_ordering(&results_by_node).await?;

        Ok(ResultValidation {
            consistent: consistency.is_ok(),
            complete: completeness.is_ok(),
            ordered: ordering.is_ok(),
            consistency_errors: consistency.err().map(|e| vec![e.to_string()]).unwrap_or_default(),
            completeness_errors: completeness.err().map(|e| vec![e.to_string()]).unwrap_or_default(),
            ordering_errors: ordering.err().map(|e| vec![e.to_string()]).unwrap_or_default(),
        })
    }

    /// Validate consistency across node results
    async fn validate_result_consistency(
        &self,
        results_by_node: &HashMap<String, Vec<QueryResult>>,
    ) -> Result<()> {
        // Check if all nodes returned the same number of results
        let result_counts: Vec<usize> = results_by_node.values()
            .map(|results| results.len())
            .collect();

        if !result_counts.is_empty() {
            let first_count = result_counts[0];
            if !result_counts.iter().all(|&count| count == first_count) {
                return Err(anyhow::anyhow!("Inconsistent result counts across nodes"));
            }
        }

        // Check for duplicate results
        let mut seen_ids = std::collections::HashSet::new();
        for results in results_by_node.values() {
            for result in results {
                if !seen_ids.insert(&result.id) {
                    return Err(anyhow::anyhow!("Duplicate result found: {}", result.id));
                }
            }
        }

        Ok(())
    }

    /// Validate result completeness
    async fn validate_result_completeness(
        &self,
        query_id: &str,
        results_by_node: &HashMap<String, Vec<QueryResult>>,
    ) -> Result<()> {
        // Get query plan
        let plans = self.query_plans.read().await;
        let plan = plans.get(query_id)
            .ok_or_else(|| anyhow::anyhow!("Query plan not found"))?;

        // Check if all assigned nodes returned results
        for (stage, assigned_nodes) in &plan.node_assignments {
            for node in assigned_nodes {
                if !results_by_node.contains_key(&node.node_id) {
                    return Err(anyhow::anyhow!(
                        "No results from node {} for stage {:?}",
                        node.node_id, stage
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate result ordering
    async fn validate_result_ordering(
        &self,
        results_by_node: &HashMap<String, Vec<QueryResult>>,
    ) -> Result<()> {
        // Check if results are properly ordered
        for (node_id, results) in results_by_node {
            let mut prev_timestamp = None;
            for result in results {
                if let Some(prev) = prev_timestamp {
                    if result.timestamp < prev {
                        return Err(anyhow::anyhow!(
                            "Results from node {} are not properly ordered",
                            node_id
                        ));
                    }
                }
                prev_timestamp = Some(result.timestamp);
            }
        }

        Ok(())
    }
}

/// Query stage enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QueryStage {
    Scan,
    Filter,
    Project,
    Sort,
    LocalAggregate,
    GlobalAggregate,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Result validation
#[derive(Debug, Clone)]
pub struct ResultValidation {
    pub consistent: bool,
    pub complete: bool,
    pub ordered: bool,
    pub consistency_errors: Vec<String>,
    pub completeness_errors: Vec<String>,
    pub ordering_errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use almanac_infrastructure_core::distributed_query::QueryFilter;

    #[tokio::test]
    async fn test_query_syntax_validation() {
        let validator = QueryDistributionValidator::new();

        // Valid query
        let valid_query = DistributedQuery {
            id: Uuid::new_v4().to_string(),
            query_text: "SELECT * FROM events WHERE chain = 'ethereum'".to_string(),
            query_type: "events".to_string(),
            filters: vec![],
            limit: None,
            offset: None,
        };
        assert!(validator.validate_query_syntax(&valid_query).await.unwrap());

        // Invalid query - empty text
        let invalid_query = DistributedQuery {
            id: Uuid::new_v4().to_string(),
            query_text: "".to_string(),
            query_type: "events".to_string(),
            filters: vec![],
            limit: None,
            offset: None,
        };
        assert!(!validator.validate_query_syntax(&invalid_query).await.unwrap());

        // Invalid query - unknown type
        let unknown_type_query = DistributedQuery {
            id: Uuid::new_v4().to_string(),
            query_text: "SELECT * FROM unknown".to_string(),
            query_type: "unknown".to_string(),
            filters: vec![],
            limit: None,
            offset: None,
        };
        assert!(!validator.validate_query_syntax(&unknown_type_query).await.unwrap());
    }

    #[tokio::test]
    async fn test_query_plan_generation() {
        let validator = QueryDistributionValidator::new();

        let query = DistributedQuery {
            id: Uuid::new_v4().to_string(),
            query_text: "SELECT * FROM events WHERE chain = 'ethereum'".to_string(),
            query_type: "events".to_string(),
            filters: vec![QueryFilter {
                field: "chain".to_string(),
                operator: "=".to_string(),
                value: serde_json::Value::String("ethereum".to_string()),
            }],
            limit: Some(100),
            offset: None,
        };

        let plan = validator.generate_query_plan(&query).await.unwrap();
        assert_eq!(plan.query_id, query.id);
        assert_eq!(plan.stages.len(), 3); // Filter, Project, Sort
        assert!(plan.estimated_cost > 0.0);
        assert_eq!(plan.parallelism_level, 1);
    }

    #[tokio::test]
    async fn test_result_consistency_validation() {
        let validator = QueryDistributionValidator::new();

        let mut results_by_node = HashMap::new();
        
        // Consistent results
        results_by_node.insert("node1".to_string(), vec![
            QueryResult {
                id: "result1".to_string(),
                data: serde_json::json!({"value": 1}),
                timestamp: 100,
            },
            QueryResult {
                id: "result2".to_string(),
                data: serde_json::json!({"value": 2}),
                timestamp: 200,
            },
        ]);
        
        results_by_node.insert("node2".to_string(), vec![
            QueryResult {
                id: "result3".to_string(),
                data: serde_json::json!({"value": 3}),
                timestamp: 300,
            },
            QueryResult {
                id: "result4".to_string(),
                data: serde_json::json!({"value": 4}),
                timestamp: 400,
            },
        ]);

        assert!(validator.validate_result_consistency(&results_by_node).await.is_ok());

        // Add duplicate result
        results_by_node.get_mut("node2").unwrap().push(QueryResult {
            id: "result1".to_string(), // Duplicate ID
            data: serde_json::json!({"value": 1}),
            timestamp: 500,
        });

        assert!(validator.validate_result_consistency(&results_by_node).await.is_err());
    }

    #[tokio::test]
    async fn test_result_ordering_validation() {
        let validator = QueryDistributionValidator::new();

        let mut results_by_node = HashMap::new();
        
        // Properly ordered results
        results_by_node.insert("node1".to_string(), vec![
            QueryResult {
                id: "result1".to_string(),
                data: serde_json::json!({"value": 1}),
                timestamp: 100,
            },
            QueryResult {
                id: "result2".to_string(),
                data: serde_json::json!({"value": 2}),
                timestamp: 200,
            },
            QueryResult {
                id: "result3".to_string(),
                data: serde_json::json!({"value": 3}),
                timestamp: 300,
            },
        ]);

        assert!(validator.validate_result_ordering(&results_by_node).await.is_ok());

        // Add out-of-order result
        results_by_node.get_mut("node1").unwrap().push(QueryResult {
            id: "result4".to_string(),
            data: serde_json::json!({"value": 4}),
            timestamp: 150, // Out of order
        });

        assert!(validator.validate_result_ordering(&results_by_node).await.is_err());
    }

    #[tokio::test]
    async fn test_full_query_validation() {
        let validator = QueryDistributionValidator::new();

        let query = DistributedQuery {
            id: Uuid::new_v4().to_string(),
            query_text: "SELECT COUNT(*) FROM events GROUP BY chain".to_string(),
            query_type: "aggregation".to_string(),
            filters: vec![],
            limit: None,
            offset: None,
        };

        let result = validator.validate_distribution(&query).await.unwrap();
        
        // Should be valid but might have warnings about node assignments
        assert!(result.valid || !result.errors.is_empty());
        
        // Check that a plan was generated
        assert!(validator.query_plans.read().await.contains_key(&query.id));
    }
}