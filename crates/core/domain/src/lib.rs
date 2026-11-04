#![deny(clippy::all)]
// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![cfg_attr(test, allow(clippy::disallowed_methods))]

//! The domain: open files and what they hold, the ports they reach the world
//! through, the products that read their nodes, and the values a call is
//! given and returns.
//!
//! A [`Session`] holds a decoded file between calls, because opening one
//! costs seconds; a [`Toolkit`] holds the sessions, the ports, the products
//! and the journal's replay, each given to it when it is made. Nothing here
//! speaks JSON or touches the disk: the adapters do, through the ports.

pub mod data;
pub mod error;
pub mod event;
pub mod font;
pub mod handle;
pub mod location;
pub mod port;
pub mod product;
pub use error::DomainError;
pub use handle::Handle;
pub use location::Location;

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use figfile::FigFile;
use indexmap::IndexMap;
use kiwi::Value;
use model::SceneGraph;
use schema::{Field, Guid, Wire, WireMut};

/// What lets a closed file's unsaved edits go: the journal, which the edge
/// writes. Bringing them back is the edge's own, after the call that opened
/// the file.
pub trait Recovery: Send + Sync {
    /// Let a closed file's edits go.
    fn discard(&self, kit: &Toolkit, session: &Session);
}

/// Every open file, by handle.
pub struct Toolkit {
    sessions: IndexMap<Handle, Session>,
    /// Where edits are journaled, so they survive a restart.
    /// `None` for a toolkit that forgets, as `grafta tool` and batches do.
    journal: Option<PathBuf>,
    /// True while a journal is being replayed, so the replay is not recorded
    /// a second time.
    replaying: bool,
    /// Where files are read and written, documents encoded and drawn.
    ports: port::Ports,
    /// The products it knows, copied into each session.
    products: product::Products,
    /// The journal's replay, for `open_file` and `close_file`.
    recovery: std::sync::Arc<dyn Recovery>,
    /// Whether each call keeps a whole copy of every file, to check that a
    /// failed call's rollback gives it back exactly. Tests turn it on.
    check_rollbacks: bool,
}

/// One open file.
pub struct Session {
    pub handle: Handle,
    pub path: PathBuf,
    /// The raw decoded message. Writes and raw reads need it (research D1).
    pub fig: FigFile,
    pub graph: SceneGraph,
    /// Node id → its index in `nodeChanges`, for raw reads.
    pub raw_index: IndexMap<Guid, usize>,
    /// Edits made since the file was opened. Cursors carry it, so a list paged
    /// before an edit cannot silently shift after one.
    pub edits: u32,
    /// Ids of the nodes this session created, in order.
    pub created: Vec<Guid>,
    /// Auto-layout frames the file already had, which grafta now lays out
    /// because it changed them or their children.
    pub laid_out: Vec<Guid>,
    /// The bytes the file was opened from, fingerprinted, when edits are
    /// journaled: a journal is only replayed onto the same bytes.
    pub base: Option<Base>,
    /// The next id and last sibling positions grafta handed out, so a graft
    /// does not scan the file for them.
    pub memo: model::author::Memo,
    /// Components the call being made changes, whose instances' cached
    /// expansions are dropped once it succeeds.
    pub stale_components: Vec<Guid>,
    /// Images a call has read from disk, embedded only once it succeeds
    ///: SHA-1 in hex, and the bytes.
    pub pending_images: Vec<(String, Vec<u8>)>,
    /// What the call being made changed in the file, to put back if it fails
    ///.
    pub undo: model::author::Undo,
    /// How many node changes `raw_index` covers, so appended ones can be
    /// indexed without reindexing the rest.
    indexed: usize,
    /// The bytes `new_file` wrote, while this session is the one that made the
    /// file: only those may be saved over, as they hold nothing of anyone's.
    pub made_empty: Option<Vec<u8>>,
    pub opened_at: u64,
    pub open_seconds: f64,
    /// How much the process's resident memory grew while opening, in MB.
    /// Approximate: an allocator that keeps pages makes a second open cheaper.
    pub opened_delta_mb: Option<u64>,
    /// The toolkit's ports, for a tool that holds the session alone.
    pub ports: port::Ports,
    /// The toolkit's products, as its ports.
    pub products: product::Products,
    /// Whether this session waits for its journal to be replayed (`true`) or
    /// discarded (`false`): set by the call that opened it, done after that
    /// call, outside its unit of work, so the replay is not the call's edit.
    pub recovery: Option<bool>,
}

