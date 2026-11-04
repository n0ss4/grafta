use figfile::FigFile;
use kiwi::{Definition, DefinitionKind, Field, Schema, TypeRef};

use super::*;

// ── Scaffolding ──────────────────────────────────────────────────────────────
//
// A document, a schema and a `FigFile`, all hand-built. Nothing here reads a
// `.fig`: the whole of identity and assembly is provable without one, which is
// what makes this phase separable.

fn node(id: Guid, parent: Option<(Guid, &str)>, node_type: &str, name: &str) -> Value {
    let mut fields = kiwi::Fields::from([
        ("guid".to_string(), id.to_value()),
        ("phase".to_string(), Value::Enum("CREATED")),
        ("type".to_string(), Value::Enum(kiwi::intern(node_type))),
        ("name".to_string(), Value::Str(name.to_string())),
    ]);
    if let Some((parent, position)) = parent {
        fields.insert("parentIndex", parent_index(parent, position));
    }
    if node_type == "SYMBOL" {
        fields.insert("size", vector(24.0, 16.0));
    }
    Value::Object(fields)
}

const ROOT: Guid = Guid { session_id: 0, local_id: 0 };
const PAGE: Guid = Guid { session_id: 0, local_id: 1 };
const SECOND_PAGE: Guid = Guid { session_id: 0, local_id: 2 };
const SYMBOL: Guid = Guid { session_id: 7, local_id: 42 };
const CHILD: Guid = Guid { session_id: 7, local_id: 43 };

/// A root, two pages, a symbol on the first page and a frame under the symbol.
/// Shaped like the files the fixtures actually contain — one parentless root,
/// pages under it, everything else below.
fn document() -> Vec<Value> {
    vec![
        node(ROOT, None, "DOCUMENT", "Document"),
        node(PAGE, Some((ROOT, "!")), "CANVAS", "Page 1"),
        node(SECOND_PAGE, Some((ROOT, "~")), "CANVAS", "Internal Only Canvas"),
        node(SYMBOL, Some((PAGE, "!")), "SYMBOL", "Button"),
        node(CHILD, Some((SYMBOL, "!")), "FRAME", "Label"),
    ]
}

fn fig(changes: Vec<Value>) -> FigFile {
    FigFile {
        product: figfile::Product::Design,
        node_changes: Value::Object(kiwi::Fields::from([
            ("type".to_string(), Value::Enum("NODE_CHANGES")),
            ("nodeChanges".to_string(), Value::Array(changes)),
        ])),
        blobs: Vec::new(),
        images: Vec::new(),
        version: 101,
        schema: schema(),
        schema_chunk: Vec::new(),
        canvas_name: "canvas.fig".to_string(),
        trailing_chunks: Vec::new(),
        other_entries: Vec::new(),
    }
}

fn field(name: &str, type_ref: TypeRef, value: u32) -> Field {
    Field { name: name.to_string(), type_ref: Some(type_ref), is_array: false, value }
}

fn structure(name: &str, fields: Vec<Field>) -> Definition {
    Definition { name: name.to_string(), kind: DefinitionKind::Struct, fields }
}

fn enumeration(name: &str, names: &[&str]) -> Definition {
    Definition {
        name: name.to_string(),
        kind: DefinitionKind::Enum,
        fields: names
            .iter()
            .enumerate()
            .map(|(index, name)| Field {
                name: (*name).to_string(),
                type_ref: None,
                is_array: false,
                #[allow(clippy::cast_possible_truncation)]
                value: index as u32,
            })
            .collect(),
    }
}

