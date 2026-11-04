//! A new file, from another file's format and none of its content.
//!
//! Every `.fig` carries the schema its message is encoded in, and grafta does
//! not invent one. So a new file takes its format from a
//! template the caller has: the schema, the container version, and the two
//! entries Figma refuses a file without (`meta.json`, `thumbnail.png`, A2).
//! Everything the template holds is dropped: every node change but the root,
//! every blob, every image. The thumbnail is replaced by a blank picture and
//! the file name by the new file's own.

use figfile::FigFile;
use model::author::Graft;
use schema::{Field, Wire, WireMut};

use crate::data::{data, Data};
use crate::Command;
use crate::error::NewFileError;
use crate::{DomainError, Output, ToolError, Toolkit};

/// Entries Figma refuses a file without, measured in A2.
const REQUIRED_ENTRIES: [&str; 2] = ["meta.json", "thumbnail.png"];

/// Size of the blank thumbnail when the template's `meta.json` does not give
/// one. 400 wide is what every measured file stores.
const THUMBNAIL: (u32, u32) = (400, 240);

/// A new, empty file, in the format of a template, written and opened.
pub struct NewFile {
    pub path: std::path::PathBuf,
    pub from: std::path::PathBuf,
    pub page: String,
}

impl Command for NewFile {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let (path, from, page) = (self.path.as_path(), self.from.as_path(), self.page.as_str());
        if path.extension().and_then(|e| e.to_str()) != Some("fig") {
            // Another product's file is not made from nothing.
            let other = path.extension().and_then(|e| e.to_str()).and_then(|e| kit.products().for_extension(e));
            return Err(if let Some(product) = other {
                NewFileError::NotFromNothing { path: path.into(), product: product.name() }.into()
            } else {
                DomainError::Expected {
                    parameter: "path".into(),
                    expected: "a path ending in .fig".into(),
                    got: path.display().to_string(),
                }
                .into()
            });
        }
        let ports = kit.ports().clone();
        let path = crate::Location::new_file(ports.store.as_ref(), path, None)?.path();
        let data = ports
            .store
            .read(from)
            .map_err(|error| NewFileError::CannotReadTemplate { path: from.into(), error: error.to_string() })?;
        let template = ports.codec.decode(&data).map_err(|error| NewFileError::NotATemplate { path: from.into(), error })?;
        drop(data);
        let name = path.file_stem().map_or_else(|| "Untitled".into(), |s| s.to_string_lossy().into_owned());
        let fig = blank(&ports, kit.products(), template, &name, page)?;

        let bytes = ports.codec.encode(&fig).map_err(|error| NewFileError::CannotEncode { error })?;
        let reread = ports.codec.decode(&bytes).map_err(|error| NewFileError::NotReadBack { error })?;
        if let Some(difference) = ports.codec.compare(&fig, &reread) {
            return Err(NewFileError::ChangedInTransit { at: difference.to_string() }.into());
        }
        ports
            .store
            .write(path, &bytes)
            .map_err(|error| DomainError::CannotWrite { path: path.into(), error: error.to_string() })?;

        let handle = kit.open(path)?.handle.to_string();
        kit.session_mut(&handle)?.made_empty = Some(bytes.clone());
        let session = kit.session(&handle)?;
        let mut out = super::read::summary(session);
        out["handle"] = data!(session.handle);
        out["created_from"] = data!(from.display().to_string());
        out["bytes"] = data!(bytes.len());
        Ok(Output::Data(out))
    }
}


/// `template`'s format with none of its content: the root, one page named
/// `page`, no blobs or images, `meta.json` naming `name`, a blank thumbnail.
pub fn blank(ports: &crate::port::Ports, products: crate::product::Products, mut template: FigFile, name: &str, page: &str) -> Result<FigFile, ToolError> {
    // A new file is a Design file, the registry's first, whatever product the
    // template was.
    template.product = products.first().product();
    let missing: Vec<&'static str> = REQUIRED_ENTRIES
        .iter()
        .copied()
        .filter(|entry| !template.other_entries.iter().any(|(n, _)| n == entry))
        .collect();
    if !missing.is_empty() {
        return Err(NewFileError::MissingEntries { missing }.into());
    }

    let failed = NewFileError::CannotEmpty;
    // `resume`, so a template grafta wrote into is continued rather than refused.
    let mut graft = Graft::resume(&mut template).map_err(failed)?;
    graft.prune(&[]).map_err(failed)?;
    graft.add_page(page).map_err(failed)?;
    // Blobs live in the message itself; `FigFile::blobs` is a copy read from
    // it, so both are emptied.
    if let kiwi::Value::Object(message) = &mut template.node_changes {
        if message.has(Field::Blobs) {
            message.set(Field::Blobs, kiwi::Value::Array(Vec::new()));
        }
    }
    template.blobs.clear();
    template.images.clear();

    let mut meta: Data = entry(&template, "meta.json")
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .and_then(|text| ports.json.parse(text).ok())
        .unwrap_or_else(|| data!({}));
    let size = &meta["client_meta"]["thumbnail_size"];
    let dimension = |key: &str, default: u32| size[key].as_u64().and_then(|n| u32::try_from(n).ok()).filter(|n| *n > 0).unwrap_or(default);
    let (width, height) = (dimension("width", THUMBNAIL.0), dimension("height", THUMBNAIL.1));
    let colour = &meta["client_meta"]["background_color"];
    let channel = |key: &str, default: f64| colour[key].as_f64().unwrap_or(default) as f32;
    let background = [channel("r", 0.96), channel("g", 0.96), channel("b", 0.96), channel("a", 1.0)];
    let thumbnail = ports
        .raster
        .blank_png(width, height, background)
        .map_err(|error| NewFileError::NoThumbnail { error: error.to_string() })?;
    meta["file_name"] = data!(name);
    if let Some(object) = meta.get_mut("client_meta").and_then(Data::as_map_mut) {
        // The template's canvas position means nothing in an empty file.
        object.remove("render_coordinates");
    }
    set_entry(&mut template, "meta.json", ports.json.write(&meta).into_bytes());
    set_entry(&mut template, "thumbnail.png", thumbnail);
    Ok(template)
}

fn entry<'a>(fig: &'a FigFile, name: &str) -> Option<&'a [u8]> {
    fig.other_entries.iter().find(|(n, _)| n == name).map(|(_, bytes)| bytes.as_slice())
}

fn set_entry(fig: &mut FigFile, name: &str, bytes: Vec<u8>) {
    if let Some((_, existing)) = fig.other_entries.iter_mut().find(|(n, _)| n == name) {
        *existing = bytes;
    }
}