/// The bytes a file was opened from, fingerprinted: a journal is replayed
/// only onto the same bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Base {
    pub bytes: u64,
    pub fnv: u64,
}

impl Base {
    #[must_use]
    pub fn of(data: &[u8]) -> Self {
        Self { bytes: data.len() as u64, fnv: fnv(data) }
    }
}

impl Toolkit {
    /// A toolkit on `ports`, knowing `products`, replaying through
    /// `recovery`; journaling every edit under `journal` when one is given.
    #[must_use]
    pub fn with(ports: port::Ports, products: product::Products, recovery: std::sync::Arc<dyn Recovery>, journal: Option<PathBuf>) -> Self {
        Self { sessions: IndexMap::new(), journal, replaying: false, ports, products, recovery, check_rollbacks: false }
    }

    /// Check every rollback against a whole copy of the file taken before the
    /// call: a test's way of finding a write the rollback missed. Costs a
    /// copy of each open file per call.
    pub fn check_rollbacks(&mut self) {
        self.check_rollbacks = true;
    }

    /// Whether rollbacks are checked against a whole copy.
    #[must_use]
    pub fn checks_rollbacks(&self) -> bool {
        self.check_rollbacks
    }

    /// Whether a journal is being replayed, so its calls are not recorded
    /// again.
    #[must_use]
    pub fn replaying(&self) -> bool {
        self.replaying
    }

    /// Mark a journal's replay as begun or ended.
    pub fn set_replaying(&mut self, replaying: bool) {
        self.replaying = replaying;
    }

    /// The ports this toolkit reaches the world through.
    #[must_use]
    pub fn ports(&self) -> &port::Ports {
        &self.ports
    }

    /// The products this toolkit knows.
    #[must_use]
    pub fn products(&self) -> product::Products {
        self.products
    }

    /// What replays and discards a file's unsaved edits.
    #[must_use]
    pub fn recovery(&self) -> std::sync::Arc<dyn Recovery> {
        self.recovery.clone()
    }

    /// The directory edits are journaled in, if they are.
    #[must_use]
    pub fn journal_dir(&self) -> Option<&Path> {
        self.journal.as_deref()
    }

    /// Open a file, or return the session that already holds it.
    pub fn open(&mut self, path: &Path) -> Result<&Session, ToolError> {
        let canonical = self
            .ports
            .store
            .canonical(path)
            .map_err(|error| DomainError::CannotOpen { path: path.into(), error: error.to_string() })?;
        let handle = Handle::for_path(&canonical);
        if !self.sessions.contains_key(&handle) {
            let session = Session::open(&canonical, handle.clone(), self.journal.is_some(), self.ports.clone(), self.products)?;
            self.sessions.insert(handle.clone(), session);
        }
        Ok(&self.sessions[&handle])
    }

    /// Hold an already decoded file, for callers (and tests) that did not read
    /// it from disk themselves.
    pub fn insert(&mut self, path: PathBuf, fig: FigFile) -> Result<&Session, ToolError> {
        let handle = Handle::for_path(&path);
        let graph = model::build_scene_graph(&fig)
            .map_err(|error| DomainError::NoGraph { path: path.clone(), error: error.to_string() })?;
        let session = Session::new(handle.clone(), path, fig, graph, 0.0, None, self.ports.clone(), self.products);
        self.sessions.insert(handle.clone(), session);
        Ok(&self.sessions[&handle])
    }

    /// [`Toolkit::insert`], with the fingerprint of the bytes it stands for,
    /// so that its edits are journaled as a file read from disk would be.
    pub fn insert_with_base(&mut self, path: PathBuf, fig: FigFile, base: Base) -> Result<&Session, ToolError> {
        let handle = self.insert(path, fig)?.handle.clone();
        let session = self.sessions.get_mut(&handle).expect("just inserted");
        session.base = Some(base);
        Ok(session)
    }

    pub fn session(&self, handle: &str) -> Result<&Session, ToolError> {
        self.sessions.get(handle).ok_or_else(|| unknown_handle(handle))
    }

    pub fn session_mut(&mut self, handle: &str) -> Result<&mut Session, ToolError> {
        self.sessions.get_mut(handle).ok_or_else(|| unknown_handle(handle))
    }

