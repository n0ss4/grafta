//! Hand-built scene graphs.
//!
//! These stand in for `.fig` files in tests that only care about one part of the
//! model. Each builder documents the shape it produces, so a failing assertion
//! can be read against it without opening Figma.

use indexmap::IndexMap;
use model::{
    Color, ComponentProperty, ComponentPropertyType, Fill, Layout, LayoutMode, ModeId, NodeType,
    SceneGraph, SceneNode, TextStyle, Variable, VariableCollection, VariableMode, VariableType,
    VariableValue, VectorPath,
};
use schema::Guid;

/// Id of the page every builder roots its content under.
pub const PAGE: &str = "0:1";

/// A node id as tests write it, `session:local`.
///
/// # Panics
/// When `id` is not one.
#[must_use]
pub fn guid(id: &str) -> Guid {
    id.parse().unwrap_or_else(|_| panic!("`{id}` is not a node id"))
}

/// Assembles a graph without the ceremony of building each node in a `let`.
#[derive(Default)]
pub struct GraphBuilder {
    graph: SceneGraph,
}

impl GraphBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A builder whose graph already holds the [`PAGE`] frame, registered as a
    /// page so anything that walks `graph.pages` finds it.
    #[must_use]
    pub fn with_page() -> Self {
        Self::new().page(PAGE, "Page")
    }

    /// Add a top-level frame and register it as one of the file's pages.
    #[must_use]
    pub fn page(mut self, id: &str, name: &str) -> Self {
        self = self.add(id, name, NodeType::Frame, None, |_| {});
        self.graph.pages.push(guid(id));
        self
    }

    /// Add a node under `parent`, letting `configure` set whatever the test
    /// cares about and leaving every other field at its default.
    #[must_use]
    pub fn add(
        mut self,
        id: &str,
        name: &str,
        node_type: NodeType,
        parent: Option<&str>,
        configure: impl FnOnce(&mut SceneNode),
    ) -> Self {
        let mut node = SceneNode::new(guid(id), name, node_type);
        configure(&mut node);
        self.graph.insert(node, parent.map(guid));
        self
    }

    /// Add a variable collection and the variables belonging to it.
    #[must_use]
    pub fn collection(
        mut self,
        collection: VariableCollection,
        variables: impl IntoIterator<Item = Variable>,
    ) -> Self {
        self.graph.collections.insert(collection.id, collection);
        for variable in variables {
            self.graph.variables.insert(variable.id, variable);
        }
        self
    }

    #[must_use]
    pub fn build(self) -> SceneGraph {
        self.graph
    }
}

// ── Colour helpers ────────────────────────────────────────────────────────────

/// An opaque colour from 0–255 channels, which is how a design tool shows them.
#[must_use]
pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r: f32::from(r) / 255.0, g: f32::from(g) / 255.0, b: f32::from(b) / 255.0, a: 1.0 }
}

#[must_use]
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color { a, ..rgb(r, g, b) }
}

#[must_use]
pub fn solid(color: Color) -> Fill {
    Fill::solid(color)
}

pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

// ── Named graphs ──────────────────────────────────────────────────────────────

/// A horizontal auto-layout card (`1:1`) holding one text layer (`1:2`).
///
/// The card is 200×80 with 12/16 padding, an 8px gap, `SPACE_BETWEEN` on its
/// primary axis and `CENTER` on its counter axis — enough to exercise the whole
/// flexbox mapping in one graph.
#[must_use]
pub fn card() -> SceneGraph {
    GraphBuilder::new()
        .add("1:1", "Card", NodeType::Frame, None, |node| {
            node.width = 200.0;
            node.height = 80.0;
            node.layout = Layout {
                mode: LayoutMode::Horizontal,
                item_spacing: 8.0,
                padding_top: 12.0,
                padding_right: 16.0,
                padding_bottom: 12.0,
                padding_left: 16.0,
                primary_axis_align: "SPACE_BETWEEN".to_string(),
                counter_axis_align: "CENTER".to_string(),
                ..Layout::default()
            };
            node.fills = vec![solid(WHITE)];
        })
        .add("1:2", "Label", NodeType::Text, Some("1:1"), |node| {
            node.width = 100.0;
            node.height = 20.0;
            node.text = Some("Hello".to_string());
            node.text_style =
                Some(TextStyle { font_weight: 600, font_size: 16.0, ..TextStyle::default() });
            node.fills = vec![solid(BLACK)];
        })
        .build()
}

/// [`card`] wrapped in a page, so the card itself is rendered as a child and
/// carries its own styles rather than acting as the snapshot root.
#[must_use]
pub fn card_on_page() -> SceneGraph {
    let mut graph = card();
    graph.insert(SceneNode::new(guid(PAGE), "Page", NodeType::Frame), None);
    graph.pages.push(guid(PAGE));
    graph.nodes.get_mut(&guid(PAGE)).expect("page just inserted").children.push(guid("1:1"));
    graph
}

