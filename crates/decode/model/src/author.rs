//! Authoring node changes no file contained.
//!
//! The rest of this crate reads a document into a scene graph. This module goes
//! the other way and writes nodes *into* a document: a page, a frame, an
//! instance, each with an id grafta allocated, appended to the `nodeChanges`
//! array a real file arrived with.
//!
//! It exists to answer one question the round trip could not — *does Figma
//! accept a document grafta assembled, rather than one it re-emitted?* — and
//! four rules keep the answer trustworthy.
//!
//! **Every field is one Figma writes.** The field sets below are not a reading
//! of the schema's 553 `NodeChange` fields; they are the fields present on
//! *every* node of that type across the fixtures. All 153,936 frames measured
//! share thirteen, and seven real frames carry exactly those thirteen and
//! nothing else. An authored frame is a copy of a shape Figma itself wrote.
//!
//! **Nothing is invented to look complete.** An authored instance carries no
//! `derivedSymbolData` — that is Figma's expansion of the instance and grafta
//! cannot produce one honestly. 43 of 171,159 real instances carry none either,
//! so this is a shape the format already contains. Synthesising a plausible one
//! would be a value nobody could tell from a real one, which is the failure
//! `docs/ir/04-the-test.md` was written about.
//!
//! **An allocated id belongs to grafta and says so.** Every authored node sits
//! in one reserved session, [`GRAFTA_SESSION`], which collides with nothing in
//! any file measured — the highest session id in any fixture is 59,134 — and
//! which makes a node's author readable off its guid for as long as the node
//! exists. Collision is still checked rather than trusted: a duplicate guid
//! does not fail anywhere, it addresses somebody else's node, and every layer
//! downstream believes it.
//!
//! **Nothing already in the document is touched.** Authored nodes are appended,
//! a parent before its children. The one exception is [`Graft::reparent`],
//! which moves a node grafta authored and refuses to move one it did not.


use figfile::FigFile;
use indexmap::IndexMap;
use kiwi::Value;
use thiserror::Error;
use schema::{WireName, fields, object, BlendMode, ComponentPropNodeField, ComponentPropType, Field, LineType, NodePhase, PaintType, SourceDirectionality, StrokeAlign, StrokeJoin, StyleType, TextAlignVertical, TextAutoResize, VariableDataType, VariableField, VariableResolvedDataType, Wire, WireMut};

/// The session id every node grafta authors is allocated in: `0x67726166`, the
/// bytes `graf`.
///
/// One constant satisfies three requirements at once. It collides with nothing
/// measured (the highest session id across the four fixtures is 59,134); it is
/// deterministic, so the same input writes the same bytes on every run; and it
/// is recognisable, so "who authored this node" survives any number of round
/// trips. A session id derived from the input — `max + 1` — would satisfy the
/// first two and lose the third.
pub const GRAFTA_SESSION: u32 = 0x6772_6166;

/// The position key a first child takes. Every file measured starts its
/// children here.
const FIRST_POSITION: &str = "!";

/// The key after the largest sibling's, as Figma's own run: its last character
/// the next one up, and at the last character there is, one more. So a character
/// lasts 94 children, where a parent's keys would cost the square of them if each
/// were a character longer than the last: `nuxtui.fig`'s 11,216 under one parent
/// reach 443 characters.
fn key_after(largest: &str) -> String {
    let mut key = largest.as_bytes().to_vec();
    match key.last_mut() {
        Some(last) if *last < KEY_MAX => *last += 1,
        _ => key.extend_from_slice(FIRST_POSITION.as_bytes()),
    }
    String::from_utf8(key).unwrap_or_else(|_| format!("{largest}{FIRST_POSITION}"))
}

#[derive(Debug, Error)]
pub enum AuthorError {
    #[error("this document has no nodeChanges array to author into")]
    NoNodeChanges,
    #[error("this document has no root: every node change carries a parentIndex")]
    NoRoot,
    #[error(
        "session {0} is already in use in this document, so grafta cannot claim it \
         as its own — and quietly taking another one would make an authored node \
         indistinguishable from Figma's"
    )]
    SessionInUse(u32),
    #[error("{0} is not a node in this document")]
    UnknownNode(Guid),
    #[error("{0} carries no size, so an instance of it has no size to take")]
    NoSize(Guid),
    #[error("{0} was not authored by grafta, and this is not the feature that edits Figma's nodes")]
    NotOurs(Guid),
    #[error("a layer's {} is not bound to a property by grafta", .0.as_wire())]
    NotBindable(ComponentPropNodeField),
}

// ── Identity ─────────────────────────────────────────────────────────────────

pub use schema::Guid;

/// Whether grafta authored the node an id addresses. The session id *is* the
/// provenance mark, so this survives writing, reading and every round trip.
pub trait Provenance {
    fn is_graftas(&self) -> bool;
}

impl Provenance for Guid {
    fn is_graftas(&self) -> bool {
        self.session_id == GRAFTA_SESSION
    }
}

/// Hands out ids in grafta's reserved session, having first checked that the
/// document does not already use it.
#[derive(Debug)]
pub struct Allocator {
    next_local: u32,
}

impl Allocator {
    /// Scan a document and claim grafta's session.
    ///
    /// The scan is the point. A colliding guid raises nothing anywhere in this
    /// pipeline — it addresses another node and the whole stack believes it —
    /// so the one cheap moment to find out is before the first id is handed
    /// out.
    pub fn new(node_changes: &[Value]) -> Result<Self, AuthorError> {
        let taken = node_changes
            .iter()
            .filter_map(|nc| nc.field(Field::Guid).and_then(Guid::read))
            .any(|id| id.session_id == GRAFTA_SESSION);

        if taken {
            return Err(AuthorError::SessionInUse(GRAFTA_SESSION));
        }
        Ok(Self { next_local: 0 })
    }

    /// Continue grafta's session in a document that may already hold nodes
    /// grafta authored: a file saved by an earlier session, or a document
    /// edited more than once.
    ///
    /// Collision-free for the same reason [`Allocator::new`] is: ids start
    /// after the highest local id already in grafta's session, so nothing
    /// grafta hands out can address a node that exists. What it gives up is
    /// the guarantee that every grafta node in the document is new, which is
    /// the point of resuming.
    #[must_use]
    pub fn resume(node_changes: &[Value]) -> Self {
        let next_local = node_changes
            .iter()
            .filter_map(|nc| nc.field(Field::Guid).and_then(Guid::read))
            .filter(|id| id.session_id == GRAFTA_SESSION)
            .map(|id| id.local_id + 1)
            .max()
            .unwrap_or(0);
        Self { next_local }
    }

    /// The next id. Deterministic: no clock, no randomness, no global state.
    pub fn allocate(&mut self) -> Guid {
        let id = Guid::new(GRAFTA_SESSION, self.next_local);
        self.next_local += 1;
        id
    }

    /// How many ids have been handed out.
    #[must_use]
    pub fn allocated(&self) -> u32 {
        self.next_local
    }
}

// ── Order among siblings ─────────────────────────────────────────────────────

/// A position key that sorts after every existing child of `parent`.
///
/// Figma orders siblings by a string, compared byte-wise, and this crate's own
/// reader does the same. Real keys run from one character to 443 of them over
/// the whole printable ASCII range, which is a fractional index — but inserting
/// *between* two siblings is not something this feature ever does. Appending is
/// enough, and appending is exact: `x < x + c` for any non-empty suffix.
#[must_use]
pub fn position_after(node_changes: &[Value], parent: Guid) -> String {
    let largest = node_changes
        .iter()
        .filter(|nc| parent_of(nc) == Some(parent))
        .filter_map(position_of)
        .max();

    largest.map_or_else(|| FIRST_POSITION.to_string(), key_after)
}

/// The smallest and largest characters a position key uses: every key in the
/// fixtures, 235,649 of them, is printable ASCII from space to `~`.
const KEY_MIN: u8 = b' ';
const KEY_MAX: u8 = b'~';

/// A position key that sorts strictly between `lower` and `upper`, byte by
/// byte, or `None` when no key does. `None` for `lower` means
/// before every key; `None` for `upper`, after every key.
///
/// Built a character at a time. While the key still equals `lower`'s prefix
/// it must not go below `lower`; while it equals `upper`'s, it must stay below
/// `upper`. The first character with room on both sides ends it, in the middle
/// of that room, and never as a space. No key fits between `x` and `x ` (the
/// same key plus the smallest character), and that is the one case this
/// refuses; the keys it makes never end in a space, so it never creates one.
#[must_use]
pub fn position_between(lower: Option<&str>, upper: Option<&str>) -> Option<String> {
    let (lower, upper) = (lower.unwrap_or("").as_bytes(), upper.map(str::as_bytes));
    if let Some(upper) = upper {
        if lower >= upper {
            return None;
        }
    }
    let mut key = Vec::new();
    // The key only ever leaves `lower`'s prefix by going above it, and then it
    // is complete, so only the upper bound needs tracking.
    let mut below_upper_prefix = upper.is_some();
    for i in 0.. {
        // The range, exclusive at both ends, the next character may take.
        // Past the end of `lower` the floor is the smallest character itself,
        // so a key never ends in it: a key ending in a space leaves no room
        // between it and the key without that space.
        let low = lower.get(i).map_or(i32::from(KEY_MIN), |&c| i32::from(c));
        let high = match (below_upper_prefix, upper.and_then(|u| u.get(i))) {
            (true, Some(&c)) => i32::from(c),
            // `upper` is a prefix of the key so far: nothing longer is below it.
            (true, None) => return None,
            _ => i32::from(KEY_MAX) + 1,
        };
        if high - low >= 2 {
            key.push(u8::try_from((low + high) / 2).ok()?);
            return String::from_utf8(key).ok();
        }
        if high - low == 1 {
            // No room here: take the lower character, and go past it later.
            key.push(u8::try_from(low).ok()?);
            below_upper_prefix = false;
            continue;
        }
        // Both bounds share this character.
        key.push(u8::try_from(low).ok()?);
        if i > 1000 {
            return None;
        }
    }
    None
}

