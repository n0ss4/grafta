//! The `grafta` command line: what it accepts, and nothing else.
//!
//! This file defines the arguments and routes them. What each command actually
//! does lives in [`command`], which knows nothing about clap.

mod command;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "grafta",
    version,
    about = "Talk to any Figma file with your AI: ask what's in it, change it, or turn it into code"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Serve grafta's tools to a model over MCP, on stdio
    ///
    /// Register it once with a model client, e.g.
    /// `claude mcp add grafta -- /path/to/grafta mcp`.
    Mcp,
    /// Call one of the model's tools once, from the command line
    ///
    /// With no name, lists the tools. Arguments are a JSON object, e.g.
    /// `grafta tool open_file '{"path":"design.fig"}'`. Each call starts a
    /// fresh toolkit, so a handle does not outlive the call.
    Tool {
        /// The tool's name
        name: Option<String>,
        /// Its arguments, as a JSON object
        args: Option<String>,
        /// Run a JSON array of `{ "tool", "args" }` calls against one toolkit,
        /// in order. `"$N.field"` in an argument is call N's result field.
        #[arg(long, conflicts_with_all = ["name", "args"])]
        batch: Option<PathBuf>,
        /// Write a picture the tool returns (render_node, get_image) to this
        /// file. In a batch, an entry's `"out"` does the same.
        #[arg(long, conflicts_with = "batch")]
        out: Option<PathBuf>,
    },
}

fn main() {
    let result = match Cli::parse().command {
        Some(Commands::Mcp) => mcp::serve().map_err(|error| anyhow::anyhow!("{error}")),
        Some(Commands::Tool { batch: Some(batch), .. }) => command::tool::batch(&batch),
        Some(Commands::Tool { name, args, batch: None, out }) => {
            command::tool::run(name.as_deref(), args.as_deref(), out.as_deref())
        }
        // A bare invocation has nothing to act on, so it says what it accepts
        // instead of failing.
        None => {
            use clap::CommandFactory;
            Cli::command().print_help().map_err(Into::into)
        }
    };
    if let Err(error) = result {
        std::process::exit(exit_status(&error));
    }
}

/// Show an error, unless it was shown where it happened, and give the exit
/// status it calls for.
fn exit_status(error: &anyhow::Error) -> i32 {
    if let Some(command::tool::Shown(status)) = error.downcast_ref() {
        return *status;
    }
    anstream::eprintln!("{}", command::view::error(&format!("{error:#}"), &[]));
    1
}
// ci 101 check
// second push
