//! The helpers a tool's JSON Schema is written with.

use serde_json::{json, Value};

/// A JSON Schema object with these properties, `required` naming the ones a
/// call must carry.
pub(crate) fn schema(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required })
}

pub(crate) fn handle_property() -> Value {
    json!({ "type": "string", "description": "The handle open_file returned." })
}

pub(crate) fn paging_properties() -> Value {
    json!({
        "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "description": "Results per page." },
        "cursor": { "type": "string", "description": "next_cursor from the previous page." },
        "budget": {
            "type": "integer",
            "minimum": crate::bound::MIN_BUDGET,
            "maximum": crate::bound::MAX_BUDGET,
            "description": format!("Maximum result size in bytes. Default {}.", crate::bound::DEFAULT_BUDGET),
        },
    })
}

pub(crate) fn merge(mut a: Value, b: Value) -> Value {
    if let (Some(a), Value::Object(b)) = (a.as_object_mut(), b) {
        a.extend(b);
    }
    a
}
