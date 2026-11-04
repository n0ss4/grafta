#![deny(clippy::all)]

//! An MCP server over grafta's toolkit, on stdio.
//!
//! Nothing here knows what a tool does. It lists [`toolkit::tools::ALL`],
//! and passes each call to the tool of that name, so a tool added to the list
//! is served with no change here. The toolkit is synchronous
//! and a call can take seconds (opening a 25 MB file), so each call runs on
//! tokio's blocking pool rather than on the runtime's own thread.

use std::sync::{Arc, Mutex};

use rmcp::model::{
    CacheScope, CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    InitializeResult, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig,
    Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt};
use serde_json::Value;
use toolkit::app::data::Data;
use toolkit::{Output, Toolkit};

/// What a model is told about grafta before it calls anything.
const INSTRUCTIONS: &str = "grafta reads and writes Figma .fig files exactly, offline. It does \
    not interpret designs: you do. Start with open_file, then summarize, search_nodes and \
    read_node to find your way; read_node returns properties in the Figma plugin API's names, \
    leaving out values at Figma's defaults. Results are bounded: when something is left out, \
    the result says so under `omitted` and tells you how to get it. render_node shows you a \
    node as a picture, unsaved edits included. new_file starts an empty file, taking only the \
    format of a .fig you name. create_vector draws icons from SVG path data, create_shape rectangles, ellipses, rings, \
    lines, polygons and stars; create_component makes a frame reusable, create_variant_set \
    groups components as variants, add_component_property gives one a text, boolean or swap \
    property that each instance sets. create_variable_collection and create_variable write a design \
    system's tokens, with a value per mode and aliases; set_properties binds fills, spacing, radii \
    and sizes to them, and shows a frame in a mode (explicitVariableModes). list_styles and \
    create_style do the same for colour, text and effect styles, applied by name. \
    duplicate_node copies what you built; group_nodes and create_section organise it. \
    create_text writes rich text: runs styled apart (a bold word, a link), bulleted and \
    numbered paragraphs, and the layer's spacing, case, decoration and box. \
    To create, use \
    create_page, create_frame and \
    create_instance, check the result with render_node, then save_as to a new path, and have \
    a person open that file in Figma.";

/// How long a client may treat the tool list as fresh. The list only changes
/// when grafta is rebuilt, which restarts the server anyway.
pub const TOOL_LIST_TTL_MS: u64 = 60 * 60 * 1000;

/// The server: one toolkit, shared by every call.
#[derive(Clone)]
pub struct Grafta {
    kit: Arc<Mutex<Toolkit>>,
}

/// A server that forgets its edits when it stops.
impl Default for Grafta {
    fn default() -> Self {
        Self { kit: Arc::new(Mutex::new(toolkit::new())) }
    }
}

impl Grafta {
    /// A server whose edits survive it restarting: they are journaled under
    /// [`journal_dir`] and recovered when a file is opened again.
    #[must_use]
    pub fn new() -> Self {
        let kit = journal_dir().map_or_else(toolkit::new, toolkit::with_journal);
        Self { kit: Arc::new(Mutex::new(kit)) }
    }
}

/// Where the server journals edits: `$GRAFTA_JOURNAL_DIR`, or
/// `~/.grafta/journals`. Never beside the user's files.
#[must_use]
pub fn journal_dir() -> Option<std::path::PathBuf> {
    journal_dir_in(|name| std::env::var_os(name))
}

/// [`journal_dir`] over any environment. Windows usually sets no `HOME`, so
/// its home is `USERPROFILE`; without either there is no journal.
fn journal_dir_in(var: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<std::path::PathBuf> {
    var("GRAFTA_JOURNAL_DIR").map(std::path::PathBuf::from).or_else(|| {
        var("HOME")
            .or_else(|| var("USERPROFILE"))
            .map(|home| std::path::PathBuf::from(home).join(".grafta").join("journals"))
    })
}

/// Every tool in the toolkit's list, as MCP tool definitions.
#[must_use]
pub fn tool_definitions() -> Vec<Tool> {
    toolkit::tools::ALL
        .iter()
        .map(|tool| {
            let schema = match tool.input_schema() {
                Value::Object(map) => map,
                _ => serde_json::Map::new(),
            };
            Tool::new(tool.name(), tool.description(), Arc::new(schema))
        })
        .collect()
}

/// Run one call against a toolkit, as an MCP tool result. A tool's failure is a
/// tool error the model sees, never a protocol error it does not.
pub fn call(kit: &Mutex<Toolkit>, name: &str, arguments: Value) -> CallToolResult {
    let mut kit = match kit.lock() {
        Ok(kit) => kit,
        Err(poisoned) => poisoned.into_inner(),
    };
    match toolkit::edge::call(&mut kit, name, &arguments) {
        Ok(Output::Data(value)) => CallToolResult::success(vec![ContentBlock::text(written(&value))]),
        Ok(Output::Image { mime, bytes, caption }) => {
            let mut content = vec![ContentBlock::image(base64(&bytes), mime)];
            content.extend(caption.map(|caption| ContentBlock::text(written(&caption))));
            CallToolResult::success(content)
        }
        Err(error) => CallToolResult::error(vec![ContentBlock::text(error.to_string())]),
    }
}

impl ServerHandler for Grafta {
    fn get_info(&self) -> ServerConfig {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("grafta", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        // Protocol 2026-07-28 makes `ttlMs` and `cacheScope` required on a
        // list result, and Claude Code rejects the list without them. The
        // tools are fixed for the life of the binary and the same for every
        // user, so the list is public and can be cached for an hour.
        Ok(ListToolsResult::with_all_items(tool_definitions())
            .with_ttl_ms(TOOL_LIST_TTL_MS)
            .with_cache_scope(CacheScope::Public))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let kit = Arc::clone(&self.kit);
        let name = request.name.to_string();
        let arguments = Value::Object(request.arguments.unwrap_or_default());
        let result = tokio::task::spawn_blocking(move || call(&kit, &name, arguments))
            .await
            .map_err(|error| ErrorData::internal_error(format!("the tool panicked: {error}"), None))?;
        Ok(result.into())
    }
}

/// Serve on stdin and stdout until the client disconnects.
pub fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        let service = Grafta::new().serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;
        Ok(())
    })
}

/// Standard base64, for image content. Twenty lines rather than a dependency.
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// A result as the JSON the model reads.
fn written(data: &Data) -> String {
    serde_json::to_string(data).expect("a result is JSON")
}

#[cfg(test)]
mod tests;
