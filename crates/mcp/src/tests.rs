use super::*;

#[test]
fn base64_matches_the_standard_alphabet_and_padding() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn every_toolkit_tool_is_defined_with_an_object_schema() {
    let definitions = tool_definitions();
    assert_eq!(definitions.len(), toolkit::tools::ALL.len());
    for tool in definitions {
        assert_eq!(tool.input_schema.get("type").and_then(Value::as_str), Some("object"), "{}", tool.name);
    }
}

#[test]
fn a_tool_failure_is_a_tool_error_the_model_can_read() {
    let kit = Mutex::new(toolkit::new());
    let result = call(&kit, "summarize", serde_json::json!({}));
    assert_eq!(result.is_error, Some(true));
    let text = serde_json::to_string(&result.content).unwrap();
    assert!(text.contains("handle"), "{text}");
}

#[test]
fn journals_go_under_the_home_or_the_windows_profile() {
    use std::ffi::OsString;
    use std::path::PathBuf;
    let env = |pairs: &'static [(&'static str, &'static str)]| {
        move |name: &str| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| OsString::from(value))
    };
    let journals = |home: &str| PathBuf::from(home).join(".grafta").join("journals");
    assert_eq!(journal_dir_in(env(&[("GRAFTA_JOURNAL_DIR", "/j"), ("HOME", "/h")])), Some(PathBuf::from("/j")));
    assert_eq!(journal_dir_in(env(&[("HOME", "/h"), ("USERPROFILE", "/p")])), Some(journals("/h")));
    assert_eq!(journal_dir_in(env(&[("USERPROFILE", "/p")])), Some(journals("/p")));
    assert_eq!(journal_dir_in(env(&[])), None);
}
