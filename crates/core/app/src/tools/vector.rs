//! Drawing a vector from SVG path data.
//!
//! Only the vector network is written. A spike opened in Figma on 2026-09-27
//! showed Figma deriving a vector's fill and stroke geometry from the network
//! itself, and 6,613 real vectors are stored that way. So nothing derived is
//! written, and nothing has to be kept in step when the vector is resized.

use kiwi::Value as Raw;
use model::path::Path;
use model::SceneNode;
use schema::{fields, object, Field, PaintType, StrokeAlign, StrokeCap, StrokeJoin, Wire, WireName};

use super::style::paints;
use super::write::{author_error, created, record, require_parent, Placement};
use crate::data::{data, Data, Map};
use crate::Command;
use crate::view::num;
use crate::{Output, Session, ToolError, Toolkit};

/// A vector's paths from the network it stores, at its size, in the plugin API's
/// `vectorPaths` shape: what it fills, and what it strokes, as `NONE` says, every
/// segment of it, open or closed.
pub fn drawn_paths(session: &Session, node: &SceneNode) -> Vec<(String, &'static str)> {
    let network = session.raw(node.id).and_then(|raw| raw.field(Field::VectorData)).and_then(|data| {
        let blob = session.fig.blobs.get(data.field(Field::VectorNetworkBlob)?.as_u32()? as usize)?;
        let size = data.field(Field::NormalizedSize);
        let stored = |axis: Field, own: f32| size.and_then(|s| s.field(axis)).and_then(Raw::as_f32).filter(|v| *v > 0.0).unwrap_or(own);
        let scale = (node.width / stored(Field::X, node.width), node.height / stored(Field::Y, node.height));
        Some((model::path::Network::decode(blob)?, scale))
    });
    let Some((network, scale)) = network else { return Vec::new() };
    let mut paths = Vec::new();
    if node.fills.iter().any(|fill| fill.visible) {
        if let Some((d, nonzero)) = network.fill_path(scale) {
            paths.push((d, if nonzero { "NONZERO" } else { "EVENODD" }));
        }
    }
    if node.strokes.iter().any(|stroke| stroke.visible) {
        let d = network.stroke_path(scale);
        if !d.is_empty() {
            paths.push((d, "NONE"));
        }
    }
    paths
}

/// A vector drawn from SVG path data, scaled to its size.
pub struct CreateVector {
    pub handle: String,
    pub parent: String,
    /// The path, already in the vector's own pixels.
    pub path: Path,
    pub nonzero: bool,
    pub size: (f32, f32),
    pub weight: f32,
    pub cap: StrokeCap,
    pub join: StrokeJoin,
    pub fills: Option<Data>,
    pub strokes: Option<Data>,
    pub name: String,
    pub at: (f32, f32),
}

impl Command for CreateVector {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let parent = require_parent(session, &self.parent, Placement::Vector)?;
        let asked: Map = [("fills", &self.fills), ("strokes", &self.strokes)]
            .into_iter()
            .filter_map(|(key, value)| Some((key.to_owned(), value.clone()?)))
            .collect();
        let asked = super::images::stage(session, &asked)?;
        let fills = match asked.get("fills") {
            Some(value) => paints(session, "fills", value)?,
            None => Raw::Array(Vec::new()),
        };
        // Figma's pen draws a black 1 px stroke; so does this, unless told.
        let strokes = match asked.get("strokes") {
            Some(value) => paints(session, "strokes", value)?,
            None if self.fills.is_some() => Raw::Array(Vec::new()),
            None => paints(session, "strokes", &black())?,
        };
        let (path, nonzero, (width, height), weight, cap, join, name, at) =
            (&self.path, self.nonzero, self.size, self.weight, self.cap, self.join, &self.name, self.at);

        let network = path.network_blob(nonzero);
        let mut graft = session.graft().map_err(author_error)?;
        let blob = graft.add_blob(network).map_err(author_error)?;
        let size = object([(Field::X, Raw::Float(width)), (Field::Y, Raw::Float(height))]);
        let fields = fields([
            (Field::StrokeWeight, Raw::Float(weight)),
            (Field::StrokeAlign, Raw::from(StrokeAlign::Center)),
            (Field::StrokeCap, cap.into()),
            (Field::StrokeJoin, join.into()),
            (Field::FillPaints, fills),
            (Field::StrokePaints, strokes),
            (Field::VectorData, object([(Field::VectorNetworkBlob, Raw::Uint(blob)), (Field::NormalizedSize, size)])),
        ]);
        let id = graft.add_vector_at(parent, name, (width, height), at, fields).map_err(author_error)?;
        super::images::commit(session);
        record(session, id)?;
        let mut out = created(session, id);
        out["size"] = data!({ "width": num(width), "height": num(height) });
        out["subpaths"] = data!(path.subpaths.len());
        Ok(Output::Data(out))
    }
}

/// Figma's pen's stroke: black, 1 px.
fn black() -> Data {
    Data::List(vec![Data::Map(Map::from([
        ("type".to_owned(), Data::from(PaintType::Solid.as_wire())),
        ("color".to_owned(), Data::from("#000000")),
    ]))])
}
