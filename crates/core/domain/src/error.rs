//! What went wrong, named, and with what.
//!
//! A tool fails with a [`DomainError`]: a variant per failure, carrying what
//! the failure was about. The words a model reads are written here, once, by
//! the `From` that makes a [`ToolError`] of it. A failure only one tool has is
//! a variant of that tool's own enum, in a module below, which a variant here
//! holds.

use std::fmt;
use std::path::PathBuf;

use schema::Guid;

use crate::ToolError;

mod binding;
mod catalog;
mod component;
mod figjam;
mod images;
mod import;
mod new_file;
mod overrides;
mod properties;
mod rich_text;
mod shapes;
mod sizing;
mod slides;
mod shared_style;
mod structure;
mod style;
mod template;
mod text;
mod variable;
mod vector;
mod write;

pub use binding::BindingError;
pub use catalog::CatalogError;
pub use component::ComponentError;
pub use figjam::FigJamError;
pub use images::ImageError;
pub use import::ImportError;
pub use new_file::NewFileError;
pub use overrides::OverrideError;
pub use properties::PropertyError;
pub use rich_text::RichTextError;
pub use shapes::ShapeError;
pub use sizing::SizingError;
pub use slides::SlideError;
pub use shared_style::SharedStyleError;
pub use structure::StructureError;
pub use style::StyleError;
pub use template::TemplateError;
pub use text::TextError;
pub use variable::VariableError;
pub use vector::VectorError;
pub use write::WriteError;

