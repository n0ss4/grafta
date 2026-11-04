#![deny(clippy::all)]

//! FigJam: a board of stickies, shapes, connectors and tables.
//!
//! What a FigJam node shows is read from the layers FigJam generates for it,
//! in the plugin API's names: a sticky's or a shape's `text.characters`,
//! `fills`, `shapeType`, `authorName` and `authorVisible`; a connector's
//! `connectorStart`, `connectorEnd`, `connectorLineType` and caps; a table's
//! `numRows`, `numColumns` and its `cells`' text by row.

use kiwi::Value as Raw;
use model::{NodeType, SceneGraph, SceneNode};
use schema::{table_entries, Field, GeneratedLayer, Guid, PaintType, TableAxis, Wire, WireName};

use domain::product::{Applied, NodeBehavior, ProductProfile};
use domain::port::JsonText;
use domain::data;
use domain::data::{Data, Map};
use app::view::hex;
use domain::{Session, ToolError};

/// What FigJam does with what a sticky, a shape, a table or a connector takes.
const REGENERATED: &str = "FigJam generates its look again from these values when the file is opened";
/// And a sticky's or a shape's text, until then.
const REGENERATED_TEXT: &str =
    "FigJam generates its look again from these values when the file is opened; until then render_node lays its text out itself";

/// A FigJam node takes every key: what it does not own, it refuses.
fn whole(set: Result<Vec<String>, ToolError>, note: &'static str) -> Option<Result<Applied, ToolError>> {
    Some(set.map(|set| Applied { set, note: Some(note), whole: true }))
}

pub mod board;

pub struct FigJam;

impl ProductProfile for FigJam {
    fn product(&self) -> figfile::Product {
        figfile::Product::Whiteboard
    }
    fn name(&self) -> &'static str {
        "FigJam"
    }
    fn key(&self) -> &'static str {
        "whiteboard"
    }
    fn summary(&self, graph: &SceneGraph) -> Option<(&'static str, Data)> {
        let count = |t: NodeType| graph.nodes.values().filter(|n| n.node_type == t).count();
        Some((
            "board",
            data!({
                "stickies": count(NodeType::Sticky),
                "shapes_with_text": count(NodeType::ShapeWithText),
                "connectors": count(NodeType::Connector),
                "sections": count(NodeType::Section),
                "tables": count(NodeType::Table),
                "stamps": count(NodeType::Stamp),
                "widgets": count(NodeType::Widget),
            }),
        ))
    }
    fn behaviours(&self) -> &'static [&'static dyn NodeBehavior] {
        &[&Generated, &Connector, &Table]
    }
}

/// The layers FigJam generates for a node, each by its path.
fn generated(raw: &Raw) -> Vec<(Vec<String>, &Raw)> {
    let path = |o: &Raw| -> Option<Vec<String>> { o.field(Field::GuidPath)?.field(Field::Guids)?.as_array()?.iter().map(|v| Guid::read(v).map(|id| id.to_string())).collect() };
    raw.field(Field::NodeGenerationData)
        .and_then(|g| g.field(Field::Overrides))
        .and_then(Raw::as_array)
        .into_iter()
        .flatten()
        .filter_map(|o| Some((path(o)?, o)))
        .collect()
}

/// The generated layer `local`, below it the path `rest`.
fn layer<'a>(generated: &[(Vec<String>, &'a Raw)], local: GeneratedLayer, rest: &[String]) -> Option<&'a Raw> {
    generated.iter().find(|(p, _)| p.first().is_some_and(|f| f.parse::<Guid>().is_ok_and(|g| g.local_id == local.local_id())) && p[1..] == *rest).map(|(_, o)| *o)
}

fn characters(o: Option<&Raw>) -> Option<String> {
    o.and_then(|o| o.field(Field::TextData)).and_then(|t| t.field(Field::Characters)).and_then(Raw::as_str).map(str::to_string)
}

fn fills(o: Option<&Raw>) -> Data {
    let paints: Vec<Data> = o
        .and_then(|o| o.field(Field::FillPaints))
        .and_then(Raw::as_array)
        .into_iter()
        .flatten()
        .filter(|p| p.is(Field::Type, PaintType::Solid))
        .filter_map(|p| {
            let c = p.field(Field::Color)?;
            let channel = |k: Field| c.field(k).and_then(Raw::as_f32).unwrap_or(0.0);
            Some(data!({ "type": PaintType::Solid.as_wire(), "color": hex(model::Color { r: channel(Field::R), g: channel(Field::G), b: channel(Field::B), a: 1.0 }) }))
        })
        .collect();
    data!(paints)
}

