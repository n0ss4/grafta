//! Turning a frame into a component.
//!
//! A local component in the fixtures is a frame whose type is `SYMBOL`, plus
//! `isSymbolPublishable` and a `sharedSymbolVersion` token (11,759 measured).
//! Its children are addressed from instances by their own guid: 1 of 22,911
//! children of local components carries an `overrideKey`. So a frame grafta
//! turns into a component keeps its children as they are, and
//! `create_instance` and `override_instance` work on it as on the file's own.

use model::NodeType;
use schema::{object, Field, Guid, WireMut};

use super::read::{node_at, require_node};
use super::write::author_error;
use crate::data::{data, Data};
use crate::Command;
use crate::error::ComponentError;
use crate::{DomainError, Output, ToolError, Toolkit};

/// A frame turned into a component, in place.
pub struct CreateComponent {
    pub handle: String,
    pub id: String,
    pub name: Option<String>,
}

impl Command for CreateComponent {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let id = require_node(session, &self.id)?.id;
        super::guard::editable(session, id)?;
        let node = node_at(session, id)?;
        if node.node_type != NodeType::Frame {
            return Err(ComponentError::NotAFrame { id, is: crate::view::type_name(node.node_type) }.into());
        }
        let mut ancestor = node.parent_id;
        while let Some(parent) = ancestor {
            let parent = node_at(session, parent)?;
            if matches!(parent.node_type, NodeType::Component | NodeType::ComponentSet) {
                return Err(ComponentError::InsideComponent { id, component: parent.id, name: parent.name.clone() }.into());
            }
            if parent.node_type == NodeType::Instance {
                return Err(ComponentError::InsideInstance { id, instance: parent.id, name: parent.name.clone() }.into());
            }
            ancestor = parent.parent_id;
        }
        let name = &self.name;
        session.graft().map_err(author_error)?.make_component(id).map_err(author_error)?;
        if let (Some(name), Some(fields)) = (name, session.fields_mut(id)) {
            fields.set(Field::Name, kiwi::Value::Str(name.clone()));
        }
        session.refresh(&[id]);
        session.edits += 1;
        let node = node_at(session, id)?;
        Ok(Output::Data(data!({
            "id": id,
            "name": node.name,
            "type": crate::view::type_name(node.node_type),
            "children": node.children.len(),
            "next": "create_instance with this id as the component; override_instance to change a copy",
            "unsaved_edits": session.edits,
        })))
    }
}

/// Components grouped as the variants of one set.
pub struct CreateVariantSet {
    pub handle: String,
    pub name: String,
    /// Each component, and its values by property, as the call gave them.
    pub variants: Vec<(String, Vec<(String, String)>)>,
}

impl Command for CreateVariantSet {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let (name, variants) = (&self.name, &self.variants);
        // The same properties everywhere, each combination once.
        fn keys(pairs: &[(String, String)]) -> Vec<&str> {
            let mut keys: Vec<&str> = pairs.iter().map(|(p, _)| p.as_str()).collect();
            keys.sort_unstable();
            keys
        }
        let first = keys(&variants[0].1);
        if let Some((id, _)) = variants.iter().find(|(_, pairs)| keys(pairs) != first) {
            return Err(ComponentError::OtherProperties {
                id: id.clone(),
                first: variants[0].0.clone(),
                properties: first.iter().map(ToString::to_string).collect(),
            }
            .into());
        }
        for (i, (id, pairs)) in variants.iter().enumerate() {
            let mut mine = pairs.clone();
            mine.sort();
            if variants[..i].iter().any(|(_, other)| {
                let mut other = other.clone();
                other.sort();
                other == mine
            }) {
                return Err(ComponentError::RepeatedValues { id: id.clone() }.into());
            }
        }