    pub fn close(&mut self, handle: &str) -> Result<Session, ToolError> {
        self.sessions.shift_remove(handle).ok_or_else(|| unknown_handle(handle))
    }

    pub fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.sessions.values()
    }

    pub fn sessions_mut(&mut self) -> impl Iterator<Item = &mut Session> {
        self.sessions.values_mut()
    }
}

fn unknown_handle(handle: &str) -> ToolError {
    DomainError::UnknownHandle { handle: handle.into() }.into()
}

/// A session as a call found it, beside the `Undo` that records what the call
/// changes in the file: what to set back if the call fails.
pub struct Before {
    edits: u32,
    created: Vec<Guid>,
    laid_out: Vec<Guid>,
    memo: model::author::Memo,
    images: usize,
}

impl Before {
    pub fn edits(&self) -> u32 {
        self.edits
    }
}

impl Session {
    /// Begin a call: nothing recorded yet, and what the session holds besides
    /// the file kept.
    pub fn begin(&mut self) -> Before {
        self.undo = model::author::Undo::default();
        Before {
            edits: self.edits,
            created: self.created.clone(),
            laid_out: self.laid_out.clone(),
            memo: self.memo.clone(),
            images: self.fig.images.len(),
        }
    }

    /// Put the session back as the call found it. The graph and the index
    /// are built again from the file only when the call changed it: a call
    /// refused before it wrote costs nothing.
    pub fn roll_back(&mut self, before: Before) -> Result<(), ToolError> {
        let changed = !self.undo.is_empty() || self.fig.images.len() != before.images;
        std::mem::take(&mut self.undo).restore(&mut self.fig);
        self.fig.images.truncate(before.images);
        self.edits = before.edits;
        self.created = before.created;
        self.laid_out = before.laid_out;
        self.memo = before.memo;
        self.stale_components.clear();
        self.pending_images.clear();
        if changed {
            self.graph = model::build_scene_graph(&self.fig)
                .map_err(|error| DomainError::NoDocument { path: self.path.clone(), error: error.to_string() })?;
            self.reindex();
        }
        Ok(())
    }