fn cap(raw: &Raw, field: Field) -> Option<String> {
    raw.field(field).and_then(Raw::as_str).map(str::to_string)
}

fn object(value: Data) -> Option<Map> {
    match value {
        Data::Map(map) => Some(map),
        _ => None,
    }
}

/// A sticky or a shape with text: layers FigJam generates.
struct Generated;

impl NodeBehavior for Generated {
    fn apply(&self, session: &mut Session, id: Guid, properties: &mut Map) -> Option<Result<Applied, ToolError>> {
        whole(board::set(session, id, properties), REGENERATED_TEXT)
    }
    fn types(&self) -> &'static [NodeType] {
        &[NodeType::Sticky, NodeType::ShapeWithText]
    }
    fn decoded(&self) -> &'static [Field] {
        &[Field::NodeGenerationData, Field::DerivedImmutableFrameData, Field::ShapeWithTextType]
    }
    fn shows(&self, node: &SceneNode, raw: &Raw, _json: &dyn JsonText) -> Option<Map> {
        let generated = generated(raw);
        let text = layer(&generated, GeneratedLayer::Text, &[]);
        let mut out = data!({ "text": { "characters": characters(text).unwrap_or_default(), "fills": fills(text) }, "fills": fills(layer(&generated, GeneratedLayer::Background, &[])) });
        if let Some(shape) = cap(raw, Field::ShapeWithTextType) {
            out["shapeType"] = data!(shape);
        }
        if node.node_type == NodeType::Sticky {
            let author = layer(&generated, GeneratedLayer::Author, &[]);
            out["authorName"] = data!(characters(author).unwrap_or_default());
            out["authorVisible"] = data!(author.and_then(|a| a.field(Field::Visible)).and_then(Raw::as_bool) != Some(false));
        }
        object(out)
    }
}

/// A connector, from one node's magnet to another's.
struct Connector;

impl NodeBehavior for Connector {
    fn apply(&self, session: &mut Session, id: Guid, properties: &mut Map) -> Option<Result<Applied, ToolError>> {
        whole(board::set_connector(session, id, properties), REGENERATED)
    }
    fn types(&self) -> &'static [NodeType] {
        &[NodeType::Connector]
    }
    fn decoded(&self) -> &'static [Field] {
        &[Field::ConnectorStart, Field::ConnectorEnd, Field::ConnectorLineStyle, Field::ConnectorStartCap, Field::ConnectorEndCap]
    }
    fn shows(&self, _node: &SceneNode, raw: &Raw, _json: &dyn JsonText) -> Option<Map> {
        let end = |field: Field| {
            let e = raw.field(field)?;
            Some(data!({ "endpointNodeId": e.field(Field::EndpointNodeId).and_then(Guid::read).map(|id| id.to_string()), "magnet": e.field(Field::Magnet).and_then(Raw::as_str) }))
        };
        object(data!({
            "connectorStart": end(Field::ConnectorStart),
            "connectorEnd": end(Field::ConnectorEnd),
            "connectorLineType": cap(raw, Field::ConnectorLineStyle),
            "connectorStartStrokeCap": cap(raw, Field::ConnectorStartCap),
            "connectorEndStrokeCap": cap(raw, Field::ConnectorEndCap),
            "text": { "characters": characters(layer(&generated(raw), GeneratedLayer::Text, &[])).unwrap_or_default() },
        }))
    }
}

/// A table: rows and columns of generated cells.
struct Table;

impl NodeBehavior for Table {
    fn apply(&self, session: &mut Session, id: Guid, properties: &mut Map) -> Option<Result<Applied, ToolError>> {
        whole(board::set_table(session, id, properties), REGENERATED)
    }
    fn types(&self) -> &'static [NodeType] {
        &[NodeType::Table]
    }
    fn decoded(&self) -> &'static [Field] {
        &[Field::TableRowPositions, Field::TableColumnPositions, Field::TableRowHeights, Field::TableColumnWidths]
    }
    fn shows(&self, _node: &SceneNode, raw: &Raw, _json: &dyn JsonText) -> Option<Map> {
        let generated = generated(raw);
        let ordered = |axis: TableAxis| -> Vec<String> { table_entries(raw, axis).into_iter().map(|(_, id)| id.to_string()).collect() };
        let (rows, columns) = (ordered(TableAxis::Rows), ordered(TableAxis::Columns));
        let cells: Vec<Vec<String>> = rows
            .iter()
            .map(|row| columns.iter().map(|column| characters(layer(&generated, GeneratedLayer::Text, &[row.clone(), column.clone()])).unwrap_or_default()).collect())
            .collect();
        object(data!({ "numRows": rows.len(), "numColumns": columns.len(), "cells": cells }))
    }
}
