//! Where the toolkit reaches the world.
//!
//! A tool decides; reading and writing files, journaling, encoding a document,
//! drawing it, measuring a result and reading a file's own JSON go through
//! these seven ports. The disk, `figfile`, the journal on disk and `render`
//! are their adapters, and `serde_json` is the measure's and the JSON text's,
//! at the edge. An adapter's failure comes back as its own
//! error, and the toolkit says it in the words of `DomainError`. What drawing
//! returns is defined here, beside the port, and `render` produces it.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use figfile::{Difference, FigFile};
use model::{SceneGraph, SceneNode};
use schema::Guid;

use crate::data::Data;

/// Where documents and the other files a tool reads or writes are kept.
pub trait DocumentStore: Send + Sync {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn read_text(&self, path: &Path) -> io::Result<String>;
    /// Write a file where none is, failing if one is.
    fn create(&self, path: &Path, bytes: &[u8]) -> Result<(), Creating>;
    /// Write a file, replacing what is there.
    fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn remove(&self, path: &Path) -> io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    /// The path with every link and `..` resolved.
    fn canonical(&self, path: &Path) -> io::Result<PathBuf>;
}

/// Which step of [`DocumentStore::create`] failed.
#[derive(Debug)]
pub enum Creating {
    /// The file could not be made.
    Open(io::Error),
    /// It was made, and its bytes could not be written.
    Write(io::Error),
}

/// A document to bytes and back.
pub trait Codec: Send + Sync {
    fn decode(&self, bytes: &[u8]) -> Result<FigFile, String>;
    fn encode(&self, fig: &FigFile) -> Result<Vec<u8>, String>;
    /// Where two documents first differ, if they do.
    fn compare(&self, left: &FigFile, right: &FigFile) -> Option<Difference>;
}

/// Where the edits made to an open file are kept, by the journal's path.
pub trait Journal: Send + Sync {
    /// Append `entry`, after `header` when the journal is new, and make it
    /// last before returning.
    fn append(&self, path: &Path, header: &str, entry: &str) -> io::Result<()>;
    /// The journal's text, `None` when there is none to read.
    fn read(&self, path: &Path) -> Option<String>;
    /// Replace the journal's text in one step.
    fn rewrite(&self, path: &Path, text: &str) -> io::Result<()>;
    /// Write a journal set aside.
    fn write(&self, path: &Path, text: &str) -> io::Result<()>;
    /// Remove a journal, if there is one.
    fn remove(&self, path: &Path);
}

/// The bytes of an image a paint names by path.
pub trait ImageSource: Send + Sync {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn is_file(&self, path: &Path) -> bool;
}

/// What draws: pictures, contrast, PDFs, thumbnails, text measured, and the
/// faces of a family installed here.
pub trait Rasterizer: Send + Sync {
    fn render(&self, fig: &FigFile, graph: &SceneGraph, id: Guid, scale: f32, max_size: u32) -> Result<Rendered, RenderError>;
    fn contrast(&self, fig: &FigFile, graph: &SceneGraph, id: Guid) -> Result<Vec<Contrast>, RenderError>;
    fn pdf(&self, fig: &FigFile, graph: &SceneGraph, ids: &[Guid]) -> Result<Pdf, RenderError>;
    fn blank_png(&self, width: u32, height: u32, rgba: [f32; 4]) -> Result<Vec<u8>, RenderError>;
    fn measure_text(&self, fig: &FigFile, graph: &SceneGraph, node: &SceneNode) -> Option<TextBox>;
    /// The installed faces of `family`, `None` when it is not installed.
    fn faces(&self, family: &str) -> Option<Vec<String>>;
}

/// A file whose own format is JSON (a layout snapshot, the `meta.json` Figma
/// keeps in a `.fig`, a slide's speaker notes): its text read as `Data`, and
/// written from it, as `serde_json` reads and writes it.
pub trait JsonText: Send + Sync {
    /// The text as `Data`, or why it is not JSON.
    fn parse(&self, text: &str) -> Result<Data, String>;
    fn write(&self, data: &Data) -> String;
}

/// How big a value is as the model reads it: the bytes a result is cut to fit
/// a budget by.
pub trait Measure: Send + Sync {
    fn bytes(&self, data: &Data) -> usize;
}

/// The seven ports a toolkit and its sessions use.
#[derive(Clone)]
pub struct Ports {
    pub store: Arc<dyn DocumentStore>,
    pub codec: Arc<dyn Codec>,
    pub journal: Arc<dyn Journal>,
    pub images: Arc<dyn ImageSource>,
    pub raster: Arc<dyn Rasterizer>,
    pub measure: Arc<dyn Measure>,
    pub json: Arc<dyn JsonText>,
}

// ── What drawing returns ─────────────────────────────────────────────────────

/// A drawn node.
#[derive(Debug)]
pub struct Rendered {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// The scale the node was drawn at, after any reduction to fit `max_size`.
    pub scale: f32,
    /// What was drawn approximately or left out, in words.
    pub notes: Vec<String>,
}

#[derive(Debug)]
pub struct RenderError(pub String);

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RenderError {}

/// What `measure_text` returns.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBox {
    pub width: f32,
    pub height: f32,
    /// The font measured in, when the text's own is not installed.
    pub stand_in: Option<String>,
}

/// A text's contrast with what is behind it.
#[derive(Debug, Clone)]
pub struct Contrast {
    pub sample: TextSample,
    /// The lowest ratio over its colours and what is behind them; `None` when
    /// it cannot be measured, with the reason in `sample.unmeasurable`.
    pub ratio: Option<f32>,
    /// The ratio WCAG AA requires of it: 4.5, or 3 for large text.
    pub required: f32,
    /// The colour, and what is behind it, at the lowest ratio, as hex.
    pub foreground: Option<String>,
    pub background: Option<String>,
    /// Whether more than one colour is behind it, and the range of ratios.
    pub varies: Option<(f32, f32)>,
}

/// A text as it would have been drawn.
#[derive(Debug, Clone)]
pub struct TextSample {
    pub id: Guid,
    /// The innermost instance it is drawn in, whose layer it is.
    pub instance: Option<Guid>,
    pub name: String,
    pub characters: String,
    /// The text's transform into the checked node's space.
    pub at: [f32; 6],
    pub width: f32,
    pub height: f32,
    pub opacity: f32,
    /// Each colour it is drawn in, as RGBA 0–1, with that run's size and
    /// weight; empty when a fill is not a solid colour.
    pub runs: Vec<([f32; 4], f32, u32)>,
    /// Why it cannot be measured, when it cannot.
    pub unmeasurable: Option<String>,
}

/// A PDF of nodes, and what each page drew approximately.
#[derive(Debug)]
pub struct Pdf {
    pub bytes: Vec<u8>,
    /// Per page, in order: the node's id and its notes.
    pub pages: Vec<(Guid, Vec<String>)>,
}
