//! `grafta tool` — one call to one of the model's tools.
//!
//! The same list `grafta mcp` serves, called once. It is how a tool gets
//! tried by hand, and how a script without a model client gets at the
//! toolkit. Exit status separates "fix your request" from "it broke": 1 for a
//! bad argument, a missing thing or a refusal, 2 for a failure.

use anyhow::{bail, Result};
use serde_json::Value;
use toolkit::app::data::Data;
use toolkit::edge::data::to_json;
use toolkit::{Output, ToolError};

pub fn run(name: Option<&str>, args: Option<&str>, out: Option<&std::path::Path>) -> Result<()> {
    let Some(name) = name else {
        for tool in toolkit::tools::ALL {
            let first_sentence = tool.description().split(". ").next().unwrap_or_default();
            println!("{:<22} {first_sentence}", tool.name());
        }
        return Ok(());
    };

    let args: Value = match args {
        Some(text) => match serde_json::from_str(text) {
            Ok(value) => value,
            Err(error) => bail!("arguments are not JSON: {error}"),
        },
        None => Value::Object(serde_json::Map::new()),
    };

    let mut kit = toolkit::new();
    match toolkit::edge::call(&mut kit, name, &args) {
        Ok(Output::Data(value)) => {
            println!("{}", serde_json::to_string_pretty(&to_json(&value))?);
            Ok(())
        }
        Ok(Output::Image { mime, bytes, caption }) => {
            println!("{}", serde_json::to_string_pretty(&image_summary(mime, &bytes, caption, out)?)?);
            Ok(())
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(exit_code(&error));
        }
    }
}

/// Run a file of calls, in order, against one toolkit, so that a handle
/// opened by the first call is still open for the rest. This is how a ladder
/// is produced without a model, through exactly the path a model takes.
pub fn batch(path: &std::path::Path) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| anyhow::anyhow!("cannot read {}: {error}", path.display()))?;
    let calls: Vec<Value> = serde_json::from_str(&text)
        .map_err(|error| anyhow::anyhow!("{} is not a JSON array of calls: {error}", path.display()))?;

    let mut kit = toolkit::new();
    let mut results: Vec<Value> = Vec::new();
    for (index, entry) in calls.iter().enumerate() {
        let Some(tool) = entry.get("tool").and_then(Value::as_str) else {
            bail!("call {index} has no \"tool\"");
        };
        let args = entry.get("args").cloned().unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        let args = toolkit::edge::batch::substitute(&args, &results).map_err(|error| anyhow::anyhow!("call {index}: {error}"))?;
        match toolkit::edge::call(&mut kit, tool, &args) {
            Ok(Output::Data(value)) => {
                let value = to_json(&value);
                println!("[{index}] {tool} → {value}");
                results.push(value);
            }
            Ok(Output::Image { mime, bytes, caption }) => {
                let out = entry.get("out").and_then(Value::as_str).map(std::path::Path::new);
                let summary = image_summary(mime, &bytes, caption, out)?;
                println!("[{index}] {tool} → {summary}");
                results.push(summary);
            }
            Err(error) => {
                eprintln!("[{index}] {tool} failed: {error}");
                std::process::exit(exit_code(&error));
            }
        }
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
