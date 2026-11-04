//! Writing what a FigJam sticky or shape with text shows, where
//! it sits, a table's cells and where a connector runs.
//!
//! Neither has children: FigJam generates their layers from
//! `nodeGenerationData` (`:0` the background, `:1` the text) and caches its own
//! result in `derivedImmutableFrameData`. A change is written to the generated
//! layer's own values, and the cache is dropped for FigJam to generate again,
//! as Figma derives an instance again when its cache is gone.

use kiwi::Value as Raw;
use model::NodeType;
use schema::{object, table_entries, ConnectorLineStyle, ConnectorMagnet, Field, GeneratedLayer, Guid, NumberUnits, StrokeCap, TableAxis, TextAlignHorizontal, Wire, WireMut, WireName};
use domain::data::{Data, Map};

use app::tools::style::invalid;
use domain::error::FigJamError;
use domain::{DomainError, Session, ToolError};

/// What `set_properties` takes on a sticky or a shape with text.
pub const ACCEPTED: [&str; 9] = ["text", "characters", "fills", "x", "y", "width", "height", "fontSize", "textAlignHorizontal"];

/// What `set_properties` takes on a table, and on a connector.
pub const ACCEPTED_ON_TABLE: [&str; 3] = ["cells", "x", "y"];
pub const ACCEPTED_ON_CONNECTOR: [&str; 5] = ["connectorStart", "connectorEnd", "connectorStartStrokeCap", "connectorEndStrokeCap", "connectorLineType"];

/// A connector's end caps and lines, as the file's schema lists them
/// (`StrokeCap`, `ConnectorLineStyle`) and the plugin API names them.
const CAPS: [StrokeCap; 8] = [
    StrokeCap::None, StrokeCap::Round, StrokeCap::Square, StrokeCap::ArrowLines, StrokeCap::ArrowEquilateral, StrokeCap::TriangleFilled,
    StrokeCap::DiamondFilled, StrokeCap::CircleFilled,
];
const LINES: [ConnectorLineStyle; 3] = [ConnectorLineStyle::Straight, ConnectorLineStyle::Elbowed, ConnectorLineStyle::Curved];
const ALIGNS: [TextAlignHorizontal; 3] = [TextAlignHorizontal::Left, TextAlignHorizontal::Center, TextAlignHorizontal::Right];

