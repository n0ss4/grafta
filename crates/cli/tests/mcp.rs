//! `grafta mcp`, spoken to over stdio the way a model client does: initialise,
//! list the tools, call one. No fixture needed.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

struct Client {
    child: std::process::Child,
    reader: BufReader<std::process::ChildStdout>,
}

impl Client {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_grafta"))
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("grafta mcp starts");
        let reader = BufReader::new(child.stdout.take().unwrap());
        Self { child, reader }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let mut line = String::new();
            assert!(self.reader.read_line(&mut line).unwrap() > 0, "server closed stdout");
            let message: Value = serde_json::from_str(&line).expect("server writes JSON lines");
            if message["id"] == id {
                return message;
            }
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.child.kill().ok();
    }
}

fn initialised() -> Client {
    let mut client = Client::start();
    let init = client.request(
        1,
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "grafta-test", "version": "0" },
        }),
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "grafta", "{init}");
    assert!(init["result"]["capabilities"]["tools"].is_object(), "{init}");
    client.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    client
}

#[test]
fn a_client_sees_every_tool_with_a_schema() {
    let mut client = initialised();
    let list = client.request(2, "tools/list", json!({}));
    // Required from protocol 2026-07-28; Claude Code rejects a list without them.
    assert!(list["result"]["ttlMs"].is_u64(), "{}", list["result"]["ttlMs"]);
    assert_eq!(list["result"]["cacheScope"], "public");
    let tools = list["result"]["tools"].as_array().expect("a tool list");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    let expected: Vec<&str> = toolkit::tools::ALL.iter().map(|t| t.name()).collect();
    assert_eq!(names, expected);
    for tool in tools {
        assert_eq!(tool["inputSchema"]["type"], "object", "{tool}");
        assert!(tool["description"].as_str().is_some_and(|d| !d.is_empty()));
    }
}

#[test]
fn a_bad_call_comes_back_as_a_readable_tool_error() {
    let mut client = initialised();
    let reply = client.request(3, "tools/call", json!({ "name": "summarize", "arguments": {} }));
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    let text = reply["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("\"handle\""), "{text}");
}

#[test]
fn a_file_opened_in_one_call_is_still_open_in_the_next() {
    let Some(_) = testkit::fixture::bytes("gold-preview.fig") else { return };
    let path = testkit::fixture::dir().join("gold-preview.fig");
    let mut client = initialised();
    let opened = client.request(4, "tools/call", json!({ "name": "open_file", "arguments": { "path": path } }));
    let body: Value = serde_json::from_str(opened["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    let handle = body["handle"].as_str().unwrap();
    assert_eq!(body["nodes"], 3164);

    let listed = client.request(5, "tools/call", json!({ "name": "list_open", "arguments": {} }));
    assert!(listed["result"]["content"][0]["text"].as_str().unwrap().contains(handle));
}
