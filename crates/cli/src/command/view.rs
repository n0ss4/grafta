//! How `grafta tool` looks in a terminal.
//!
//! Only for a person reading: what a script reads through a pipe stays as it
//! was, plain JSON. Colour goes through anstream, which drops it when the
//! output is not a terminal or `NO_COLOR` is set. Everything here returns a
//! string, so it can be tested without a terminal.

use std::fmt::Write as _;
use std::io::IsTerminal;

use anstyle::{AnsiColor, Style};
use serde_json::Value;
use toolkit::tools::Tool;

pub const BOLD: Style = Style::new().bold();
pub const DIM: Style = Style::new().dimmed();
pub const RED: Style = AnsiColor::Red.on_default().bold();
pub const GREEN: Style = AnsiColor::Green.on_default();
pub const YELLOW: Style = AnsiColor::Yellow.on_default();
pub const CYAN: Style = AnsiColor::Cyan.on_default();
pub const MAGENTA: Style = AnsiColor::Magenta.on_default();

/// The tools by job, as the README and docs/TOOLS.md group them
/// (scripts/tool-reference.py). A test checks every tool is in one.
pub const GROUPS: &[(&str, &[&str])] = &[
    ("Find your way", &["open_file", "summarize", "list_screens", "search_nodes", "list_open", "close_file"]),
    ("Read", &["read_node", "read_raw", "list_components", "component_instances", "get_image"]),
    ("See", &["render_node", "export_pdf", "check_contrast"]),
    (
        "Draw",
        &["create_page", "create_frame", "create_text", "create_shape", "create_vector", "create_instance", "create_section"],
    ),
    ("Import", &["import_layout"]),
    (
        "Change",
        &["set_properties", "override_instance", "move_node", "duplicate_node", "group_nodes", "ungroup", "delete_node"],
    ),
    ("Design system", &["create_component", "create_variant_set", "set_variant", "add_component_property", "fill_template"]),
    (
        "Tokens",
        &["list_variables", "create_variable_collection", "create_variable", "set_variable", "set_variable_collection"],
    ),
    ("Styles", &["list_styles", "create_style", "set_style"]),
    ("Save", &["new_file", "save_as"]),
];

/// Whether standard output is a person's terminal rather than a pipe or a file.
#[must_use]
pub fn stdout_is_terminal() -> bool {
    std::io::stdout().is_terminal()
}

/// The terminal's width, when standard output is one.
#[must_use]
pub fn width() -> Option<usize> {
    if !stdout_is_terminal() {
        return None;
    }
    terminal_size::terminal_size().map(|(terminal_size::Width(w), _)| usize::from(w))
}

fn paint(style: Style, text: &str) -> String {
    format!("{style}{text}{style:#}")
}

/// `text` cut to `max` characters, ending in an ellipsis when cut.
#[must_use]
pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", kept.trim_end_matches(|c: char| c == '…' || c.is_whitespace()))
}

/// `text` cut to its last `max` characters, starting with an ellipsis when
/// cut: for a path, whose end says the most.
#[must_use]
pub fn truncate_front(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_owned();
    }
    let kept: String = text.chars().skip(count - max.saturating_sub(1)).collect();
    format!("…{kept}")
}

