#![deny(clippy::all)]

//! grafta in a browser: the read tools, on bytes a page hands it.
//!
//! A driving adapter, as `grafta mcp` and `grafta tool` are: the same tools
//! from `ALL`, on the same domain, with memory for a disk. A page runs it in a
//! Web Worker and talks to it by messages. It makes no network request: it
//! reads only the bytes it is given, and what a tool writes stays in memory
//! until the page takes it.
//!
//! [`Reader`] is the adapter, testable natively; the functions exported to
//! JavaScript hold one and pass its errors on as grafta's own words.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use toolkit::port::{disk, memory, Ports};
use toolkit::{Output, ToolError, Toolkit};
use wasm_bindgen::prelude::*;

/// The tools a page may call: those that read. A browser saves no `.fig`, so
/// nothing that edits a file is offered; `export_tokens` writes its files into
/// memory, for [`Reader::take`].
pub const TOOLS: [&str; 9] = [
    "summarize",
    "list_screens",
    "search_nodes",
    "read_node",
    "list_components",
    "list_variables",
    "export_tokens",
    "list_styles",
    "check_contrast",
];
/// The tool [`Reader::picture`] calls.
const PICTURE: &str = "render_node";
/// Where an opened file's bytes are kept, in the reader's memory.
const FILES: &str = "/files";

/// grafta on memory, holding at most one open file.
pub struct Reader {
    kit: Toolkit,
    store: Arc<memory::Memory>,
    open: Option<(String, PathBuf)>,
}

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}

impl Reader {
    /// A reader with nothing open.
    #[must_use]
    pub fn new() -> Self {
        let store = Arc::new(memory::Memory::default());
        let ports = Ports {
            store: store.clone(),
            codec: Arc::new(disk::FigCodec),
            journal: Arc::new(memory::MemoryJournal::default()),
            images: store.clone(),
            raster: Arc::new(disk::Render),
            // As the server measures, so a result is cut where it is cut there.
            measure: Arc::new(toolkit::edge::measure::Json),
            json: Arc::new(toolkit::edge::measure::Text),
        };
        Self { kit: toolkit::with_ports(ports, None), store, open: None }
    }

    /// Open a file from its bytes, closing the one open before: `open_file`'s
    /// result, as JSON.
    pub fn open(&mut self, bytes: Vec<u8>, name: &str) -> Result<String, ToolError> {
        self.close();
        let path = Path::new(FILES).join(file_name(name));
        toolkit::port::DocumentStore::write(self.store.as_ref(), &path, &bytes)
            .map_err(|error| ToolError::Failed(format!("cannot hold {name} in memory: {error}")))?;
        let result = self.run("open_file", &serde_json::json!({ "path": path }));
        match &result {
            Ok(opened) => {
                let handle = opened["handle"].as_str().unwrap_or_default().to_owned();
                self.open = Some((handle, path));
            }
            Err(_) => drop(toolkit::port::DocumentStore::remove(self.store.as_ref(), &path)),
        }
        result.map(|value| value.to_string())
    }

    /// Call a read tool with its arguments as JSON: its result, as JSON.
    pub fn call(&mut self, tool: &str, args: &str) -> Result<String, ToolError> {
        if !TOOLS.contains(&tool) {
            return Err(ToolError::Refused(format!(
                "{tool} is not offered in a browser, which reads: {}; {PICTURE} through picture",
                TOOLS.join(", ")
            )));
        }
        self.run(tool, &arguments(args)?).map(|value| value.to_string())
    }

    /// Draw a node, with `render_node`'s arguments as JSON: the PNG, and its
    /// caption as JSON.
    pub fn picture(&mut self, args: &str) -> Result<(Vec<u8>, String), ToolError> {
        match toolkit::edge::call(&mut self.kit, PICTURE, &arguments(args)?)? {
            Output::Image { bytes, caption, .. } => {
                Ok((bytes, caption.map_or_else(|| "null".to_owned(), |c| toolkit::edge::data::to_json(&c).to_string())))
            }
            Output::Data(_) => Err(ToolError::Failed(format!("{PICTURE} drew no picture"))),
        }
    }

