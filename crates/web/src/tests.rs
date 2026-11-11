//! The reader on the website's sample: what a page gets back, natively.

use serde_json::Value;

use super::*;

/// The website's sample, grafta's own file: the one a visitor opens first.
const SAMPLE: &[u8] = include_bytes!("../../../web/public/sample/order.fig");
/// Its card, the one frame at the top of its page.
const ORDER: &str = "1735549286:1";

fn bytes() -> Vec<u8> {
    SAMPLE.to_vec()
}

fn opened(reader: &mut Reader) -> Value {
    serde_json::from_str(&reader.open(bytes(), "order.fig").unwrap()).unwrap()
}

#[test]
fn a_file_opens_from_its_bytes_and_reads_as_natively() {
    let mut reader = Reader::new();
    let opened = opened(&mut reader);
    assert_eq!(opened["file"], "order.fig", "{opened}");
    let handle = opened["handle"].as_str().unwrap();
    let summary: Value = serde_json::from_str(&reader.call("summarize", &format!(r#"{{"handle":"{handle}"}}"#)).unwrap()).unwrap();
    assert_eq!(summary["pages"][0]["name"], "Showcase", "{summary}");
}

#[test]
fn a_tool_that_writes_is_refused_by_name() {
    let mut reader = Reader::new();
    let handle = opened(&mut reader)["handle"].as_str().unwrap().to_owned();
    for tool in ["save_as", "set_properties", "delete_node", "open_file"] {
        let error = reader.call(tool, &format!(r#"{{"handle":"{handle}"}}"#)).unwrap_err();
        assert!(matches!(&error, ToolError::Refused(m) if m.starts_with(tool) && m.contains("summarize")), "{error}");
    }
}

#[test]
fn a_node_is_drawn_with_its_caption() {
    let mut reader = Reader::new();
    let handle = opened(&mut reader)["handle"].as_str().unwrap().to_owned();
    let (png, caption) = reader.picture(&format!(r#"{{"handle":"{handle}","id":"{ORDER}"}}"#)).unwrap();
    assert_eq!(png[..8], *b"\x89PNG\r\n\x1a\n");
    let caption: Value = serde_json::from_str(&caption).unwrap();
    assert!(caption["width"].as_f64().is_some_and(|w| w > 0.0), "{caption}");
}

#[test]
fn opening_another_file_closes_the_first() {
    let mut reader = Reader::new();
    let first = opened(&mut reader)["handle"].as_str().unwrap().to_owned();
    reader.open(bytes(), "again.fig").unwrap();
    let error = reader.call("summarize", &format!(r#"{{"handle":"{first}"}}"#)).unwrap_err();
    assert!(matches!(error, ToolError::NotFound(_)), "{error}");
    assert!(reader.store.get(Path::new("/files/order.fig")).is_none(), "its bytes are let go");
}

#[test]
fn what_is_not_a_figma_file_is_refused_in_grafta_words() {
    let mut reader = Reader::new();
    let error = reader.open(b"not a design".to_vec(), "notes.txt").unwrap_err();
    assert!(error.to_string().starts_with("notes.txt is not"), "named by the file, not where it is held: {error}");
    assert!(reader.store.get(Path::new("/files/notes.txt")).is_none(), "nothing kept");
}

#[test]
fn a_file_a_tool_wrote_is_taken_once() {
    let mut reader = Reader::new();
    toolkit::port::DocumentStore::write(reader.store.as_ref(), Path::new("/out/tokens.json"), b"{}").unwrap();
    assert_eq!(reader.take("/out/tokens.json").unwrap(), b"{}");
    assert!(matches!(reader.take("/out/tokens.json").unwrap_err(), ToolError::NotFound(_)));
}

#[test]
fn arguments_that_are_not_json_say_so() {
    let mut reader = Reader::new();
    let error = reader.call("summarize", "{handle").unwrap_err();
    assert!(matches!(&error, ToolError::BadArgument(m) if m.contains("not JSON")), "{error}");
}

#[test]
fn a_name_cannot_reach_out_of_the_files() {
    assert_eq!(file_name("../../etc/passwd"), "passwd");
    assert_eq!(file_name("C:\\Users\\a\\b.fig"), "b.fig");
    assert_eq!(file_name(".."), "file.fig");
}

#[test]
fn tokens_are_written_to_memory_and_taken() {
    let mut reader = Reader::new();
    let handle = opened(&mut reader)["handle"].as_str().unwrap().to_owned();
    let out: Value = serde_json::from_str(&reader.call("export_tokens", &format!(r#"{{"handle":"{handle}","dir":"/tokens"}}"#)).unwrap()).unwrap();
    let path = out["files"][0]["path"].as_str().unwrap().to_owned();
    let doc: Value = serde_json::from_slice(&reader.take(&path).unwrap()).unwrap();
    assert!(doc.as_object().is_some_and(|o| !o.is_empty()), "{doc}");
    assert!(reader.take(&path).is_err(), "taken once");
}

#[test]
fn a_page_lists_its_screens() {
    let mut reader = Reader::new();
    let opened = opened(&mut reader);
    let (handle, page) = (opened["handle"].as_str().unwrap(), opened["pages"][0]["id"].as_str().unwrap());
    let screens: Value = serde_json::from_str(&reader.call("list_screens", &format!(r#"{{"handle":"{handle}","page":"{page}"}}"#)).unwrap()).unwrap();
    assert_eq!(screens["results"][0]["id"], ORDER, "{screens}");
}
