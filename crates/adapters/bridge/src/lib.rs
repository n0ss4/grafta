#![deny(clippy::all)]

//! The bridge to the files open in Figma.
//!
//! grafta's panel, in Figma's desktop app, opens a WebSocket to this computer;
//! the bridge is its other end, inside `grafta mcp`. It listens on
//! `127.0.0.1` and `::1` alone (Figma resolves `localhost` to `::1`), on the
//! first free of five ports, so that several `grafta mcp` may run at once and
//! the panel holds a connection to each. It reaches nothing itself.
//!
//! A connection names its file once the person allows this grafta in the
//! panel; then each call is a JSON text frame carried to it and answered
//! there, in the tool's own result keys. Nothing of the file is kept here.

use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener as StdListener};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use domain::data::Data;
use domain::port::{LiveError, LiveFile, LiveFiles};
use domain::Handle;
use edge::data::{from_json, to_json};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::handshake::server::{Callback, ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

/// The ports the bridge may listen on, in order; the panel tries each.
pub const PORTS: [u16; 5] = [47823, 47824, 47825, 47826, 47827];
/// The `Origin` a panel's connection carries: a plugin's panel is a document
/// of no origin, so its browser sends `null`. A web page sends its own.
pub const PANEL_ORIGIN: &str = "null";
/// How long a call waits for Figma's answer.
pub const TIMEOUT: Duration = Duration::from_secs(30);
/// The largest message either side takes. A read of 2,707 nodes was 2.2 MB
/// (research M2); a drawing at 2× of a large frame is more.
pub const MAX_MESSAGE: usize = 64 * 1024 * 1024;
/// Who calls, when the AI app has not said.
pub const NOBODY: &str = "an app using grafta";
/// Where in grafta's data folder the secret is kept.
const SECRET_FILE: &str = "bridge-secret";

/// How a bridge listens and answers.
#[derive(Debug, Clone)]
pub struct Options {
    /// Tried in order; `0` takes any free port, as the tests do.
    pub ports: Vec<u16>,
    pub origin: String,
    /// What the panel remembers this install of grafta by.
    pub secret: String,
    pub timeout: Duration,
    pub max_message: usize,
}

impl Options {
    /// The bridge as `grafta mcp` runs it, with this install's secret.
    #[must_use]
    pub fn new(secret: String) -> Self {
        Self { ports: PORTS.to_vec(), origin: PANEL_ORIGIN.into(), secret, timeout: TIMEOUT, max_message: MAX_MESSAGE }
    }
}

/// The bridge, serving on a runtime until it is dropped with the runtime.
pub struct Bridge {
    shared: Arc<Shared>,
    port: u16,
}

struct Shared {
    options: Options,
    /// The AI app's name, sent in `hello` and again when it changes.
    caller: watch::Sender<String>,
    files: Mutex<Vec<Open>>,
    /// Call ids and connection numbers, both from one count.
    next: AtomicU64,
}

/// A connection whose panel allowed this grafta.
struct Open {
    connection: u64,
    handle: Handle,
    name: String,
    stopped: bool,
    out: mpsc::UnboundedSender<String>,
    waiting: HashMap<u64, SyncSender<Result<Data, LiveError>>>,
}

/// What a panel's frame asks of its connection.
enum Then {
    Keep,
    Close,
}

impl Bridge {
    /// Listen on the first free of `options.ports`, on both loopback
    /// addresses, and serve on `runtime`.
    pub fn start(runtime: &tokio::runtime::Handle, options: Options) -> io::Result<Self> {
        let (listeners, port) = bind(&options.ports)?;
        let (caller, _) = watch::channel(NOBODY.to_string());
        let shared = Arc::new(Shared { options, caller, files: Mutex::default(), next: AtomicU64::new(1) });
        let _inside = runtime.enter();
        for listener in listeners {
            let listener = TcpListener::from_std(listener)?;
            let shared = Arc::clone(&shared);
            runtime.spawn(async move {
                while let Ok((stream, _)) = listener.accept().await {
                    tokio::spawn(connection(Arc::clone(&shared), stream));
                }
            });
        }
        Ok(Self { shared, port })
    }

    /// The port it listens on.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Name the AI app that calls, as MCP's `clientInfo` gives it: panels
    /// already connected are told again when it changes.
    pub fn set_caller(&self, name: &str) {
        let name = if name.trim().is_empty() { NOBODY } else { name.trim() };
        self.shared.caller.send_if_modified(|now| {
            let changed = now != name;
            if changed {
                *now = name.to_string();
            }
            changed
        });
    }
}

/// Both loopback addresses on the first port of `ports` free on both. A
/// computer with no IPv6 is listened to on `127.0.0.1` alone.
fn bind(ports: &[u16]) -> io::Result<(Vec<StdListener>, u16)> {
    let mut last = io::Error::new(io::ErrorKind::InvalidInput, "no port to listen on");
    for &wanted in ports {
        let v4 = match StdListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, wanted))) {
            Ok(listener) => listener,
            Err(error) => {
                last = error;
                continue;
            }
        };
        let port = v4.local_addr()?.port();
        let mut listeners = vec![v4];
        match StdListener::bind(SocketAddr::from((Ipv6Addr::LOCALHOST, port))) {
            Ok(v6) => listeners.push(v6),
            Err(error) if error.kind() == io::ErrorKind::AddrNotAvailable => {}
            Err(error) => {
                last = error;
                continue;
            }
        }
        for listener in &listeners {
            listener.set_nonblocking(true)?;
        }
        return Ok((listeners, port));
    }
    Err(io::Error::new(last.kind(), format!("no free port among {ports:?} on this computer: {last}")))
}