/// The part of Figma's schema an authored node touches, with the real field
/// tags. It is not Figma's schema and does not need to be — what it proves is
/// that a built node encodes and decodes back to itself, and that is a property
/// of the builders, not of any one file.
fn schema() -> Schema {
    Schema {
        definitions: vec![
            structure(
                "GUID",
                vec![field("sessionID", TypeRef::Uint, 1), field("localID", TypeRef::Uint, 2)],
            ),
            structure(
                "ParentIndex",
                vec![
                    field("guid", TypeRef::Named("GUID".into()), 1),
                    field("position", TypeRef::Str, 2),
                ],
            ),
            structure("Vector", vec![field("x", TypeRef::Float, 1), field("y", TypeRef::Float, 2)]),
            structure(
                "Matrix",
                vec![
                    field("m00", TypeRef::Float, 1),
                    field("m01", TypeRef::Float, 2),
                    field("m02", TypeRef::Float, 3),
                    field("m10", TypeRef::Float, 4),
                    field("m11", TypeRef::Float, 5),
                    field("m12", TypeRef::Float, 6),
                ],
            ),
            enumeration("NodePhase", &["CREATED", "REMOVED"]),
            enumeration(
                "NodeType",
                &["NONE", "DOCUMENT", "CANVAS", "GROUP", "FRAME", "SYMBOL", "INSTANCE"],
            ),
            enumeration("StrokeAlign", &["CENTER", "INSIDE", "OUTSIDE"]),
            enumeration("StrokeJoin", &["MITER", "BEVEL", "ROUND"]),
            enumeration("MessageType", &["NODE_CHANGES"]),
            Definition {
                name: "SymbolData".to_string(),
                kind: DefinitionKind::Message,
                fields: vec![field("symbolID", TypeRef::Named("GUID".into()), 1)],
            },
            Definition {
                name: "NodeChange".to_string(),
                kind: DefinitionKind::Message,
                fields: vec![
                    field("guid", TypeRef::Named("GUID".into()), 1),
                    field("phase", TypeRef::Named("NodePhase".into()), 2),
                    field("parentIndex", TypeRef::Named("ParentIndex".into()), 3),
                    field("type", TypeRef::Named("NodeType".into()), 4),
                    field("name", TypeRef::Str, 5),
                    field("visible", TypeRef::Bool, 6),
                    field("opacity", TypeRef::Float, 8),
                    field("size", TypeRef::Named("Vector".into()), 11),
                    field("transform", TypeRef::Named("Matrix".into()), 12),
                    field("strokeWeight", TypeRef::Float, 26),
                    field("strokeAlign", TypeRef::Named("StrokeAlign".into()), 29),
                    field("strokeJoin", TypeRef::Named("StrokeJoin".into()), 31),
                    field("symbolData", TypeRef::Named("SymbolData".into()), 113),
                    field("frameMaskDisabled", TypeRef::Bool, 115),
                ],
            },
            Definition {
                name: "Message".to_string(),
                kind: DefinitionKind::Message,
                fields: vec![
                    field("type", TypeRef::Named("MessageType".into()), 1),
                    Field {
                        name: "nodeChanges".to_string(),
                        type_ref: Some(TypeRef::Named("NodeChange".into())),
                        is_array: true,
                        value: 4,
                    },
                ],
            },
        ],
    }
}

fn field_names(node_change: &Value) -> Vec<&str> {
    match node_change {
        Value::Object(fields) => fields.keys().collect(),
        _ => Vec::new(),
    }
}

// ── Identity ─────────────────────────────────────────────────────────────────

#[test]
fn an_allocated_id_appears_nowhere_in_the_document() {
    let changes = document();
    let mut allocator = Allocator::new(&changes).expect("the document does not use grafta's session");

    let existing: Vec<Guid> = changes.iter().filter_map(id_of).collect();
    for _ in 0..100 {
        let id = allocator.allocate();
        assert!(!existing.contains(&id), "{id} collides with a node already in the document");
    }
}

#[test]
fn the_same_document_allocates_the_same_ids_twice() {
    let changes = document();
    let mut first = Allocator::new(&changes).expect("allocates");
    let mut second = Allocator::new(&changes).expect("allocates");

    let one: Vec<Guid> = (0..5).map(|_| first.allocate()).collect();
    let two: Vec<Guid> = (0..5).map(|_| second.allocate()).collect();
    assert_eq!(one, two, "allocation is not deterministic, so neither are the bytes");
}