/// A page holding one vector layer (`1:1`) with stroke geometry.
///
/// Pass a zero dimension to get a separator: a layer with no thickness on one
/// axis, whose geometry is the only record of how thick the line it paints is.
#[must_use]
pub fn vector_icon(width: f32, height: f32) -> SceneGraph {
    GraphBuilder::with_page()
        .add("1:1", "icon", NodeType::Vector, Some(PAGE), |node| {
            node.width = width;
            node.height = height;
            node.stroke_geometry = vec![VectorPath {
                d: "M 0 0 L 10 0 L 10 10 Z".to_string(),
                fill_rule: "nonzero".to_string(),
            }];
            node.vector_bounds = Some((-0.5, -0.5, 10.5, 10.5));
            node.strokes = vec![solid(rgb(51, 51, 51))];
        })
        .build()
}

/// A `Button` component set (`1:1`) with `Small` (`1:2`) and `Large` (`1:3`)
/// variants, one declared variant property, and one instance (`2:1`) of the
/// small variant.
#[must_use]
pub fn button_variants() -> SceneGraph {
    let mut builder = GraphBuilder::new().add("1:1", "Button", NodeType::ComponentSet, None, |set| {
        set.component_properties = vec![ComponentProperty {
            id: guid("1:10"),
            name: "Size".to_string(),
            prop_type: ComponentPropertyType::Variant,
            default_value: Some("Small".to_string()),
            options: Vec::new(),
        }];
        set.variant_options
            .insert("Size".to_string(), vec!["Small".to_string(), "Large".to_string()]);
    });

    for (id, size) in [("1:2", "Small"), ("1:3", "Large")] {
        builder = builder.add(id, &format!("Size={size}"), NodeType::Component, Some("1:1"), |v| {
            v.width = 100.0;
            v.height = 32.0;
            v.variant_values.insert("Size".to_string(), size.to_string());
        });
    }

    builder
        .add("2:1", "Button", NodeType::Instance, None, |instance| {
            instance.component_id = Some(guid("1:2"));
        })
        .build()
}

/// A `Button` component (`2:1`) whose label (`2:2`) an instance (`3:1`) on the
/// page overrides the text of.
#[must_use]
pub fn instance_with_override() -> SceneGraph {
    GraphBuilder::with_page()
        .add("2:1", "Button", NodeType::Component, None, |_| {})
        .add("2:2", "Label", NodeType::Text, Some("2:1"), |label| {
            label.text = Some("Default".to_string());
            label.text_style = Some(TextStyle::default());
            label.override_key = Some(guid("9:2"));
        })
        .add("3:1", "Button", NodeType::Instance, Some(PAGE), |instance| {
            instance.component_id = Some(guid("2:1"));
            instance.symbol_overrides = vec![model::SymbolOverride {
                path: vec![guid("9:2")],
                text: Some("Save".to_string()),
                ..model::SymbolOverride::default()
            }];
        })
        .build()
}

/// A component (`2:1`) that contains an instance of itself, plus an outer
/// instance (`3:1`) on the page. Expanding it naively never terminates.
#[must_use]
pub fn recursive_component() -> SceneGraph {
    GraphBuilder::with_page()
        .add("2:1", "Loop", NodeType::Component, None, |_| {})
        .add("2:2", "Loop", NodeType::Instance, Some("2:1"), |inner| {
            inner.component_id = Some(guid("2:1"));
        })
        .add("3:1", "Loop", NodeType::Instance, Some(PAGE), |outer| {
            outer.component_id = Some(guid("2:1"));
        })
        .build()
}

/// A `Theme` collection (`5:1`) with `Light` (`5:2`) and `Dark` (`5:3`) modes,
/// a concrete colour variable (`6:1`) and an alias pointing at it (`6:2`).
#[must_use]
pub fn theme_variables() -> SceneGraph {
    let (collection, light_mode, dark_mode) = (guid("5:1"), guid("5:2"), guid("5:3"));
    let (surface, card) = (guid("6:1"), guid("6:2"));
    let mut light = IndexMap::new();
    light.insert(light_mode, VariableValue::Color { value: WHITE });
    light.insert(dark_mode, VariableValue::Color { value: BLACK });

    let mut alias = IndexMap::new();
    alias.insert(light_mode, VariableValue::Alias { variable_id: surface });

    GraphBuilder::new().collection(
        VariableCollection {
            id: collection,
            name: "Theme".to_string(),
            modes: vec![
                VariableMode { id: ModeId::Declared(light_mode), name: "Light".to_string() },
                VariableMode { id: ModeId::Declared(dark_mode), name: "Dark".to_string() },
            ],
            variable_ids: vec![surface, card],
        },
        [
            Variable {
                id: surface,
                name: "Colors/Surface".to_string(),
                var_type: VariableType::Color,
                collection_id: collection,
                values_by_mode: light,
            },
            Variable {
                id: card,
                name: "Semantic/Card Background".to_string(),
                var_type: VariableType::Color,
                collection_id: collection,
                values_by_mode: alias,
            },
        ],
    )
    .build()
}
