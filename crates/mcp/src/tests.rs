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

/// Where the plugin reads the contract from.
const CONTRACT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../figma-plugin/src/contract.json");

#[test]
fn the_plugin_s_contract_is_as_recorded() {
    let actual = serde_json::to_string_pretty(&contract()).expect("the contract serialises") + "\n";
    if std::env::var_os("GRAFTA_RECORD").is_some() {
        std::fs::write(CONTRACT, actual).expect("the contract written");
        return;
    }
    let expected = std::fs::read_to_string(CONTRACT).expect("the recorded contract");
    for (line, (want, got)) in expected.lines().zip(actual.lines()).enumerate() {
        assert_eq!(got, want, "line {} of figma-plugin/src/contract.json changed: GRAFTA_RECORD=1 records it", line + 1);
    }
    assert_eq!(actual.lines().count(), expected.lines().count(), "the contract changed length: GRAFTA_RECORD=1 records it");
}

#[test]
fn the_ai_app_is_named_by_its_title_for_people_when_it_has_one() {
    let mut client = Implementation::new("claude-ai", "1.0");
    assert_eq!(caller(&client), "claude-ai");
    client.title = Some("Claude".into());
    assert_eq!(caller(&client), "Claude");
}

#[test]
fn the_data_folder_is_under_the_home_or_the_windows_profile() {
    use std::ffi::OsString;
    use std::path::PathBuf;
    let env = |pairs: &'static [(&'static str, &'static str)]| {
        move |name: &str| pairs.iter().find(|(key, _)| *key == name).map(|(_, value)| OsString::from(value))
    };
    assert_eq!(data_dir_in(env(&[("HOME", "/h"), ("GRAFTA_JOURNAL_DIR", "/j")])), Some(PathBuf::from("/h/.grafta")));
    assert_eq!(data_dir_in(env(&[("USERPROFILE", "/p")])), Some(PathBuf::from("/p/.grafta")));
    assert_eq!(data_dir_in(env(&[])), None);
}

#[test]
fn the_plugin_may_reach_the_bridge_s_ports_and_nothing_else() {
    let manifest: Value = serde_json::from_str(include_str!("../../../figma-plugin/manifest.json")).expect("the plugin's manifest is JSON");
    let allowed: Vec<&str> = manifest["networkAccess"]["allowedDomains"].as_array().expect("allowedDomains").iter().filter_map(Value::as_str).collect();
    let ports: Vec<String> = bridge::PORTS.iter().map(|port| format!("ws://localhost:{port}")).collect();
    assert_eq!(allowed, ports);
    assert!(manifest["networkAccess"].get("devAllowedDomains").is_none(), "nothing else, even in development");
    assert!(manifest["networkAccess"]["reasoning"].as_str().is_some_and(|r| r.len() > 40), "Figma requires a reasoning");
}
