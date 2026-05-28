//! Test contract types.

use serde::{Deserialize, Serialize};

/// Test contract — unit tests, integration tests, and acceptance criteria.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestContract {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_tests: Option<Vec<UnitTestGroup>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_tests: Option<Vec<IntegrationTest>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance_criteria: Option<Vec<String>>,
}

/// Unit test group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTestGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_cases: Option<Vec<UnitTestCase>>,
}

/// A single unit test case (given/when/then format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTestCase {
    pub id: String,
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub then: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_exception: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_side_effects: Option<Vec<String>>,
}

/// Integration test spanning multiple components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationTest {
    pub id: String,
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_result: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assertions: Option<Vec<String>>,
}