fn parent_of(node_change: &Value) -> Option<Guid> {
    node_change.field(Field::ParentIndex)?.field(Field::Guid).and_then(Guid::read)
}

fn position_of(node_change: &Value) -> Option<&str> {
    node_change.field(Field::ParentIndex)?.field(Field::Position)?.as_str()
}

fn id_of(node_change: &Value) -> Option<Guid> {
    node_change.field(Field::Guid).and_then(Guid::read)
}

/// The parameter a layer's property-bound field is consumed as; `None` for a
/// field grafta does not bind.
fn consumed_field(field: ComponentPropNodeField) -> Option<VariableField> {
    match field {
        ComponentPropNodeField::TextData => Some(VariableField::TextData),
        ComponentPropNodeField::Visible => Some(VariableField::Visible),
        ComponentPropNodeField::OverriddenSymbolId => Some(VariableField::OverriddenSymbolId),
        ComponentPropNodeField::InheritFillStyleId | ComponentPropNodeField::SlotContentId => None,
    }
}

// ── The nodes ────────────────────────────────────────────────────────────────

fn parent_index(parent: Guid, position: &str) -> Value {
    object([
        (Field::Guid, parent.to_value()),
        (Field::Position, Value::Str(position.to_string())),
    ])
}

fn vector(x: f32, y: f32) -> Value {
    object([
        (Field::X, Value::Float(x)),
        (Field::Y, Value::Float(y)),
    ])
}

/// The identity transform. Every authored node uses it: position inside a
/// parent is the parent's business, and nothing here is being placed by hand.
fn identity_transform() -> Value {
    object([
        (Field::M00, Value::Float(1.0)),
        (Field::M01, Value::Float(0.0)),
        (Field::M02, Value::Float(0.0)),
        (Field::M10, Value::Float(0.0)),
        (Field::M11, Value::Float(1.0)),
        (Field::M12, Value::Float(0.0)),
    ])
}

/// Move an authored node by setting its transform's translation.
fn translated(mut node: Value, (x, y): (f32, f32)) -> Value {
    if let Value::Object(fields) = &mut node {
        if let Some(Value::Object(transform)) = fields.field_mut(Field::Transform) {
            transform.set(Field::M02, Value::Float(x));
            transform.set(Field::M12, Value::Float(y));
        }
    }
    node
}

/// The eight fields every authored node carries, whatever its type. They are
/// also exactly the `CANVAS` intersection, which is why [`page`] adds nothing
/// to them.
fn common(id: Guid, parent: Guid, position: &str, node_type: schema::NodeType, name: &str) -> kiwi::Fields {
    fields([
        (Field::Guid, id.to_value()),
        (Field::Phase, Value::from(NodePhase::Created)),
        (Field::ParentIndex, parent_index(parent, position)),
        (Field::Type, Value::from(node_type)),
        (Field::Name, Value::Str(name.to_string())),
        (Field::Visible, Value::Bool(true)),
        (Field::Opacity, Value::Float(1.0)),
        (Field::Transform, identity_transform()),
    ])
}

fn with_stroke(fields: &mut kiwi::Fields) {
    fields.set(Field::StrokeWeight, Value::Float(1.0));
    fields.set(Field::StrokeAlign, Value::from(StrokeAlign::Inside));
    fields.set(Field::StrokeJoin, Value::from(StrokeJoin::Miter));
}

/// A frame: the thirteen fields all 153,936 measured frames share, and which
/// seven real frames carry alone.
///
/// No `fillGeometry` and no `strokeGeometry`. Both index into the message's
/// `blobs` array, and the real thirteen-field frame carries neither — so
/// authoring cannot invalidate a blob index, by construction.
#[must_use]
pub fn frame(id: Guid, parent: Guid, position: &str, name: &str, size: (f32, f32)) -> Value {
    let mut fields = common(id, parent, position, schema::NodeType::Frame, name);
    fields.set(Field::Size, vector(size.0, size.1));
    with_stroke(&mut fields);
    fields.set(Field::FrameMaskDisabled, Value::Bool(true));
    Value::Object(fields)
}

/// An instance: a frame's fields plus the symbol it points at.
///
/// `derivedSymbolData` is deliberately absent — see the module documentation.
/// The symbol must live in the same file; instantiating from a published
/// library needs identity the profile does not carry, and that is a later
/// feature.
#[must_use]
pub fn instance(
    id: Guid,
    parent: Guid,
    position: &str,
    name: &str,
    size: (f32, f32),
    symbol: Guid,
) -> Value {
    let mut fields = common(id, parent, position, schema::NodeType::Instance, name);
    fields.set(Field::Size, vector(size.0, size.1));
    with_stroke(&mut fields);
    fields.set(Field::FrameMaskDisabled, Value::Bool(true));
    fields.set(Field::SymbolData,
        object([(Field::SymbolId, symbol.to_value())]),
    );
    Value::Object(fields)
}

/// A page: the eight fields all 166 measured canvases share.
///
/// This is the one authored shape smaller than anything a file actually
/// contains — the smallest real canvas carries twelve. Rung R3 is where that
/// difference gets tested, and a refusal there is a finding rather than a
/// surprise.
#[must_use]
pub fn page(id: Guid, parent: Guid, position: &str, name: &str) -> Value {
    Value::Object(common(id, parent, position, schema::NodeType::Canvas, name))
}

// ── The graft ────────────────────────────────────────────────────────────────

/// What a prune removed, so that a file which stops opening can be compared
/// against one that did not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PruneReport {
    pub kept: usize,
    pub removed: usize,
}

/// Authoring nodes into a document that already exists.
///
/// Everything it adds is appended, a parent before its children. Nothing that
/// was already in the array is reordered, renamed or repaired — a graft that
/// improves the file is not a graft.
pub struct Graft<'a> {
    fig: &'a mut FigFile,
    allocator: Allocator,
    memo: Option<&'a mut Memo>,
    index: Option<&'a IndexMap<Guid, usize>>,
    undo: Option<&'a mut Undo>,
}

/// The id Figma gives a variant property: the SHA-1 of its name, the first
/// word little-endian with its low bit cleared, the second with its high bit
/// set. All 229 variant property names in four fixtures, from four sources,
/// have exactly this id in every set that uses them. A set whose ids differ
/// is shown by Figma as "conflicting" (the first file of a spike opened in Figma).
#[must_use]
pub fn variant_property_id(name: &str) -> Guid {
    use sha1::{Digest, Sha1};
    let digest = Sha1::digest(name.as_bytes());
    let word = |at: usize| u32::from_le_bytes([digest[at], digest[at + 1], digest[at + 2], digest[at + 3]]);
    Guid::new(word(4) | 0x8000_0000, word(0) & !1)
}

/// A component becoming a variant: its id, its `Property=Value` pairs, and
/// where it sits in the set.
pub type Variant = (Guid, Vec<(String, String)>, (f32, f32));

// ── Variables ────────────────────────────────────────────────────────────────

/// A variable's value in one mode.
#[derive(Debug, Clone, PartialEq)]
pub enum VarValue {
    /// Channels from 0 to 1, alpha last.
    Color(f32, f32, f32, f32),
    Float(f32),
    String(String),
    Boolean(bool),
    /// Another local variable, of the same type.
    Alias(Guid),
}

/// A variable's type, as `variableResolvedType` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarType {
    Color,
    Float,
    String,
    Boolean,
}

impl VarType {
    #[must_use]
    pub fn resolved_type(self) -> VariableResolvedDataType {
        match self {
            Self::Color => VariableResolvedDataType::Color,
            Self::Float => VariableResolvedDataType::Float,
            Self::String => VariableResolvedDataType::String,
            Self::Boolean => VariableResolvedDataType::Boolean,
        }
    }
}

/// `{ value, dataType, resolvedDataType }`, as on all 9,272 values measured:
/// `dataType` is the value's own, `ALIAS` for an alias, and `resolvedDataType`
/// the variable's.
fn variable_data(var_type: VarType, value: &VarValue) -> Value {
    let (field, inner, own) = match value {
        VarValue::Color(r, g, b, a) => (
            Field::ColorValue,
            object([
                (Field::R, Value::Float(*r)),
                (Field::G, Value::Float(*g)),
                (Field::B, Value::Float(*b)),
                (Field::A, Value::Float(*a)),
            ]),
            VariableDataType::Color,
        ),
        VarValue::Float(n) => (Field::FloatValue, Value::Float(*n), VariableDataType::Float),
        VarValue::String(s) => (Field::TextValue, Value::Str(s.clone()), VariableDataType::String),
        VarValue::Boolean(b) => (Field::BoolValue, Value::Bool(*b), VariableDataType::Boolean),
        VarValue::Alias(target) => (
            Field::Alias,
            object([(Field::Guid, target.to_value())]),
            VariableDataType::Alias,
        ),
    };
    object([
        (Field::Value, object([(field, inner)])),
        (Field::DataType, Value::from(own)),
        (Field::ResolvedDataType, Value::from(var_type.resolved_type())),
    ])
}

