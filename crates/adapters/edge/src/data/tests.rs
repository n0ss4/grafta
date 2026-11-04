use serde_json::{json, Value};

use super::{from_json, to_json};
use crate::app::data::{Data, Number};

fn written_alike(value: &Value) {
    assert_eq!(from_json(value).to_string(), value.to_string(), "{value}");
    assert_eq!(&to_json(&from_json(value)), value, "{value} back");
}

#[test]
fn it_is_written_as_serde_json_writes_it() {
    for value in [
        json!(null),
        json!(true),
        json!(0),
        json!(-0.0),
        json!(1.0),
        json!(0.1),
        json!(1e21),
        json!(1e-7),
        json!(123_456_789.123),
        json!(f32::MAX),
        json!(u64::MAX),
        json!(i64::MIN),
        json!("quote \" backslash \\ slash / tab \t newline \n cr \r bell \u{7} nul \u{0} del \u{7f}"),
        json!("\u{8}\u{c}\u{1f} é 字 🎨"),
        json!({ "b": [1, 2.5, "x", null, { "z": {}, "a": [] }], "a": -3 }),
    ] {
        written_alike(&value);
    }
}

/// Every call the golden corpus makes, and every schema a model reads.
#[test]
fn every_call_and_schema_on_record_is_written_alike() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../toolkit/tests/golden");
    let mut seen = 0;
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let text = std::fs::read_to_string(entry.path()).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        for item in value.as_array().unwrap() {
            written_alike(item);
            seen += 1;
        }
    }
    assert!(seen > 100, "{seen} values");
}

#[test]
fn a_whole_number_and_a_float_read_back_as_they_came() {
    let one = from_json(&json!(1));
    let one_point_oh = from_json(&json!(1.0));
    assert_eq!(one.as_u64(), Some(1));
    assert_eq!(one_point_oh.as_u64(), None, "as serde_json reads 1.0");
    assert_eq!(one_point_oh.as_f64(), Some(1.0));
    assert_eq!(from_json(&json!(-2)).as_u64(), None);
    assert_eq!(from_json(&json!(-2)).as_i64(), Some(-2));
    assert_eq!(Data::from(5_i64), Data::Number(Number::Unsigned(5)), "as a JSON reader has it");
    assert_eq!(Data::from(-5_i64), Data::Number(Number::Signed(-5)));
}

#[test]
fn what_is_absent_reads_as_null() {
    let data = from_json(&json!({ "a": [1] }));
    assert!(data["b"].is_null());
    assert!(data["a"][3].is_null());
    assert_eq!(data["a"][0].as_u64(), Some(1));
    assert_eq!(data.get("a").and_then(Data::as_list).map(Vec::len), Some(1));
}

/// `data!` and `json!` given the same tokens build the same value.
macro_rules! alike {
    ($($tt:tt)+) => {
        assert_eq!(crate::app::data::data!($($tt)+), from_json(&json!($($tt)+)), "{}", stringify!($($tt)+));
    };
}

#[test]
fn data_is_built_as_json_is() {
    let id = schema::Guid::new(1, 2);
    let (size, none, some): (f32, Option<u32>, Option<&str>) = (0.1, None, Some("x"));
    let names = vec!["a".to_string(), "b".to_string()];
    let mut by_name = indexmap::IndexMap::new();
    by_name.insert("z".to_string(), 1_usize);
    by_name.insert("a".to_string(), 2_usize);
    let mut by_number = std::collections::HashMap::new();
    by_number.insert(7_u32, 1.5_f64);
    alike!(null);
    alike!(true);
    alike!([]);
    alike!({});
    alike!(id);
    alike!(size);
    alike!(-3_i32);
    alike!(u64::MAX);
    alike!(f64::NAN);
    alike!({ "id": id, "size": size, "none": none, "some": some, "names": names, "by_name": by_name, "by_number": by_number });
    alike!({ "nested": { "list": [1, 2.5, "x", null, true, [id], { "deep": [] }], "after": false }, "last": -0.0 });
    alike!([size, { "a": 1 }, [], null,]);
    let key = "computed";
    alike!({ (key): 1, "trailing": 2, });
}

#[test]
fn a_key_is_set_as_json_s_is() {
    let mut data = crate::app::data::Data::Null;
    data["a"] = crate::app::data::data!(1);
    data["b"]["c"] = crate::app::data::data!([true]);
    assert_eq!(data, from_json(&json!({ "a": 1, "b": { "c": [true] } })));
}
