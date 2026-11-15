//! A call on a file open in Figma, with Figma stood in for in memory: carried
//! with its arguments as given, or refused by name before anything is carried.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::app::data::Data;
use crate::edge::{self, Live, Request, Tool};
use crate::port::memory::{FixedFaces, Memory, MemoryJournal, MemoryLive};
use crate::port::{LiveError, LiveFile, Ports};
use crate::{Handle, ToolError, Toolkit};

const OPEN: u64 = 7;

fn open() -> Handle {
    Handle::in_figma(OPEN)
}

/// A toolkit with one file open in Figma, which answers every call as `answer` does.
fn kit_with(answer: impl Fn(&str, &str, &Data) -> Result<Data, LiveError> + Send + Sync + 'static) -> (Toolkit, Arc<MemoryLive>) {
    let live = Arc::new(MemoryLive::with(vec![LiveFile { handle: open(), name: "Checkout".into() }], answer));
    let ports = crate::port::memory::ports(&Arc::new(Memory::default()), &Arc::new(MemoryJournal::default()), FixedFaces::default());
    (crate::with_ports(Ports { live: live.clone(), ..ports }, None), live)
}

fn kit() -> (Toolkit, Arc<MemoryLive>) {
    kit_with(|_, _, args| Ok(args.clone()))
}

fn call(kit: &mut Toolkit, name: &str, args: Value) -> Result<Value, ToolError> {
    super::call(kit, name, args)
}

/// A tool the same on a file open in Figma, as tools will be once built there.
struct Echo;

impl Tool for Echo {
    fn name(&self) -> &'static str {
        "read_node"
    }
    fn description(&self) -> &'static str {
        "read_node, stood in for"
    }
    fn input_schema(&self) -> Value {
        edge::find("read_node").unwrap().input_schema()
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        edge::find("read_node").unwrap().parse(args)
    }
    fn live(&self) -> Live {
        Live::Same
    }
}

#[test]
fn the_file_open_in_figma_is_listed_beside_the_open_files() {
    let (mut kit, _) = kit();
    kit.insert("/tmp/hand-built.fig".into(), super::document()).unwrap();
    let listed = call(&mut kit, "list_open", json!({})).unwrap();
    let files = listed["files"].as_array().unwrap();
    assert_eq!(files.len(), 2, "{listed}");
    assert_eq!(files[1], json!({ "handle": open().as_str(), "in": "Figma", "name": "Checkout" }), "the stand-in answers {{}}: no page, no selection");
}

#[test]
fn a_tool_the_same_there_is_carried_with_its_arguments_and_without_the_handle() {
    let (kit, live) = kit_with(|_, _, _| Ok(Data::from("from Figma")));
    let args = json!({ "handle": open().as_str(), "id": "12:34", "depth": 2 });
    let output = edge::live::call(&kit, &Echo, open().as_str(), &args).unwrap();
    assert_eq!(output, crate::Output::Data(Data::from("from Figma")));
    let calls = live.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!((calls[0].0.as_str(), calls[0].1.as_str()), (open().as_str(), "read_node"));
    assert_eq!(edge::data::to_json(&calls[0].2), json!({ "id": "12:34", "depth": 2 }));
}

#[test]
fn arguments_a_file_would_refuse_are_refused_before_anything_is_carried() {
    let (mut kit, live) = kit();
    let unknown = call(&mut kit, "read_node", json!({ "handle": open().as_str(), "id": "1:2", "deep": 3 })).unwrap_err();
    let wrong = call(&mut kit, "read_node", json!({ "handle": open().as_str(), "id": "1:2", "depth": 99 })).unwrap_err();
    assert!(matches!(&wrong, ToolError::BadArgument(m) if m.contains("depth")), "{wrong}");
    assert!(matches!(&unknown, ToolError::BadArgument(m) if m.contains("no argument \"deep\"")), "{unknown}");
    let missing = edge::live::call(&kit, &Echo, open().as_str(), &json!({ "handle": open().as_str() })).unwrap_err();
    assert!(matches!(&missing, ToolError::BadArgument(m) if m.contains("\"id\"")), "{missing}");
    assert!(live.calls().is_empty());
}

#[test]
fn a_tool_that_cannot_act_there_is_refused_by_name_and_reason() {
    let (mut kit, live) = kit();
    let save = call(&mut kit, "save_as", json!({ "handle": open().as_str(), "path": "/tmp/out.fig" })).unwrap_err();
    assert_eq!(
        save,
        ToolError::Refused(
            "save_as does not act on a file open in Figma: Figma makes and saves its own files; to get a .fig of \
             it, File → Save local copy… in Figma"
                .into()
        )
    );
    let fill = call(&mut kit, "summarize", json!({ "handle": open().as_str() })).unwrap_err();
    assert_eq!(fill, ToolError::Refused("summarize does not act on a file open in Figma: not built for the open file yet".into()));
    assert!(live.calls().is_empty());
}