/// One panel, from its upgrade to its leaving.
async fn connection(shared: Arc<Shared>, stream: TcpStream) {
    let config = WebSocketConfig::default()
        .max_message_size(Some(shared.options.max_message))
        .max_frame_size(Some(shared.options.max_message));
    let check = PanelOnly(shared.options.origin.clone());
    // A plain request, or an upgrade from elsewhere, gets nothing more.
    let Ok(socket) = tokio_tungstenite::accept_hdr_async_with_config(stream, check, Some(config)).await else { return };
    let (mut sink, mut frames) = socket.split();
    let number = shared.next.fetch_add(1, Ordering::Relaxed);
    let (out, mut outgoing) = mpsc::unbounded_channel::<String>();
    let mut caller = shared.caller.subscribe();
    let mut ended = None;
    let hello = shared.hello(&caller.borrow_and_update());
    if sink.send(Message::text(hello)).await.is_ok() {
        loop {
            tokio::select! {
                frame = frames.next() => match frame {
                    Some(Ok(Message::Text(text))) => {
                        if let Then::Close = shared.receive(number, &out, text.as_str()) {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        ended = Some(error);
                        break;
                    }
                },
                Some(text) = outgoing.recv() => {
                    if sink.send(Message::text(text)).await.is_err() {
                        break;
                    }
                }
                Ok(()) = caller.changed() => {
                    let hello = shared.hello(&caller.borrow_and_update());
                    if sink.send(Message::text(hello)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
    drop(sink.close().await);
    shared.leave(number, ended.as_ref());
}

/// The upgrade answered when it carries the panel's `Origin`, given here,
/// and refused otherwise.
struct PanelOnly(String);

impl Callback for PanelOnly {
    fn on_request(self, request: &Request, response: Response) -> Result<Response, ErrorResponse> {
        let given = request.headers().get("origin").and_then(|value| value.to_str().ok());
        if given == Some(self.0.as_str()) {
            return Ok(response);
        }
        let mut refusal = ErrorResponse::new(Some("only grafta's panel in Figma may connect".into()));
        *refusal.status_mut() = StatusCode::FORBIDDEN;
        Err(refusal)
    }
}

impl Shared {
    fn files(&self) -> MutexGuard<'_, Vec<Open>> {
        self.files.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn hello(&self, caller: &str) -> String {
        json!({ "hello": { "name": caller, "version": env!("CARGO_PKG_VERSION"), "secret": self.options.secret } }).to_string()
    }

    /// A frame from the panel on connection `number`.
    fn receive(&self, number: u64, out: &mpsc::UnboundedSender<String>, text: &str) -> Then {
        let Ok(message) = serde_json::from_str::<Value>(text) else { return Then::Keep };
        if let Some(allowed) = message.get("allowed").and_then(Value::as_bool) {
            return if allowed { self.allow(number, out, &message["file"]) } else { Then::Close };
        }
        if let Some(stopped) = message.get("stopped").and_then(Value::as_bool) {
            if let Some(open) = self.files().iter_mut().find(|open| open.connection == number) {
                open.stopped = stopped;
            }
            return Then::Keep;
        }
        if let Some(id) = message.get("id").and_then(Value::as_u64) {
            let waiting = self.files().iter_mut().find(|open| open.connection == number).and_then(|open| open.waiting.remove(&id));
            // An answer no call waits for any more came too late: dropped.
            if let Some(waiting) = waiting {
                drop(waiting.send(answer(&message)));
            }
        }
        Then::Keep
    }

    /// The panel allowed this grafta on `file`: listed from now on.
    fn allow(&self, number: u64, out: &mpsc::UnboundedSender<String>, file: &Value) -> Then {
        if file.get("editor").and_then(Value::as_str) != Some("figma") {
            return Then::Close;
        }
        let name = file.get("name").and_then(Value::as_str).unwrap_or("a file Figma did not name").to_string();
        let mut files = self.files();
        if let Some(open) = files.iter_mut().find(|open| open.connection == number) {
            open.name = name;
            return Then::Keep;
        }
        let mut seed = number;
        let handle = loop {
            let handle = Handle::in_figma(seed);
            if !files.iter().any(|open| open.handle == handle) {
                break handle;
            }
            seed = seed.wrapping_add(PORTS.len() as u64);
        };
        files.push(Open { connection: number, handle, name, stopped: false, out: out.clone(), waiting: HashMap::new() });
        Then::Keep
    }

    /// Connection `number` is gone: its file is no longer listed, and every
    /// call waiting on it is answered.
    fn leave(&self, number: u64, ended: Option<&WsError>) {
        let gone = {
            let mut files = self.files();
            files.iter().position(|open| open.connection == number).map(|at| files.remove(at))
        };
        let Some(open) = gone else { return };
        let error = match ended {
            Some(WsError::Capacity(_)) => LiveError::Refused(format!(
                "Figma's answer was larger than {} MB, the most grafta takes, and the panel's connection \
                 was lost with it. Open the panel again and ask for less: a node's children, or less depth",
                self.options.max_message / (1024 * 1024)
            )),
            _ => LiveError::Gone,
        };
        for (_, waiting) in open.waiting {
            drop(waiting.send(Err(error.clone())));
        }
    }
}

/// A panel's answer to a call: its result, or its words.
fn answer(message: &Value) -> Result<Data, LiveError> {
    if let Some(error) = message.get("error") {
        return Err(LiveError::Refused(error.as_str().map_or_else(|| error.to_string(), str::to_string)));
    }
    message
        .get("result")
        .map(from_json)
        .ok_or_else(|| LiveError::Refused("the panel answered with neither a result nor an error".into()))
}

impl LiveFiles for Bridge {
    fn list(&self) -> Vec<LiveFile> {
        self.shared.files().iter().map(|open| LiveFile { handle: open.handle.clone(), name: open.name.clone() }).collect()
    }

    /// Wait for Figma's answer; called off the runtime's own threads, as
    /// `grafta mcp` runs every call.
    fn call(&self, handle: &str, tool: &str, args: &Data) -> Result<Data, LiveError> {
        let (sender, answered) = sync_channel(1);
        let id = self.shared.next.fetch_add(1, Ordering::Relaxed);
        {
            let mut files = self.shared.files();
            let open = files.iter_mut().find(|open| open.handle.as_str() == handle).ok_or(LiveError::NotOpen)?;
            if open.stopped {
                return Err(LiveError::Stopped);
            }
            let frame = json!({ "id": id, "tool": tool, "args": to_json(args) }).to_string();
            open.out.send(frame).map_err(|_| LiveError::Gone)?;
            open.waiting.insert(id, sender);
        }
        let timeout = self.shared.options.timeout;
        match answered.recv_timeout(timeout) {
            Ok(answer) => answer,
            Err(RecvTimeoutError::Timeout) => {
                for open in self.shared.files().iter_mut() {
                    open.waiting.remove(&id);
                }
                Err(LiveError::TimedOut { seconds: timeout.as_secs().max(1) })
            }
            Err(RecvTimeoutError::Disconnected) => Err(LiveError::Gone),
        }
    }
}

/// This install's secret, kept in grafta's data folder `dir`: made the first
/// time, from the system's randomness, and readable by its owner alone.
pub fn secret_in(dir: &Path) -> io::Result<String> {
    let path = dir.join(SECRET_FILE);
    if let Ok(kept) = std::fs::read_to_string(&path) {
        if kept.trim().len() == 64 {
            return Ok(kept.trim().to_string());
        }
    }
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
    let secret: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    std::fs::create_dir_all(dir)?;
    write_private(&path, &secret)?;
    Ok(secret)
}

#[cfg(unix)]
fn write_private(path: &Path, text: &str) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    file.write_all(text.as_bytes())
}

#[cfg(not(unix))]
fn write_private(path: &Path, text: &str) -> io::Result<()> {
    std::fs::write(path, text)
}

#[cfg(test)]
mod tests;