#[test]
fn a_document_already_using_graftas_session_is_refused() {
    let mut changes = document();
    changes.push(node(Guid::new(GRAFTA_SESSION, 9), Some((PAGE, "~")), "FRAME", "Someone else's"));

    let refused = Allocator::new(&changes);
    assert!(
        matches!(refused, Err(AuthorError::SessionInUse(GRAFTA_SESSION))),
        "a session grafta cannot claim must be an error, not a quiet move to another one"
    );
}

#[test]
fn an_allocated_id_is_still_graftas_after_a_write_and_a_read() {
    let mut allocator = Allocator::new(&document()).expect("allocates");
    let id = allocator.allocate();

    let message = Value::Object(kiwi::Fields::from([
        ("type".to_string(), Value::Enum("NODE_CHANGES")),
        (
            "nodeChanges".to_string(),
            Value::Array(vec![frame(id, PAGE, "!", "Authored", (100.0, 40.0))]),
        ),
    ]));

    let bytes = kiwi::encode_message("Message", &message, &schema()).expect("encodes");
    let read = kiwi::decode_message("Message", &bytes, &schema()).expect("decodes");

    let written = read
        .get("nodeChanges")
        .and_then(Value::as_array)
        .and_then(|changes| changes.first())
        .and_then(id_of)
        .expect("the node came back");

    assert_eq!(written, id);
    assert!(written.is_graftas(), "who authored a node must survive the file it went into");
}

// ── Order among siblings ─────────────────────────────────────────────────────

#[test]
fn an_appended_key_sorts_after_every_sibling() {
    let changes = document();
    let key = position_after(&changes, ROOT);

    let siblings: Vec<&str> = changes.iter().filter(|nc| parent_of(nc) == Some(ROOT)).filter_map(position_of).collect();
    assert!(!siblings.is_empty(), "the document under test has no siblings to sort against");
    for sibling in siblings {
        assert!(key.as_str() > sibling, "{key:?} does not sort after {sibling:?}");
    }
}

#[test]
fn a_parent_with_no_children_starts_at_the_first_position() {
    assert_eq!(position_after(&document(), CHILD), "!");
}

#[test]
fn keys_from_the_measured_alphabet_still_sort() {
    // The real range is the whole printable ASCII set, one character to 443 of
    // them. These are keys taken from the fixtures.
    for existing in ["!", "~", " ~", "~!", "~z", "!!!!", " ~ U"] {
        let changes = vec![
            node(ROOT, None, "DOCUMENT", "Document"),
            node(PAGE, Some((ROOT, existing)), "CANVAS", "Page"),
        ];
        let key = position_after(&changes, ROOT);
        assert!(key.as_str() > existing, "{key:?} does not sort after {existing:?}");
    }
}

#[test]
fn ordering_matches_the_readers_own_sibling_sort() {
    // `index_changes` sorts children with `str::cmp`. An appended key has to
    // land last under that comparison and not merely under some other one.
    let changes = document();
    let key = position_after(&changes, ROOT);

    let mut keys: Vec<String> = changes
        .iter()
        .filter(|nc| parent_of(nc) == Some(ROOT))
        .filter_map(position_of)
        .map(str::to_string)
        .collect();
    keys.push(key.clone());
    keys.sort();

    assert_eq!(keys.last(), Some(&key), "the appended key is not last once the reader sorts");
}

// ── The nodes ────────────────────────────────────────────────────────────────

#[test]
fn a_frame_carries_the_thirteen_measured_fields() {
    let built = frame(Guid::new(GRAFTA_SESSION, 0), PAGE, "!", "Authored", (320.0, 240.0));
    assert_eq!(
        field_names(&built),
        vec![
            "guid",
            "phase",
            "parentIndex",
            "type",
            "name",
            "visible",
            "opacity",
            "transform",
            "size",
            "strokeWeight",
            "strokeAlign",
            "strokeJoin",
            "frameMaskDisabled",
        ]
    );
    assert_eq!(built.get("type").and_then(Value::as_str), Some("FRAME"));
    assert_eq!(built.get("size").and_then(|s| s.get("x")).and_then(Value::as_f32), Some(320.0));
}

