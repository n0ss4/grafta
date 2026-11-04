//! Nodes written as one PDF, a page each, from the same drawing as a picture
//!.
//!
//! The SVG `render` rasterises is handed to krilla instead, which writes it as
//! vector paths and embeds the fonts that text grafta wrote is set in, so that
//! text stays text. Figma's own text is drawn from its stored glyph outlines
//!, so it is outlines here too. Blurs and other filters are rasterised by
//! krilla, as a PDF has no filter of its own.

use krilla::geom::Size;
use krilla::page::PageSettings;
use krilla::Document;
use krilla_svg::{SurfaceExt, SvgSettings};
pub use domain::port::Pdf;
use schema::Guid;

use crate::{draw, FigFile, RenderError, SceneGraph};

/// Said of a page whose drawing holds a filter.
pub const RASTERISED: &str = "a shadow or blur is drawn as an image at 288 ppi, with everything it covers, text included: a PDF has no filters";


/// Said of a page with a gradient inside opacity, a mask or a blend.
pub const GRADIENT_IN_GROUP: &str = "a gradient inside a layer with opacity, a mask or a blend comes out misplaced through Apple's renderer, Preview included: it reads right in Adobe Reader and others";

/// One page per node, each the node's own size in points, as Figma exports a
/// frame to PDF at 1×.
pub fn pdf(fig: &FigFile, graph: &SceneGraph, ids: &[Guid]) -> Result<Pdf, RenderError> {
    crate::on_own_stack(|| pdf_here(fig, graph, ids))
}

fn pdf_here(fig: &FigFile, graph: &SceneGraph, ids: &[Guid]) -> Result<Pdf, RenderError> {
    if ids.is_empty() {
        return Err(RenderError("no node to write".into()));
    }
    let options = resvg::usvg::Options { fontdb: crate::text::fonts(), ..resvg::usvg::Options::default() };
    let mut document = Document::new();
    let mut pages = Vec::new();
    for id in ids {
        let node = graph.node(*id).ok_or_else(|| RenderError(format!("unknown node {id}")))?;
        if node.width <= 0.0 || node.height <= 0.0 {
            return Err(RenderError(format!("{id} has no area to draw ({}×{})", node.width, node.height)));
        }
        let drawing = draw(fig, graph, node, 0.0, node.height, node.width, node.height);
        let tree = resvg::usvg::Tree::from_str(&drawing.svg, &options)
            .map_err(|error| RenderError(format!("the drawing of {id} did not parse: {error}")))?;
        let settings = PageSettings::from_wh(node.width, node.height)
            .ok_or_else(|| RenderError(format!("{id} is too large for a PDF page")))?;
        let size = Size::from_wh(node.width, node.height).expect("checked above");
        let mut page = document.start_page_with(settings);
        let mut surface = page.surface();
        surface
            .draw_svg(&tree, size, SvgSettings { embed_text: true, filter_scale: 4.0 })
            .ok_or_else(|| RenderError(format!("{id} could not be written to the PDF")))?;
        surface.finish();
        page.finish();
        let mut notes = drawing.notes;
        if drawing.svg.contains(" filter=\"url(#") {
            notes.push(RASTERISED.into());
        }
        if drawing.gradient_in_group {
            notes.push(GRADIENT_IN_GROUP.into());
        }
        pages.push((*id, notes));
    }
    let bytes = document.finish().map_err(|error| RenderError(format!("the PDF could not be written: {error:?}")))?;
    Ok(Pdf { bytes, pages })
}
