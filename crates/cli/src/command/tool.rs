//! `grafta tool` — one call to one of the model's tools.
//!
//! The same list `grafta mcp` serves, called once. It is how a tool gets
//! tried by hand, and how a script without a model client gets at the
//! toolkit. Exit status separates "fix your request" from "it broke": 1 for a
//! bad argument, a missing thing or a refusal, 2 for a failure.
//!
//! A result is JSON. In a terminal it is coloured, a batch shows one short
//! line per call, and an error says what to try; through a pipe the output is
//! plain JSON, as a script expects (see [`view`]).

use anyhow::{bail, Result};
use serde_json::Value;
use toolkit::app::data::Data;
use toolkit::edge::data::to_json;
use toolkit::tools::{Tool, ALL};
use toolkit::{Output, ToolError};

use super::view;

/// An error already shown to the person, with the exit status to leave with.
#[derive(Debug)]
pub struct Shown(pub i32);

impl std::fmt::Display for Shown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exit {}", self.0)
    }
}

impl std::error::Error for Shown {}

fn find(name: &str) -> Option<&'static dyn Tool> {
    ALL.iter().copied().find(|tool| tool.name() == name)
}

pub fn run(name: Option<&str>, args: Option<&str>, out: Option<&std::path::Path>) -> Result<()> {
    let Some(name) = name else {
        anstream::println!("{}", view::tool_list(ALL, view::width()));
        return Ok(());
    };

    let Some(tool) = find(name) else {
        let mut hints = Vec::new();
        if let Some(closest) = view::closest(name, ALL.iter().map(|tool| tool.name())) {
            hints.push(format!("did you mean `{closest}`?"));
        }
        hints.push("`grafta tool` lists them all".to_owned());
        anstream::eprintln!("{}", view::error(&format!("no tool named `{name}`"), &hints));
        return Err(Shown(1).into());
    };

    let args: Value = match args {
        Some(text) => match serde_json::from_str(text) {
            Ok(value) => value,
            Err(error) => {
                let hints = vec![
                    "arguments are one JSON object, in single quotes: '{\"path\":\"design.fig\"}'".to_owned(),
                    format!("`grafta tool {name}` shows what it takes"),
                ];
                anstream::eprintln!("{}", view::error(&format!("arguments are not JSON: {error}"), &hints));
                return Err(Shown(1).into());
            }
        },
        // With nothing to call it with, a tool that needs arguments says what
        // they are, as a bare `grafta` prints its help.
        None if requires_arguments(tool) => {
            anstream::println!("{}", view::tool_help(tool, view::width()));
            return Ok(());
        }
        None => Value::Object(serde_json::Map::new()),
    };

    let terminal = view::stdout_is_terminal();
    let mut kit = toolkit::new();
    match toolkit::edge::call(&mut kit, name, &args) {
        Ok(Output::Data(value)) => {
            let value = to_json(&value);
            if terminal {
                anstream::println!("{}", view::json(&value));
            } else {
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
            Ok(())
        }
        Ok(Output::Image { mime, bytes, caption }) => {
            let summary = image_summary(mime, &bytes, caption, out)?;
            if terminal {
                anstream::println!("{}", view::json(&summary));
                if out.is_none() {
                    anstream::eprintln!("{} add --out picture.png to save the picture", view::paint_hint());
                }
            } else {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
            Ok(())
        }
        Err(error) => {
            anstream::eprintln!("{}", view::error(&error.to_string(), &hints_for(tool, &args, &error)));
            Err(Shown(exit_code(&error)).into())
        }
    }
}

/// What to try next after a tool's error, when the command line can tell.
fn hints_for(tool: &dyn Tool, args: &Value, error: &ToolError) -> Vec<String> {
    match error {
        ToolError::NotFound(message) if args.get("handle").is_some() && message.contains("handle") => vec![
            "each `grafta tool` call starts afresh, so a handle from another call is gone".to_owned(),
            "run open_file and the calls after it together: `grafta tool --batch calls.json`".to_owned(),
        ],
        ToolError::BadArgument(_) => vec![format!("`grafta tool {}` shows what it takes", tool.name())],
        _ => Vec::new(),
    }
}

fn requires_arguments(tool: &dyn Tool) -> bool {
    tool.input_schema().get("required").and_then(Value::as_array).is_some_and(|required| !required.is_empty())
}

/// Run a file of calls, in order, against one toolkit, so that a handle
/// opened by the first call is still open for the rest. This is how a ladder
/// is produced without a model, through exactly the path a model takes.
pub fn batch(path: &std::path::Path) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("cannot read {}: {error}", path.display()))?;
    let calls: Vec<Value> = serde_json::from_str(&text)
        .map_err(|error| anyhow::anyhow!("{} is not a JSON array of calls: {error}", path.display()))?;

    let width = view::width();
    let terminal = view::stdout_is_terminal();
    let column = calls.iter().filter_map(|entry| entry.get("tool").and_then(Value::as_str)).map(str::len).max().unwrap_or(0);
    let started = std::time::Instant::now();
    let mut kit = toolkit::new();
    let mut results: Vec<Value> = Vec::new();
    for (index, entry) in calls.iter().enumerate() {
        let Some(tool) = entry.get("tool").and_then(Value::as_str) else {
            bail!("call {index} has no \"tool\"");
        };
        let args = entry.get("args").cloned().unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        let args = toolkit::edge::batch::substitute(&args, &results).map_err(|error| anyhow::anyhow!("call {index}: {error}"))?;
        let result = match toolkit::edge::call(&mut kit, tool, &args) {
            Ok(Output::Data(value)) => to_json(&value),
            Ok(Output::Image { mime, bytes, caption }) => {
                let out = entry.get("out").and_then(Value::as_str).map(std::path::Path::new);
                image_summary(mime, &bytes, caption, out)?
            }
            Err(error) => {
                if terminal {
                    let hints = find(tool).map(|found| hints_for(found, &args, &error)).unwrap_or_default();
                    anstream::eprintln!("{}", view::error(&format!("call {index}, {tool}: {error}"), &hints));
                } else {
                    eprintln!("[{index}] {tool} failed: {error}");
                }
                return Err(Shown(exit_code(&error)).into());
            }
        };
        if terminal {
            anstream::println!("{}", view::batch_line(index, tool, &result, column, width));
        } else {
            println!("[{index}] {tool} → {result}");
        }
        results.push(result);
    }
    if terminal {
        let calls = if results.len() == 1 { "1 call".to_owned() } else { format!("{} calls", results.len()) };
        anstream::println!("{}", view::done(&format!("{calls} in {:.1} s", started.elapsed().as_secs_f64())));
    }
    Ok(())
}

/// What the command line shows for a picture: its type, size and caption,
/// and where it was written when `out` names a file.
fn image_summary(mime: &str, bytes: &[u8], caption: Option<Data>, out: Option<&std::path::Path>) -> Result<Value> {
    let mut summary = serde_json::json!({ "mime": mime, "bytes": bytes.len() });
    if let Some(caption) = caption {
        summary["caption"] = to_json(&caption);
    }
    if let Some(out) = out {
        std::fs::write(out, bytes).map_err(|error| anyhow::anyhow!("cannot write {}: {error}", out.display()))?;
        summary["written"] = Value::String(out.display().to_string());
    }
    Ok(summary)
}

/// 1 when the caller can fix it, 2 when something failed.
#[must_use]
pub fn exit_code(error: &ToolError) -> i32 {
    match error {
        ToolError::BadArgument(_) | ToolError::NotFound(_) | ToolError::Refused(_) => 1,
        ToolError::Failed(_) => 2,
    }
}