#[test]
fn an_instance_carries_its_symbol_and_no_derived_data() {
    let built =
        instance(Guid::new(GRAFTA_SESSION, 0), PAGE, "!", "Authored", (24.0, 16.0), SYMBOL);

    assert_eq!(
        built.get("symbolData").and_then(|data| data.get("symbolID")).and_then(Guid::read),
        Some(SYMBOL)
    );
    assert!(
        built.get("derivedSymbolData").is_none(),
        "grafta cannot produce Figma's expansion of an instance, so it must not appear to have one"
    );
    assert!(!field_names(&built).contains(&"fillGeometry"), "an authored node references no blob");
}

#[test]
fn a_page_carries_the_eight_measured_fields() {
    let built = page(Guid::new(GRAFTA_SESSION, 0), ROOT, "!", "Authored");
    assert_eq!(
        field_names(&built),
        vec!["guid", "phase", "parentIndex", "type", "name", "visible", "opacity", "transform"]
    );
    assert_eq!(built.get("type").and_then(Value::as_str), Some("CANVAS"));
}

#[test]
fn a_built_node_survives_an_encode_and_a_decode() {
    let id = Guid::new(GRAFTA_SESSION, 0);
    let built = vec![
        page(id, ROOT, "!", "Authored page"),
        frame(Guid::new(GRAFTA_SESSION, 1), id, "!", "Authored frame", (320.0, 240.0)),
        instance(Guid::new(GRAFTA_SESSION, 2), id, "\"", "Authored instance", (24.0, 16.0), SYMBOL),
    ];

    let message = Value::Object(kiwi::Fields::from([
        ("type".to_string(), Value::Enum("NODE_CHANGES")),
        ("nodeChanges".to_string(), Value::Array(built.clone())),
    ]));

    let bytes = kiwi::encode_message("Message", &message, &schema()).expect("encodes");
    let read = kiwi::decode_message("Message", &bytes, &schema()).expect("decodes");

    assert_eq!(read.get("nodeChanges").and_then(Value::as_array), Some(&built));
}

// ── The graft ────────────────────────────────────────────────────────────────

#[test]
fn authored_nodes_are_appended_parent_before_child() {
    let mut file = fig(document());
    let before = node_changes(&file).len();

    let (page_id, frame_id, instance_id) = {
        let mut graft = Graft::new(&mut file).expect("claims grafta's session");
        let page_id = graft.add_page("Authored page").expect("adds a page");
        let frame_id = graft.add_frame(page_id, "Authored frame", (320.0, 240.0)).expect("adds a frame");
        let instance_id = graft.add_instance(frame_id, "Authored instance", SYMBOL).expect("adds an instance");
        (page_id, frame_id, instance_id)
    };

    let changes = node_changes(&file);
    assert_eq!(changes.len(), before + 3);

    let authored: Vec<Guid> = changes[before..].iter().filter_map(id_of).collect();
    assert_eq!(authored, vec![page_id, frame_id, instance_id]);
    assert!(authored.iter().all(|id| id.is_graftas()));

    assert_eq!(parent_of(&changes[before]), Some(ROOT));
    assert_eq!(parent_of(&changes[before + 1]), Some(page_id));
    assert_eq!(parent_of(&changes[before + 2]), Some(frame_id));
}

#[test]
fn the_instance_takes_the_size_of_the_symbol_it_points_at() {
    let mut file = fig(document());
    let mut graft = Graft::new(&mut file).expect("claims grafta's session");
    let frame_id = graft.add_frame(PAGE, "Authored frame", (320.0, 240.0)).expect("adds a frame");
    let instance_id = graft.add_instance(frame_id, "Authored instance", SYMBOL).expect("adds an instance");

    let built = node_changes(&file)
        .iter()
        .find(|nc| id_of(nc) == Some(instance_id))
        .expect("the instance is in the document");

    // The symbol in `document()` is 24 × 16.
    assert_eq!(built.get("size").and_then(|s| s.get("x")).and_then(Value::as_f32), Some(24.0));
    assert_eq!(built.get("size").and_then(|s| s.get("y")).and_then(Value::as_f32), Some(16.0));
}