/// Write `characters` into the text layer and `fills` into the background,
/// then drop FigJam's cached result. Returns the keys set.
pub fn set(session: &mut Session, id: Guid, properties: &Map) -> Result<Vec<String>, ToolError> {
    if let Some(unknown) = properties.keys().find(|k| !ACCEPTED.contains(&k.as_str())) {
        return Err(not_set(session, id, unknown, &ACCEPTED));
    }
    // Validate every value before writing any.
    // `text: { characters }`, as the plugin API sets a sticky's text, or
    // `characters` for short.
    // `text: { characters?, fills? }`, as the plugin API sets a sticky's
    // text and its colour.
    let given = match (properties.get("text"), properties.get("characters")) {
        (Some(_), Some(_)) => return Err(FigJamError::TextAndCharacters.into()),
        (Some(text), None) => {
            if text.get("characters").is_none() && text.get("fills").is_none() {
                return Err(invalid("text", "{ \"characters\"?, \"fills\"? }", text));
            }
            text.get("characters")
        }
        (None, given) => given,
    };
    let text_fills = properties.get("text").and_then(|t| t.get("fills")).map(|v| app::tools::style::paints(session, "text.fills", v)).transpose()?;
    let text = match given {
        Some(Data::String(s)) => Some(app::tools::text::text_data(s)),
        Some(other) => return Err(invalid("characters", "text", other)),
        None => None,
    };
    let fills = properties.get("fills").map(|v| app::tools::style::paints(session, "fills", v)).transpose()?;
    let place = place(properties)?;
    // A shape takes a size; FigJam sizes a sticky itself.
    let size = |key: &str| -> Result<Option<f32>, ToolError> {
        match properties.get(key) {
            None => Ok(None),
            Some(_) if session.graph.node(id).is_some_and(|n| n.node_type == NodeType::Sticky) => {
                Err(FigJamError::SizeOnSticky { key: key.into() }.into())
            }
            Some(v) => match v.as_f64() {
                Some(n) if n > 0.0 => Ok(Some(n as f32)),
                _ => Err(invalid(key, "a number above 0", v)),
            },
        }
    };
    let (width, height) = (size("width")?, size("height")?);
    // A shape's text: its size, and how it sits across the shape.
    let font_size = size("fontSize")?;
    let align = match properties.get("textAlignHorizontal") {
        None => None,
        Some(_) if session.graph.node(id).is_some_and(|n| n.node_type == NodeType::Sticky) => {
            return Err(FigJamError::AlignOnSticky.into())
        }
        Some(Data::String(a)) if TextAlignHorizontal::listed(a, &ALIGNS).is_some() => TextAlignHorizontal::listed(a, &ALIGNS),
        Some(other) => return Err(invalid("textAlignHorizontal", "LEFT, CENTER or RIGHT", other)),
    };

    let fields = fields_of(session, id)?;
    if fields.field(Field::NodeGenerationData).is_none() {
        return Err(FigJamError::NoGeneratedLayers { id }.into());
    }
    let mut set = move_to(fields, place);
    if let Some(Raw::Object(box_)) = fields.field_mut(Field::Size) {
        for (key, field, value) in [("width", Field::X, width), ("height", Field::Y, height)] {
            if let Some(value) = value {
                box_.set(field, Raw::Float(value));
                set.push(key.to_string());
            }
        }
    }
    let Some(Raw::Array(layers)) = (match fields.field_mut(Field::NodeGenerationData) {
        Some(Raw::Object(generation)) => generation.field_mut(Field::Overrides),
        _ => None,
    }) else {
        return Err(FigJamError::NoGeneratedLayers { id }.into());
    };
    if font_size.is_some() || align.is_some() {
        let layer = layers.iter_mut().find(|layer| generated_layer(layer) == Some(GeneratedLayer::Text)).ok_or(FigJamError::NoGeneratedText { id })?;
        if let Raw::Object(layer) = layer {
            if let Some(size) = font_size {
                layer.set(Field::FontSize, Raw::Float(size));
                // The line FigJam gives a shape's text: a third more than its size.
                layer.set(Field::LineHeight, object([(Field::Value, Raw::Float((size * 4.0 / 3.0).round())), (Field::Units, Raw::from(NumberUnits::Pixels))]));
                set.push("fontSize".to_string());
            }
            if let Some(align) = align {
                layer.set(Field::TextAlignHorizontal, align.into());
                set.push("textAlignHorizontal".to_string());
            }
        }
    }
    for (local, field, value) in [(GeneratedLayer::Background, Field::FillPaints, fills), (GeneratedLayer::Text, Field::TextData, text), (GeneratedLayer::Text, Field::FillPaints, text_fills)] {
        let Some(value) = value else { continue };
        let layer = layers
            .iter_mut()
            .find(|layer| generated_layer(layer) == Some(local))
            .ok_or(FigJamError::NoGeneratedLayer { id, layer: local, field })?;
        if let Raw::Object(layer) = layer {
            layer.set(field, value);
        }
        set.push(match (local, field) {
            (_, Field::TextData) => "characters".to_string(),
            (GeneratedLayer::Text, _) => "text.fills".to_string(),
            _ => "fills".to_string(),
        });
    }
    // FigJam's result is of the old values: it generates the new one.
    if set.iter().any(|k| k != "x" && k != "y") || set.is_empty() {
        fields.take(Field::DerivedImmutableFrameData);
    }
    session.refresh(&[id]);
    session.edits += 1;
    Ok(set)
}