fn mode_entry(mode: Guid, var_type: VarType, value: &VarValue) -> Value {
    object([
        (Field::ModeId, mode.to_value()),
        (Field::VariableData, variable_data(var_type, value)),
    ])
}

/// The `version` every local collection and variable carries. None can be
/// derived; this one, the node's own id, opened in Figma,
/// and a collection without it did not (v2).
fn own_version(id: Guid) -> Value {
    Value::Str(id.to_string())
}

// ── Component properties ─────────────────────────────────────────────────────

/// A component property's value: a text, a boolean, or the
/// component an instance-swap property shows.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    Text(String),
    Bool(bool),
    Component(Guid),
}

impl PropValue {
    /// The definition's `type`.
    #[must_use]
    pub fn kind(&self) -> ComponentPropType {
        match self {
            Self::Text(_) => ComponentPropType::Text,
            Self::Bool(_) => ComponentPropType::Bool,
            Self::Component(_) => ComponentPropType::InstanceSwap,
        }
    }

    /// The data type its `varValue` carries.
    fn data_type(&self) -> VariableDataType {
        match self {
            Self::Text(_) => VariableDataType::TextData,
            Self::Bool(_) => VariableDataType::Boolean,
            Self::Component(_) => VariableDataType::SymbolId,
        }
    }

    /// The same, as a value resolves to.
    fn resolved_type(&self) -> VariableResolvedDataType {
        match self {
            Self::Text(_) => VariableResolvedDataType::TextData,
            Self::Bool(_) => VariableResolvedDataType::Boolean,
            Self::Component(_) => VariableResolvedDataType::SymbolId,
        }
    }

    /// `{ textValue | boolValue | guidValue }`.
    fn value(&self) -> Value {
        let (k, v) = match self {
            Self::Text(t) => (Field::TextValue, text_data_value(t)),
            Self::Bool(b) => (Field::BoolValue, Value::Bool(*b)),
            Self::Component(c) => (Field::GuidValue, c.to_value()),
        };
        object([(k, v)])
    }

    /// The same value, typed, as every definition and assignment carries it.
    fn var_value(&self) -> Value {
        let (k, v) = match self {
            Self::Text(t) => (Field::TextDataValue, text_data_value(t)),
            Self::Bool(b) => (Field::BoolValue, Value::Bool(*b)),
            Self::Component(c) => (Field::SymbolIdValue, object([(Field::Guid, c.to_value())])),
        };
        object([
            (Field::Value, object([(k, v)])),
            (Field::DataType, Value::from(self.data_type())),
            (Field::ResolvedDataType, Value::from(self.resolved_type())),
        ])
    }
}

/// A text's `textData`: one `PLAIN` line per paragraph, as every measured file.
fn text_data_value(characters: &str) -> Value {
    let lines: Vec<Value> = characters
        .split('\n')
        .map(|_| {
            object([
                (Field::LineType, Value::from(LineType::Plain)),
                (Field::LineStyleId, Value::Int(0)),
                (Field::IndentationLevel, Value::Int(0)),
                (Field::SourceDirectionality, Value::from(SourceDirectionality::Auto)),
                (Field::ListStartOffset, Value::Int(0)),
                (Field::IsFirstLineOfList, Value::Bool(false)),
            ])
        })
        .collect();
    object([
        (Field::Characters, Value::Str(characters.to_string())),
        (Field::Lines, Value::Array(lines)),
    ])
}

/// Where grafta's property ids start. The small ids right after grafta's own
/// nodes made Figma call a property "not used within component", while
/// `1735549286:900000` worked, as did the same component with ids far from
/// its nodes (each a spike's file opened in Figma). Why is not known.
pub const PROPERTY_IDS_FROM: u32 = 900_000;

// ── Shapes ───────────────────────────────────────────────────────────────────

/// A shape's kind, and the node type it is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    /// Written `ROUNDED_RECTANGLE`, as 9,935 of 9,963 local rectangles are.
    Rectangle,
    Ellipse,
    Line,
    Polygon,
    Star,
}

impl ShapeKind {
    #[must_use]
    pub fn node_type(self) -> schema::NodeType {
        match self {
            Self::Rectangle => schema::NodeType::RoundedRectangle,
            Self::Ellipse => schema::NodeType::Ellipse,
            Self::Line => schema::NodeType::Line,
            Self::Polygon => schema::NodeType::RegularPolygon,
            Self::Star => schema::NodeType::Star,
        }
    }
}

// ── Styles ───────────────────────────────────────────────────────────────────

/// A style's kind, and the node it is: 4,542 live and
/// soft-deleted fill styles are rounded rectangles, 423 text styles texts, 75
/// effect styles rounded rectangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleKind {
    Fill,
    Text,
    Effect,
}

impl StyleKind {
    fn style_type(self) -> StyleType {
        match self {
            Self::Fill => StyleType::Fill,
            Self::Text => StyleType::Text,
            Self::Effect => StyleType::Effect,
        }
    }
}

fn solid_paint(r: f32, g: f32, b: f32) -> Value {
    object([
        (Field::Type, Value::from(PaintType::Solid)),
        (Field::Color,
            object([
                (Field::R, Value::Float(r)),
                (Field::G, Value::Float(g)),
                (Field::B, Value::Float(b)),
                (Field::A, Value::Float(1.0)),
            ]),
        ),
        (Field::Opacity, Value::Float(1.0)),
        (Field::Visible, Value::Bool(true)),
        (Field::BlendMode, Value::from(BlendMode::Normal)),
    ])
}

/// The fields every live text style carries besides its typography, as the
/// a spike wrote them: a sample text Figma lays out itself.
fn text_style_fields(fields: &mut kiwi::Fields) {
    fields.set(Field::Size, vector(120.0, 34.0));
    fields.set(Field::FillPaints, Value::Array(vec![solid_paint(0.0, 0.0, 0.0)]));
    fields.set(Field::TextAlignVertical, Value::from(TextAlignVertical::Top));
    fields.set(Field::TextAutoResize, Value::from(TextAutoResize::WidthAndHeight));
    fields.set(Field::AutoRename, Value::Bool(false));
    fields.set(Field::FontVariantCommonLigatures, Value::Bool(false));
    fields.set(Field::FontVariantContextualLigatures, Value::Bool(false));
    fields.set(Field::FontVersion, Value::Str(String::new()));
    fields.set(Field::TextUserLayoutVersion, Value::Uint(3));
    let line = object([
        (Field::LineType, Value::from(LineType::Plain)),
        (Field::LineStyleId, Value::Int(0)),
        (Field::IndentationLevel, Value::Int(0)),
        (Field::SourceDirectionality, Value::from(SourceDirectionality::Auto)),
        (Field::ListStartOffset, Value::Int(0)),
        (Field::IsFirstLineOfList, Value::Bool(false)),
    ]);
    fields.set(Field::TextData,
        object([
            (Field::Characters, Value::Str("Rag 123".to_string())),
            (Field::Lines, Value::Array(vec![line])),
        ]),
    );
}

/// A mode as all 104 measured are: `{ id, name, sortPosition }`.
fn mode_value(id: Guid, name: &str, sort: &str) -> Value {
    object([
        (Field::Id, id.to_value()),
        (Field::Name, Value::Str(name.to_string())),
        (Field::SortPosition, Value::Str(sort.to_string())),
    ])
}

fn change_entries(change: &mut Value) -> Option<&mut Value> {
    let Value::Object(fields) = change else { return None };
    let Some(Value::Object(data)) = fields.field_mut(Field::VariableDataValues) else { return None };
    data.field_mut(Field::Entries)
}

/// What a caller that grafts many times into one document can keep between
/// grafts, so each one costs what it adds rather than a scan of the file
///: the next id, and the last position handed out under each
/// parent. Both only grow, so a node removed since cannot make them collide
/// with anything; they may just skip what a fresh scan would reuse.
#[derive(Debug, Default, Clone)]
pub struct Memo {
    next_local: Option<u32>,
    last_position: std::collections::HashMap<Guid, String>,
}

/// What a call changed in a document, to put it back when the call fails
///: the node changes appended or removed and the blobs added, in order,
/// and the first value of each node change changed in place. It holds what was
/// touched, not a copy of the document.
#[derive(Debug, Default)]
pub struct Undo {
    steps: Vec<Step>,
    changed: std::collections::HashMap<Guid, Value>,
}

#[derive(Debug)]
enum Step {
    Appended,
    /// Node changes taken out, each with its position before they went.
    Removed(Vec<(usize, Value)>),
    /// A blob added; `created` when the message had no blobs before it.
    Blob { created: bool },
}