/// A failure, by name.
#[derive(Debug)]
pub enum DomainError {
    // ── Arguments ────────────────────────────────────────────────────────────
    /// No tool is called `name`.
    NoSuchTool { name: String, tools: Vec<&'static str> },
    /// The arguments were not a JSON object.
    ArgumentsNotAnObject { got: String },
    /// An argument the tool does not take: without this it would do nothing,
    /// and the call would still report success.
    UnknownArgument { tool: &'static str, argument: String, takes: Vec<String> },
    /// A parameter a call must carry is missing.
    Required { parameter: String },
    /// A parameter of the wrong kind or out of its range.
    Expected { parameter: String, expected: String, got: String },
    /// A cursor the toolkit did not issue.
    NotACursor { text: String },
    /// A cursor issued before the file was edited.
    StaleCursor,
    /// A node type search_nodes does not know.
    NotANodeType { given: String },
    /// A value, at `at` in the arguments, of the wrong kind or out of range.
    Invalid { at: String, expected: String, got: String },
    /// A value at `at` that is not what is expected, with nothing to quote.
    Unexpected { at: String, expected: String },
    /// A width and height that are not both above 0.
    NotPositiveSize { width: f32, height: f32 },
    /// A size or weight of 0 or less, quoted when the call gave it.
    NotPositive { at: String, got: Option<String> },
    /// A key an object at `at` does not take; `takes` says what it does.
    Unaccepted { at: String, key: String, takes: &'static str },
    /// A value at `at` that is none of those `allowed`.
    NotOneOf { at: String, got: String, allowed: String },
    /// A name of blanks.
    Empty { at: String },
    /// A call that changes nothing; `pass` says what would.
    NothingToChange { pass: &'static str },
    /// A name more than one variable has.
    AmbiguousVariable { at: String, wanted: String, ids: Vec<String> },
    /// A variable imported from a library, bound.
    LibraryVariable { at: String, name: String },

    // ── Files ────────────────────────────────────────────────────────────────
    /// A path that does not resolve.
    CannotOpen { path: PathBuf, error: String },
    /// A file that cannot be read.
    CannotRead { path: PathBuf, error: String },
    /// A file that is neither a ZIP archive nor a fig-kiwi container.
    NotAFig { path: PathBuf },
    /// A `.fig` whose message does not decode.
    Undecodable { path: PathBuf, error: String },
    /// A decoded file with no document to build a graph from.
    NoDocument { path: PathBuf, error: String },
    /// A file handed in already decoded whose graph could not be built.
    NoGraph { path: PathBuf, error: String },
    /// A path a file is already at; `own` when it is the session's own file.
    WouldOverwrite { path: PathBuf, own: bool },
    /// An edit made in memory that could not be journaled.
    NotJournaled { path: PathBuf, error: String },
    /// A journal that could not be rewritten after a replay.
    CannotRewriteJournal { path: PathBuf, error: String },
    /// A journal that could not be set aside.
    CannotSetAside { path: PathBuf, error: String },
    /// A new file that could not be created.
    CannotCreate { path: PathBuf, error: String },
    /// A file whose bytes could not be written.
    CannotWrite { path: PathBuf, error: String },

    // ── Sessions and nodes ───────────────────────────────────────────────────
    /// No open file has this handle.
    UnknownHandle { handle: String },
    /// No node has this id, in a file named `file`.
    UnknownNode { id: String, file: String },
    /// No node has this id, read raw.
    UnknownRawNode { id: String, file: String },
    /// An edit to the document node.
    TheDocument,
    /// A change inside a wrapping auto-layout frame, which grafta cannot lay out.
    WrappingLayout { frame: Guid },
    /// A node that should be an instance and is a node of type `is`.
    NotAnInstance { id: Guid, is: &'static str },
    /// An instance whose component is not in the file.
    NoComponent { id: Guid },
    /// A node with no parent, named as the call named it.
    NoParent { id: String },

    // ── The file's message ───────────────────────────────────────────────────
    /// A node the graph has and the message does not.
    NotInFile { id: Guid },
    /// A node the graph has but whose change is not indexed.
    InGraphNotInFile { id: Guid },
    /// A node with no change in the message.
    NoNodeChange { id: Guid },
    /// A node whose change is not an object.
    NodeChangeNotAnObject { id: Guid },
    /// A node with no guid to key an override by.
    NoGuid { id: Guid },
    /// A message that is not an object.
    MessageNotAnObject,
    /// A message with no node changes.
    NoNodeChanges,
    /// The Internal Only Canvas, written and not read back.
    CanvasNotReadBack { error: String },
    /// A node created and not read back.
    CreatedNotReadBack { id: Guid, error: String },
    /// A node moved and not read back.
    MovedNotReadBack { id: Guid, error: String },
    /// A change the model's author refused.
    Authoring(model::author::AuthorError),
    /// A drawing, contrast check or PDF the renderer could not make.
    Render { error: String },

    // ── A tool's own ─────────────────────────────────────────────────────────
    Binding(BindingError),
    Catalog(CatalogError),
    Component(ComponentError),
    FigJam(FigJamError),
    Image(ImageError),
    Import(ImportError),
    NewFile(NewFileError),
    Override(OverrideError),
    Property(PropertyError),
    RichText(RichTextError),
    Shape(ShapeError),
    Sizing(SizingError),
    Slide(SlideError),
    SharedStyle(SharedStyleError),
    Structure(StructureError),
    Style(StyleError),
    Template(TemplateError),
    Text(TextError),
    Variable(VariableError),
    Vector(VectorError),
    Write(WriteError),
}

/// Implements `From<E>` for [`DomainError`] and [`ToolError`] for a tool's own
/// failures, so that `?` carries one out of any tool.
macro_rules! area {
    ($error:ident => $variant:ident) => {
        impl From<$error> for DomainError {
            fn from(error: $error) -> Self {
                Self::$variant(error)
            }
        }

        impl From<$error> for ToolError {
            fn from(error: $error) -> Self {
                DomainError::$variant(error).into()
            }
        }
    };
}

area!(BindingError => Binding);
area!(CatalogError => Catalog);
area!(ComponentError => Component);
area!(FigJamError => FigJam);
area!(ImageError => Image);
area!(ImportError => Import);
area!(NewFileError => NewFile);
area!(OverrideError => Override);
area!(PropertyError => Property);
area!(RichTextError => RichText);
area!(ShapeError => Shape);
area!(SizingError => Sizing);
area!(SlideError => Slide);
area!(SharedStyleError => SharedStyle);
area!(StructureError => Structure);
area!(StyleError => Style);
area!(TemplateError => Template);
area!(TextError => Text);
area!(VariableError => Variable);
area!(VectorError => Vector);
area!(WriteError => Write);

/// Which of [`ToolError`]'s four kinds a failure is.
pub(crate) enum Kind {
    BadArgument,
    NotFound,
    Refused,
    Failed,
}

impl DomainError {
    fn kind(&self) -> Kind {
        match self {
            Self::ArgumentsNotAnObject { .. }
            | Self::UnknownArgument { .. }
            | Self::Required { .. }
            | Self::Expected { .. }
            | Self::NotACursor { .. }
            | Self::NotANodeType { .. }
            | Self::Invalid { .. }
            | Self::Unexpected { .. }
            | Self::NotPositive { .. }
            | Self::NotPositiveSize { .. }
            | Self::Unaccepted { .. }
            | Self::NotOneOf { .. }
            | Self::Empty { .. }
            | Self::NothingToChange { .. }
            | Self::AmbiguousVariable { .. }
            | Self::NotAFig { .. } => Kind::BadArgument,
            Self::NoSuchTool { .. }
            | Self::CannotOpen { .. }
            | Self::CannotRead { .. }
            | Self::UnknownHandle { .. }
            | Self::UnknownNode { .. }
            | Self::UnknownRawNode { .. } => Kind::NotFound,
            Self::StaleCursor
            | Self::WouldOverwrite { .. }
            | Self::TheDocument
            | Self::WrappingLayout { .. }
            | Self::NotAnInstance { .. }
            | Self::NoComponent { .. }
            | Self::LibraryVariable { .. } => Kind::Refused,
            Self::Undecodable { .. }
            | Self::NoDocument { .. }
            | Self::NoGraph { .. }
            | Self::NotJournaled { .. }
            | Self::CannotRewriteJournal { .. }
            | Self::CannotSetAside { .. }
            | Self::NotInFile { .. }
            | Self::InGraphNotInFile { .. }
            | Self::NoNodeChange { .. }
            | Self::NodeChangeNotAnObject { .. }
            | Self::NoGuid { .. }
            | Self::MessageNotAnObject
            | Self::NoNodeChanges
            | Self::CanvasNotReadBack { .. }
            | Self::CreatedNotReadBack { .. }
            | Self::MovedNotReadBack { .. }
            | Self::NoParent { .. }
            | Self::CannotCreate { .. }
            | Self::CannotWrite { .. }
            | Self::Authoring(_)
            | Self::Render { .. } => Kind::Failed,
            Self::Binding(error) => error.kind(),
            Self::Catalog(error) => error.kind(),
            Self::Component(error) => error.kind(),
            Self::FigJam(error) => error.kind(),
            Self::Image(error) => error.kind(),
            Self::Import(error) => error.kind(),
            Self::NewFile(error) => error.kind(),
            Self::Override(error) => error.kind(),
            Self::Property(error) => error.kind(),
            Self::RichText(error) => error.kind(),
            Self::Shape(error) => error.kind(),
            Self::Sizing(error) => error.kind(),
            Self::Slide(error) => error.kind(),
            Self::SharedStyle(error) => error.kind(),
            Self::Structure(error) => error.kind(),
            Self::Style(error) => error.kind(),
            Self::Template(error) => error.kind(),
            Self::Text(error) => error.kind(),
            Self::Variable(error) => error.kind(),
            Self::Vector(error) => error.kind(),
            Self::Write(error) => error.kind(),
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuchTool { name, tools } => write!(f, "no tool named `{name}`. The tools are: {}", tools.join(", ")),
            Self::ArgumentsNotAnObject { got } => write!(f, "arguments must be a JSON object, got {got}"),
            Self::UnknownArgument { tool, argument, takes } => {
                write!(f, "{tool} has no argument \"{argument}\", and nothing was done. It takes: {}", takes.join(", "))
            }
            Self::Required { parameter } => write!(f, "parameter \"{parameter}\" is required"),
            Self::Expected { parameter, expected, got } => {
                write!(f, "parameter \"{parameter}\": expected {expected}, got {got}")
            }
            Self::NotACursor { text } => {
                write!(f, "parameter \"cursor\": `{text}` is not a cursor this toolkit issued")
            }
            Self::StaleCursor => f.write_str(
                "cursor is stale: the file was edited after it was issued, so the list may have \
                 shifted. Ask again without a cursor",
            ),
            Self::NotANodeType { given } => write!(
                f,
                "parameter \"type\": `{given}` is not a node type. Use FRAME, TEXT, INSTANCE, \
                 COMPONENT, COMPONENT_SET, PAGE, SECTION, GROUP, RECTANGLE, ELLIPSE, VECTOR, …"
            ),
            Self::Invalid { at, expected, got } => write!(f, "{at}: expected {expected}, got {got}"),
            Self::Unexpected { at, expected } => write!(f, "{at}: expected {expected}"),
            Self::NotPositiveSize { width, height } => {
                write!(f, "width and height must be positive, got {width}×{height}")
            }
            Self::NotPositive { at, got: None } => write!(f, "{at} must be greater than 0"),
            Self::NotPositive { at, got: Some(got) } => write!(f, "{at} must be greater than 0, got {got}"),
            Self::Unaccepted { at, key, takes } => write!(f, "{at}: `{key}` is not accepted; {takes}"),
            Self::NotOneOf { at, got, allowed } => write!(f, "{at}: `{got}` is not one of {allowed}"),
            Self::Empty { at } => write!(f, "{at}: must not be empty"),
            Self::NothingToChange { pass } => write!(f, "nothing to change: pass {pass}"),
            Self::AmbiguousVariable { at, wanted, ids } => {
                write!(f, "{at}: {} variables are named `{wanted}`; pass one id: {}", ids.len(), ids.join(", "))
            }
            Self::LibraryVariable { at, name } => write!(
                f,
                "{at}: `{name}` is imported from a library. Binding library variables is not measured \
                 yet; only the file's own variables can be bound"
            ),
            Self::CannotOpen { path, error } => write!(f, "cannot open {}: {error}", path.display()),
            Self::CannotRead { path, error } => write!(f, "cannot read {}: {error}", path.display()),
            Self::NotAFig { path } => write!(
                f,
                "{} is not a .fig file: it is neither a ZIP archive nor a fig-kiwi container",
                path.display()
            ),
            Self::Undecodable { path, error } => {
                write!(f, "{} is a .fig that failed to decode: {error}", path.display())
            }
            Self::NoDocument { path, error } => write!(f, "{} decoded but has no document: {error}", path.display()),
            Self::NoGraph { path, error } => write!(f, "{}: {error}", path.display()),
            Self::WouldOverwrite { path, own } => {
                let what = if *own { "it is the file this session was opened from" } else { "a file is already there" };
                write!(f, "will not write {}: {what}. The toolkit never overwrites; choose a new path", path.display())
            }
            Self::NotJournaled { path, error } => write!(
                f,
                "the edit was made, but only in memory: it could not be journaled to {} ({error}), \
                 so it will not survive a restart. save_as keeps it",
                path.display()
            ),
            Self::CannotRewriteJournal { path, error } => {
                write!(f, "cannot rewrite the journal {}: {error}", path.display())
            }
            Self::CannotSetAside { path, error } => {
                write!(f, "cannot set a journal aside at {}: {error}", path.display())
            }
            Self::CannotCreate { path, error } => write!(f, "cannot create {}: {error}", path.display()),
            Self::CannotWrite { path, error } => write!(f, "cannot write {}: {error}", path.display()),
            Self::UnknownHandle { handle } => write!(
                f,
                "no open file has handle `{handle}`: it was closed, or never opened. Call open_file"
            ),
            Self::UnknownNode { id, file } => {
                write!(f, "unknown node {id} in {file}. Node ids look like 12:345; search_nodes finds them")
            }
            Self::UnknownRawNode { id, file } => write!(f, "unknown node {id} in {file}"),
            Self::TheDocument => f.write_str("the document itself cannot be changed"),
            Self::WrappingLayout { frame } => write!(
                f,
                "{frame} is a wrapping auto-layout frame. Figma does not lay auto-layout out on open, and \
                 grafta cannot lay out wrapping yet, so changing its children would leave them misplaced"
            ),
            Self::NotAnInstance { id, is } => write!(f, "{id} is a {is}, not an instance"),
            Self::NoComponent { id } => write!(f, "{id}'s component is not in this file"),
            Self::NoParent { id } => write!(f, "{id} has no parent"),
            Self::NotInFile { id } => write!(f, "{id} is not in the file"),
            Self::InGraphNotInFile { id } => write!(f, "{id} is in the graph but not in the file"),
            Self::NoNodeChange { id } => write!(f, "{id} has no node change"),
            Self::NodeChangeNotAnObject { id } => write!(f, "{id}'s node change is not an object"),
            Self::NoGuid { id } => write!(f, "{id} has no guid"),
            Self::MessageNotAnObject => f.write_str("the file's message is not an object"),
            Self::NoNodeChanges => f.write_str("the file has no nodeChanges"),
            Self::CanvasNotReadBack { error } => {
                write!(f, "wrote an Internal Only Canvas but could not read it back: {error}")
            }
            Self::CreatedNotReadBack { id, error } => write!(f, "created {id} but could not read it back: {error}"),
            Self::MovedNotReadBack { id, error } => write!(f, "moved {id} but could not read it back: {error}"),
            Self::Authoring(error) => error.fmt(f),
            Self::Render { error } => f.write_str(error),
            Self::Binding(error) => error.fmt(f),
            Self::Catalog(error) => error.fmt(f),
            Self::Component(error) => error.fmt(f),
            Self::FigJam(error) => error.fmt(f),
            Self::Image(error) => error.fmt(f),
            Self::Import(error) => error.fmt(f),
            Self::NewFile(error) => error.fmt(f),
            Self::Override(error) => error.fmt(f),
            Self::Property(error) => error.fmt(f),
            Self::RichText(error) => error.fmt(f),
            Self::Shape(error) => error.fmt(f),
            Self::Sizing(error) => error.fmt(f),
            Self::Slide(error) => error.fmt(f),
            Self::SharedStyle(error) => error.fmt(f),
            Self::Structure(error) => error.fmt(f),
            Self::Style(error) => error.fmt(f),
            Self::Template(error) => error.fmt(f),
            Self::Text(error) => error.fmt(f),
            Self::Variable(error) => error.fmt(f),
            Self::Vector(error) => error.fmt(f),
            Self::Write(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for DomainError {}

impl From<DomainError> for ToolError {
    fn from(error: DomainError) -> Self {
        let text = error.to_string();
        match error.kind() {
            Kind::BadArgument => Self::BadArgument(text),
            Kind::NotFound => Self::NotFound(text),
            Kind::Refused => Self::Refused(text),
            Kind::Failed => Self::Failed(text),
        }
    }
}