    /// The bytes of a file a tool wrote, forgotten once taken.
    pub fn take(&mut self, path: &str) -> Result<Vec<u8>, ToolError> {
        let path = Path::new(path);
        let bytes = self.store.get(path).ok_or_else(|| ToolError::NotFound(format!("no file at {} in memory", path.display())))?;
        drop(toolkit::port::DocumentStore::remove(self.store.as_ref(), path));
        Ok(bytes)
    }

    fn close(&mut self) {
        if let Some((handle, path)) = self.open.take() {
            drop(self.run("close_file", &serde_json::json!({ "handle": handle })));
            drop(toolkit::port::DocumentStore::remove(self.store.as_ref(), &path));
        }
    }

    fn run(&mut self, tool: &str, args: &Value) -> Result<Value, ToolError> {
        match toolkit::edge::call(&mut self.kit, tool, args)? {
            Output::Data(data) => Ok(toolkit::edge::data::to_json(&data)),
            Output::Image { .. } => Err(ToolError::Failed(format!("{tool} drew a picture: use picture"))),
        }
    }
}

/// Add the faces in a font file's bytes; pictures drawn after use them. How
/// many faces it held.
pub fn add_font(bytes: Vec<u8>) -> usize {
    render::text::add_font(bytes)
}

fn arguments(args: &str) -> Result<Value, ToolError> {
    serde_json::from_str(args).map_err(|error| ToolError::BadArgument(format!("the arguments are not JSON: {error}")))
}

/// The name a file is held under: its last part, so a name cannot reach out
/// of [`FILES`].
fn file_name(name: &str) -> String {
    let last = name.rsplit(['/', '\\']).next().unwrap_or_default();
    if last.is_empty() || last == "." || last == ".." { "file.fig".to_owned() } else { last.to_owned() }
}

// ── What JavaScript calls ────────────────────────────────────────────────────

thread_local! {
    static READER: RefCell<Reader> = RefCell::new(Reader::new());
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(message: &str);
}

/// A panic is a bug in grafta: said on the console in its own words before
/// the module stops, so it can be reported, not a bare "unreachable".
#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| console_error(&format!("grafta stopped: {info}"))));
}

fn js(error: ToolError) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Open a file from its bytes: `open_file`'s result, as JSON.
#[wasm_bindgen(js_name = open)]
pub fn js_open(bytes: Vec<u8>, name: &str) -> Result<String, JsValue> {
    READER.with_borrow_mut(|reader| reader.open(bytes, name)).map_err(js)
}

/// Call a read tool: its result, as JSON.
#[wasm_bindgen(js_name = call)]
pub fn js_call(tool: &str, args: &str) -> Result<String, JsValue> {
    READER.with_borrow_mut(|reader| reader.call(tool, args)).map_err(js)
}

/// A node drawn, with `render_node`'s arguments.
#[wasm_bindgen(js_name = picture)]
pub fn js_picture(args: &str) -> Result<Picture, JsValue> {
    READER.with_borrow_mut(|reader| reader.picture(args)).map(|(png, caption)| Picture { png, caption }).map_err(js)
}

/// Add a font from its bytes: how many faces it held.
#[wasm_bindgen(js_name = addFont)]
pub fn js_add_font(bytes: Vec<u8>) -> usize {
    add_font(bytes)
}

/// The bytes of a file a tool wrote, forgotten once taken.
#[wasm_bindgen(js_name = take)]
pub fn js_take(path: &str) -> Result<Vec<u8>, JsValue> {
    READER.with_borrow_mut(|reader| reader.take(path)).map_err(js)
}

/// A drawn node: the PNG and its caption, as JSON.
#[wasm_bindgen]
pub struct Picture {
    png: Vec<u8>,
    caption: String,
}

#[wasm_bindgen]
impl Picture {
    #[wasm_bindgen(getter)]
    pub fn png(&self) -> Vec<u8> {
        self.png.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn caption(&self) -> String {
        self.caption.clone()
    }
}

#[cfg(test)]
mod tests;
