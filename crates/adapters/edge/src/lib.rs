#![deny(clippy::all)]

//! Where a call's JSON meets the toolkit.
//!
//! A tool's edge half names it, describes it, gives its JSON Schema and reads
//! its arguments into a [`Command`] or a [`Query`]. The dispatcher runs a
//! query as it is and a command inside its unit of work, then journals the
//! command. MCP, `grafta tool` and the journal's replay all call [`call`].

pub mod args;
pub mod batch;
pub mod catalog;
pub mod component;
pub mod data;
pub mod import;
pub mod journal;
pub mod measure;
pub mod new_file;
pub mod overrides;
pub mod read;
pub mod schema;
pub mod session;
pub mod shapes;
pub mod shared_style;
pub mod structure;
pub mod style;
pub mod template;
pub mod text;
pub mod variable;
pub mod vector;
pub mod write;

/// The application's items, by the paths the edge's halves name them.
pub(crate) mod app {
    pub use ::app::{data, unit, Command, Query};
}
pub(crate) use ::app::{bound, tools, view};
pub(crate) use domain::{error, fnv, port, Base, DomainError, Handle, Output, Recovery, Session, ToolError, Toolkit};

use serde_json::{json, Value};

use crate::app::{Command, Query};

/// One operation a model can call: its edge half.
pub trait Tool: Sync {
    fn name(&self) -> &'static str;
    /// What it does, for the model deciding whether to call it.
    fn description(&self) -> &'static str;
    /// Its parameters, as JSON Schema.
    fn input_schema(&self) -> Value;
    /// The call's arguments read into what it asks.
    fn parse(&self, args: &Value) -> Result<Request, ToolError>;
}

/// What a call asks, read from its arguments.
pub enum Request {
    Command(Box<dyn Command>),
    Query(Box<dyn Query>),
}

/// Every tool, in the order a model is likely to need them.
pub static ALL: &[&dyn Tool] = &[
    &session::OpenFile,
    &new_file::NewFile,
    &session::CloseFile,
    &session::ListOpen,
    &read::Summarize,
    &read::SearchNodes,
    &read::ReadNode,
    &read::ReadRaw,
    &catalog::ListComponents,
    &catalog::ComponentInstances,
    &catalog::ListVariables,
    &variable::CreateVariableCollection,
    &variable::CreateVariable,
    &variable::SetVariable,
    &variable::SetVariableCollection,
    &shared_style::ListStyles,
    &shared_style::CreateStyle,
    &shared_style::SetStyle,
    &catalog::GetImage,
    &catalog::RenderNode,
    &catalog::ExportPdf,
    &catalog::CheckContrast,
    &write::CreatePage,
    &write::CreateFrame,
    &write::CreateInstance,
    &component::CreateComponent,
    &component::CreateVariantSet,
    &component::SetVariant,
    &component::AddComponentProperty,
    &text::CreateText,
    &vector::CreateVector,
    &shapes::CreateShape,
    &overrides::OverrideInstance,
    &template::FillTemplate,
    &import::ImportLayout,
    &style::SetProperties,
    &write::MoveNode,
    &write::DeleteNode,
    &structure::GroupNodes,
    &structure::Ungroup,
    &structure::CreateSection,
    &structure::DuplicateNode,
    &write::SaveAs,
];

/// Find a tool by name.
#[must_use]
pub fn find(name: &str) -> Option<&'static dyn Tool> {
    ALL.iter().copied().find(|tool| tool.name() == name)
}

/// Call a tool by name, with arguments as JSON.
pub fn call(kit: &mut Toolkit, name: &str, args: &Value) -> Result<Output, ToolError> {
    let tool = find(name).ok_or_else(|| DomainError::NoSuchTool { name: name.into(), tools: ALL.iter().map(|t| t.name()).collect() })?;
    if !args.is_object() {
        return Err(DomainError::ArgumentsNotAnObject { got: args.to_string() }.into());
    }
    let (mut output, edited_handles) = match run(kit, tool, args) {
        Ok(done) => done,
        Err(error) => {
            if kit.checks_rollbacks() {
                log_failure(name, args, &error);
            }
            for session in kit.sessions_mut() {
                session.recovery = None;
            }
            return Err(error);
        }
    };
    // Then it is journaled, before the model hears it was done.
    // A journal that cannot be written leaves the edit made.
    for handle in edited_handles {
        journal::record(kit, handle.as_str(), name, args, &output)?;
    }
    if name == write::SaveAs.name() {
        if let (Some(handle), Some(path)) = (args.get("handle").and_then(Value::as_str), args.get("path").and_then(Value::as_str)) {
            journal::mark_saved(kit, handle, path)?;
        }
    }
    // A file the call opened has its journal replayed now, after the call and
    // outside its unit, so the replayed edits are not journaled as the call's.
    let waiting: Vec<(crate::Handle, bool)> = kit.sessions_mut().filter_map(|s| s.recovery.take().map(|r| (s.handle.clone(), r))).collect();
    for (handle, recover) in waiting {
        if let (Some(report), Output::Data(out)) = (journal::recover(kit, handle.as_str(), recover)?, &mut output) {
            out["recovered"] = report;
        }
    }
    Ok(output)
}

/// The call read, then run: a query as it is, a command as a unit of work,
/// its result saying how many instances it refreshed.
fn run(kit: &mut Toolkit, tool: &dyn Tool, args: &Value) -> Result<(Output, Vec<crate::Handle>), ToolError> {
    match tool.parse(args)? {
        Request::Query(query) => Ok((query.run(kit)?, Vec::new())),
        Request::Command(command) => {
            let mut done = crate::app::unit::run(kit, tool.name(), command.as_ref())?;
            if let (Output::Data(crate::app::data::Data::Map(out)), true) = (&mut done.output, done.refreshed > 0) {
                out.insert("instances_refreshed".into(), crate::app::data::Data::from(done.refreshed as u64));
            }
            Ok((done.output, done.edited))
        }
    }
}

/// Where the tests write each failing call, when they are asked to: so a
/// refactor can be checked to leave every refusal's words as they were.
const FAILURES: &str = "GRAFTA_FAILURES";

/// Append a failing call to the file `GRAFTA_FAILURES` names, one line each.
fn log_failure(name: &str, args: &Value, error: &ToolError) {
    let Some(path) = std::env::var_os(FAILURES) else { return };
    let line = json!({ "tool": name, "args": args, "error": format!("{error:?}") }).to_string() + "\n";
    ::disk::append_line(std::path::Path::new(&path), &line);
}
