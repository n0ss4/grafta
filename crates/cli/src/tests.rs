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

mod view {
    use crate::command::view::*;
    use serde_json::json;
    use toolkit::tools::ALL;

    /// Colour taken out, as anstream does through a pipe.
    fn plain(text: &str) -> String {
        let mut out = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn every_tool_is_in_exactly_one_group() {
        for tool in ALL {
            let count = GROUPS.iter().filter(|(_, names)| names.contains(&tool.name())).count();
            assert_eq!(count, 1, "{} is in {count} groups", tool.name());
        }
        let grouped: usize = GROUPS.iter().map(|(_, names)| names.len()).sum();
        assert_eq!(grouped, ALL.len(), "a group names a tool that does not exist");
    }

    #[test]
    fn the_list_names_every_tool_once_and_fits_the_width() {
        let list = plain(&tool_list(ALL, Some(80)));
        for tool in ALL {
            assert_eq!(list.matches(&format!("  {} ", tool.name())).count(), 1, "{}", tool.name());
        }
        assert!(list.lines().all(|line| line.chars().count() <= 80), "{list}");
    }

    #[test]
    fn coloured_json_is_serde_json_pretty_once_the_colour_is_gone() {
        let value = json!({
            "a": "text \"quoted\" é", "b": 1.5, "c": [1, true, null, {}], "d": {"e": [], "f": {"g": -2}}
        });
        assert_eq!(plain(&json(&value)), serde_json::to_string_pretty(&value).unwrap());
    }

    #[test]
    fn a_first_sentence_does_not_stop_at_an_abbreviation() {
        assert_eq!(first_sentence("Make modes, e.g. Light and Dark. Then more."), "Make modes, e.g. Light and Dark");
        assert_eq!(first_sentence("One sentence."), "One sentence");
    }

    #[test]
    fn cut_text_ends_or_starts_with_one_ellipsis() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("abc", 4), "abc");
        assert_eq!(truncate(&truncate("abcdefgh", 6), 4), "abc…");
        assert_eq!(truncate_front("/a/b/card.png", 9), "…card.png");
    }

    #[test]
    fn a_mistyped_tool_name_finds_the_one_meant() {
        let names = || ALL.iter().map(|tool| tool.name());
        assert_eq!(closest("reder_node", names()), Some("render_node"));
        assert_eq!(closest("render", names()), Some("render_node"));
        assert_eq!(closest("open-file", names()), Some("open_file"));
        assert_eq!(closest("zzzzzzzz", names()), None);
    }

    #[test]
    fn a_brief_result_puts_what_a_later_call_needs_first() {
        let value = json!({"bytes": 10, "pages": [1, 2], "handle": "h1", "meta": {"x": 1}, "written": "/a/b.png"});
        assert_eq!(brief(&value), "handle=h1  written=/a/b.png  bytes=10  meta={…}  pages=[2]");
    }

    #[test]
    fn every_tool_has_help_that_names_each_argument() {
        for tool in ALL {
            let help = plain(&tool_help(*tool, Some(100)));
            let schema = tool.input_schema();
            if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
                for name in properties.keys() {
                    assert!(help.contains(&format!("  {name} ")), "{}: {name} missing from\n{help}", tool.name());
                }
            }
            assert_eq!(help.contains("--batch"), takes_handle(*tool), "{}", tool.name());
        }
    }

    #[test]
    fn an_error_says_error_then_its_hints() {
        let shown = plain(&error("no tool named `x`", &["did you mean `y`?".to_owned()]));
        assert_eq!(shown, "error: no tool named `x`\nhint: did you mean `y`?");
    }
}