/// The local id of a generated layer's first `guidPath` id, for a path of one.
fn generated_layer(layer: &Raw) -> Option<GeneratedLayer> {
    let guids = layer.field(Field::GuidPath)?.field(Field::Guids)?.as_array()?;
    match guids.as_slice() {
        [only] => Guid::read(only).and_then(|id| GeneratedLayer::of(id.local_id)),
        _ => None,
    }
}

/// `x` and `y`, in the parent's coordinates, validated.
fn place(properties: &Map) -> Result<(Option<f32>, Option<f32>), ToolError> {
    let number = |key: &str| -> Result<Option<f32>, ToolError> {
        match properties.get(key) {
            None => Ok(None),
            Some(v) => v.as_f64().map(|n| Some(n as f32)).ok_or_else(|| invalid(key, "a number", v)),
        }
    };
    Ok((number("x")?, number("y")?))
}

/// Move a node change to `place`; returns the keys set.
fn move_to(fields: &mut kiwi::Fields, place: (Option<f32>, Option<f32>)) -> Vec<String> {
    let mut set = Vec::new();
    if let Some(Raw::Object(transform)) = fields.field_mut(Field::Transform) {
        for (key, field, value) in [("x", Field::M02, place.0), ("y", Field::M12, place.1)] {
            if let Some(value) = value {
                transform.set(field, Raw::Float(value));
                set.push(key.to_string());
            }
        }
    }
    set
}

/// A node change, to change in place.
fn fields_of(session: &mut Session, id: Guid) -> Result<&mut kiwi::Fields, ToolError> {
    session.fields_mut(id).ok_or_else(|| DomainError::NotInFile { id }.into())
}

