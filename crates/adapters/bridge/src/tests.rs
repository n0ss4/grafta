//! The bridge against a WebSocket client standing in for grafta's panel.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::*;

type Panel = WebSocketStream<TcpStream>;

const SECRET: &str = "a secret for the tests";

fn options() -> Options {
    Options { ports: vec![0], secret: SECRET.into(), ..Options::new(String::new()) }
}

fn start(options: Options) -> Bridge {
    Bridge::start(&tokio::runtime::Handle::current(), options).expect("a free port")
}

/// A connection to the bridge at `host`, with `origin` if any.
async fn connect_to(host: &str, port: u16, origin: Option<&str>) -> Result<Panel, WsError> {
    let mut request = format!("ws://{host}:{port}/").into_client_request().unwrap();
    if let Some(origin) = origin {
        request.headers_mut().insert("origin", origin.parse().unwrap());
    }
    let stream = TcpStream::connect((host.trim_matches(['[', ']']), port)).await.unwrap();
    let config = WebSocketConfig::default().max_message_size(None).max_frame_size(None);
    tokio_tungstenite::client_async_with_config(request, stream, Some(config)).await.map(|(socket, _)| socket)
}

async fn connect(bridge: &Bridge) -> Panel {
    connect_to("127.0.0.1", bridge.port(), Some(PANEL_ORIGIN)).await.expect("the panel connects")
}

/// The next text frame, as JSON.
async fn next(panel: &mut Panel) -> Value {
    loop {
        let frame = tokio::time::timeout(Duration::from_secs(5), panel.next()).await.expect("a frame in time");
        match frame.expect("the connection open").expect("a frame") {
            Message::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
            Message::Close(_) => panic!("closed"),
            _ => {}
        }
    }
}

async fn send(panel: &mut Panel, value: Value) {
    panel.send(Message::text(value.to_string())).await.unwrap();
}

