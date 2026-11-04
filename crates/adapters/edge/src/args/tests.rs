use serde_json::json;

use super::budget;

#[test]
fn a_budget_outside_the_range_names_the_range() {
    let error = budget(&json!({ "budget": 10 })).unwrap_err();
    assert!(error.to_string().contains("1024"));
}
