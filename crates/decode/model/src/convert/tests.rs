use super::*;
use kiwi::Value;
use std::collections::HashMap;

fn object(pairs: &[(&str, Value)]) -> Value {
    Value::Object(pairs.iter().map(|(k, v)| ((*k).to_string(), v.clone())).collect())
}

/// A file that consumes a published library addresses every style by asset
/// key rather than by GUID, and keeps a stale copy of the paint on the node
/// itself. Reading only the GUID form drops the whole design system.
#[test]
fn resolves_styles_referenced_by_asset_key() {
    let mut asset_refs = HashMap::new();
    asset_refs.insert("abc123".to_string(), Guid::new(10, 1));
    asset_refs.insert("abc123@9671:59".to_string(), Guid::new(10, 1));

    let converter = Converter {
        resolver: VariableResolver { asset_refs: &asset_refs },
        styles: IndexMap::new(),
        blobs: &[],
    };

    let node = object(&[(
        "styleIdForFill",
        object(&[(
            "assetRef",
            object(&[
                ("key", Value::Str("abc123".to_string())),
                ("version", Value::Str("9671:59".to_string())),
            ]),
        )]),
    )]);
    assert_eq!(converter.style_id(&node, Field::StyleIdForFill), Some(Guid::new(10, 1)));

    // A key the file does not carry resolves to nothing rather than guessing.
    let unknown = object(&[(
        "styleIdForFill",
        object(&[("assetRef", object(&[("key", Value::Str("nope".to_string()))]))]),
    )]);
    assert!(converter.style_id(&unknown, Field::StyleIdForFill).is_none());
}
