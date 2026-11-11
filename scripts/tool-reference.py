#!/usr/bin/env python3
"""Write grafta's tool reference from the tool list its MCP server returns.

    python3 scripts/tool-reference.py target/release/grafta           # writes the reference
    python3 scripts/tool-reference.py target/release/grafta --check   # exits 1 if it differs

It asks `grafta mcp` for `tools/list`, as a client does, and writes
docs/TOOLS.md and the website's web/src/generated/tools.json from the answer. Nothing in them is written
by hand but the families the tools are grouped in, so CI can tell when a
tool's description or arguments changed and the reference did not.
"""
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MARKDOWN = ROOT / "docs" / "TOOLS.md"
DATA = ROOT / "web" / "src" / "generated" / "tools.json"

# The README's families, in its order. A tool the server lists and no family
# names, or a family naming a tool the server does not list, is an error.
FAMILIES = [
    ("Find your way", "Pages, counts, and nodes by name or type, with their path.",
     ["open_file", "summarize", "search_nodes", "list_open", "close_file"]),
    ("Read", "A node exactly, in the Figma plugin API's names.",
     ["read_node", "read_raw", "list_components", "component_instances", "get_image"]),
    ("See", "A frame as a picture, a PDF of frames, and every text's contrast.",
     ["render_node", "export_pdf", "check_contrast"]),
    ("Draw", "Frames, rich text, shapes, SVG icons, instances and sections.",
     ["create_page", "create_frame", "create_text", "create_shape", "create_vector", "create_instance",
      "create_section"]),
    ("Import", "A web page's layout as design layers.", ["import_layout"]),
    ("Change", "Fills, strokes, effects, auto-layout, sizing, constraints, images, order.",
     ["set_properties", "override_instance", "move_node", "duplicate_node", "group_nodes", "ungroup",
      "delete_node"]),
    ("Design system", "Components, variants and their properties, placed from rows of data.",
     ["create_component", "create_variant_set", "set_variant", "add_component_property", "fill_template"]),
    ("Tokens", "Variable collections with modes, aliases and bindings; exported as W3C Design Tokens.",
     ["list_variables", "export_tokens", "create_variable_collection", "create_variable", "set_variable",
      "set_variable_collection"]),
    ("Styles", "Colour, text and effect styles.", ["list_styles", "create_style", "set_style"]),
    ("Save", "A new file from nothing, or the open one with its edits.", ["new_file", "save_as"]),
]

REGENERATE = "python3 scripts/tool-reference.py target/release/grafta"


