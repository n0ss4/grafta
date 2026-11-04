//! The writing tools' failures: placing, instantiating, saving, deleting and
//! moving nodes.

use std::fmt;
use std::path::PathBuf;

use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum WriteError {
    /// Something other than a slide, put in a row of slides.
    NotOnASlide { what: &'static str, parent: Guid },
    /// A parent nothing has been measured going in.
    NotAParent { what: &'static str, parent: Guid, is: &'static str },
    /// A component set, instantiated.
    InstanceOfSet { id: String },
    /// A node other than a component, instantiated.
    NotADefinedComponent { id: String, is: &'static str },
    /// A path whose extension is not its product's.
    WrongExtension { product: &'static str, extension: &'static str, path: PathBuf },
    /// A file without the entries Figma needs.
    MissingEntries { file: String, missing: Vec<&'static str> },
    /// A file that does not encode.
    CannotEncode { error: String },
    /// An encoded file that does not decode.
    NotReadBack { error: String },
    /// An encoded file that decodes to something else.
    ChangedInTransit { at: String },
    /// The file's only page, deleted.
    OnlyPage,
    /// A node other nodes point at, deleted.
    Referenced { id: Guid, referrers: Vec<String> },
    /// A node of a type move_node does not move.
    NotMovable { id: Guid, is: &'static str },
    /// None, or more than one, of where to move to.
    OnePlace,
    /// A node put behind or in front of itself.
    BesideItself { id: Guid },
    /// A sibling with no parent to move beside.
    SiblingWithoutParent { sibling: String, id: Guid },
    /// A component moved into another.
    ComponentInComponent { id: Guid, parent: String, holder: Guid },
    /// A slide moved out of the rows of slides.
    SlideOutsideRow { id: Guid, parent: String },
    /// A node moved into itself.
    IntoItself { id: Guid, parent: String },
    /// A sibling not among the parent's children.
    NotAmongChildren { sibling: Guid, parent: String },
    /// Two order keys with no key between them.
    NoPositionKey { lower: Option<String>, upper: Option<String>, id: Guid },
}

impl WriteError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::WrongExtension { .. } | Self::OnePlace | Self::BesideItself { .. } => Kind::BadArgument,
            Self::NotOnASlide { .. }
            | Self::NotAParent { .. }
            | Self::InstanceOfSet { .. }
            | Self::NotADefinedComponent { .. }
            | Self::MissingEntries { .. }
            | Self::OnlyPage
            | Self::Referenced { .. }
            | Self::NotMovable { .. }
            | Self::SiblingWithoutParent { .. }
            | Self::ComponentInComponent { .. }
            | Self::SlideOutsideRow { .. }
            | Self::IntoItself { .. }
            | Self::NoPositionKey { .. } => Kind::Refused,
            Self::CannotEncode { .. }
            | Self::NotReadBack { .. }
            | Self::ChangedInTransit { .. }
            | Self::NotAmongChildren { .. } => Kind::Failed,
        }
    }
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotOnASlide { what, parent } => {
                write!(f, "{what} cannot go in {parent}, a row of slides, which holds slides alone; put it on a slide")
            }
            Self::NotAParent { what, parent, is } => write!(
                f,
                "{what} can only go on a page or in a frame; {parent} is a {is}. Other parents have not \
                 been measured in Figma yet"
            ),
            Self::InstanceOfSet { id } => write!(
                f,
                "{id} is a component set; instantiate one of its variants. \
                 read_node on the set lists them as children"
            ),
            Self::NotADefinedComponent { id, is } => write!(f, "{id} is a {is}, not a component this file defines"),
            Self::WrongExtension { product, extension, path } => write!(
                f,
                "parameter \"path\": this is a {product} file, so it is saved as .{extension}, got {}",
                path.display()
            ),
            Self::MissingEntries { file, missing } => write!(
                f,
                "{file} has no {}, and Figma refuses a file without it (\"Internal error during \
                 import\", measured). grafta does not invent one",
                missing.join(" or ")
            ),
            Self::CannotEncode { error } => write!(f, "could not encode the file: {error}"),
            Self::NotReadBack { error } => write!(f, "wrote a file grafta cannot read back: {error}"),
            Self::ChangedInTransit { at } => write!(f, "the file changed in transit at {at}: nothing was written"),
            Self::OnlyPage => f.write_str("that is the file's only page; a file needs one"),
            Self::Referenced { id, referrers } => write!(
                f,
                "{id} cannot be deleted: other nodes point at it or at something inside it, and would \
                 break. For example: {}. Delete or change those first",
                referrers.join(", ")
            ),
            Self::NotMovable { id, is } => write!(
                f,
                "{id} is a {is}; only frames, components, component sets, instances, text layers, shapes, vectors and slides can be moved"
            ),
            Self::OnePlace => f.write_str("give exactly one of parent, behind or in_front_of"),
            Self::BesideItself { id } => write!(f, "{id} cannot go behind or in front of itself"),
            Self::SiblingWithoutParent { sibling, id } => write!(f, "{sibling} has no parent to put {id} in"),
            Self::ComponentInComponent { id, parent, holder } => write!(
                f,
                "{id} is a component, and {parent} is in component {holder}: Figma does not put a component \
                 inside another. Place an instance there instead"
            ),
            Self::SlideOutsideRow { id, parent } => {
                write!(f, "{id} is a slide, and a slide goes in a row of slides; {parent} is not one")
            }
            Self::IntoItself { id, parent } => write!(f, "cannot move {id} into {parent}: {parent} is inside {id}"),
            Self::NotAmongChildren { sibling, parent } => write!(f, "{sibling} is not among {parent}'s children"),
            Self::NoPositionKey { lower, upper, id } => write!(
                f,
                "no position key fits between {lower:?} and {upper:?}: one is the other plus spaces. \
                 Put {id} next to a different sibling"
            ),
        }
    }
}
