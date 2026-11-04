//! FigJam's failures: stickies, shapes, tables and connectors, and the layers
//! FigJam generates for them.

use std::fmt;

use schema::{Field, GeneratedLayer, Guid, WireName};

use super::Kind;

#[derive(Debug)]
pub enum FigJamError {
    /// A key a FigJam node of type `is` does not take.
    NotSet { key: String, is: &'static str, accepted: Vec<&'static str> },
    /// Both text and characters.
    TextAndCharacters,
    /// A size on a sticky, which FigJam sizes.
    SizeOnSticky { key: String },
    /// An alignment on a sticky, which FigJam aligns.
    AlignOnSticky,
    /// A node with no generated layers.
    NoGeneratedLayers { id: Guid },
    /// A node with no generated text layer.
    NoGeneratedText { id: Guid },
    /// A node without the generated layer that holds `field`.
    NoGeneratedLayer { id: Guid, layer: GeneratedLayer, field: Field },
    /// A row of more texts than the table has columns.
    RowTooLong { row: usize, texts: usize, columns: usize },
    /// A table with no rows or columns.
    NoRowsOrColumns { id: Guid },
    /// No order key after the table's last row.
    NoKeyAfterLastRow,
    /// A connector's end at a node the file does not have.
    EndpointNotInFile { key: String, node: String },
    /// A connector's end at a node on another page.
    EndpointOffPage { key: String, node: String },
}

impl FigJamError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NotSet { .. }
            | Self::TextAndCharacters
            | Self::SizeOnSticky { .. }
            | Self::AlignOnSticky
            | Self::RowTooLong { .. }
            | Self::EndpointNotInFile { .. }
            | Self::EndpointOffPage { .. } => Kind::BadArgument,
            Self::NoGeneratedLayers { .. }
            | Self::NoGeneratedText { .. }
            | Self::NoGeneratedLayer { .. }
            | Self::NoRowsOrColumns { .. } => Kind::Refused,
            Self::NoKeyAfterLastRow => Kind::Failed,
        }
    }
}

impl fmt::Display for FigJamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSet { key, is, accepted } => {
                write!(f, "`{key}` is not set on a FigJam {is}; it takes {}", accepted.join(", "))
            }
            Self::TextAndCharacters => f.write_str("give text or characters, not both"),
            Self::SizeOnSticky { key } => write!(f, "`{key}` is not set on a sticky: FigJam sizes it"),
            Self::AlignOnSticky => f.write_str("`textAlignHorizontal` is not set on a sticky: FigJam aligns it"),
            Self::NoGeneratedLayers { id } => write!(f, "{id} has no generated layers to write"),
            Self::NoGeneratedText { id } => write!(f, "{id} has no generated text layer :1"),
            Self::NoGeneratedLayer { id, layer, field } => {
                write!(f, "{id} has no generated layer :{} for {}", layer.local_id(), field.as_wire())
            }
            Self::RowTooLong { row, texts, columns } => {
                write!(f, "cells[{row}] has {texts} texts; the table has {columns} columns")
            }
            Self::NoRowsOrColumns { id } => write!(f, "{id} has no rows or columns to write"),
            Self::NoKeyAfterLastRow => f.write_str("no order key fits after the table's last row"),
            Self::EndpointNotInFile { key, node } => write!(f, "{key}.endpointNodeId: {node} is not in the file"),
            Self::EndpointOffPage { key, node } => {
                write!(f, "{key}.endpointNodeId: {node} is not on the connector's page")
            }
        }
    }
}