#[test]
fn nothing_that_was_already_there_changes() {
    let original = document();
    let mut file = fig(original.clone());

    {
        let mut graft = Graft::new(&mut file).expect("claims grafta's session");
        let page_id = graft.add_page("Authored page").expect("adds a page");
        graft.add_frame(page_id, "Authored frame", (320.0, 240.0)).expect("adds a frame");
    }

    assert_eq!(&node_changes(&file)[..original.len()], original.as_slice());
}

#[test]
fn an_instance_needs_a_symbol_the_document_holds() {
    let mut file = fig(document());
    let mut graft = Graft::new(&mut file).expect("claims grafta's session");
    let absent = Guid::new(99, 99);

    let refused = graft.add_instance(PAGE, "Authored instance", absent);
    assert!(matches!(refused, Err(AuthorError::UnknownNode(id)) if id == absent));
    assert_eq!(node_changes(&file).len(), document().len(), "a refusal must author nothing");
}

#[test]
fn an_instance_needs_a_symbol_with_a_size() {
    let mut changes = document();
    let sizeless = Guid::new(7, 44);
    changes.push(node(sizeless, Some((PAGE, "~")), "FRAME", "Not a symbol"));
    let mut file = fig(changes);
    let mut graft = Graft::new(&mut file).expect("claims grafta's session");

    let refused = graft.add_instance(PAGE, "Authored instance", sizeless);
    assert!(matches!(refused, Err(AuthorError::NoSize(id)) if id == sizeless));
}

#[test]
fn a_frame_needs_a_parent_the_document_holds() {
    let mut file = fig(document());
    let mut graft = Graft::new(&mut file).expect("claims grafta's session");
    let absent = Guid::new(99, 99);

    let refused = graft.add_frame(absent, "Authored frame", (10.0, 10.0));
    assert!(matches!(refused, Err(AuthorError::UnknownNode(id)) if id == absent));
}

#[test]
fn the_root_is_found_rather_than_assumed() {
    // Same document, renumbered so that the root is not 0:0. Nothing should
    // notice.
    let root = Guid::new(4321, 17);
    let changes = vec![
        node(root, None, "DOCUMENT", "Document"),
        node(PAGE, Some((root, "!")), "CANVAS", "Page 1"),
    ];
    let mut file = fig(changes);
    let graft = Graft::new(&mut file).expect("claims grafta's session");

    assert_eq!(graft.root().expect("finds the root"), root);
}

#[test]
fn reparent_moves_graftas_node_and_refuses_figmas() {
    let mut file = fig(document());
    let mut graft = Graft::new(&mut file).expect("claims grafta's session");

    let frame_id = graft.add_frame(PAGE, "Authored frame", (320.0, 240.0)).expect("adds a frame");
    let page_id = graft.add_page("Authored page").expect("adds a page");
    graft.reparent(frame_id, page_id).expect("moves grafta's own frame");

    let refused = graft.reparent(SYMBOL, page_id);
    assert!(matches!(refused, Err(AuthorError::NotOurs(id)) if id == SYMBOL));

    let moved = node_changes(&file).iter().find(|nc| id_of(nc) == Some(frame_id)).expect("still there");
    assert_eq!(parent_of(moved), Some(page_id));
}

#[test]
fn pruning_keeps_the_root_the_kept_subtree_and_nothing_else() {
    let mut file = fig(document());
    let report = {
        let mut graft = Graft::new(&mut file).expect("claims grafta's session");
        graft.prune(&[SYMBOL]).expect("prunes")
    };

    let kept: Vec<Guid> = node_changes(&file).iter().filter_map(id_of).collect();
    // The root, the page the symbol hangs from, the symbol, and its child.
    assert_eq!(kept, vec![ROOT, PAGE, SYMBOL, CHILD]);
    assert_eq!(report, PruneReport { kept: 4, removed: 1 });
}