/// `text` wrapped at `width`, every line after the first indented by `indent`.
#[must_use]
pub fn wrap(text: &str, width: usize, indent: usize) -> String {
    let room = width.saturating_sub(indent).max(20);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > room {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines.join(&format!("\n{}", " ".repeat(indent)))
}

/// A description's first sentence. "e.g." and "i.e." do not end one.
#[must_use]
pub fn first_sentence(description: &str) -> &str {
    let mut from = 0;
    while let Some(at) = description[from..].find(". ") {
        let end = from + at;
        let before = &description[..end];
        if before.ends_with("e.g") || before.ends_with("i.e") {
            from = end + 2;
            continue;
        }
        return &description[..end];
    }
    description.trim_end_matches('.')
}

/// `grafta tool` with no name: the tools by job, one line each.
#[must_use]
pub fn tool_list(tools: &[&dyn Tool], width: Option<usize>) -> String {
    let column = tools.iter().map(|tool| tool.name().len()).max().unwrap_or(0) + 2;
    let mut out = String::new();
    for (group, names) in GROUPS {
        let _ = writeln!(out, "{}", paint(BOLD, group));
        for name in *names {
            let Some(tool) = tools.iter().find(|tool| tool.name() == *name) else { continue };
            let summary = first_sentence(tool.description());
            let summary = match width {
                Some(width) => truncate(summary, width.saturating_sub(column + 2).max(20)),
                None => summary.to_owned(),
            };
            let _ = writeln!(out, "  {}{} {}", paint(CYAN, name), " ".repeat(column - name.len() - 1), summary);
        }
        out.push('\n');
    }
    let _ = writeln!(out, "{}", paint(DIM, "grafta tool <name>         a tool's arguments"));
    let _ = writeln!(out, "{}", paint(DIM, "grafta tool <name> '<json>' call it"));
    let _ = write!(out, "{}", paint(DIM, "grafta tool --batch <file>  several calls on one open file"));
    out
}

/// A parameter's kind, as a person reads it: its type, its choices, its range.
fn kind(schema: &Value) -> String {
    if let Some(choices) = schema.get("enum").and_then(Value::as_array) {
        let names: Vec<String> =
            choices.iter().map(|choice| choice.as_str().map_or_else(|| choice.to_string(), str::to_owned)).collect();
        return truncate(&names.join(" | "), 40);
    }
    let mut kind = match schema.get("type") {
        Some(Value::String(name)) => name.clone(),
        Some(Value::Array(names)) => names.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" | "),
        _ => "any".to_owned(),
    };
    if kind == "array" {
        if let Some(item) = schema.get("items").and_then(|items| items.get("type")).and_then(Value::as_str) {
            kind = format!("array of {item}");
        }
    }
    match (schema.get("minimum"), schema.get("maximum")) {
        (Some(min), Some(max)) => format!("{kind} {min}–{max}"),
        (Some(min), None) => format!("{kind} ≥ {min}"),
        (None, Some(max)) => format!("{kind} ≤ {max}"),
        (None, None) => kind,
    }
}