impl Undo {
    /// Whether nothing was recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty() && self.changed.is_empty()
    }

    /// `change` is about to be changed in place: its value now is kept, if it
    /// is the first change to it.
    pub fn changing(&mut self, change: &Value) {
        if let Some(id) = id_of(change) {
            self.changed.entry(id).or_insert_with(|| change.clone());
        }
    }

    /// Take out of `changes` every node change `remove` names, keeping each.
    pub fn remove_where(&mut self, changes: &mut Vec<Value>, remove: impl Fn(&Value) -> bool) {
        let mut removed = Vec::new();
        let mut kept = Vec::with_capacity(changes.len());
        for (at, change) in std::mem::take(changes).into_iter().enumerate() {
            if remove(&change) {
                removed.push((at, change));
            } else {
                kept.push(change);
            }
        }
        *changes = kept;
        if !removed.is_empty() {
            self.steps.push(Step::Removed(removed));
        }
    }

    fn appended(&mut self) {
        self.steps.push(Step::Appended);
    }

    fn blob_added(&mut self, created: bool) {
        self.steps.push(Step::Blob { created });
    }

    /// Put `fig` back as it was before the first thing recorded.
    pub fn restore(self, fig: &mut FigFile) {
        let Self { steps, mut changed } = self;
        for step in steps.into_iter().rev() {
            match step {
                Step::Appended => {
                    if let Some(changes) = node_changes_mut(fig) {
                        changes.pop();
                    }
                }
                Step::Removed(removed) => {
                    if let Some(changes) = node_changes_mut(fig) {
                        let mut merged = Vec::with_capacity(changes.len() + removed.len());
                        let mut rest = std::mem::take(changes).into_iter();
                        for (at, change) in removed {
                            merged.extend(rest.by_ref().take(at - merged.len()));
                            merged.push(change);
                        }
                        merged.extend(rest);
                        *changes = merged;
                    }
                }
                Step::Blob { created } => {
                    if let Value::Object(message) = &mut fig.node_changes {
                        if created {
                            message.take(Field::Blobs);
                        } else if let Some(Value::Array(blobs)) = message.field_mut(Field::Blobs) {
                            blobs.pop();
                        }
                    }
                    fig.blobs.pop();
                }
            }
        }
        if changed.is_empty() {
            return;
        }
        if let Some(changes) = node_changes_mut(fig) {
            for change in changes.iter_mut() {
                if let Some(first) = id_of(change).and_then(|id| changed.remove(&id)) {
                    *change = first;
                }
            }
        }
    }
}

impl<'a> Graft<'a> {
    /// Claim grafta's session in this document, or say why not.
    pub fn new(fig: &'a mut FigFile) -> Result<Self, AuthorError> {
        if node_changes_mut(fig).is_none() {
            return Err(AuthorError::NoNodeChanges);
        }
        let allocator = Allocator::new(node_changes(fig))?;
        Ok(Self { fig, allocator, memo: None, index: None, undo: None })
    }

    /// Author into a document that may already hold grafta's nodes, continuing
    /// its session rather than refusing it. See [`Allocator::resume`].
    pub fn resume(fig: &'a mut FigFile) -> Result<Self, AuthorError> {
        if node_changes_mut(fig).is_none() {
            return Err(AuthorError::NoNodeChanges);
        }
        let allocator = Allocator::resume(node_changes(fig));
        Ok(Self { fig, allocator, memo: None, index: None, undo: None })
    }

    /// [`Graft::resume`], remembering between grafts in `memo`, and finding
    /// nodes through `index` (id → position in `nodeChanges`) when it is given.
    pub fn resume_with(
        fig: &'a mut FigFile,
        memo: &'a mut Memo,
        index: Option<&'a IndexMap<Guid, usize>>,
    ) -> Result<Self, AuthorError> {
        if node_changes_mut(fig).is_none() {
            return Err(AuthorError::NoNodeChanges);
        }
        let allocator = match memo.next_local {
            Some(next_local) => Allocator { next_local },
            None => Allocator::resume(node_changes(fig)),
        };
        Ok(Self { fig, allocator, memo: Some(memo), index, undo: None })
    }

    /// Record in `undo` what this graft changes, so it can be put back.
    #[must_use]
    pub fn recording(mut self, undo: &'a mut Undo) -> Self {
        self.undo = Some(undo);
        self
    }

    /// An id no node or other authored object of this file has, remembered
    /// for the next graft: for a prototype interaction grafta adds.
    pub fn allocate_id(&mut self) -> Guid {
        self.allocate()
    }

    /// The next id, remembered for the next graft.
    fn allocate(&mut self) -> Guid {
        let id = self.allocator.allocate();
        if let Some(memo) = self.memo.as_deref_mut() {
            memo.next_local = Some(self.allocator.next_local);
        }
        id
    }

    /// A position after every child `parent` has, remembered for the next one.
    fn next_position(&mut self, parent: Guid) -> String {
        let remembered = self.memo.as_deref().and_then(|memo| memo.last_position.get(&parent).cloned());
        let position = match remembered {
            Some(last) => key_after(&last),
            None => position_after(node_changes(self.fig), parent),
        };
        if let Some(memo) = self.memo.as_deref_mut() {
            memo.last_position.insert(parent, position.clone());
        }
        position
    }

    /// A child reached `parent` another way, so the key remembered may no longer
    /// be its largest.
    fn forget_position(&mut self, parent: Guid) {
        if let Some(memo) = self.memo.as_deref_mut() {
            memo.last_position.remove(&parent);
        }
    }

    /// The document root: the one node change with no `parentIndex`.
    ///
    /// It is `0:0` in every file measured, and it is still *found* rather than
    /// assumed. Finding it costs one pass and removes a hardcoded fact about
    /// somebody else's format.
    pub fn root(&self) -> Result<Guid, AuthorError> {
        node_changes(self.fig)
            .iter()
            .find(|nc| nc.field(Field::ParentIndex).is_none())
            .and_then(id_of)
            .ok_or(AuthorError::NoRoot)
    }

    /// The first page in the document, for callers that need somewhere to graft
    /// onto and do not care which.
    #[must_use]
    pub fn first_page(&self) -> Option<Guid> {
        node_changes(self.fig)
            .iter()
            .find(|nc| nc.is(Field::Type, schema::NodeType::Canvas))
            .and_then(id_of)
    }

    /// The first component definition in the document, for callers that need
    /// something to instantiate and do not care which.
    #[must_use]
    pub fn first_symbol(&self) -> Option<Guid> {
        node_changes(self.fig)
            .iter()
            .find(|nc| nc.is(Field::Type, schema::NodeType::Symbol))
            .and_then(id_of)
    }

    /// How many ids have been allocated so far.
    #[must_use]
    pub fn allocated(&self) -> u32 {
        self.allocator.allocated()
    }

    /// Add a page to the document root.
    pub fn add_page(&mut self, name: &str) -> Result<Guid, AuthorError> {
        let parent = self.root()?;
        let position = self.next_position(parent);
        let id = self.allocate();
        self.append(page(id, parent, &position, name))?;
        Ok(id)
    }

    /// Add a frame to `parent`, which must be a node the document contains.
    pub fn add_frame(
        &mut self,
        parent: Guid,
        name: &str,
        size: (f32, f32),
    ) -> Result<Guid, AuthorError> {
        self.add_frame_at(parent, name, size, (0.0, 0.0))
    }

    /// Add a frame to `parent` at `at`, relative to the parent.
    ///
    /// A position is the transform's translation. The fields written are the
    /// same thirteen as at the origin; only two of their values differ.
    pub fn add_frame_at(
        &mut self,
        parent: Guid,
        name: &str,
        size: (f32, f32),
        at: (f32, f32),
    ) -> Result<Guid, AuthorError> {
        self.require_node(parent)?;
        let position = self.next_position(parent);
        let id = self.allocate();
        self.append(translated(frame(id, parent, &position, name, size), at))?;
        Ok(id)
    }

    /// Add an instance of `symbol` to `parent`.
    ///
    /// The instance takes the symbol's own size, because a component
    /// instantiated at a size nobody chose is a layout decision this feature
    /// has no business making. Both nodes must exist: an instance pointing at
    /// nothing is not a smaller test, it is a different one.
    pub fn add_instance(
        &mut self,
        parent: Guid,
        name: &str,
        symbol: Guid,
    ) -> Result<Guid, AuthorError> {
        self.add_instance_at(parent, name, symbol, (0.0, 0.0))
    }

    /// Add an instance of `symbol` to `parent` at `at`, relative to the parent.
    pub fn add_instance_at(
        &mut self,
        parent: Guid,
        name: &str,
        symbol: Guid,
        at: (f32, f32),
    ) -> Result<Guid, AuthorError> {
        self.require_node(parent)?;
        let definition = self.require_node(symbol)?;
        let size = definition
            .field(Field::Size)
            .and_then(|size| Some((size.field(Field::X)?.as_f32()?, size.field(Field::Y)?.as_f32()?)))
            .ok_or(AuthorError::NoSize(symbol))?;

        // Figma copies these from the component into every instance, and a
        // value that differs is an override: opacity, stroke and clip equal
        // their component's on 96.3–100% of 79,384 instances measured. Fixed
        // defaults here overrode the component's own (the eighth session: an
        // opacity that never reached the instances).
        let inherited: Vec<(Field, Value)> = [Field::Opacity, Field::StrokeWeight, Field::StrokeAlign, Field::StrokeJoin, Field::FrameMaskDisabled]
            .into_iter()
            .filter_map(|field| Some((field, definition.field(field)?.clone())))
            .collect();

        let position = self.next_position(parent);
        let id = self.allocate();
        let mut node = instance(id, parent, &position, name, size, symbol);
        if let Value::Object(fields) = &mut node {
            for (field, value) in inherited {
                fields.set(field, value);
            }
        }
        self.append(translated(node, at))?;
        Ok(id)
    }

