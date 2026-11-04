//! A `.fig` message built by hand, for tests that write and read through the
//! toolkit with no fixture.

use figfile::FigFile;
use kiwi::{Schema, Value as Raw};

pub fn obj(fields: Vec<(&str, Raw)>) -> Raw {
    Raw::Object(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

pub fn guid(session: u32, local: u32) -> Raw {
    obj(vec![("sessionID", Raw::Uint(session)), ("localID", Raw::Uint(local))])
}

pub fn node(id: u32, parent: Option<(u32, &str)>, node_type: &str, name: &str, extra: Vec<(&str, Raw)>) -> Raw {
    let mut fields = vec![
        ("guid", guid(1, id)),
        ("phase", Raw::Enum("CREATED")),
        ("type", Raw::Enum(kiwi::intern(node_type))),
        ("name", Raw::Str(name.into())),
        ("visible", Raw::Bool(true)),
    ];
    if let Some((parent, position)) = parent {
        fields.push((
            "parentIndex",
            obj(vec![("guid", guid(1, parent)), ("position", Raw::Str(position.into()))]),
        ));
    }
    fields.extend(extra);
    obj(fields)
}

pub fn size(w: f32, h: f32) -> Raw {
    obj(vec![("x", Raw::Float(w)), ("y", Raw::Float(h))])
}

pub fn at(x: f32, y: f32) -> Raw {
    obj(vec![
        ("m00", Raw::Float(1.0)), ("m01", Raw::Float(0.0)), ("m02", Raw::Float(x)),
        ("m10", Raw::Float(0.0)), ("m11", Raw::Float(1.0)), ("m12", Raw::Float(y)),
    ])
}

pub fn solid(r: f32, g: f32, b: f32, opacity: f32) -> Raw {
    obj(vec![
        ("type", Raw::Enum("SOLID")),
        ("color", obj(vec![("r", Raw::Float(r)), ("g", Raw::Float(g)), ("b", Raw::Float(b)), ("a", Raw::Float(1.0))])),
        ("opacity", Raw::Float(opacity)),
        ("visible", Raw::Bool(true)),
    ])
}

pub fn colour_value(r: f32, g: f32, b: f32) -> Raw {
    obj(vec![(
        "colorValue",
        obj(vec![("r", Raw::Float(r)), ("g", Raw::Float(g)), ("b", Raw::Float(b)), ("a", Raw::Float(1.0))]),
    )])
}

pub fn variable(id: u32, name: &str, kind: &str, value: Raw, extra: Vec<(&str, Raw)>) -> Raw {
    let entry = obj(vec![
        ("modeID", guid(1, 99)),
        ("variableData", obj(vec![("value", value), ("dataType", Raw::Enum(kiwi::intern(kind)))])),
    ]);
    let mut fields = vec![
        ("variableSetID", obj(vec![("guid", guid(1, 20))])),
        ("variableResolvedType", Raw::Enum(kiwi::intern(kind))),
        ("variableDataValues", obj(vec![("entries", Raw::Array(vec![entry]))])),
    ];
    fields.extend(extra);
    node(id, Some((20, "!")), "VARIABLE", name, fields)
}

/// A page holding a card frame (red fill, radius, auto-layout) with a title
/// text inside, a button component, and an instance of it; plus a second page.
pub fn document() -> FigFile {
    let changes = vec![
        node(0, None, "DOCUMENT", "Document", vec![]),
        node(1, Some((0, "!")), "CANVAS", "Screens", vec![]),
        node(
            2,
            Some((1, "!")),
            "FRAME",
            "Card",
            vec![
                ("size", size(320.0, 200.0)),
                ("transform", at(40.0, 60.0)),
                ("fillPaints", Raw::Array(vec![solid(1.0, 0.0, 0.0, 0.5)])),
                ("cornerRadius", Raw::Float(8.0)),
                ("stackMode", Raw::Enum("VERTICAL")),
                ("stackSpacing", Raw::Float(12.0)),
                ("derivedSomething", Raw::Bool(true)),
            ],
        ),
        node(
            3,
            Some((2, "!")),
            "TEXT",
            "Title",
            vec![
                ("size", size(100.0, 20.0)),
                ("transform", at(0.0, 0.0)),
                ("textData", obj(vec![("characters", Raw::Str("Hola".into())), ("glyphs", Raw::Array(vec![]))])),
                ("fontSize", Raw::Float(18.0)),
                ("fontName", obj(vec![("family", Raw::Str("Inter".into())), ("style", Raw::Str("Bold".into()))])),
            ],
        ),
        node(4, Some((1, "\"")), "SYMBOL", "Button", vec![("size", size(96.0, 32.0)), ("transform", at(400.0, 0.0))]),
        node(
            5,
            Some((1, "#")),
            "INSTANCE",
            "Button",
            vec![
                ("size", size(96.0, 32.0)),
                ("transform", at(400.0, 100.0)),
                ("symbolData", obj(vec![("symbolID", guid(1, 4))])),
            ],
        ),
        node(6, Some((0, "\"")), "CANVAS", "Notes", vec![]),
        // An icon component: two vector layers (one with a colour style) over a
        // background rectangle that recolouring must leave alone.
        node(30, Some((6, "!")), "SYMBOL", "Icon/Play", vec![("size", size(24.0, 24.0)), ("transform", at(0.0, 0.0))]),
        node(31, Some((30, "!")), "VECTOR", "Shape", vec![("size", size(20.0, 20.0)), ("transform", at(2.0, 2.0)), ("fillPaints", Raw::Array(vec![solid(0.0, 0.0, 0.0, 1.0)])), ("overrideKey", guid(2, 31))]),
        node(32, Some((30, "\"")), "VECTOR", "Accent", vec![("size", size(4.0, 4.0)), ("transform", at(0.0, 0.0)), ("fillPaints", Raw::Array(vec![solid(0.0, 0.0, 0.0, 1.0)])), ("styleIdForFill", obj(vec![("guid", guid(1, 98))]))]),
        // A vector used as bounds, with no paint of its own: a recolour must not
        // give it one.
        node(34, Some((30, "$")), "VECTOR", "Bounds", vec![("size", size(24.0, 24.0)), ("transform", at(0.0, 0.0))]),
        node(33, Some((30, "#")), "RECTANGLE", "Background", vec![("size", size(24.0, 24.0)), ("transform", at(0.0, 0.0)), ("fillPaints", Raw::Array(vec![solid(1.0, 1.0, 1.0, 1.0)]))]),
        // A chip: a label and a nested instance of the icon.
        node(40, Some((6, "\"")), "SYMBOL", "Chip", vec![("size", size(80.0, 24.0)), ("transform", at(100.0, 0.0))]),
        node(41, Some((40, "!")), "TEXT", "Label", vec![("size", size(40.0, 16.0)), ("transform", at(28.0, 4.0)), ("textData", obj(vec![("characters", Raw::Str("Chip".into()))])), ("overrideKey", guid(2, 41))]),
        node(42, Some((40, "\"")), "INSTANCE", "Icon", vec![("size", size(24.0, 24.0)), ("transform", at(0.0, 0.0)), ("symbolData", obj(vec![("symbolID", guid(1, 30))])), ("overrideKey", guid(2, 42))]),
        // A collection with one mode, and three variables: a local colour, a
        // colour imported from a library, and a number.
        node(
            20,
            Some((0, "#")),
            "VARIABLE_SET",
            "Tokens",
            vec![(
                "variableSetModes",
                Raw::Array(vec![obj(vec![("id", guid(1, 99)), ("name", Raw::Str("Light".into()))])]),
            )],
        ),
        variable(21, "primary/500", "COLOR", colour_value(0.118, 0.533, 0.898), vec![]),
        variable(22, "library/blue", "COLOR", colour_value(0.0, 0.0, 1.0), vec![("sourceLibraryKey", Raw::Str("lib".into()))]),
        variable(23, "spacing/4", "FLOAT", obj(vec![("floatValue", Raw::Float(4.0))]), vec![]),
    ];
    FigFile {
        product: figfile::Product::Design,
        node_changes: Raw::Object(kiwi::Fields::from([
            ("type".to_string(), Raw::Enum("NODE_CHANGES")),
            ("nodeChanges".to_string(), Raw::Array(changes)),
        ])),
        blobs: Vec::new(),
        images: vec![("abc123".to_string(), b"\x89PNG\r\n\x1a\nrest".to_vec())],
        version: 101,
        schema: Schema { definitions: Vec::new() },
        schema_chunk: Vec::new(),
        canvas_name: "canvas.fig".into(),
        trailing_chunks: Vec::new(),
        other_entries: Vec::new(),
    }
}
