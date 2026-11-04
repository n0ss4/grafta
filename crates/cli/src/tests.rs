//! The command line itself. A binary's types are not reachable from `tests/`,
//! so these live in-crate.

use clap::CommandFactory;

use super::*;

/// clap validates the whole command tree here: duplicate flags, conflicting
/// short options, defaults that fail their own value parser. Every one of those
/// is a panic at first run otherwise.
#[test]
fn the_command_definition_is_valid() {
    Cli::command().debug_assert();
}

#[test]
fn only_the_toolkit_commands_are_left() {
    let names: Vec<_> = Cli::command().get_subcommands().map(|command| command.get_name().to_owned()).collect();
    assert_eq!(names, ["mcp", "tool"]);
    assert!(Cli::try_parse_from(["grafta", "audit", "design.fig"]).is_err());
}

#[test]
fn it_prints_its_version() {
    let error = Cli::try_parse_from(["grafta", "--version"]).err().unwrap();
    assert_eq!(error.kind(), clap::error::ErrorKind::DisplayVersion);
}

#[test]
fn tool_takes_an_optional_name_and_json_arguments() {
    let cli = Cli::try_parse_from(["grafta", "tool", "summarize", r#"{"handle":"x"}"#]).unwrap();
    match cli.command {
        Some(Commands::Tool { name, args, .. }) => {
            assert_eq!(name.as_deref(), Some("summarize"));
            assert_eq!(args.as_deref(), Some(r#"{"handle":"x"}"#));
        }
        _ => panic!("expected the tool command"),
    }
    let cli = Cli::try_parse_from(["grafta", "tool"]).unwrap();
    assert!(matches!(cli.command, Some(Commands::Tool { name: None, args: None, batch: None, out: None })));
}

#[test]
fn mcp_takes_nothing() {
    assert!(matches!(Cli::try_parse_from(["grafta", "mcp"]).unwrap().command, Some(Commands::Mcp)));
    assert!(Cli::try_parse_from(["grafta", "mcp", "extra"]).is_err());
}

#[test]
fn a_tool_error_the_caller_can_fix_exits_1_and_a_failure_exits_2() {
    use crate::command::tool::exit_code;
    use toolkit::ToolError;
    assert_eq!(exit_code(&ToolError::BadArgument(String::new())), 1);
    assert_eq!(exit_code(&ToolError::NotFound(String::new())), 1);
    assert_eq!(exit_code(&ToolError::Refused(String::new())), 1);
    assert_eq!(exit_code(&ToolError::Failed(String::new())), 2);
}

#[test]
fn a_batch_takes_a_file_and_excludes_a_name() {
    let cli = Cli::try_parse_from(["grafta", "tool", "--batch", "calls.json"]).unwrap();
    assert!(matches!(cli.command, Some(Commands::Tool { batch: Some(_), .. })));
    assert!(Cli::try_parse_from(["grafta", "tool", "summarize", "--batch", "calls.json"]).is_err());
}