    /// Add a text layer to `parent` at `at`: the fields every authored node
    /// carries, a size, and the text's own fields as the caller built them.
    ///
    /// No `derivedTextData` is written, ever. Figma lays text out itself when
    /// it is absent, which a spike opened in Figma showed on 2026-09-25, even
    /// for characters the box was not sized for.
    pub fn add_text_at(
        &mut self,
        parent: Guid,
        name: &str,
        size: (f32, f32),
        at: (f32, f32),
        text_fields: kiwi::Fields,
    ) -> Result<Guid, AuthorError> {
        self.require_node(parent)?;
        let position = self.next_position(parent);
        let id = self.allocate();
        let mut fields = common(id, parent, &position, schema::NodeType::Text, name);
        fields.set(Field::Size, vector(size.0, size.1));
        fields.set(Field::StrokeJoin, Value::from(StrokeJoin::Miter));
        fields.extend(text_fields.into_iter().filter(|(k, _)| *k != "derivedTextData"));
        self.append(translated(Value::Object(fields), at))?;
        Ok(id)
    }

    /// Add a vector to `parent`: the fields every vector carries, `size`, and
    /// `vector_fields` (its network, paints, stroke and geometry), which the
    /// caller builds with [`crate::path`] and [`Graft::add_blob`].
    pub fn add_vector_at(
        &mut self,
        parent: Guid,
        name: &str,
        size: (f32, f32),
        at: (f32, f32),
        vector_fields: kiwi::Fields,
    ) -> Result<Guid, AuthorError> {
        self.require_node(parent)?;
        let position = self.next_position(parent);
        let id = self.allocate();
        let mut fields = common(id, parent, &position, schema::NodeType::Vector, name);
        fields.set(Field::Size, vector(size.0, size.1));
        fields.extend(vector_fields);
        self.append(translated(Value::Object(fields), at))?;
        Ok(id)
    }

    /// Turn a frame into a component, in place: its type becomes `SYMBOL`, its
    /// children stay, and it gains the two fields every local component in
    /// the fixtures carries (11,759 of them): `isSymbolPublishable` and a
    /// `sharedSymbolVersion` token, `session:counter`, which names no node.
    pub fn make_component(&mut self, node: Guid) -> Result<(), AuthorError> {
        let version = format!("{}:{}", GRAFTA_SESSION, node.local_id);
        let Value::Object(fields) = self.change_mut(node)? else { return Err(AuthorError::UnknownNode(node)) };
        fields.set(Field::Type, Value::from(schema::NodeType::Symbol));
        fields.set(Field::IsSymbolPublishable, Value::Bool(true));
        fields.set(Field::SharedSymbolVersion, Value::Str(version));
        Ok(())
    }

    /// Group components into a variant set: a frame marked
    /// `isStateGroup`, with one `VARIANT` property definition per property
    /// named and the order of each one's values, made the components' parent.
    /// Each component is renamed `Property=Value, …` and points at the
    /// definitions (`variantPropSpecs`), and keeps `at` relative to the set.
    /// The shape is the one all 646 local sets and their variants in three
    /// fixtures share, with Figma's default set styling: a 1 px dashed violet
    /// border, radius 5.
    pub fn make_variant_set(
        &mut self,
        parent: Guid,
        name: &str,
        (at, size): ((f32, f32), (f32, f32)),
        variants: &[Variant],
    ) -> Result<Guid, AuthorError> {
        self.require_node(parent)?;
        for (variant, _, _) in variants {
            self.require_node(*variant)?;
        }
        let position = self.next_position(parent);
        let set = self.allocate();

        // Properties and their values, in the order first seen.
        let mut properties: Vec<(String, Guid, Vec<String>)> = Vec::new();
        for (_, values, _) in variants {
            for (property, value) in values {
                let index = match properties.iter().position(|(p, _, _)| p == property) {
                    Some(index) => index,
                    None => {
                        properties.push((property.clone(), variant_property_id(property), Vec::new()));
                        properties.len() - 1
                    }
                };
                if !properties[index].2.contains(value) {
                    properties[index].2.push(value.clone());
                }
            }
        }
        let text = |s: &str| object([(Field::Characters, Value::Str(s.to_string()))]);
        let definitions: Vec<Value> = properties
            .iter()
            .enumerate()
            .map(|(i, (property, id, _))| {
                object([
                    (Field::Id, id.to_value()),
                    (Field::Name, Value::Str(property.clone())),
                    (Field::InitialValue, object([(Field::TextValue, text(""))])),
                    (Field::SortPosition, Value::Str(format!("!_{i}"))),
                    (Field::Type, Value::from(ComponentPropType::Variant)),
                    (Field::PreferredValues, Value::Object(kiwi::Fields::new())),
                    (Field::VarValue, object([
                        (Field::Value, object([(Field::TextValue, Value::Str(String::new()))])),
                        (Field::DataType, Value::from(VariableDataType::String)),
                        (Field::ResolvedDataType, Value::from(VariableResolvedDataType::String)),
                    ])),
                ])
            })
            .collect();
        let orders: Vec<Value> = properties
            .iter()
            .map(|(property, _, values)| {
                object([
                    (Field::Property, Value::Str(property.clone())),
                    (Field::Values, Value::Array(values.iter().map(|v| Value::Str(v.clone())).collect())),
                ])
            })
            .collect();
        let version = Value::Str(format!("{}:{}", GRAFTA_SESSION, set.local_id));
        let mut fields = common(set, parent, &position, schema::NodeType::Frame, name);
        fields.set(Field::Size, vector(size.0, size.1));
        fields.set(Field::IsStateGroup, Value::Bool(true));
        fields.set(Field::IsPublishable, Value::Bool(true));
        fields.set(Field::Version, version.clone());
        fields.set(Field::UserFacingVersion, version);
        fields.set(Field::StateGroupPropertyValueOrders, Value::Array(orders));
        fields.set(Field::ComponentPropDefs, Value::Array(definitions));
        fields.set(Field::FrameMaskDisabled, Value::Bool(true));
        fields.set(Field::CornerRadius, Value::Float(5.0));
        fields.set(Field::StrokeWeight, Value::Float(1.0));
        fields.set(Field::StrokeAlign, Value::from(StrokeAlign::Inside));
        fields.set(Field::StrokeJoin, Value::from(StrokeJoin::Miter));
        fields.set(Field::DashPattern, Value::Array(vec![Value::Float(10.0), Value::Float(5.0)]));
        let violet = object([
            (Field::R, Value::Float(0.592)),
            (Field::G, Value::Float(0.278)),
            (Field::B, Value::Float(1.0)),
            (Field::A, Value::Float(1.0)),
        ]);
        fields.set(Field::StrokePaints,
            Value::Array(vec![object([
                (Field::Type, Value::from(PaintType::Solid)),
                (Field::Color, violet),
                (Field::Opacity, Value::Float(1.0)),
                (Field::Visible, Value::Bool(true)),
                (Field::BlendMode, Value::from(BlendMode::Normal)),
            ])]),
        );
        self.append(translated(Value::Object(fields), at))?;

        for (variant, values, place) in variants {
            let label: Vec<String> = values.iter().map(|(p, v)| format!("{p}={v}")).collect();
            let specs: Vec<Value> = values
                .iter()
                .filter_map(|(property, value)| {
                    let (_, id, _) = properties.iter().find(|(p, _, _)| p == property)?;
                    Some(object([(Field::PropDefId, id.to_value()), (Field::Value, Value::Str(value.clone()))]))
                })
                .collect();
            let position = self.next_position(set);
            let Value::Object(fields) = self.change_mut(*variant)? else { return Err(AuthorError::UnknownNode(*variant)) };
            fields.set(Field::ParentIndex, parent_index(set, &position));
            fields.set(Field::Name, Value::Str(label.join(", ")));
            fields.set(Field::VariantPropSpecs, Value::Array(specs));
            if let Some(Value::Object(transform)) = fields.field_mut(Field::Transform) {
                transform.set(Field::M02, Value::Float(place.0));
                transform.set(Field::M12, Value::Float(place.1));
            }
        }
        Ok(set)
    }

    /// The page collections and variables live under: the one marked
    /// `internalOnly`, which each of the five fixtures has exactly one of. A
    /// file without one (as `new_file` makes) gets one, in the twelve fields
    /// the fixtures' own carry, as a spike wrote it. Returns it, and whether it
    /// was written now.
    pub fn internal_canvas(&mut self) -> Result<(Guid, bool), AuthorError> {
        let found = node_changes(self.fig)
            .iter()
            .find(|nc| nc.is(Field::Type, schema::NodeType::Canvas) && nc.bool(Field::InternalOnly) == Some(true))
            .and_then(id_of);
        if let Some(canvas) = found {
            return Ok((canvas, false));
        }
        let root = self.root()?;
        let position = self.next_position(root);
        let id = self.allocate();
        let mut fields = common(id, root, &position, schema::NodeType::Canvas, "Internal Only Canvas");
        fields.set(Field::Visible, Value::Bool(false));
        fields.set(Field::StrokeWeight, Value::Float(0.0));
        fields.set(Field::StrokeAlign, Value::from(StrokeAlign::Center));
        fields.set(Field::StrokeJoin, Value::from(StrokeJoin::Bevel));
        fields.set(Field::InternalOnly, Value::Bool(true));
        self.append(Value::Object(fields))?;
        Ok((id, true))
    }

