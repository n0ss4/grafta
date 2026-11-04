use serde_json::json;

use super::{substitute, ReferenceError};

#[test]
fn a_reference_is_replaced_by_an_earlier_results_field() {
    let results = vec![json!({ "handle": "h-1", "pages": [{ "id": "0:1" }] }), json!({ "id": "9:9" })];
    let args = json!({ "handle": "$0.handle", "parent": "$1.id", "page": "$0.pages.0.id", "name": "$literal" });
    assert_eq!(
        substitute(&args, &results).unwrap(),
        json!({ "handle": "h-1", "parent": "9:9", "page": "0:1", "name": "$literal" })
    );
}

#[test]
fn a_reference_to_a_call_not_run_or_a_field_not_there_is_refused() {
    let results = vec![json!({ "id": "9:9" })];
    assert_eq!(substitute(&json!("$5.id"), &results), Err(ReferenceError::NotRun { text: "$5.id".into(), index: 5 }));
    assert_eq!(
        substitute(&json!("$0.name"), &results),
        Err(ReferenceError::Missing { text: "$0.name".into(), index: 0, part: "name".into() })
    );
}