/// Wait until the bridge lists `count` files.
async fn listed(bridge: &Bridge, count: usize) -> Vec<LiveFile> {
    for _ in 0..200 {
        let files = bridge.list();
        if files.len() == count {
            return files;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("{} listed, {count} wanted", bridge.list().len());
}

/// A panel connected, its hello read, and grafta allowed on "Checkout".
async fn allowed(bridge: &Bridge) -> (Panel, Handle) {
    let mut panel = connect(bridge).await;
    next(&mut panel).await;
    send(&mut panel, json!({ "allowed": true, "file": { "name": "Checkout", "editor": "figma" } })).await;
    let handle = listed(bridge, 1).await.remove(0).handle;
    (panel, handle)
}

/// `call` run off the runtime, as `grafta mcp` runs it.
fn calling(bridge: &Arc<Bridge>, handle: &Handle, args: Value) -> tokio::task::JoinHandle<Result<Data, LiveError>> {
    let (bridge, handle) = (Arc::clone(bridge), handle.clone());
    tokio::task::spawn_blocking(move || bridge.call(handle.as_str(), "read_node", &from_json(&args)))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_panel_is_greeted_and_its_file_listed_once_it_allows() {
    let bridge = start(options());
    let mut panel = connect(&bridge).await;
    let hello = next(&mut panel).await;
    assert_eq!(hello, json!({ "hello": { "name": NOBODY, "version": env!("CARGO_PKG_VERSION"), "secret": SECRET } }));
    assert!(bridge.list().is_empty(), "listed before it allowed");
    send(&mut panel, json!({ "allowed": true, "file": { "name": "Checkout", "editor": "figma" } })).await;
    let files = listed(&bridge, 1).await;
    assert_eq!(files[0].name, "Checkout");
    assert!(Handle::looks_in_figma(files[0].handle.as_str()), "{}", files[0].handle);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_is_carried_and_answered_in_the_tool_s_keys() {
    let bridge = Arc::new(start(options()));
    let (mut panel, handle) = allowed(&bridge).await;
    let waiting = calling(&bridge, &handle, json!({ "id": "12:34", "depth": 1 }));
    let call = next(&mut panel).await;
    assert_eq!((&call["tool"], &call["args"]), (&json!("read_node"), &json!({ "id": "12:34", "depth": 1 })));
    send(&mut panel, json!({ "id": call["id"], "result": { "id": "12:34", "type": "FRAME" }, "ms": 4 })).await;
    let answer = waiting.await.unwrap().unwrap();
    assert_eq!(to_json(&answer), json!({ "id": "12:34", "type": "FRAME" }));
}

#[tokio::test(flavor = "multi_thread")]
async fn figma_s_refusal_comes_back_in_its_words() {
    let bridge = Arc::new(start(options()));
    let (mut panel, handle) = allowed(&bridge).await;
    let waiting = calling(&bridge, &handle, json!({}));
    let call = next(&mut panel).await;
    let words = "in set_x: This property cannot be overridden in an instance";
    send(&mut panel, json!({ "id": call["id"], "error": words, "ms": 1 })).await;
    assert_eq!(waiting.await.unwrap(), Err(LiveError::Refused(words.into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn while_the_person_has_stopped_grafta_nothing_is_carried() {
    let bridge = Arc::new(start(options()));
    let (mut panel, handle) = allowed(&bridge).await;
    send(&mut panel, json!({ "stopped": true })).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(calling(&bridge, &handle, json!({})).await.unwrap(), Err(LiveError::Stopped));
    send(&mut panel, json!({ "stopped": false })).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let waiting = calling(&bridge, &handle, json!({}));
    let call = next(&mut panel).await;
    send(&mut panel, json!({ "id": call["id"], "result": {} })).await;
    assert!(waiting.await.unwrap().is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_unanswered_in_time_times_out_and_the_late_answer_is_dropped() {
    let bridge = Arc::new(start(Options { timeout: Duration::from_millis(200), ..options() }));
    let (mut panel, handle) = allowed(&bridge).await;
    let waiting = calling(&bridge, &handle, json!({}));
    let call = next(&mut panel).await;
    assert_eq!(waiting.await.unwrap(), Err(LiveError::TimedOut { seconds: 1 }));
    send(&mut panel, json!({ "id": call["id"], "result": {} })).await;
    let again = calling(&bridge, &handle, json!({}));
    let call = next(&mut panel).await;
    send(&mut panel, json!({ "id": call["id"], "result": { "second": true } })).await;
    assert_eq!(to_json(&again.await.unwrap().unwrap()), json!({ "second": true }));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_panel_leaving_mid_call_answers_it_gone_and_unlists_the_file() {
    let bridge = Arc::new(start(options()));
    let (mut panel, handle) = allowed(&bridge).await;
    let waiting = calling(&bridge, &handle, json!({}));
    next(&mut panel).await;
    drop(panel);
    assert_eq!(waiting.await.unwrap(), Err(LiveError::Gone));
    listed(&bridge, 0).await;
    assert_eq!(calling(&bridge, &handle, json!({})).await.unwrap(), Err(LiveError::NotOpen));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_answer_of_megabytes_comes_whole() {
    let bridge = Arc::new(start(options()));
    let (mut panel, handle) = allowed(&bridge).await;
    let waiting = calling(&bridge, &handle, json!({}));
    let call = next(&mut panel).await;
    let text = "x".repeat(3 * 1024 * 1024);
    send(&mut panel, json!({ "id": call["id"], "result": { "text": text } })).await;
    assert_eq!(to_json(&waiting.await.unwrap().unwrap())["text"].as_str().map(str::len), Some(text.len()));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_answer_past_the_bound_is_refused_with_the_bound_named() {
    let bridge = Arc::new(start(Options { max_message: 1024 * 1024, ..options() }));
    let (mut panel, handle) = allowed(&bridge).await;
    let waiting = calling(&bridge, &handle, json!({}));
    let call = next(&mut panel).await;
    let text = "x".repeat(2 * 1024 * 1024);
    drop(panel.send(Message::text(json!({ "id": call["id"], "result": { "text": text } }).to_string())).await);
    match waiting.await.unwrap() {
        Err(LiveError::Refused(words)) => assert!(words.contains("larger than 1 MB"), "{words}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_plain_request_gets_nothing() {
    let bridge = start(options());
    let mut stream = TcpStream::connect(("127.0.0.1", bridge.port())).await.unwrap();
    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").await.unwrap();
    let mut read = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut read)).await.unwrap().unwrap();
    assert!(read.is_empty(), "{}", String::from_utf8_lossy(&read));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_upgrade_from_a_web_page_or_from_no_page_is_refused() {
    let bridge = start(options());
    for origin in [Some("https://example.com"), Some("http://localhost:3000"), None] {
        let refused = connect_to("127.0.0.1", bridge.port(), origin).await;
        assert!(matches!(&refused, Err(WsError::Http(response)) if response.status() == StatusCode::FORBIDDEN), "{origin:?}");
    }
    assert!(bridge.list().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_panel_that_does_not_allow_this_grafta_is_never_listed() {
    let bridge = start(options());
    let mut panel = connect(&bridge).await;
    next(&mut panel).await;
    send(&mut panel, json!({ "allowed": false, "why": "the person did not allow it" })).await;
    let closed = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(Ok(frame)) = panel.next().await {
            if frame.is_close() {
                break;
            }
        }
    });
    closed.await.expect("closed by the bridge");
    assert!(bridge.list().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn only_figma_design_is_accepted() {
    let bridge = start(options());
    let mut panel = connect(&bridge).await;
    next(&mut panel).await;
    send(&mut panel, json!({ "allowed": true, "file": { "name": "Board", "editor": "figjam" } })).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(bridge.list().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_panel_is_told_the_ai_app_s_name_when_it_is_known() {
    let bridge = start(options());
    let mut panel = connect(&bridge).await;
    assert_eq!(next(&mut panel).await["hello"]["name"], NOBODY);
    bridge.set_caller("Claude Desktop");
    assert_eq!(next(&mut panel).await["hello"]["name"], "Claude Desktop");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_bridge_takes_the_next_port_and_the_panel_holds_both() {
    let first = start(options());
    // The first port taken, the next in the list is used: here any free one.
    let second = start(Options { ports: vec![first.port(), 0], ..options() });
    assert_ne!(second.port(), first.port());
    let (_one, a) = allowed(&first).await;
    let (_two, b) = allowed(&second).await;
    assert_eq!((first.list().len(), second.list().len()), (1, 1));
    drop((a, b));
}

#[tokio::test(flavor = "multi_thread")]
async fn it_listens_on_ipv6_too_as_figma_resolves_localhost_there() {
    let bridge = start(options());
    if std::net::TcpListener::bind("[::1]:0").is_err() {
        return; // no IPv6 on this machine
    }
    let mut panel = connect_to("[::1]", bridge.port(), Some(PANEL_ORIGIN)).await.expect("on ::1");
    assert!(next(&mut panel).await.get("hello").is_some());
}

#[test]
fn the_secret_is_made_once_and_kept() {
    let dir = std::env::temp_dir().join(format!("grafta-bridge-secret-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    let made = secret_in(&dir).unwrap();
    assert_eq!(made.len(), 64);
    assert_eq!(secret_in(&dir).unwrap(), made);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.join(SECRET_FILE)).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn no_port_free_says_which_were_tried() {
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();
    let error = bind(&[port]).expect_err("taken");
    assert!(error.to_string().contains(&port.to_string()), "{error}");
}