    /// Add a variable collection with `modes`, in the shape a spike wrote and Figma opened:
    /// the eight fields all 34 local collections carry, each mode `{ id, name,
    /// sortPosition }`. Mode ids come from grafta's session, so no node shares
    /// one, as none does in the fixtures. Returns the collection and its modes.
    pub fn add_variable_collection(&mut self, name: &str, modes: &[&str]) -> Result<(Guid, Vec<Guid>), AuthorError> {
        let (canvas, _) = self.internal_canvas()?;
        let position = self.next_position(canvas);
        let set = self.allocate();
        let mut mode_ids = Vec::new();
        let mut entries = Vec::new();
        let mut sort: Option<String> = None;
        for mode in modes {
            let id = self.allocate();
            let key = position_between(sort.as_deref(), None).unwrap_or_else(|| FIRST_POSITION.to_string());
            entries.push(mode_value(id, mode, &key));
            sort = Some(key);
            mode_ids.push(id);
        }
        self.append(object([
            (Field::Guid, set.to_value()),
            (Field::Phase, Value::from(NodePhase::Created)),
            (Field::ParentIndex, parent_index(canvas, &position)),
            (Field::Type, Value::from(schema::NodeType::VariableSet)),
            (Field::Name, Value::Str(name.to_string())),
            (Field::IsPublishable, Value::Bool(true)),
            (Field::Version, own_version(set)),
            (Field::VariableSetModes, Value::Array(entries)),
        ]))?;
        Ok((set, mode_ids))
    }

    /// Add a variable to collection `set`, with a value per mode, in the
    /// shape a spike wrote and Figma opened: beside the collection, its `sortPosition` its
    /// position. The caller has checked the values against the modes.
    pub fn add_variable(
        &mut self,
        set: Guid,
        name: &str,
        var_type: VarType,
        values: &[(Guid, VarValue)],
    ) -> Result<Guid, AuthorError> {
        let canvas = parent_of(self.require_node(set)?).ok_or(AuthorError::UnknownNode(set))?;
        let position = self.next_position(canvas);
        let id = self.allocate();
        let entries = values.iter().map(|(mode, value)| mode_entry(*mode, var_type, value)).collect();
        self.append(object([
            (Field::Guid, id.to_value()),
            (Field::Phase, Value::from(NodePhase::Created)),
            (Field::ParentIndex, parent_index(canvas, &position)),
            (Field::Type, Value::from(schema::NodeType::Variable)),
            (Field::Name, Value::Str(name.to_string())),
            (Field::IsPublishable, Value::Bool(true)),
            (Field::Version, own_version(id)),
            (Field::SortPosition, Value::Str(position)),
            (Field::VariableSetId, object([(Field::Guid, set.to_value())])),
            (Field::VariableResolvedType, Value::from(var_type.resolved_type())),
            (Field::VariableDataValues,
                object([(Field::Entries, Value::Array(entries))]),
            ),
        ]))?;
        Ok(id)
    }

    /// Add a shape to `parent`: the twelve fields every local rectangle and
    /// ellipse carries, and `extra` (paints, an arc, a count). No geometry:
    /// Figma derives it for every shape kind (measured in Figma). A line is drawn
    /// by its stroke, centred, as lines are.
    pub fn add_shape_at(
        &mut self,
        parent: Guid,
        kind: ShapeKind,
        name: &str,
        size: (f32, f32),
        at: (f32, f32),
        extra: Vec<(Field, Value)>,
    ) -> Result<Guid, AuthorError> {
        self.require_node(parent)?;
        let position = self.next_position(parent);
        let id = self.allocate();
        let mut fields = common(id, parent, &position, kind.node_type(), name);
        fields.set(Field::Size, vector(size.0, size.1));
        fields.set(Field::StrokeWeight, Value::Float(1.0));
        fields.set(Field::StrokeAlign, Value::from(if kind == ShapeKind::Line { StrokeAlign::Center } else { StrokeAlign::Inside }));
        fields.set(Field::StrokeJoin, Value::from(StrokeJoin::Miter));
        for (key, value) in extra {
            fields.set(key, value);
        }
        self.append(translated(Value::Object(fields), at))?;
        Ok(id)
    }

    /// Add a style, in the shape a spike wrote, which Figma
    /// lists and applies: a hidden node under the Internal Only Canvas with
    /// its kind's values (`fillPaints`, `effects`, or a text's `fontName`,
    /// `fontSize`, `lineHeight`, `letterSpacing`). No `fillGeometry`, which
    /// would index a blob, and no publishing `key`.
    pub fn add_style(&mut self, kind: StyleKind, name: &str, values: Vec<(Field, Value)>) -> Result<Guid, AuthorError> {
        let (canvas, _) = self.internal_canvas()?;
        let position = self.next_position(canvas);
        let id = self.allocate();
        let node_type = if kind == StyleKind::Text { schema::NodeType::Text } else { schema::NodeType::RoundedRectangle };
        let mut fields = common(id, canvas, &position, node_type, name);
        fields.set(Field::IsPublishable, Value::Bool(true));
        fields.set(Field::StyleType, Value::from(kind.style_type()));
        fields.set(Field::Version, own_version(id));
        fields.set(Field::SortPosition, Value::Str(position));
        fields.set(Field::Visible, Value::Bool(false));
        fields.set(Field::StrokeWeight, Value::Float(if kind == StyleKind::Text { 0.0 } else { 1.0 }));
        fields.set(Field::StrokeAlign, Value::from(StrokeAlign::Inside));
        fields.set(Field::StrokeJoin, Value::from(StrokeJoin::Miter));
        match kind {
            StyleKind::Text => text_style_fields(&mut fields),
            StyleKind::Fill => {
                fields.set(Field::Size, vector(100.0, 100.0));
            }
            StyleKind::Effect => {
                // Beside its effects, the grey fill all 75 effect styles carry.
                fields.set(Field::Size, vector(100.0, 100.0));
                fields.set(Field::FillPaints, Value::Array(vec![solid_paint(0.769, 0.769, 0.769)]));
            }
        }
        for (key, value) in values {
            fields.set(key, value);
        }
        self.append(Value::Object(fields))?;
        Ok(id)
    }

    /// Add a text, boolean or instance-swap property to a component, with
    /// its default, in the seven fields all 42,173 measured carry.
    pub fn add_component_property(&mut self, component: Guid, name: &str, default: &PropValue) -> Result<Guid, AuthorError> {
        let taken = node_changes(self.fig)
            .iter()
            .flat_map(|nc| nc.field(Field::ComponentPropDefs).and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default())
            .filter_map(|d| d.field(Field::Id).and_then(Guid::read))
            .filter(|id| id.session_id == GRAFTA_SESSION && id.local_id >= PROPERTY_IDS_FROM)
            .map(|id| id.local_id + 1)
            .max()
            .unwrap_or(PROPERTY_IDS_FROM);
        let id = Guid::new(GRAFTA_SESSION, taken);
        let fields = self.fields_mut(component)?;
        let last = fields
            .field(Field::ComponentPropDefs)
            .and_then(Value::as_array)
            .and_then(|defs| defs.iter().filter_map(|d| d.field(Field::SortPosition).and_then(Value::as_str)).max())
            .map(str::to_string);
        let sort = position_between(last.as_deref(), None).unwrap_or_else(|| FIRST_POSITION.to_string());
        let kind = default.kind();
        let preferred = match default {
            PropValue::Component(_) => object([(Field::InstanceSwapValues, Value::Array(Vec::new()))]),
            _ => Value::Object(kiwi::Fields::new()),
        };
        let definition = object([
            (Field::Id, id.to_value()),
            (Field::Name, Value::Str(name.to_string())),
            (Field::InitialValue, default.value()),
            (Field::SortPosition, Value::Str(sort)),
            (Field::Type, Value::from(kind)),
            (Field::PreferredValues, preferred),
            (Field::VarValue, default.var_value()),
        ]);
        match fields.field_mut(Field::ComponentPropDefs) {
            Some(Value::Array(defs)) => defs.push(definition),
            _ => {
                fields.set(Field::ComponentPropDefs, Value::Array(vec![definition]));
            }
        }
        Ok(id)
    }

