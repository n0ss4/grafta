use kiwi::{Fields, Value};

use crate::Guid;

fn raw(session: u32, local: u32) -> Value {
    Value::Object(Fields::from([("sessionID", Value::Uint(session)), ("localID", Value::Uint(local))]))
}

#[test]
fn text_and_value_convert_both_ways() {
    let id: Guid = "12:345".parse().unwrap();
    assert_eq!(id, Guid::new(12, 345));
    assert_eq!(id.to_string(), "12:345");
    assert_eq!(Guid::read(&id.to_value()), Some(id));
    assert_eq!(Guid::read(&raw(12, 345)), Some(id));
}

#[test]
fn an_envelope_is_read_through() {
    let wrapped = Value::Object(Fields::from([("guid", raw(1, 2))]));
    assert_eq!(Guid::read_wrapped(&wrapped), Some(Guid::new(1, 2)));
    assert_eq!(Guid::read_wrapped(&raw(1, 2)), Some(Guid::new(1, 2)));
}

#[test]
fn text_that_is_not_an_id_is_refused() {
    for text in ["12", "a:1", "1:b", "", ":"] {
        assert!(text.parse::<Guid>().is_err(), "{text}");
    }
}