fn unknown(session: &Session, id: Guid, properties: &Map, accepted: &[&'static str]) -> Result<(), ToolError> {
    match properties.keys().find(|k| !accepted.contains(&k.as_str())) {
        Some(key) => Err(not_set(session, id, key, accepted)),
        None => Ok(()),
    }
}

/// `key`, which the FigJam node `id` does not take.
fn not_set(session: &Session, id: Guid, key: &str, accepted: &[&'static str]) -> ToolError {
    let is = app::view::type_name(session.graph.node(id).map_or(NodeType::Unknown, |n| n.node_type));
    FigJamError::NotSet { key: key.into(), is, accepted: accepted.to_vec() }.into()
}

/// A table's cells as `cells` gives them: rows of texts, the first its header.
/// The rows given replace the table's from the top; rows past the table's last
/// are added, each as tall as the last. A row has at most the table's columns.
pub fn set_table(session: &mut Session, id: Guid, properties: &Map) -> Result<Vec<String>, ToolError> {
    unknown(session, id, properties, &ACCEPTED_ON_TABLE)?;
    let place = place(properties)?;
    let cells: Option<Vec<Vec<String>>> = match properties.get("cells") {
        None => None,
        Some(Data::List(rows)) => Some(
            rows.iter()
                .enumerate()
                .map(|(r, row)| match row {
                    Data::List(texts) => texts
                        .iter()
                        .enumerate()
                        .map(|(c, t)| match t {
                            Data::String(s) => Ok(s.clone()),
                            Data::Number(n) => Ok(n.to_string()),
                            other => Err(invalid(&format!("cells[{r}][{c}]"), "text", other)),
                        })
                        .collect(),
                    other => Err(invalid(&format!("cells[{r}]"), "a row of texts", other)),
                })
                .collect::<Result<_, _>>()?,
        ),
        Some(other) => return Err(invalid("cells", "rows of texts", other)),
    };
    let fields = fields_of(session, id)?;
    let mut set = move_to(fields, place);
    if let Some(cells) = cells {
        let columns = table_entries(fields, TableAxis::Columns);
        let mut rows = table_entries(fields, TableAxis::Rows);
        if let Some((r, row)) = cells.iter().enumerate().find(|(_, row)| row.len() > columns.len()) {
            return Err(FigJamError::RowTooLong { row: r, texts: row.len(), columns: columns.len() }.into());
        }
        if rows.is_empty() || columns.is_empty() {
            return Err(FigJamError::NoRowsOrColumns { id }.into());
        }
        // Rows past the last: a new id in the table's own session, a key after
        // the last, as tall as the last.
        let heights = |fields: &kiwi::Fields| -> Vec<(Guid, f32)> {
            fields
                .field(Field::TableRowHeights)
                .and_then(|f| f.field(Field::Entries))
                .and_then(Raw::as_array)
                .into_iter()
                .flatten()
                .filter_map(|e| Some((Guid::read(e.field(Field::Id)?)?, e.field(Field::Size)?.as_f32()?)))
                .collect()
        };
        let known = heights(fields);
        let last_height = rows.last().and_then(|(_, id)| known.iter().find(|(k, _)| k == id)).map_or(96.0, |(_, h)| *h);
        let session_id = rows[0].1.session_id;
        let mut next = rows.iter().chain(columns.iter()).map(|(_, id)| id.local_id).max().unwrap_or(0) + 1;
        let mut added = 0.0_f32;
        while rows.len() < cells.len() {
            let key = model::author::position_between(rows.last().map(|(k, _)| k.as_str()), None)
                .ok_or(FigJamError::NoKeyAfterLastRow)?;
            let row = Guid::new(session_id, next);
            next += 1;
            for (field, entry) in [
                (Field::TableRowPositions, object([(Field::Id, row.to_value()), (Field::Position, Raw::Str(key.clone()))])),
                (Field::TableRowHeights, object([(Field::Id, row.to_value()), (Field::Size, Raw::Float(last_height))])),
            ] {
                if let Some(Raw::Object(list)) = fields.field_mut(field) {
                    if let Some(Raw::Array(items)) = list.field_mut(Field::Entries) {
                        items.push(entry);
                    }
                }
            }
            rows.push((key, row));
            added += last_height;
        }
        if added > 0.0 {
            if let Some(Raw::Object(size)) = fields.field_mut(Field::Size) {
                let y = size.field(Field::Y).and_then(Raw::as_f32).unwrap_or(0.0);
                size.set(Field::Y, Raw::Float(y + added));
            }
        }
        let Some(Raw::Array(layers)) = (match fields.field_mut(Field::NodeGenerationData) {
            Some(Raw::Object(generation)) => generation.field_mut(Field::Overrides),
            _ => None,
        }) else {
            return Err(FigJamError::NoGeneratedLayers { id }.into());
        };
        // A cell's text is the generated text layer at [row, column].
        let text_layer = layers
            .iter()
            .find_map(|l| l.field(Field::GuidPath)?.field(Field::Guids)?.as_array()?.first().and_then(Guid::read).filter(|g| GeneratedLayer::of(g.local_id) == Some(GeneratedLayer::Text)))
            .unwrap_or(GeneratedLayer::Text.guid());
        for (r, row) in cells.iter().enumerate() {
            for (c, text) in row.iter().enumerate() {
                let path = [text_layer, rows[r].1, columns[c].1];
                let found = layers.iter_mut().find(|l| {
                    l.field(Field::GuidPath).and_then(|g| g.field(Field::Guids)).and_then(Raw::as_array).is_some_and(|g| g.iter().filter_map(Guid::read).eq(path.iter().copied()))
                });
                match found {
                    Some(Raw::Object(layer)) => {
                        layer.set(Field::TextData, app::tools::text::text_data(text));
                    }
                    _ => layers.push(object([
                        (Field::GuidPath, object([(Field::Guids, Raw::Array(path.iter().map(|g| g.to_value()).collect()))])),
                        (Field::TextData, app::tools::text::text_data(text)),
                    ])),
                }
            }
        }
        fields.take(Field::DerivedImmutableFrameData);
        set.push("cells".to_string());
    }
    session.refresh(&[id]);
    session.edits += 1;
    Ok(set)
}

/// The plugin API's magnets, and where each sits on a node of `w` by `h`.
const MAGNETS: [ConnectorMagnet; 7] = [
    ConnectorMagnet::Auto, ConnectorMagnet::Top, ConnectorMagnet::Bottom, ConnectorMagnet::Left, ConnectorMagnet::Right, ConnectorMagnet::Center,
    ConnectorMagnet::None,
];

fn magnet_point(magnet: ConnectorMagnet, w: f32, h: f32) -> (f32, f32) {
    match magnet {
        ConnectorMagnet::Top => (w / 2.0, 0.0),
        ConnectorMagnet::Bottom => (w / 2.0, h),
        ConnectorMagnet::Left => (0.0, h / 2.0),
        ConnectorMagnet::Right => (w, h / 2.0),
        _ => (w / 2.0, h / 2.0),
    }
}

/// The page a node is on.
fn page_of(session: &Session, id: Guid) -> Option<Guid> {
    let mut at = session.graph.node(id);
    while let Some(node) = at {
        if node.node_type == NodeType::Canvas {
            return Some(node.id);
        }
        at = node.parent_id.and_then(|p| session.graph.node(p));
    }
    None
}

/// Where a node sits on the page: its own and its ancestors' offsets.
fn absolute(session: &Session, id: Guid) -> (f32, f32) {
    let (mut x, mut y) = (0.0, 0.0);
    let mut at = session.graph.node(id);
    while let Some(node) = at {
        if matches!(node.node_type, NodeType::Canvas | NodeType::Document) {
            break;
        }
        x += node.x;
        y += node.y;
        at = node.parent_id.and_then(|p| session.graph.node(p));
    }
    (x, y)
}

/// A connector's ends as the plugin API's `ConnectorEndpoint` gives them:
/// `{ endpointNodeId, magnet }` attaches it to a node, `{ position: { x, y } }`
/// leaves it free in the connector's parent. FigJam draws the line again.
pub fn set_connector(session: &mut Session, id: Guid, properties: &Map) -> Result<Vec<String>, ToolError> {
    unknown(session, id, properties, &ACCEPTED_ON_CONNECTOR)?;
    let parent = session.graph.node(id).and_then(|n| n.parent_id).ok_or_else(|| DomainError::NoParent { id: id.to_string() })?;
    let parent_at = absolute(session, parent);
    // Each end: the raw endpoint and where it is in the parent's coordinates.
    // Caps and the line's kind, each the schema's own name.
    let mut styles: Vec<(Field, Raw)> = Vec::new();
    fn style<E: WireName>(properties: &Map, key: &str, allowed: &[E]) -> Result<Option<E>, ToolError> {
        let Some(value) = properties.get(key) else { return Ok(None) };
        E::listed(value.as_str().unwrap_or_default(), allowed)
            .map(Some)
            .ok_or_else(|| DomainError::NotOneOf { at: key.into(), got: value.to_string(), allowed: E::joined(allowed, ", ") }.into())
    }
    for (key, field) in [("connectorStartStrokeCap", Field::ConnectorStartCap), ("connectorEndStrokeCap", Field::ConnectorEndCap)] {
        if let Some(cap) = style(properties, key, &CAPS)? {
            styles.push((field, cap.into()));
        }
    }
    if let Some(line) = style(properties, "connectorLineType", &LINES)? {
        styles.push((Field::ConnectorLineStyle, line.into()));
    }
    let mut ends: Vec<(&str, Field, Raw, (f32, f32))> = Vec::new();
    for (key, field) in [("connectorStart", Field::ConnectorStart), ("connectorEnd", Field::ConnectorEnd)] {
        let Some(end) = properties.get(key) else { continue };
        let node = end.get("endpointNodeId").and_then(Data::as_str);
        let (raw, at) = match (node, end.get("position")) {
            (Some(node), _) => {
                let target = node
                    .parse()
                    .ok()
                    .and_then(|endpoint| session.graph.node(endpoint))
                    .ok_or_else(|| FigJamError::EndpointNotInFile { key: key.into(), node: node.into() })?;
                let endpoint = target.id;
                if page_of(session, endpoint) != page_of(session, id) {
                    return Err(FigJamError::EndpointOffPage { key: key.into(), node: node.into() }.into());
                }
                let named = end.get("magnet").and_then(Data::as_str).unwrap_or(ConnectorMagnet::Auto.as_wire());
                let Some(magnet) = ConnectorMagnet::listed(named, &MAGNETS) else {
                    return Err(DomainError::NotOneOf {
                        at: format!("{key}.magnet"),
                        got: named.into(),
                        allowed: ConnectorMagnet::joined(&MAGNETS, ", "),
                    }
                    .into());
                };
                let (w, h) = (target.width, target.height);
                let (px, py) = magnet_point(magnet, w, h);
                let (ax, ay) = absolute(session, endpoint);
                let raw = object([
                    (Field::EndpointNodeId, endpoint.to_value()),
                    (Field::Magnet, magnet.into()),
                    (Field::Position, object([(Field::X, Raw::Float(px)), (Field::Y, Raw::Float(py))])),
                ]);
                (raw, (ax + px - parent_at.0, ay + py - parent_at.1))
            }
            (None, Some(position)) => {
                let (x, y) = (position.get("x").and_then(Data::as_f64), position.get("y").and_then(Data::as_f64));
                let (Some(x), Some(y)) = (x, y) else { return Err(invalid(&format!("{key}.position"), "{ x, y }", position)) };
                // A free end, as FigJam keeps one: on the parent, no magnet.
                let raw = object([
                    (Field::EndpointNodeId, parent.to_value()),
                    (Field::Magnet, Raw::from(ConnectorMagnet::None)),
                    (Field::Position, object([(Field::X, Raw::Float(x as f32)), (Field::Y, Raw::Float(y as f32))])),
                ]);
                (raw, (x as f32, y as f32))
            }
            _ => return Err(invalid(key, "{ endpointNodeId, magnet? } or { position: { x, y } }", end)),
        };
        ends.push((key, field, raw, at));
    }
    let fields = fields_of(session, id)?;
    // Where each end is now, for the one not given.
    let current = |fields: &kiwi::Fields, end: Field| -> Option<(f32, f32)> {
        let t = fields.field(Field::Transform)?;
        let (x, y) = (t.field(Field::M02)?.as_f32()?, t.field(Field::M12)?.as_f32()?);
        let size = fields.field(Field::Size)?;
        let (w, h) = (size.field(Field::X)?.as_f32()?, size.field(Field::Y)?.as_f32()?);
        Some(if end == Field::ConnectorStart { (x, y) } else { (x + w, y + h) })
    };
    let start = ends.iter().find(|e| e.1 == Field::ConnectorStart).map(|e| e.3).or_else(|| current(fields, Field::ConnectorStart)).unwrap_or_default();
    let end = ends.iter().find(|e| e.1 == Field::ConnectorEnd).map(|e| e.3).or_else(|| current(fields, Field::ConnectorEnd)).unwrap_or_default();
    let mut set = Vec::new();
    for (field, value) in styles {
        fields.set(field, value);
        set.push(match field {
            Field::ConnectorStartCap => "connectorStartStrokeCap",
            Field::ConnectorEndCap => "connectorEndStrokeCap",
            _ => "connectorLineType",
        }.to_string());
    }
    let moved = !ends.is_empty();
    for (key, field, raw, _) in ends {
        fields.set(field, raw);
        set.push(key.to_string());
    }
    // The line's box, from its start, until FigJam routes it again.
    if moved {
        if let Some(Raw::Object(transform)) = fields.field_mut(Field::Transform) {
            transform.set(Field::M02, Raw::Float(start.0.min(end.0)));
            transform.set(Field::M12, Raw::Float(start.1.min(end.1)));
        }
        if let Some(Raw::Object(size)) = fields.field_mut(Field::Size) {
            size.set(Field::X, Raw::Float((end.0 - start.0).abs()));
            size.set(Field::Y, Raw::Float((end.1 - start.1).abs()));
        }
    }
    fields.take(Field::DerivedImmutableFrameData);
    session.refresh(&[id]);
    session.edits += 1;
    Ok(set)
}