    /// Bind a layer's `field` (`TEXT_DATA`, `VISIBLE`, `OVERRIDDEN_SYMBOL_ID`)
    /// to a property, or unbind it with `None`. The reference is written in
    /// both forms every measured one has (203,522 of 203,522): the layer's
    /// `componentPropRefs`, and a `PROP_REF` in its `parameterConsumptionMap`,
    /// without which Figma calls the property unused (measured in Figma). A bound
    /// layer shows the property's default.
    pub fn set_property_reference(&mut self, layer: Guid, field: ComponentPropNodeField, property: Option<(Guid, &PropValue)>) -> Result<(), AuthorError> {
        let consumed_as = consumed_field(field).ok_or(AuthorError::NotBindable(field))?;
        let fields = self.fields_mut(layer)?;
        let mut refs: Vec<Value> = fields.field(Field::ComponentPropRefs).and_then(Value::as_array).cloned().unwrap_or_default();
        refs.retain(|r| !r.is(Field::ComponentPropNodeField, field));
        let mut consumed: Vec<Value> = fields
            .field(Field::ParameterConsumptionMap)
            .and_then(|m| m.field(Field::Entries))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        consumed.retain(|e| {
            !(e.is(Field::VariableField, consumed_as) && e.field(Field::VariableData).is_some_and(|d| d.is(Field::DataType, VariableDataType::PropRef)))
        });
        if let Some((def, default)) = property {
            refs.push(object([
                (Field::DefId, def.to_value()),
                (Field::ComponentPropNodeField, Value::from(field)),
            ]));
            let prop_ref = object([(Field::PropRefDefId, def.to_value())]);
            consumed.push(object([
                (Field::VariableData,
                    object([
                        (Field::Value, object([(Field::PropRefValue, prop_ref)])),
                        (Field::DataType, Value::from(VariableDataType::PropRef)),
                        (Field::ResolvedDataType, Value::from(default.resolved_type())),
                    ]),
                ),
                (Field::VariableField, Value::from(consumed_as)),
            ]));
            match default {
                PropValue::Text(t) => {
                    fields.set(Field::TextData, text_data_value(t));
                    fields.take(Field::DerivedTextData);
                }
                PropValue::Bool(b) => {
                    fields.set(Field::Visible, Value::Bool(*b));
                }
                PropValue::Component(c) => {
                    if let Some(Value::Object(symbol)) = fields.field_mut(Field::SymbolData) {
                        symbol.set(Field::SymbolId, c.to_value());
                    }
                    fields.take(Field::DerivedSymbolData);
                }
            }
        }
        if refs.is_empty() {
            fields.take(Field::ComponentPropRefs);
        } else {
            fields.set(Field::ComponentPropRefs, Value::Array(refs));
        }
        if consumed.is_empty() {
            fields.take(Field::ParameterConsumptionMap);
        } else {
            fields.set(Field::ParameterConsumptionMap, object([(Field::Entries, Value::Array(consumed))]));
        }
        Ok(())
    }

    /// Set an instance's value for a property, in the three fields all 49,349
    /// measured assignments carry, replacing an earlier one.
    pub fn assign_property(&mut self, instance: Guid, property: Guid, value: &PropValue) -> Result<(), AuthorError> {
        let fields = self.fields_mut(instance)?;
        let mut assignments: Vec<Value> = fields.field(Field::ComponentPropAssignments).and_then(Value::as_array).cloned().unwrap_or_default();
        assignments.retain(|a| a.field(Field::DefId).and_then(Guid::read) != Some(property));
        assignments.push(object([
            (Field::DefId, property.to_value()),
            (Field::Value, value.value()),
            (Field::VarValue, value.var_value()),
        ]));
        fields.set(Field::ComponentPropAssignments, Value::Array(assignments));
        // Figma derives an instance again when its cache is gone.
        fields.take(Field::DerivedSymbolData);
        Ok(())
    }

    /// Replace fields of a style grafta made.
    pub fn set_fields(&mut self, node: Guid, values: Vec<(Field, Value)>) -> Result<(), AuthorError> {
        if !node.is_graftas() {
            return Err(AuthorError::NotOurs(node));
        }
        let fields = self.fields_mut(node)?;
        for (key, value) in values {
            fields.set(key, value);
        }
        Ok(())
    }

    /// Add a mode to a collection grafta made. Every variable in it takes, in
    /// the new mode, its value in the first mode: every measured variable has
    /// a value in each mode of its collection (1,725 of 1,725).
    pub fn add_mode(&mut self, set: Guid, name: &str) -> Result<Guid, AuthorError> {
        if !set.is_graftas() {
            return Err(AuthorError::NotOurs(set));
        }
        let mode = self.allocate();
        let fields = self.fields_mut(set)?;
        let Some(Value::Array(modes)) = fields.field_mut(Field::VariableSetModes) else { return Err(AuthorError::UnknownNode(set)) };
        let first = modes.first().and_then(|m| m.field(Field::Id)).and_then(Guid::read);
        let last = modes.iter().filter_map(|m| m.field(Field::SortPosition).and_then(Value::as_str)).max().map(str::to_string);
        let key = position_between(last.as_deref(), None).unwrap_or_else(|| FIRST_POSITION.to_string());
        modes.push(mode_value(mode, name, &key));

        let changes = node_changes_mut(self.fig).ok_or(AuthorError::NoNodeChanges)?;
        for change in changes.iter_mut() {
            if !change.is(Field::Type, schema::NodeType::Variable)
                || change.field(Field::VariableSetId).and_then(|s| s.field(Field::Guid)).and_then(Guid::read) != Some(set)
            {
                continue;
            }
            if let Some(undo) = self.undo.as_deref_mut() {
                undo.changing(change);
            }
            let Some(Value::Array(entries)) = change_entries(change) else { continue };
            let copied = entries
                .iter()
                .find(|e| e.field(Field::ModeId).and_then(Guid::read) == first)
                .and_then(|e| e.field(Field::VariableData))
                .cloned();
            if let Some(data) = copied {
                entries.push(object([
                    (Field::ModeId, mode.to_value()),
                    (Field::VariableData, data),
                ]));
            }
        }
        Ok(mode)
    }

    /// Replace a variable's value in the modes named, leaving the others.
    pub fn set_variable_values(&mut self, variable: Guid, var_type: VarType, values: &[(Guid, VarValue)]) -> Result<(), AuthorError> {
        if !variable.is_graftas() {
            return Err(AuthorError::NotOurs(variable));
        }
        let fields = self.fields_mut(variable)?;
        let Some(Value::Object(data)) = fields.field_mut(Field::VariableDataValues) else { return Err(AuthorError::UnknownNode(variable)) };
        let Some(Value::Array(entries)) = data.field_mut(Field::Entries) else { return Err(AuthorError::UnknownNode(variable)) };
        for (mode, value) in values {
            let entry = mode_entry(*mode, var_type, value);
            match entries.iter_mut().find(|e| e.field(Field::ModeId).and_then(Guid::read) == Some(*mode)) {
                Some(existing) => *existing = entry,
                None => entries.push(entry),
            }
        }
        Ok(())
    }

    /// Rename a node grafta authored: a collection or a variable here.
    pub fn rename(&mut self, node: Guid, name: &str) -> Result<(), AuthorError> {
        if !node.is_graftas() {
            return Err(AuthorError::NotOurs(node));
        }
        self.fields_mut(node)?.set(Field::Name, Value::Str(name.to_string()));
        Ok(())
    }

    fn fields_mut(&mut self, id: Guid) -> Result<&mut kiwi::Fields, AuthorError> {
        match self.change_mut(id)? {
            Value::Object(fields) => Ok(fields),
            _ => Err(AuthorError::UnknownNode(id)),
        }
    }

    /// The node change of `id`, to change in place: found by the index, or
    /// by a scan, and recorded first.
    fn change_mut(&mut self, id: Guid) -> Result<&mut Value, AuthorError> {
        let at = match self.indexed(id) {
            Some(at) => at,
            None => node_changes(self.fig).iter().position(|change| id_of(change) == Some(id)).ok_or(AuthorError::UnknownNode(id))?,
        };
        let changes = node_changes_mut(self.fig).ok_or(AuthorError::NoNodeChanges)?;
        let change = changes.get_mut(at).ok_or(AuthorError::UnknownNode(id))?;
        if let Some(undo) = self.undo.as_deref_mut() {
            undo.changing(change);
        }
        Ok(change)
    }

    /// Store bytes as a new blob of the message, and return its index, which
    /// a node change refers to it by. Blobs live in the message itself;
    /// `FigFile::blobs` is a copy read from it, and both are kept in step.
    pub fn add_blob(&mut self, bytes: Vec<u8>) -> Result<u32, AuthorError> {
        let Value::Object(message) = &mut self.fig.node_changes else { return Err(AuthorError::NoNodeChanges) };
        let created = message.field(Field::Blobs).is_none();
        let blobs = message.field_or_insert_with(Field::Blobs, || Value::Array(Vec::new()));
        let Value::Array(blobs) = blobs else { return Err(AuthorError::NoNodeChanges) };
        let index = u32::try_from(blobs.len()).map_err(|_| AuthorError::NoNodeChanges)?;
        blobs.push(object([(Field::Bytes, Value::Bytes(bytes.clone()))]));
        self.fig.blobs.push(bytes);
        if let Some(undo) = self.undo.as_deref_mut() {
            undo.blob_added(created);
        }
        Ok(index)
    }

    /// Move a node grafta authored under a new parent.
    ///
    /// Refuses to move anything else. Re-parenting one of Figma's nodes is an
    /// edit to somebody's design, and this feature assembles documents rather
    /// than editing them.
    pub fn reparent(&mut self, node: Guid, parent: Guid) -> Result<(), AuthorError> {
        if !node.is_graftas() {
            return Err(AuthorError::NotOurs(node));
        }
        self.move_under(node, parent)
    }

    /// Move any node under a new parent, last among its siblings. Unlike
    /// [`Graft::reparent`], it does not require the node to be grafta's: the
    /// toolkit decides what may be moved.
    pub fn move_under(&mut self, node: Guid, parent: Guid) -> Result<(), AuthorError> {
        self.require_node(node)?;
        self.require_node(parent)?;
        let position = self.next_position(parent);

        if let Value::Object(fields) = self.change_mut(node)? {
            fields.set(Field::ParentIndex, parent_index(parent, &position));
        }
        Ok(())
    }

