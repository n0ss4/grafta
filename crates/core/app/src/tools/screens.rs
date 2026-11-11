//! A page's screens, as a person sees them in Figma.

use model::NodeType;

use super::read::require_node;
use crate::bound::{page, Paging};
use crate::data::data;
use crate::error::CatalogError;
use crate::view::{num, type_name};
use crate::{Output, Query, ToolError, Toolkit};

/// How many screens a page of results holds when the model does not say.
const DEFAULT_LIMIT: usize = 100;

/// A page's screens, each ready to draw, in the order a person sees them.
pub struct ListScreens {
    pub handle: String,
    pub page: String,
    pub paging: Paging,
}

impl Query for ListScreens {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let graph = &session.graph;
        let page_node = require_node(session, &self.page)?;
        if page_node.node_type != NodeType::Canvas {
            return Err(CatalogError::NotAPage { id: page_node.id, is: type_name(page_node.node_type) }.into());
        }
        let page_json = data!({ "id": page_node.id, "name": page_node.name });
        let reserve = session.ports.measure.bytes(&page_json);
        let screens: Vec<_> = session.products.screens(graph, page_node.id).into_iter().filter_map(|id| graph.node(id)).collect();
        let mut out = page(screens, &self.paging, session.edits, DEFAULT_LIMIT, reserve, session.ports.measure.as_ref(), |node| {
            data!({
                "id": node.id,
                "name": node.name,
                "type": type_name(node.node_type),
                "width": num(node.width),
                "height": num(node.height),
                "visible": node.visible,
            })
        })?;
        out["page"] = page_json;
        Ok(Output::Data(out))
    }
}