/// The parameters of a schema, required ones first in their order, then the
/// rest by name.
fn parameters(schema: &Value) -> Vec<(String, &Value, bool)> {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else { return Vec::new() };
    let required: Vec<&str> =
        schema.get("required").and_then(Value::as_array).map(|names| names.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
    let mut out: Vec<(String, &Value, bool)> = required
        .iter()
        .filter_map(|name| properties.get(*name).map(|property| ((*name).to_owned(), property, true)))
        .collect();
    let mut rest: Vec<_> = properties.iter().filter(|(name, _)| !required.contains(&name.as_str())).collect();
    rest.sort_by(|a, b| a.0.cmp(b.0));
    out.extend(rest.into_iter().map(|(name, property)| (name.clone(), property, false)));
    out
}

/// Whether a tool needs a handle, which only a batch can carry from one call
/// to the next.
#[must_use]
pub fn takes_handle(tool: &dyn Tool) -> bool {
    tool.input_schema().get("properties").and_then(|properties| properties.get("handle")).is_some()
}

/// `grafta tool <name>` with nothing to call it with: what it does and takes.
#[must_use]
pub fn tool_help(tool: &dyn Tool, width: Option<usize>) -> String {
    let width = width.unwrap_or(100).min(120);
    let schema = tool.input_schema();
    let params = parameters(&schema);
    let mut out = String::new();
    let _ = writeln!(out, "{}\n", paint(BOLD, tool.name()));
    let _ = writeln!(out, "  {}\n", wrap(tool.description(), width, 2));

    if params.is_empty() {
        let _ = writeln!(out, "{}", paint(BOLD, "Takes no arguments"));
    } else {
        let _ = writeln!(out, "{}", paint(BOLD, "Arguments"));
        let name_column = params.iter().map(|(name, _, _)| name.len()).max().unwrap_or(0) + 2;
        let kinds: Vec<String> = params.iter().map(|(_, property, _)| kind(property)).collect();
        let kind_column = kinds.iter().map(|kind| kind.chars().count()).max().unwrap_or(0).min(28) + 2;
        let indent = 2 + name_column + kind_column;
        for ((name, property, required), kind) in params.iter().zip(&kinds) {
            let mut about = property.get("description").and_then(Value::as_str).unwrap_or_default().to_owned();
            if *required {
                about = if about.is_empty() { "Required.".to_owned() } else { format!("Required. {about}") };
            }
            let kind_shown = truncate(kind, kind_column - 2);
            let _ = writeln!(
                out,
                "  {}{}{}{}{}",
                paint(if *required { YELLOW } else { CYAN }, name),
                " ".repeat(name_column - name.len()),
                paint(DIM, &kind_shown),
                " ".repeat(kind_column - kind_shown.chars().count()),
                wrap(&about, width, indent)
            );
        }
    }

    let _ = writeln!(out, "\n{}", paint(BOLD, "Example"));
    if takes_handle(tool) {
        // A handle lives as long as the toolkit, and each single call has its
        // own, so this one is shown as a batch.
        let _ = writeln!(out, "  {}", paint(DIM, "A handle lasts one `grafta tool` call, so open the file in the same batch:"));
        let _ = writeln!(out, "  [{{\"tool\":\"open_file\",\"args\":{{\"path\":\"design.fig\"}}}},");
        let _ = writeln!(out, "   {{\"tool\":\"{}\",\"args\":{}}}]", tool.name(), example_args(&params, "$0.handle"));
        let _ = write!(out, "  {}", paint(DIM, "saved as calls.json, then: grafta tool --batch calls.json"));
    } else {
        let args = example_args(&params, "…");
        if args == "{}" {
            let _ = write!(out, "  grafta tool {}", tool.name());
        } else {
            let _ = write!(out, "  grafta tool {} '{args}'", tool.name());
        }
    }
    out
}

/// A tool's required arguments, filled with placeholders, as JSON.
fn example_args(params: &[(String, &Value, bool)], handle: &str) -> String {
    let mut args = serde_json::Map::new();
    for (name, property, required) in params {
        if !required {
            continue;
        }
        let value = match (name.as_str(), property.get("type").and_then(Value::as_str)) {
            ("path", _) => Value::from("design.fig"),
            ("handle", _) => Value::from(handle),
            ("id", _) => Value::from("12:345"),
            (_, Some("integer" | "number")) => Value::from(0),
            (_, Some("boolean")) => Value::from(true),
            (_, Some("array")) => Value::Array(Vec::new()),
            (_, Some("object")) => Value::Object(serde_json::Map::new()),
            _ => match property.get("enum").and_then(Value::as_array).and_then(|choices| choices.first()) {
                Some(choice) => choice.clone(),
                None => Value::from("…"),
            },
        };
        args.insert(name.clone(), value);
    }
    Value::Object(args).to_string()
}

/// The tool a mistyped name most likely meant.
#[must_use]
pub fn closest<'a>(name: &str, names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let name = name.to_lowercase().replace('-', "_");
    let scored = names.into_iter().map(|candidate| {
        let score = if candidate.contains(name.as_str()) && name.len() >= 3 { 0 } else { distance(&name, candidate) };
        (score, candidate)
    });
    let (score, best) = scored.min_by_key(|(score, _)| *score)?;
    (score <= (name.len() / 3).max(2)).then_some(best)
}

/// Levenshtein distance, by characters.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous + usize::from(ca != *cb);
            previous = row[j + 1];
            row[j + 1] = substitution.min(row[j] + 1).min(row[j + 1] + 1);
        }
    }
    row[b.len()]
}