#[test]
fn pruning_authors_nothing() {
    let original = document();
    let mut file = fig(original.clone());
    {
        let mut graft = Graft::new(&mut file).expect("claims grafta's session");
        graft.prune(&[SYMBOL]).expect("prunes");
    }

    for survivor in node_changes(&file) {
        let id = id_of(survivor).expect("has an id");
        let before = original.iter().find(|nc| id_of(nc) == Some(id)).expect("was there before");
        assert_eq!(survivor, before, "pruning changed a node instead of removing one");
    }
}

#[test]
fn a_message_with_no_node_changes_is_refused() {
    let mut file = fig(document());
    file.node_changes = Value::Object(kiwi::Fields::new());

    assert!(matches!(Graft::new(&mut file), Err(AuthorError::NoNodeChanges)));
}

// ── Resuming, positions, and reading back ────────────────────────────────────

#[test]
fn resuming_continues_after_graftas_highest_id() {
    let mut file = fig(document());
    let first = {
        let mut graft = Graft::new(&mut file).expect("claims grafta's session");
        graft.add_frame(PAGE, "one", (10.0, 10.0)).expect("adds a frame")
    };

    // A second graft on the same document: `new` refuses, `resume` continues.
    assert!(matches!(Graft::new(&mut file), Err(AuthorError::SessionInUse(_))));
    let second = {
        let mut graft = Graft::resume(&mut file).expect("resumes");
        graft.add_frame(PAGE, "two", (10.0, 10.0)).expect("adds a frame")
    };

    assert!(second.is_graftas());
    assert_eq!(second.local_id, first.local_id + 1);
}

#[test]
fn a_position_changes_two_values_and_no_fields() {
    let mut at_origin = fig(document());
    let mut placed = fig(document());
    let origin = Graft::new(&mut at_origin).unwrap().add_frame(PAGE, "f", (8.0, 8.0)).unwrap();
    let moved =
        Graft::new(&mut placed).unwrap().add_frame_at(PAGE, "f", (8.0, 8.0), (24.0, 40.0)).unwrap();

    let find = |file: &FigFile, id: Guid| {
        node_changes(file).iter().find(|nc| id_of(nc) == Some(id)).cloned().expect("authored")
    };
    let (a, b) = (find(&at_origin, origin), find(&placed, moved));
    assert_eq!(field_names(&a), field_names(&b), "a position must not add or remove fields");

    let transform = b.get("transform").expect("has a transform");
    assert_eq!(transform.get("m02").and_then(Value::as_f32), Some(24.0));
    assert_eq!(transform.get("m12").and_then(Value::as_f32), Some(40.0));
}

#[test]
fn an_authored_node_can_be_read_without_rebuilding_the_graph() {
    let mut file = fig(document());
    let mut graph = crate::build_scene_graph(&file).expect("builds");
    let before = graph.nodes.len();

    let frame = Graft::new(&mut file)
        .unwrap()
        .add_frame_at(PAGE, "placed", (100.0, 50.0), (24.0, 40.0))
        .unwrap();
    crate::insert_appended(&mut graph, &file, &[frame]).expect("inserts");

    assert_eq!(graph.nodes.len(), before + 1);
    let node = graph.node(frame).expect("the new frame is in the graph");
    assert_eq!((node.x, node.y, node.width, node.height), (24.0, 40.0, 100.0, 50.0));
    assert_eq!(node.parent_id, Some(PAGE));
    let page = graph.node(PAGE).unwrap();
    assert_eq!(page.children.last(), Some(&frame), "appended last among siblings");

    // And it is what a full rebuild would have produced.
    let rebuilt = crate::build_scene_graph(&file).unwrap();
    assert_eq!(rebuilt.node(frame), Some(node));
    assert_eq!(rebuilt.node(PAGE).unwrap().children, page.children);
}