        let mut parent: Option<Option<Guid>> = None;
        let mut bounds: Option<(f32, f32, f32, f32)> = None;
        let mut members: Vec<(Guid, Vec<(String, String)>)> = Vec::with_capacity(variants.len());
        for (id, pairs) in variants {
            let guid = require_node(session, id)?.id;
            super::guard::editable(session, guid)?;
            let node = node_at(session, guid)?;
            if node.node_type != NodeType::Component {
                return Err(ComponentError::NotAComponent { id: id.clone(), is: crate::view::type_name(node.node_type) }.into());
            }
            let here = node.parent_id;
            if let Some(set) = here.and_then(|h| session.graph.node(h)).filter(|p| p.node_type == NodeType::ComponentSet) {
                return Err(ComponentError::AlreadyAVariant { id: id.clone(), set: set.id }.into());
            }
            if parent.is_some_and(|p| p != here) {
                return Err(ComponentError::NotBeside { id: id.clone() }.into());
            }
            parent = Some(here);
            let (x, y, r, b) = (node.x, node.y, node.x + node.width, node.y + node.height);
            bounds = Some(bounds.map_or((x, y, r, b), |(a, c, d, e)| (a.min(x), c.min(y), d.max(r), e.max(b))));
            members.push((guid, pairs.clone()));
        }
        let parent = parent.flatten().ok_or_else(|| DomainError::NoParent { id: variants[0].0.clone() })?;
        let (left, top, right, bottom) = bounds.unwrap_or_default();
        const PADDING: f32 = 20.0;
        let at = (left - PADDING, top - PADDING);
        let size = (right - left + 2.0 * PADDING, bottom - top + 2.0 * PADDING);
        let mut placed = Vec::new();
        for (id, pairs) in &members {
            let node = node_at(session, *id)?;
            placed.push((*id, pairs.clone(), (node.x - at.0, node.y - at.1)));
        }
        let set = session
            .graft()
            .map_err(author_error)?
            .make_variant_set(parent, name, (at, size), &placed)
            .map_err(author_error)?;

        // The graph: the set, then each variant under it, renamed.
        session.insert_appended(&[set]).map_err(|e| ComponentError::SetNotReadBack { set, error: e.to_string() })?;
        for (id, pairs) in &members {
            let children = session.graph.node(*id).map(|n| n.children.clone()).unwrap_or_default();
            model::remove(&mut session.graph, &[*id]);
            session.insert_appended(&[*id]).map_err(|e| DomainError::MovedNotReadBack { id: *id, error: e.to_string() })?;
            if let Some(node) = session.graph.nodes.get_mut(id) {
                node.children = children;
                node.variant_values = pairs.iter().cloned().collect();
            }
        }
        if let Some(node) = session.graph.nodes.get_mut(&set) {
            let mut options: indexmap::IndexMap<String, Vec<String>> = indexmap::IndexMap::new();
            for (_, pairs) in variants {
                for (p, v) in pairs {
                    let values = options.entry(p.clone()).or_default();
                    if !values.contains(v) {
                        values.push(v.clone());
                    }
                }
            }
            node.variant_options = options;
        }
        session.created.push(set);
        session.edits += 1;
        let names: Vec<Data> = members
            .iter()
            .map(|(id, _)| data!({ "id": id, "name": session.graph.node(*id).map(|n| n.name.clone()) }))
            .collect();
        Ok(Output::Data(data!({
            "id": set,
            "name": name,
            "type": crate::view::type_name(NodeType::ComponentSet),
            "variants": names,
            "next": "create_instance with a variant's id; set_variant to switch an instance to another",
            "unsaved_edits": session.edits,
        })))
    }
}

/// An instance switched to another variant of its set.
pub struct SetVariant {
    pub handle: String,
    pub id: String,
    /// The values that change, by property.
    pub values: Vec<(String, String)>,
}