/// A JSON value laid out as serde_json's pretty printer does, in colour:
/// keys cyan, strings green, numbers yellow, true, false and null magenta.
#[must_use]
pub fn json(value: &Value) -> String {
    let mut out = String::new();
    json_into(&mut out, value, 0);
    out
}

fn json_into(out: &mut String, value: &Value, depth: usize) {
    let pad = |depth: usize| "  ".repeat(depth);
    match value {
        Value::Object(map) if map.is_empty() => out.push_str("{}"),
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Object(map) => {
            out.push_str("{\n");
            for (index, (key, item)) in map.iter().enumerate() {
                let _ = write!(out, "{}{}: ", pad(depth + 1), paint(CYAN, &Value::String(key.clone()).to_string()));
                json_into(out, item, depth + 1);
                out.push_str(if index + 1 < map.len() { ",\n" } else { "\n" });
            }
            let _ = write!(out, "{}}}", pad(depth));
        }
        Value::Array(items) => {
            out.push_str("[\n");
            for (index, item) in items.iter().enumerate() {
                out.push_str(&pad(depth + 1));
                json_into(out, item, depth + 1);
                out.push_str(if index + 1 < items.len() { ",\n" } else { "\n" });
            }
            let _ = write!(out, "{}]", pad(depth));
        }
        Value::String(_) => out.push_str(&paint(GREEN, &value.to_string())),
        Value::Number(_) => out.push_str(&paint(YELLOW, &value.to_string())),
        Value::Bool(_) | Value::Null => out.push_str(&paint(MAGENTA, &value.to_string())),
    }
}

/// The fields a later call or a person needs most, shown first.
const FIRST: &[&str] = &["handle", "id", "written", "path", "name", "file"];

/// A result in a few words: its top-level fields as `key=value`, a list by
/// its length and an object as `{…}`, so ids, handles and paths stand out.
#[must_use]
pub fn brief(value: &Value) -> String {
    let Value::Object(map) = value else { return value.to_string() };
    let rank = |key: &str| FIRST.iter().position(|first| *first == key).unwrap_or(FIRST.len());
    let mut fields: Vec<_> = map.iter().collect();
    fields.sort_by_key(|(key, _)| rank(key));
    fields
        .into_iter()
        .map(|(key, item)| {
            let shown = match item {
                Value::String(text) if matches!(key.as_str(), "written" | "path") => truncate_front(text, 40),
                Value::String(text) => truncate(text, 40),
                Value::Array(items) => format!("[{}]", items.len()),
                Value::Object(_) => "{…}".to_owned(),
                other => other.to_string(),
            };
            format!("{key}={shown}")
        })
        .collect::<Vec<_>>()
        .join("  ")
}

/// One line of a batch, for a person: the call, and its result in brief, cut
/// to fit. `column` is the width the tool names are padded to.
#[must_use]
pub fn batch_line(index: usize, tool: &str, result: &Value, column: usize, width: Option<usize>) -> String {
    let number = format!("[{index}]");
    let used = number.chars().count() + 1 + column.max(tool.len()) + 1;
    let room = width.map_or(usize::MAX, |width| width.saturating_sub(used).max(20));
    format!(
        "{} {}{} {}",
        paint(DIM, &number),
        paint(BOLD, tool),
        " ".repeat(column.saturating_sub(tool.len())),
        truncate(&brief(result), room)
    )
}

/// An error, as `error: …` in red, then any hints under it.
#[must_use]
pub fn error(message: &str, hints: &[String]) -> String {
    let mut out = format!("{} {message}", paint(RED, "error:"));
    for hint in hints {
        let _ = write!(out, "\n{} {hint}", paint(BOLD, "hint:"));
    }
    out
}

/// `hint:`, as an error's hints start.
#[must_use]
pub fn paint_hint() -> String {
    paint(BOLD, "hint:")
}

/// The last line of a batch in a terminal.
#[must_use]
pub fn done(text: &str) -> String {
    format!("{} {}", paint(GREEN, "done"), paint(DIM, text))
}