def tools_from(binary):
    requests = [
        {"jsonrpc": "2.0", "id": 1, "method": "initialize",
         "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                    "clientInfo": {"name": "tool-reference", "version": "1"}}},
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
    ]
    stdin = "".join(json.dumps(r) + "\n" for r in requests)
    out = subprocess.run([binary, "mcp"], input=stdin, capture_output=True, text=True, check=True).stdout
    for line in out.splitlines():
        message = json.loads(line)
        if message.get("id") == 2:
            return message["result"]["tools"]
    sys.exit(f"{binary} mcp answered no tools/list")


def families_of(tools):
    by_name = {tool["name"]: tool for tool in tools}
    named = [name for _, _, names in FAMILIES for name in names]
    unknown = sorted(set(named) - set(by_name))
    homeless = [name for name in by_name if name not in named]
    if unknown or homeless:
        sys.exit("scripts/tool-reference.py FAMILIES: "
                 + "; ".join(filter(None, [
                     unknown and f"not listed by the server: {', '.join(unknown)}",
                     homeless and f"in no family: {', '.join(homeless)}"])))
    return [(title, blurb, [by_name[name] for name in names]) for title, blurb, names in FAMILIES]


def type_of(schema):
    """A schema as a reader says it: `integer, 1–1000`, `"auto" | "absolute"`."""
    if not schema:
        return "any"
    for key in ("oneOf", "anyOf"):
        if key in schema:
            return " or ".join(type_of(part) for part in schema[key])
    if "enum" in schema:
        return " | ".join(json.dumps(value) for value in schema["enum"])
    kind = schema.get("type", "any")
    if kind == "array":
        text = f"array of {type_of(schema.get('items', {}))}"
        low, high = schema.get("minItems"), schema.get("maxItems")
        if low is not None and low == high:
            text += f", {low} items"
        elif low:
            text += f", at least {low}"
        return text
    if kind == "object" and schema.get("properties"):
        return "object { " + ", ".join(schema["properties"]) + " }"
    bounds = []
    if "minimum" in schema and "maximum" in schema:
        bounds.append(f"{schema['minimum']}–{schema['maximum']}")
    else:
        if "minimum" in schema:
            bounds.append(f"≥ {schema['minimum']}")
        if "maximum" in schema:
            bounds.append(f"≤ {schema['maximum']}")
    if "exclusiveMinimum" in schema:
        bounds.append(f"> {schema['exclusiveMinimum']}")
    if "exclusiveMaximum" in schema:
        bounds.append(f"< {schema['exclusiveMaximum']}")
    return ", ".join([kind] + bounds)


def arguments_of(tool):
    schema = tool.get("inputSchema", {})
    required = set(schema.get("required", []))
    rows = [(name, type_of(prop), name in required, prop.get("description", ""))
            for name, prop in schema.get("properties", {}).items()]
    return sorted(rows, key=lambda row: not row[2])


def cell(text):
    return text.replace("|", "\\|").replace("\n", " ")


def markdown(families, count):
    lines = [
        "# Tool reference",
        "",
        "<!-- Generated by scripts/tool-reference.py from `grafta mcp`'s tool list. Do not edit. -->",
        "",
        f"The {count} tools grafta's MCP server lists, in its words, with their arguments. "
        "A model reads the same descriptions and schemas; `grafta tool <name> '<json>'` calls one "
        "from a terminal. Every call but `open_file`, `new_file` and `list_open` takes the `handle` "
        "`open_file` returns.",
        "",
    ]
    for title, blurb, tools in families:
        lines.append(f"- **{title}**: " + " ".join(f"[`{t['name']}`](#{t['name']})" for t in tools))
    for title, blurb, tools in families:
        lines += ["", f"## {title}", "", blurb]
        for tool in tools:
            lines += ["", f"### `{tool['name']}`", "", tool["description"], ""]
            args = arguments_of(tool)
            if not args:
                lines.append("No arguments.")
                continue
            lines += ["| Argument | Type | | Description |", "|---|---|---|---|"]
            for name, kind, required, description in args:
                lines.append(f"| `{name}` | {cell(kind)} | {'required' if required else ''} | {cell(description)} |")
    return "\n".join(lines) + "\n"


def data(families, count):
    """The website's copy."""
    return json.dumps({"count": count, "families": [
        {"title": title, "blurb": blurb, "tools": [
            {"name": tool["name"], "description": tool["description"], "arguments": [
                {"name": name, "type": kind, "required": required, "description": description}
                for name, kind, required, description in arguments_of(tool)]}
            for tool in tools]}
        for title, blurb, tools in families]}, indent=1, ensure_ascii=False) + "\n"


def main():
    args = [arg for arg in sys.argv[1:] if arg != "--check"]
    if len(args) != 1:
        sys.exit(__doc__)
    tools = tools_from(args[0])
    families = families_of(tools)
    wanted = {MARKDOWN: markdown(families, len(tools)), DATA: data(families, len(tools))}
    if "--check" in sys.argv:
        stale = [str(path.relative_to(ROOT)) for path, text in wanted.items()
                 if not path.exists() or path.read_text(encoding="utf-8") != text]
        if stale:
            sys.exit(f"{', '.join(stale)} differ from the server's tool list; run: {REGENERATE}")
        return
    for path, text in wanted.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as out:
            out.write(text)
        print(path.relative_to(ROOT))


if __name__ == "__main__":
    main()