#[test]
fn an_edited_node_refreshes_to_what_a_rebuild_would_read() {
    let mut file = fig(document());
    let mut graph = crate::build_scene_graph(&file).expect("builds");
    let frame = Graft::new(&mut file).unwrap().add_frame(PAGE, "styled", (10.0, 10.0)).unwrap();
    crate::insert_appended(&mut graph, &file, &[frame]).unwrap();

    // Edit the authored frame's opacity in place, as a style tool would.
    let changes = node_changes_mut(&mut file).unwrap();
    let change = changes.iter_mut().find(|nc| id_of(nc) == Some(frame)).unwrap();
    if let Value::Object(fields) = change {
        fields.insert("opacity", Value::Float(0.25));
    }
    crate::refresh(&mut graph, &file, &[frame]).expect("refreshes");

    let refreshed = graph.node(frame).unwrap();
    assert_eq!(refreshed.opacity, 0.25);
    let rebuilt = crate::build_scene_graph(&file).unwrap();
    assert_eq!(rebuilt.node(frame), Some(refreshed));
}

#[test]
fn a_removed_node_leaves_the_graph_and_its_parent() {
    let mut file = fig(document());
    let mut graph = crate::build_scene_graph(&file).unwrap();
    let frame = Graft::new(&mut file).unwrap().add_frame(PAGE, "gone", (10.0, 10.0)).unwrap();
    crate::insert_appended(&mut graph, &file, &[frame]).unwrap();
    crate::remove(&mut graph, &[frame]);
    assert!(graph.node(frame).is_none());
    assert!(!graph.node(PAGE).unwrap().children.contains(&frame));
}

// ── Positions between siblings ─────────────────────────────────

fn between(lower: Option<&str>, upper: Option<&str>) -> String {
    let key = position_between(lower, upper).unwrap_or_else(|| panic!("no key between {lower:?} and {upper:?}"));
    assert!(lower.is_none_or(|l| l < key.as_str()), "{key:?} is not after {lower:?}");
    assert!(upper.is_none_or(|u| key.as_str() < u), "{key:?} is not before {upper:?}");
    assert!(key.bytes().all(|b| (b' '..=b'~').contains(&b)), "{key:?} leaves printable ASCII");
    key
}

#[test]
fn a_key_is_found_between_neighbours_and_at_either_end() {
    between(None, None);
    between(None, Some("!"));
    between(Some("!"), None);
    between(Some("!"), Some("\""));
    between(Some("!"), Some("!!"));
    between(Some("a"), Some("b"));
    between(Some("~"), None);
    between(Some("~~~~~~Z"), Some("~~~~~~["));
    between(Some("x "), Some("x!"));
    between(None, Some("!!"));
    // Repeatedly between the same two keys, as a model putting layer after
    // layer behind the same one would.
    let mut upper = String::from("\"");
    for _ in 0..200 {
        upper = between(Some("!"), Some(&upper));
    }
}

#[test]
fn no_key_fits_between_a_key_and_itself_plus_spaces_and_that_is_said() {
    assert_eq!(position_between(Some("x"), Some("x ")), None);
    assert_eq!(position_between(Some("x"), Some("x")), None);
    assert_eq!(position_between(Some("b"), Some("a")), None);
    assert_eq!(position_between(None, Some(" ")), None);
}

#[test]
fn keys_put_again_and_again_at_the_same_place_never_end_in_a_space() {
    for (lower, start) in [("!", "\""), ("a", "b"), ("", "!")] {
        let lower = (!lower.is_empty()).then_some(lower);
        let mut upper = start.to_string();
        for _ in 0..500 {
            upper = between(lower, Some(&upper));
            assert!(!upper.ends_with(' '), "{upper:?}");
        }
        let mut lower_key = between(lower, Some(start));
        for _ in 0..500 {
            lower_key = between(Some(&lower_key), Some(start));
            assert!(!lower_key.ends_with(' '), "{lower_key:?}");
        }
    }
}

// ── Newer node types ───────────────────────────────────────────