impl Command for SetVariant {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let instance = require_node(session, &self.id)?;
        let id = instance.id;
        if instance.node_type != NodeType::Instance {
            return Err(DomainError::NotAnInstance { id, is: crate::view::type_name(instance.node_type) }.into());
        }
        let (width, height) = (instance.width, instance.height);
        let current = instance
            .component_id
            .and_then(|c| session.graph.node(c))
            .ok_or(DomainError::NoComponent { id })?;
        let set = current
            .parent_id
            .and_then(|p| session.graph.node(p))
            .filter(|p| p.node_type == NodeType::ComponentSet)
            .ok_or_else(|| ComponentError::NotAVariant { id, component: current.name.clone() })?;
        let mut wanted = current.variant_values.clone();
        for (property, value) in &self.values {
            if !wanted.contains_key(property) {
                return Err(ComponentError::NoSuchProperty {
                    set: set.name.clone(),
                    property: property.clone(),
                    properties: wanted.keys().cloned().collect(),
                }
                .into());
            }
            wanted.insert(property.clone(), value.clone());
        }
        let target = session
            .graph
            .children(set.id)
            .find(|variant| variant.variant_values.len() == wanted.len() && wanted.iter().all(|(p, v)| variant.variant_values.get(p) == Some(v)))
            .ok_or_else(|| {
                let names: Vec<String> = session.graph.children(set.id).map(|v| v.name.clone()).collect();
                ComponentError::NoSuchVariant { set: set.name.clone(), variants: names }
            })?;
        let (target_id, target_size) = (target.id, (target.width, target.height));
        let (old_w, old_h) = (current.width, current.height);
        let fields = session.fields_mut(id).ok_or(DomainError::NotInFile { id })?;
        let data = fields.field_or_insert_with(Field::SymbolData, || kiwi::Value::Object(kiwi::Fields::new()));
        if let kiwi::Value::Object(data) = data {
            data.set(Field::SymbolId, target_id.to_value());
        }
        // Its expansion was of the old variant. Figma derives the new one.
        fields.take(Field::DerivedSymbolData);
        // An instance at its old variant's size takes the new one's, as in Figma.
        if (width - old_w).abs() < 0.01 && (height - old_h).abs() < 0.01 {
            fields.set(Field::Size, object([(Field::X, kiwi::Value::Float(target_size.0)), (Field::Y, kiwi::Value::Float(target_size.1))]));
        }
        session.refresh(&[id]);
        session.edits += 1;
        let name = session.graph.node(target_id).map(|n| n.name.clone());
        Ok(Output::Data(data!({ "id": id, "variant": target_id, "variant_name": name, "unsaved_edits": session.edits })))
    }
}

/// A property a component's instances set, with its default.
pub struct AddComponentProperty {
    pub handle: String,
    pub component: String,
    pub name: String,
    /// The type, in the file's spelling.
    pub kind: &'static str,
    pub default: Data,
}

impl Command for AddComponentProperty {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let name = self.name.clone();
        let id = require_node(session, &self.component)?.id;
        super::guard::editable(session, id)?;
        let node = node_at(session, id)?;
        if node.node_type != NodeType::Component {
            return Err(ComponentError::PropertyOffComponent { id, is: crate::view::type_name(node.node_type) }.into());
        }
        if let Some(parent) = node.parent_id.filter(|p| session.graph.node(*p).is_some_and(|p| p.node_type == NodeType::ComponentSet)) {
            return Err(ComponentError::PropertyOnVariant { id, set: parent }.into());
        }
        let kind = self.kind;
        if super::properties::definitions(session, id).iter().any(|d| d.name == name) {
            return Err(ComponentError::PropertyTaken { id, name }.into());
        }
        let default = super::properties::value(session, "default", kind, &self.default)?;
        let property = session
            .graft()
            .map_err(author_error)?
            .add_component_property(id, &name, &default)
            .map_err(author_error)?;
        session.refresh(&[id]);
        session.edits += 1;
        Ok(Output::Data(data!({
            "id": property.to_string(),
            "component": id,
            "name": name,
            "type": super::properties::plugin_type(kind),
            "next": "bind a layer inside the component: set_properties componentPropertyReferences { characters | visible | mainComponent: <this name> }",
            "unsaved_edits": session.edits,
        })))
    }
}