    /// Put any node under `parent` at `position`, a key the caller chose
    /// (from [`position_between`]) to place it among its siblings.
    pub fn place(&mut self, node: Guid, parent: Guid, position: &str) -> Result<(), AuthorError> {
        self.require_node(parent)?;
        self.forget_position(parent);
        let Value::Object(fields) = self.change_mut(node)? else { return Err(AuthorError::UnknownNode(node)) };
        fields.set(Field::ParentIndex, parent_index(parent, position));
        Ok(())
    }

    /// Put `node` under `parent` at `position`, translated to `at` in the
    /// parent's space.
    pub fn place_at(&mut self, node: Guid, parent: Guid, position: &str, at: (f32, f32)) -> Result<(), AuthorError> {
        self.place(node, parent, position)?;
        let fields = self.fields_mut(node)?;
        if let Some(Value::Object(transform)) = fields.field_mut(Field::Transform) {
            transform.set(Field::M02, Value::Float(at.0));
            transform.set(Field::M12, Value::Float(at.1));
        }
        Ok(())
    }

    /// A group: a `FRAME` with `resizeToFit` and no auto-layout, in the
    /// fourteen fields all 699 local groups carry, `frameMaskDisabled` false and
    /// no fill as on 637 of them. Placed at `position` among the
    /// parent's children; its children are placed by the caller.
    pub fn add_group_at(&mut self, parent: Guid, position: &str, name: &str, size: (f32, f32), at: (f32, f32)) -> Result<Guid, AuthorError> {
        self.require_node(parent)?;
        let id = self.allocate();
        let mut fields = common(id, parent, position, schema::NodeType::Frame, name);
        fields.set(Field::Size, vector(size.0, size.1));
        with_stroke(&mut fields);
        fields.set(Field::FrameMaskDisabled, Value::Bool(false));
        fields.set(Field::ResizeToFit, Value::Bool(true));
        self.append(translated(Value::Object(fields), at))?;
        Ok(id)
    }

    /// A section: a `SECTION` in a frame's fields with a white fill, without
    /// `editInfo` (an author and dates grafta has not got) or `fillGeometry`,
    /// as a spike wrote it and Figma opened it.
    pub fn add_section_at(&mut self, parent: Guid, name: &str, size: (f32, f32), at: (f32, f32)) -> Result<Guid, AuthorError> {
        self.require_node(parent)?;
        let position = self.next_position(parent);
        let id = self.allocate();
        let mut fields = common(id, parent, &position, schema::NodeType::Section, name);
        fields.set(Field::Size, vector(size.0, size.1));
        with_stroke(&mut fields);
        fields.set(Field::FrameMaskDisabled, Value::Bool(false));
        fields.set(Field::FillPaints, Value::Array(vec![solid_paint(1.0, 1.0, 1.0)]));
        self.append(translated(Value::Object(fields), at))?;
        Ok(id)
    }

    /// Copy `root` and everything under it, each node with a new id and every
    /// other field as it is, the copy of `root` placed under `parent` at
    /// `position` and translated to `at`. Returns (original, copy) pairs,
    /// parents before children.
    pub fn duplicate(&mut self, root: Guid, parent: Guid, position: &str, at: (f32, f32)) -> Result<Vec<(Guid, Guid)>, AuthorError> {
        self.require_node(root)?;
        self.require_node(parent)?;
        self.forget_position(parent);
        // The subtree, breadth first, from one pass over who is whose child.
        let mut children: std::collections::HashMap<Guid, Vec<Guid>> = std::collections::HashMap::new();
        for nc in node_changes(self.fig) {
            if let (Some(id), Some(p)) = (id_of(nc), parent_of(nc)) {
                children.entry(p).or_default().push(id);
            }
        }
        let mut order = vec![root];
        let mut i = 0;
        while i < order.len() {
            order.extend(children.get(&order[i]).cloned().unwrap_or_default());
            i += 1;
        }
        let pairs: Vec<(Guid, Guid)> = order.iter().map(|old| (*old, self.allocate())).collect();
        let copy_of = |g: Guid| pairs.iter().find(|(o, _)| *o == g).map(|(_, n)| *n);
        let mut copies = Vec::new();
        for (old, new) in &pairs {
            let Value::Object(mut fields) = self.require_node(*old)?.clone() else { return Err(AuthorError::UnknownNode(*old)) };
            fields.set(Field::Guid, new.to_value());
            if *old == root {
                fields.set(Field::ParentIndex, parent_index(parent, position));
                if let Some(Value::Object(transform)) = fields.field_mut(Field::Transform) {
                    transform.set(Field::M02, Value::Float(at.0));
                    transform.set(Field::M12, Value::Float(at.1));
                }
            } else if let Some(Value::Object(index)) = fields.field_mut(Field::ParentIndex) {
                let p = index.field(Field::Guid).and_then(Guid::read).and_then(copy_of).ok_or(AuthorError::UnknownNode(*old))?;
                index.set(Field::Guid, p.to_value());
            }
            // A copy's interactions are its own: each under a new id, and a
            // link to a layer inside the copy points at that layer's copy;
            // a link out keeps its destination. A copy starts no flow.
            if let Some(Value::Array(interactions)) = fields.field_mut(Field::PrototypeInteractions) {
                for interaction in interactions.iter_mut() {
                    interaction.set(Field::Id, self.allocate().to_value());
                    if let Some(Value::Array(actions)) = interaction.field_mut(Field::Actions) {
                        for action in actions.iter_mut() {
                            if let Some(to) = action.field(Field::TransitionNodeId).and_then(Guid::read).and_then(copy_of) {
                                action.set(Field::TransitionNodeId, to.to_value());
                            }
                        }
                    }
                }
            }
            fields.take(Field::PrototypeStartingPoint);
            copies.push(Value::Object(fields));
        }
        for copy in copies {
            self.append(copy)?;
        }
        Ok(pairs)
    }

    /// Remove everything except the root, the given nodes, their ancestors and
    /// their descendants.
    ///
    /// This authors nothing: every surviving node is exactly the node change
    /// that was read. It is also the one operation here that can break a
    /// reference — a survivor pointing at a style, a variable or a symbol that
    /// pruning took away — which is why it is the last rung of the ladder and
    /// why it is allowed to fail.
    pub fn prune(&mut self, keep: &[Guid]) -> Result<PruneReport, AuthorError> {
        let root = self.root()?;
        let changes = node_changes(self.fig);

        // The chain from each kept node up to the root, so that what survives is
        // still reachable. Ancestors are kept for the path, not for their other
        // children — expanding them would keep the whole document, since every
        // node descends from the root.
        let mut wanted: Vec<Guid> = vec![root];
        for id in keep {
            let mut ancestor = Some(*id);
            while let Some(current) = ancestor {
                if !wanted.contains(&current) {
                    wanted.push(current);
                }
                ancestor = changes
                    .iter()
                    .find(|nc| id_of(nc) == Some(current))
                    .and_then(parent_of);
            }
        }

        // Then everything below the kept nodes themselves, breadth-first.
        let mut subtree: Vec<Guid> = keep.to_vec();
        let mut index = 0;
        while index < subtree.len() {
            let parent = subtree[index];
            for child in changes.iter().filter(|nc| parent_of(nc) == Some(parent)).filter_map(id_of)
            {
                if !subtree.contains(&child) {
                    subtree.push(child);
                }
            }
            index += 1;
        }
        for id in subtree {
            if !wanted.contains(&id) {
                wanted.push(id);
            }
        }

        let before = changes.len();
        let changes = node_changes_mut(self.fig).ok_or(AuthorError::NoNodeChanges)?;
        let unwanted = |nc: &Value| !id_of(nc).is_some_and(|id| wanted.contains(&id));
        match self.undo.as_deref_mut() {
            Some(undo) => undo.remove_where(changes, unwanted),
            None => changes.retain(|nc| !unwanted(nc)),
        }
        let kept = changes.len();

        Ok(PruneReport { kept, removed: before - kept })
    }

    fn require_node(&self, id: Guid) -> Result<&Value, AuthorError> {
        if let Some(at) = self.indexed(id) {
            return Ok(&node_changes(self.fig)[at]);
        }
        node_changes(self.fig)
            .iter()
            .find(|nc| id_of(nc) == Some(id))
            .ok_or(AuthorError::UnknownNode(id))
    }

    /// Where `id` is in `nodeChanges`, by the caller's index, checked against
    /// the node change actually there. `None` sends the caller to a scan.
    fn indexed(&self, id: Guid) -> Option<usize> {
        let at = *self.index?.get(&id)?;
        (node_changes(self.fig).get(at).and_then(id_of) == Some(id)).then_some(at)
    }

    fn append(&mut self, node_change: Value) -> Result<(), AuthorError> {
        node_changes_mut(self.fig).ok_or(AuthorError::NoNodeChanges)?.push(node_change);
        if let Some(undo) = self.undo.as_deref_mut() {
            undo.appended();
        }
        Ok(())
    }
}

/// The document's node changes, or nothing when the message has none.
#[must_use]
pub fn node_changes(fig: &FigFile) -> &[Value] {
    fig.node_changes
        .field(Field::NodeChanges)
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
}

fn node_changes_mut(fig: &mut FigFile) -> Option<&mut Vec<Value>> {
    let Value::Object(message) = &mut fig.node_changes else { return None };
    match message.field_mut(Field::NodeChanges) {
        Some(Value::Array(changes)) => Some(changes),
        _ => None,
    }
}

#[cfg(test)]
mod tests;

