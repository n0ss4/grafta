#![deny(clippy::all)]

//! The ports' adapters in memory, so a tool, a save or a journal's replay is
//! tested with no file on disk.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use figfile::FigFile;
use model::{SceneGraph, SceneNode};
use schema::Guid;

use disk::{FigCodec, Render};
use domain::data::Data;
use domain::port::{Contrast, Creating, DocumentStore, ImageSource, Journal, Measure, Pdf, Ports, Rasterizer, RenderError, Rendered, TextBox};

/// Files by path. A document store and an image source both, so an image a
/// tool writes, as the import writes a `data:` URL's, is one a paint reads.
#[derive(Default)]
pub struct Memory {
    files: Mutex<HashMap<PathBuf, Vec<u8>>>,
}

impl Memory {
    /// A store holding `files`.
    pub fn with(files: impl IntoIterator<Item = (PathBuf, Vec<u8>)>) -> Self {
        Self { files: Mutex::new(files.into_iter().collect()) }
    }

    /// The bytes at `path`, if any.
    #[must_use]
    pub fn get(&self, path: &Path) -> Option<Vec<u8>> {
        self.files().get(path).cloned()
    }

    /// Every path held, sorted.
    #[must_use]
    pub fn paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self.files().keys().cloned().collect();
        paths.sort();
        paths
    }

    fn files(&self) -> MutexGuard<'_, HashMap<PathBuf, Vec<u8>>> {
        self.files.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn not_found(path: &Path) -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, format!("no file at {} in memory", path.display()))
}

impl DocumentStore for Memory {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.get(path).ok_or_else(|| not_found(path))
    }

    fn read_text(&self, path: &Path) -> io::Result<String> {
        String::from_utf8(DocumentStore::read(self, path)?)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "stream did not contain valid UTF-8"))
    }

    fn create(&self, path: &Path, bytes: &[u8]) -> Result<(), Creating> {
        let mut files = self.files();
        if files.contains_key(path) {
            return Err(Creating::Open(io::Error::new(io::ErrorKind::AlreadyExists, "File exists")));
        }
        files.insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.files().insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        self.files().remove(path).map(drop).ok_or_else(|| not_found(path))
    }

    fn exists(&self, path: &Path) -> bool {
        self.files().contains_key(path)
    }

    /// A path in memory has no links: it is its own real path, when held.
    fn canonical(&self, path: &Path) -> io::Result<PathBuf> {
        if self.exists(path) {
            Ok(path.to_path_buf())
        } else {
            Err(not_found(path))
        }
    }
}

impl ImageSource for Memory {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        DocumentStore::read(self, path)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.exists(path)
    }
}

/// Journals by path.
#[derive(Default)]
pub struct MemoryJournal {
    journals: Mutex<HashMap<PathBuf, String>>,
}

impl MemoryJournal {
    /// Every journal held, set aside ones included, by path, sorted.
    #[must_use]
    pub fn all(&self) -> Vec<(PathBuf, String)> {
        let mut all: Vec<(PathBuf, String)> = self.journals().iter().map(|(p, t)| (p.clone(), t.clone())).collect();
        all.sort();
        all
    }

    fn journals(&self) -> MutexGuard<'_, HashMap<PathBuf, String>> {
        self.journals.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Journal for MemoryJournal {
    fn append(&self, path: &Path, header: &str, entry: &str) -> io::Result<()> {
        let mut journals = self.journals();
        let text = journals.entry(path.to_path_buf()).or_insert_with(|| format!("{header}\n"));
        text.push_str(entry);
        text.push('\n');
        Ok(())
    }

    fn read(&self, path: &Path) -> Option<String> {
        self.journals().get(path).cloned()
    }

    fn rewrite(&self, path: &Path, text: &str) -> io::Result<()> {
        self.journals().insert(path.to_path_buf(), text.to_string());
        Ok(())
    }

    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        self.rewrite(path, text)
    }

    fn remove(&self, path: &Path) {
        self.journals().remove(path);
    }
}

/// `render`, with a fixed table of installed faces instead of the machine's.
/// Drawing is computation, so it is `render`'s own.
#[derive(Default)]
pub struct FixedFaces {
    faces: HashMap<String, Vec<String>>,
}

impl FixedFaces {
    /// Families and their faces, as a machine would have them installed.
    pub fn new<'a>(families: impl IntoIterator<Item = (&'a str, &'a [&'a str])>) -> Self {
        let faces = families
            .into_iter()
            .map(|(family, styles)| (family.to_lowercase(), styles.iter().map(ToString::to_string).collect()))
            .collect();
        Self { faces }
    }
}

impl Rasterizer for FixedFaces {
    fn render(&self, fig: &FigFile, graph: &SceneGraph, id: Guid, scale: f32, max_size: u32) -> Result<Rendered, RenderError> {
        Render.render(fig, graph, id, scale, max_size)
    }

    fn contrast(&self, fig: &FigFile, graph: &SceneGraph, id: Guid) -> Result<Vec<Contrast>, RenderError> {
        Render.contrast(fig, graph, id)
    }

    fn pdf(&self, fig: &FigFile, graph: &SceneGraph, ids: &[Guid]) -> Result<Pdf, RenderError> {
        Render.pdf(fig, graph, ids)
    }

    fn blank_png(&self, width: u32, height: u32, rgba: [f32; 4]) -> Result<Vec<u8>, RenderError> {
        Render.blank_png(width, height, rgba)
    }

    fn measure_text(&self, fig: &FigFile, graph: &SceneGraph, node: &SceneNode) -> Option<TextBox> {
        Render.measure_text(fig, graph, node)
    }

    /// As the machine's are matched: by the family's name, in any case.
    fn faces(&self, family: &str) -> Option<Vec<String>> {
        self.faces.get(&family.to_lowercase()).cloned()
    }
}

/// A value measured by its own writing, which is JSON's, byte for byte: the
/// measure with no JSON library behind it.
pub struct Written;

impl Measure for Written {
    fn bytes(&self, data: &Data) -> usize {
        data.to_string().len()
    }
}

/// Ports in memory: `store` holds documents and images, `journal` the
/// journals, `faces` the fonts installed. `figfile` is the codec, as on disk,
/// since it touches none; a value is measured by its own writing.
#[must_use]
pub fn ports(store: &Arc<Memory>, journal: &Arc<MemoryJournal>, faces: FixedFaces) -> Ports {
    Ports {
            store: store.clone(),
            codec: Arc::new(FigCodec),
            journal: journal.clone(),
            images: store.clone(),
            raster: Arc::new(faces),
            measure: Arc::new(Written),
            json: Arc::new(edge::measure::Text),
    }
}