    fn open(path: &Path, handle: Handle, fingerprint: bool, ports: port::Ports, products: product::Products) -> Result<Self, ToolError> {
        let before = resident_mb();
        let started = Instant::now();
        let data = ports
            .store
            .read(path)
            .map_err(|error| DomainError::CannotRead { path: path.into(), error: error.to_string() })?;
        if !data.starts_with(b"PK") && !data.starts_with(b"fig-kiwi") {
            return Err(DomainError::NotAFig { path: path.into() }.into());
        }
        let fig = ports.codec.decode(&data).map_err(|error| DomainError::Undecodable { path: path.into(), error })?;
        let base = fingerprint.then(|| Base::of(&data));
        drop(data);
        let graph = model::build_scene_graph(&fig)
            .map_err(|error| DomainError::NoDocument { path: path.into(), error: error.to_string() })?;
        let seconds = started.elapsed().as_secs_f64();
        let delta = match (before, resident_mb()) {
            (Some(before), Some(after)) => Some(after.saturating_sub(before)),
            _ => None,
        };
        let mut session = Self::new(handle, path.to_path_buf(), fig, graph, seconds, delta, ports, products);
        session.base = base;
        Ok(session)
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        handle: Handle,
        path: PathBuf,
        fig: FigFile,
        graph: SceneGraph,
        open_seconds: f64,
        opened_delta_mb: Option<u64>,
        ports: port::Ports,
        products: product::Products,
    ) -> Self {
        let mut session = Self {
            handle,
            path,
            fig,
            graph,
            raw_index: IndexMap::new(),
            edits: 0,
            created: Vec::new(),
            laid_out: Vec::new(),
            base: None,
            memo: model::author::Memo::default(),
            stale_components: Vec::new(),
            pending_images: Vec::new(),
            undo: model::author::Undo::default(),
            indexed: 0,
            made_empty: None,
            opened_at: SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()),
            open_seconds,
            opened_delta_mb,
            ports,
            products,
            recovery: None,
        };
        session.reindex();
        session
    }

    /// Rebuild the id → raw position index, after node changes were removed.
    pub fn reindex(&mut self) {
        let changes = model::author::node_changes(&self.fig);
        self.raw_index =
            changes.iter().enumerate().filter_map(|(position, change)| Some((raw_id(change)?, position))).collect();
        self.indexed = changes.len();
    }

    /// Index node changes appended since the index was last brought up to
    /// date. Appending moves nothing, so what was indexed stays right.
    pub fn index_appended(&mut self) {
        let changes = model::author::node_changes(&self.fig);
        if changes.len() < self.indexed {
            return self.reindex();
        }
        for (position, change) in changes.iter().enumerate().skip(self.indexed) {
            if let Some(id) = raw_id(change) {
                self.raw_index.insert(id, position);
            }
        }
        self.indexed = changes.len();
    }

    /// Bring the graph up to date with nodes whose properties were edited,
    /// finding their node changes through the index.
    pub fn refresh(&mut self, ids: &[Guid]) {
        let changes = model::author::node_changes(&self.fig);
        let index = &self.raw_index;
        model::refresh_with(&mut self.graph, &self.fig, ids, &|id| index.get(&id).and_then(|&at| changes.get(at)));
    }

    /// Convert style node changes, new or changed, into the graph's styles
    ///.
    pub fn insert_styles(&mut self, ids: &[Guid]) {
        self.index_appended();
        let changes = model::author::node_changes(&self.fig);
        let index = &self.raw_index;
        model::insert_styles_with(&mut self.graph, &self.fig, ids, &|id| index.get(&id).and_then(|&at| changes.get(at)));
    }

    /// Add appended nodes to the graph and the index.
    pub fn insert_appended(&mut self, ids: &[Guid]) -> Result<(), model::ModelError> {
        self.index_appended();
        let changes = model::author::node_changes(&self.fig);
        let index = &self.raw_index;
        model::insert_appended_with(&mut self.graph, &self.fig, ids, &|id| index.get(&id).and_then(|&at| changes.get(at)))
    }

    /// A graft into this file that remembers what it handed out.
    pub fn graft(&mut self) -> Result<model::author::Graft<'_>, model::author::AuthorError> {
        Ok(model::author::Graft::resume_with(&mut self.fig, &mut self.memo, Some(&self.raw_index))?.recording(&mut self.undo))
    }

    /// A node change, to change in place.
    pub fn fields_mut(&mut self, id: Guid) -> Option<&mut kiwi::Fields> {
        let at = *self.raw_index.get(&id)?;
        let Value::Object(message) = &mut self.fig.node_changes else { return None };
        let Some(Value::Array(changes)) = message.field_mut(Field::NodeChanges) else { return None };
        let change = changes.get_mut(at)?;
        self.undo.changing(change);
        match change {
            Value::Object(fields) => Some(fields),
            _ => None,
        }
    }

    /// The node change a node id was decoded from.
    #[must_use]
    pub fn raw(&self, id: Guid) -> Option<&Value> {
        let position = *self.raw_index.get(&id)?;
        model::author::node_changes(&self.fig).get(position)
    }

    /// The file name, for messages.
    #[must_use]
    pub fn file_name(&self) -> String {
        self.path.file_name().map_or_else(|| self.path.display().to_string(), |n| {
            n.to_string_lossy().into_owned()
        })
    }
}

/// The id of a node change.
pub fn raw_id_of(change: &Value) -> Option<Guid> {
    raw_id(change)
}

fn raw_id(change: &Value) -> Option<Guid> {
    change.field(Field::Guid).and_then(Guid::read)
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// FNV-1a, 64-bit. A stable fingerprint, not a security hash.
#[must_use]
pub fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME))
}

/// The process's resident memory in MB, from `ps`, which every Unix has.
/// `None` where it is not available: then the figure is left out, not guessed.
#[must_use]
pub fn resident_mb() -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()?;
    let kb: u64 = String::from_utf8_lossy(&output.stdout).trim().parse().ok()?;
    Some(kb / 1024)
}

/// What went wrong, in words a model can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// A parameter was missing, of the wrong kind, or out of range.
    BadArgument(String),
    /// A file, handle, node or image that does not exist.
    NotFound(String),
    /// A request the toolkit will not carry out, and why.
    Refused(String),
    /// Something that should have worked did not.
    Failed(String),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadArgument(message)
            | Self::NotFound(message)
            | Self::Refused(message)
            | Self::Failed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ToolError {}

/// What a tool returns; the edge writes it as JSON.
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    Data(data::Data),
    /// A picture, with what the model should read beside it: for a drawn
    /// node, its size, scale and notes on what is approximate.
    Image { mime: &'static str, bytes: Vec<u8>, caption: Option<data::Data> },
}