#[test]
fn a_handle_shaped_as_figma_s_and_open_nowhere_says_where_to_open_it() {
    let (mut kit, _) = kit();
    let gone = Handle::in_figma(OPEN + 1);
    assert_ne!(gone, open());
    let error = call(&mut kit, "summarize", json!({ "handle": gone.as_str() })).unwrap_err();
    assert!(matches!(&error, ToolError::NotFound(m) if m.contains("open grafta's panel") || m.contains("Open grafta's panel")), "{error}");
}

#[test]
fn a_file_grafta_opened_keeps_its_handle_whatever_is_open_in_figma() {
    let (mut kit, live) = kit();
    let handle = kit.insert("/tmp/hand-built.fig".into(), super::document()).unwrap().handle.to_string();
    call(&mut kit, "summarize", json!({ "handle": handle })).unwrap();
    assert!(live.calls().is_empty());
}

#[test]
fn what_figma_answers_instead_of_a_result_is_said_in_its_words() {
    let cases = [
        (LiveError::Refused("Limited to 1 modes only".into()), "Limited to 1 modes only"),
        (LiveError::Stopped, "stopped in Figma's panel"),
        (LiveError::TimedOut { seconds: 30 }, "within 30 s"),
        (LiveError::Gone, "whether it was done is not known"),
    ];
    for (error, words) in cases {
        let (kit, _) = kit_with(move |_, _, _| Err(error.clone()));
        let said = edge::live::call(&kit, &Echo, open().as_str(), &json!({ "handle": open().as_str(), "id": "1:2" })).unwrap_err();
        assert!(said.to_string().contains(words), "{said}");
    }
}

#[test]
fn what_acts_there_is_what_was_built_and_what_never_will_says_why() {
    let built = ["list_open", "read_node", "list_screens", "component_instances", "list_variables", "list_styles"];
    let differs = [
        "search_nodes", "set_properties", "render_node", "list_components", "create_component", "create_variant_set", "set_variant",
        "add_component_property", "create_instance", "override_instance", "create_variable_collection", "create_variable",
        "set_variable", "set_variable_collection", "create_style", "set_style",
    ];
    let never = ["open_file", "close_file", "new_file", "save_as", "read_raw", "check_contrast"];
    for tool in crate::tools::ALL {
        match (tool.name(), tool.live()) {
            (name, Live::Same) if built.contains(&name) => {}
            (name, Live::Differs(why)) if differs.contains(&name) => assert!(why.len() > 40, "{name}"),
            (name, Live::Cannot(why)) if never.contains(&name) => assert_ne!(why, edge::NOT_BUILT, "{name}"),
            (_, Live::Cannot(edge::NOT_BUILT)) => {}
            (name, live) => panic!("{name}: {live:?}"),
        }
    }
}

#[test]
fn list_open_says_the_page_and_the_selection_figma_has_now() {
    let (mut kit, live) = kit_with(|_, tool, _| {
        assert_eq!(tool, "list_open");
        Ok(crate::edge::data::from_json(&json!({ "page": { "id": "0:1", "name": "Cover" }, "selection": [{ "id": "1:2", "name": "Card", "type": "FRAME" }], "other": 1 })))
    });
    let listed = call(&mut kit, "list_open", json!({})).unwrap();
    assert_eq!(
        listed["files"][0],
        json!({ "handle": open().as_str(), "in": "Figma", "name": "Checkout", "page": { "id": "0:1", "name": "Cover" }, "selection": [{ "id": "1:2", "name": "Card", "type": "FRAME" }] })
    );
    assert_eq!(live.calls().len(), 1);
}

#[test]
fn list_open_names_a_file_figma_did_not_answer_for_and_lists_it_still() {
    let (mut kit, _) = kit_with(|_, _, _| Err(LiveError::Stopped));
    let listed = call(&mut kit, "list_open", json!({})).unwrap();
    assert!(listed["files"][0]["unread"].as_str().unwrap().contains("stopped"), "{listed}");
}

#[test]
fn read_node_on_the_open_file_is_figma_s_answer() {
    let (mut kit, live) = kit_with(|_, _, args| Ok(args.clone()));
    let read = call(&mut kit, "read_node", json!({ "handle": open().as_str(), "id": "1:2", "depth": 0 })).unwrap();
    assert_eq!(read, json!({ "id": "1:2", "depth": 0 }));
    assert_eq!(live.calls()[0].1, "read_node");
}

#[test]
fn render_node_on_the_open_file_is_figma_s_picture() {
    let (mut kit, _) = kit_with(|_, _, _| {
        Ok(crate::edge::data::from_json(&json!({ "image": { "mime": "image/png", "base64": "iVBORw==" }, "caption": { "id": "1:2", "drawn_by": "Figma" } })))
    });
    let drawn = call(&mut kit, "render_node", json!({ "handle": open().as_str(), "id": "1:2" })).unwrap();
    assert_eq!(drawn, json!({ "mime": "image/png", "len": 4, "caption": { "id": "1:2", "drawn_by": "Figma" } }));
}