#[test]
fn brushes_stay_out_of_the_scene_and_text_on_a_path_is_a_known_layer() {
    let mut changes = document();
    changes.push(node(Guid::new(7, 50), Some((SECOND_PAGE, "\"")), "BRUSH", "Heist"));
    changes.push(node(Guid::new(7, 51), Some((PAGE, "\"")), "TEXT_PATH", "Click me"));
    let graph = crate::build_scene_graph(&fig(changes)).unwrap();
    assert!(graph.node(Guid::new(7, 50)).is_none(), "a brush is an asset, not a layer");
    assert_eq!(graph.node(Guid::new(7, 51)).map(|n| n.node_type), Some(crate::NodeType::TextPath));
    assert!(crate::NodeType::TextPath.is_vector_shape(), "drawn from its stored geometry");
    assert!(graph.nodes.values().all(|n| n.node_type != crate::NodeType::Unknown));
}

#[test]
fn a_variant_property_id_is_derived_from_its_name_as_figma_derives_it() {
    // Measured in real files.
    assert_eq!(variant_property_id("Estado"), Guid::new(2_688_757_310, 2_217_398_150));
    assert_eq!(variant_property_id("Tipo"), Guid::new(2_712_565_150, 2_367_276_652));
    assert_eq!(variant_property_id("# of lines"), Guid::new(2_246_076_123, 2_072_951_688));
}

// ── Undo ───────────────────────────────────────────────────────────────

/// The document and its blobs, to compare before and after an undo.
fn snapshot(file: &FigFile) -> (Value, Vec<Vec<u8>>) {
    (file.node_changes.clone(), file.blobs.clone())
}

#[test]
fn an_undo_puts_back_what_a_graft_appended_moved_renamed_and_stored() {
    let mut file = fig(document());
    let before = snapshot(&file);
    let mut undo = Undo::default();
    {
        let mut graft = Graft::new(&mut file).expect("claims grafta's session").recording(&mut undo);
        let frame = graft.add_frame(PAGE, "Authored frame", (320.0, 240.0)).expect("adds a frame");
        graft.add_instance(frame, "Authored instance", SYMBOL).expect("adds an instance");
        graft.move_under(CHILD, SECOND_PAGE).expect("moves");
        graft.place(SYMBOL, SECOND_PAGE, "~").expect("places");
        graft.rename(frame, "Renamed").expect("renames");
        graft.add_blob(vec![1, 2, 3]).expect("stores a blob");
    }
    assert!(!undo.is_empty());
    assert_ne!(snapshot(&file), before);
    undo.restore(&mut file);
    assert_eq!(snapshot(&file), before);
}

#[test]
fn an_undo_puts_pruned_node_changes_back_where_they_were() {
    let mut file = fig(document());
    let before = snapshot(&file);
    let mut undo = Undo::default();
    Graft::new(&mut file).expect("claims grafta's session").recording(&mut undo).prune(&[SYMBOL]).expect("prunes");
    assert_ne!(snapshot(&file), before);
    undo.restore(&mut file);
    assert_eq!(snapshot(&file), before);
}

#[test]
fn a_change_then_a_removal_of_the_same_node_change_is_undone_whole() {
    let mut file = fig(document());
    let before = snapshot(&file);
    let mut undo = Undo::default();
    Graft::new(&mut file).expect("claims grafta's session").recording(&mut undo).move_under(CHILD, SECOND_PAGE).expect("moves");
    let changes = node_changes_mut(&mut file).expect("has node changes");
    undo.remove_where(changes, |nc| id_of(nc) == Some(CHILD) || id_of(nc) == Some(PAGE));
    undo.restore(&mut file);
    assert_eq!(snapshot(&file), before);
}

#[test]
fn a_graft_that_writes_nothing_records_nothing() {
    let mut file = fig(document());
    let mut undo = Undo::default();
    let refused = Graft::new(&mut file).expect("claims grafta's session").recording(&mut undo).rename(Guid::new(9, 9), "x");
    assert!(refused.is_err());
    assert!(undo.is_empty());
}
