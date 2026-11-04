#![deny(clippy::all)]

//! The ports' adapters to the machine: its file system, `figfile`, the journal
//! on disk and `render`.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use figfile::{Difference, FigFile};
use model::{SceneGraph, SceneNode};
use render::contrast::Contrast;
use render::pdf::Pdf;
use render::{RenderError, Rendered, TextBox};
use schema::Guid;

use domain::port::{Codec, Creating, DocumentStore, ImageSource, Journal, Rasterizer};

/// The file system.
pub struct Disk;

impl DocumentStore for Disk {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn read_text(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn create(&self, path: &Path, bytes: &[u8]) -> Result<(), Creating> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path).map_err(Creating::Open)?;
        file.write_all(bytes).map_err(Creating::Write)
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        fs::write(path, bytes)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn canonical(&self, path: &Path) -> io::Result<PathBuf> {
        path.canonicalize()
    }
}

impl ImageSource for Disk {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
}

/// `figfile`, which reads and writes `.fig` bytes and touches no disk.
pub struct FigCodec;

impl Codec for FigCodec {
    fn decode(&self, bytes: &[u8]) -> Result<FigFile, String> {
        figfile::parse_fig(bytes).map_err(|error| error.to_string())
    }

    fn encode(&self, fig: &FigFile) -> Result<Vec<u8>, String> {
        figfile::write_fig(fig).map_err(|error| error.to_string())
    }

    fn compare(&self, left: &FigFile, right: &FigFile) -> Option<Difference> {
        figfile::compare(left, right)
    }
}

/// Journals as files, each synced before an edit is reported.
pub struct DiskJournal;

impl Journal for DiskJournal {
    fn append(&self, path: &Path, header: &str, entry: &str) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let new = !path.exists();
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        let mut text = String::new();
        if new {
            text.push_str(header);
            text.push('\n');
        }
        text.push_str(entry);
        text.push('\n');
        file.write_all(text.as_bytes())?;
        file.sync_data()
    }

    fn read(&self, path: &Path) -> Option<String> {
        fs::read_to_string(path).ok()
    }

    /// Through a temporary file renamed over the journal, so a stop halfway
    /// leaves the old journal or the new one, never half of either.
    fn rewrite(&self, path: &Path, text: &str) -> io::Result<()> {
        let temporary = path.with_extension("jsonl.tmp");
        let mut file = File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_data()?;
        fs::rename(temporary, path)
    }

    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        fs::write(path, text)
    }

    fn remove(&self, path: &Path) {
        let _ = fs::remove_file(path);
    }
}

/// Append `line` to the file at `path`, for the tests' record of failing
/// calls; a line that cannot be written is left out.
pub fn append_line(path: &Path, line: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// `render`, with the machine's fonts.
pub struct Render;

impl Rasterizer for Render {
    fn render(&self, fig: &FigFile, graph: &SceneGraph, id: Guid, scale: f32, max_size: u32) -> Result<Rendered, RenderError> {
        render::render(fig, graph, id, scale, max_size)
    }

    fn contrast(&self, fig: &FigFile, graph: &SceneGraph, id: Guid) -> Result<Vec<Contrast>, RenderError> {
        render::contrast::contrast(fig, graph, id)
    }

    fn pdf(&self, fig: &FigFile, graph: &SceneGraph, ids: &[Guid]) -> Result<Pdf, RenderError> {
        render::pdf::pdf(fig, graph, ids)
    }

    fn blank_png(&self, width: u32, height: u32, rgba: [f32; 4]) -> Result<Vec<u8>, RenderError> {
        render::blank_png(width, height, rgba)
    }

    fn measure_text(&self, fig: &FigFile, graph: &SceneGraph, node: &SceneNode) -> Option<TextBox> {
        render::measure_text(fig, graph, node)
    }

    fn faces(&self, family: &str) -> Option<Vec<String>> {
        render::text::faces(&render::text::fonts(), family)
    }
}
