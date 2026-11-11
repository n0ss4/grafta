//! The Figma products a file can be from.
//!
//! Design, Slides and FigJam share one container and one schema; what differs
//! is said once, by each product's [`ProductProfile`], and found here. Shared
//! code asks the registry, never which product a file is from.
//!
//! What a node type that belongs to one product admits, shows and accepts is
//! its [`NodeBehavior`], listed by that product. Shared code asks
//! [`Products::behaviours_of`] a node's type, never whether it is a slide or a
//! sticky. What a product does when a node moves, goes or is copied, it does
//! by hearing the [`NodeEvent`] the tool emits through [`Products::emit`].

use kiwi::Value as Raw;
use model::{NodeType, SceneGraph, SceneNode};
use schema::{Field, Guid};

use crate::data::{Data, Map};
use crate::event::NodeEvent;
use crate::{Session, ToolError};

/// What is being put in a parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Frame,
    Instance,
    Text,
    Vector,
    Shape,
    Group,
    Duplicate,
    MovedNode,
    MovedSlide,
}

impl Placement {
    #[must_use]
    pub fn noun(self) -> &'static str {
        match self {
            Self::Frame => "a frame",
            Self::Instance => "an instance",
            Self::Text => "a text",
            Self::Vector => "a vector",
            Self::Shape => "a shape",
            Self::Group => "a group",
            Self::Duplicate => "a duplicate",
            Self::MovedNode => "a moved node",
            Self::MovedSlide => "a moved slide",
        }
    }
}


/// What a product is, as the toolkit tells it apart.
pub trait ProductProfile: Sync {
    /// The container it answers for.
    fn product(&self) -> figfile::Product;
    /// Its name as a person reads it.
    fn name(&self) -> &'static str;
    /// Its name in `summarize`'s `product` key.
    fn key(&self) -> &'static str;
    /// The key and value `summarize` adds for a file of it, after the shared
    /// ones.
    fn summary(&self, _graph: &SceneGraph) -> Option<(&'static str, Data)> {
        None
    }
    /// Its node types' behaviours. Design lists none: the shared tools are its
    /// base, and the end of every chain.
    fn behaviours(&self) -> &'static [&'static dyn NodeBehavior] {
        &[]
    }
    /// What it does when `event` happens in `session`; its error fails the
    /// call.
    fn on(&self, _session: &mut Session, _event: &NodeEvent) -> Result<(), ToolError> {
        Ok(())
    }
}

/// What a node type that belongs to one product does differently from the
/// base. Each question's default is "not mine": the base answers.
pub trait NodeBehavior: Sync {
    /// The node types it answers for.
    fn types(&self) -> &'static [NodeType];
    /// The stored fields it reads, which a view does not call undecoded.
    fn decoded(&self) -> &'static [Field] {
        &[]
    }
    /// What a node of its types shows beside the base's view, in the plugin
    /// API's names.
    fn shows(&self, _node: &SceneNode, _raw: &Raw, _json: &dyn crate::port::JsonText) -> Option<Map> {
        None
    }
    /// Whether a node of its types, as a parent, takes what `placement` puts
    /// in it.
    fn admits(&self, _parent: Guid, _placement: Placement) -> Option<Result<(), ToolError>> {
        None
    }
    /// Whether `move_node` moves a node of its types.
    fn movable(&self) -> bool {
        false
    }
    /// Where node `id` may move: into `parent`, of `parent_type` when it
    /// exists, and as what.
    fn moves_into(&self, _id: Guid, _parent: &str, _parent_type: Option<NodeType>) -> Option<Result<Placement, ToolError>> {
        None
    }
    /// Whether a copy made in the original's parent goes right after it.
    fn copies_beside_itself(&self) -> bool {
        false
    }
    /// Whether grafta lays out a node's children; `false` when its product
    /// places them.
    fn lays_out_children(&self) -> bool {
        true
    }
    /// Set the keys of `properties` it owns on node `id`, taking them out;
    /// `None` when it owns none of them.
    fn apply(&self, _session: &mut Session, _id: Guid, _properties: &mut Map) -> Option<Result<Applied, ToolError>> {
        None
    }
    /// The keys the base sets on a node of its types, when it takes a base
    /// type's (a slide, a frame's).
    fn base_keys(&self) -> Option<&'static [&'static str]> {
        None
    }
    /// The screens a node of its types holds at the top of a page, which a
    /// person sees in its place (a deck's grid, its slides); `None` when the
    /// node is a screen itself.
    fn screens(&self, _graph: &SceneGraph, _node: &SceneNode) -> Option<Vec<Guid>> {
        None
    }
}

/// What a behaviour set on a node.
pub struct Applied {
    /// The keys set, in the order set.
    pub set: Vec<String>,
    /// Said beside them: what the product does with them when the file opens.
    pub note: Option<&'static str>,
    /// Whether it took every key, so the base is not asked.
    pub whole: bool,
}

/// The products a toolkit knows, in order: given to it when it is made and
/// copied into each session, as its ports are, so no shared code names one.
#[derive(Clone, Copy)]
pub struct Products(&'static [&'static dyn ProductProfile]);

impl Products {
    /// `profiles`, the first being the product of a new file.
    #[must_use]
    pub const fn new(profiles: &'static [&'static dyn ProductProfile]) -> Self {
        Self(profiles)
    }

    /// Every profile, in order.
    pub fn iter(self) -> impl Iterator<Item = &'static dyn ProductProfile> {
        self.0.iter().copied()
    }

    /// The product a new file is.
    #[must_use]
    pub fn first(self) -> &'static dyn ProductProfile {
        self.0[0]
    }

    /// The profile of a file's product.
    #[must_use]
    pub fn of(self, product: figfile::Product) -> &'static dyn ProductProfile {
        self.iter().find(|p| p.product() == product).expect("every product has a profile")
    }

    /// Tell every product, in order, that `event` happened.
    pub fn emit(self, session: &mut Session, event: &NodeEvent) -> Result<(), ToolError> {
        self.iter().try_for_each(|p| p.on(session, event))
    }

    /// The behaviours that answer for `node_type`, in their products' order.
    /// A node's type decides, not the file's product: a sticky pasted into a
    /// Design file is still a sticky.
    pub fn behaviours_of(self, node_type: NodeType) -> impl Iterator<Item = &'static dyn NodeBehavior> {
        self.0.iter().flat_map(|p| p.behaviours()).copied().filter(move |b| b.types().contains(&node_type))
    }

    /// A page's screens, as a person sees them: its top-level nodes in layer
    /// order, each replaced by the screens it holds when a behaviour says it
    /// holds some.
    #[must_use]
    pub fn screens(self, graph: &SceneGraph, page: Guid) -> Vec<Guid> {
        graph
            .children(page)
            .flat_map(|node| {
                self.behaviours_of(node.node_type).find_map(|b| b.screens(graph, node)).unwrap_or_else(|| vec![node.id])
            })
            .collect()
    }

    /// Whether a behaviour reads `field`, on whatever node it is stored.
    #[must_use]
    pub fn decodes(self, field: Field) -> bool {
        self.0.iter().flat_map(|p| p.behaviours()).any(|b| b.decoded().contains(&field))
    }

    /// The product whose files Figma saves with `extension`, if any.
    #[must_use]
    pub fn for_extension(self, extension: &str) -> Option<&'static dyn ProductProfile> {
        figfile::Product::from_extension(extension).map(|product| self.of(product))
    }
}


