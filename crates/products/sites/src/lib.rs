#![deny(clippy::all)]

//! Figma Sites: a site's pages, each named by its route and holding one frame
//! per breakpoint, side by side. Inside a breakpoint everything is Design's.

use std::fmt;

use kiwi::Value as Raw;
use model::{NodeType, SceneGraph, SceneNode};
use schema::{Field, Guid, Wire};

use app::tools::write::Placement;
use domain::data;
use domain::data::{Data, Map};
use domain::port::JsonText;
use domain::product::{NodeBehavior, ProductProfile};
use domain::ToolError;

pub struct Sites;

impl ProductProfile for Sites {
    fn product(&self) -> figfile::Product {
        figfile::Product::Site
    }
    fn name(&self) -> &'static str {
        "Figma Sites"
    }
    fn key(&self) -> &'static str {
        "site"
    }
    fn summary(&self, graph: &SceneGraph) -> Option<(&'static str, Data)> {
        Some(("site_pages", data!(graph.nodes.values().filter(|n| n.node_type == NodeType::ResponsiveSet).count())))
    }
    fn behaviours(&self) -> &'static [&'static dyn NodeBehavior] {
        &[&Page, &Breakpoint]
    }
}

/// What Figma Sites refuses that a Design file would take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteError {
    /// Something put straight into a site page, beside its breakpoints.
    PageHoldsBreakpoints { what: &'static str, page: Guid },
}

impl fmt::Display for SiteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PageHoldsBreakpoints { what, page } => write!(
                f,
                "cannot put {what} in {page}: a site page holds its breakpoints alone, one frame per screen width; put it inside a breakpoint"
            ),
        }
    }
}

impl From<SiteError> for ToolError {
    fn from(error: SiteError) -> Self {
        ToolError::Refused(error.to_string())
    }
}

/// A page of the site (`RESPONSIVE_SET`), named by its route.
struct Page;

impl NodeBehavior for Page {
    fn types(&self) -> &'static [NodeType] {
        &[NodeType::ResponsiveSet]
    }
    /// A page holds its breakpoints alone: adding, removing or reordering
    /// them follows rules of Figma Sites not measured yet.
    fn admits(&self, page: Guid, placement: Placement) -> Option<Result<(), ToolError>> {
        Some(Err(SiteError::PageHoldsBreakpoints { what: placement.noun(), page }.into()))
    }
    /// Figma Sites places a page's breakpoints itself, side by side.
    fn lays_out_children(&self) -> bool {
        false
    }
}

/// A breakpoint: a frame in a site page, one per screen width, one of them
/// the page's primary.
struct Breakpoint;

impl NodeBehavior for Breakpoint {
    fn types(&self) -> &'static [NodeType] {
        &[NodeType::Frame]
    }
    fn decoded(&self) -> &'static [Field] {
        &[Field::IsPrimaryBreakpoint]
    }
    fn shows(&self, _node: &SceneNode, raw: &Raw, _json: &dyn JsonText) -> Option<Map> {
        (raw.field(Field::IsPrimaryBreakpoint).and_then(Raw::as_bool) == Some(true))
            .then(|| Map::from([("isPrimaryBreakpoint".to_owned(), data!(true))]))
    }
}

#[cfg(test)]
mod tests;
