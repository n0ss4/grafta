---
name: grafta-audit
description: Audit a Figma .fig file with grafta without changing it - text contrast against WCAG AA, colours and spacing not bound to a variable or style, unused components, fonts missing where grafta draws. Use when the person asks to audit, review, lint or check the quality, accessibility or design-system consistency of a .fig, or of the file they have open in Figma.
---

# Auditing a .fig with grafta

An audit reads; it writes nothing. Use only the reading tools: `open_file`,
`summarize`, `list_screens`, `search_nodes`, `read_node`, `list_components`,
`component_instances`, `list_variables`, `list_styles`, `check_contrast`,
`render_node`. Never call `save_as` or a `create_`/`set_` tool during an audit,
even to "fix one thing": offer fixes at the end, and make them only when asked.

The `grafta` skill has the general loop and habits; this one is the audit.

## 1. Know what is there

1. `open_file { path }` (or, for what the person has open, `list_open` and the
   handle with `in: "Figma"`).
2. `summarize` for pages and totals. Note the counts: the report states them.
3. `list_screens { page }` for each page the person cares about. Ask which pages
   when there are many; an audit of a whole library file is long.
4. `list_variables`, `list_styles` and `list_components`: the design system the
   screens should be using. Follow `next_cursor` until it is absent.

## 2. Checks, each with the tool that answers it

| Check | How |
|---|---|
| **Text contrast** | `check_contrast { id }` on every screen. Follow `next_cursor`. A frame shown in several modes (`explicitVariableModes`) is checked in each: report a failure with its mode. List `unmeasured` texts with the reason grafta gives, not as passes. |
| **Colours not from the system** | `read_node { id, depth }` on a screen. A paint with a `variable` or a node with a `fillStyle`/`strokeStyle` is bound; a hex with neither is not. Compare the hex with `list_variables` and `list_styles`: an unbound hex equal to a token's value is "should bind to X"; one equal to none is "not in the system". |
| **Spacing and radii not from tokens** | Same `read_node` result: `itemSpacing`, padding and corner radii with a `variable` are bound. Only report these if the file has FLOAT variables for them; otherwise say the file has no spacing tokens. |
| **Text not from a text style** | `read_node` on TEXT nodes: no `textStyle` means local formatting. `search_nodes { type: "TEXT", within }` finds them on a screen. |
| **Unused components** | `list_components`: instance count 0. For one in doubt, `component_instances { id }`. |
| **Library parts used** | instances whose component is not in `list_components` come from a library: name them, do not count them as unused or wrong. |
| **Fonts grafta lacks** | `render_node` on each screen; the caption's `notes` say when a font was missing and a stand-in was used. That is the machine's fonts, not a defect of the file: report it apart. |
| **Things grafta does not decode** | `read_node` lists `undecoded` fields. Name them under "not checked", never guess at them. |

Bound results are cut by `budget`: when a result has `omitted`, follow what it
says (`children_from`, `cursor`, a narrower `within`) rather than raising the
budget for everything.

## 3. What is a fact and what is your judgement

grafta's results are exact. Your reading of them is not: "this frame looks like a
detached button" or "these two greys should be one token" is a judgement. Write
facts and judgements in separate columns or sections, and never present a
judgement as something grafta found.

## 4. The report

Give the person:

1. **Scope**: file, pages and screens audited, and the totals from `summarize`.
2. **Findings**, most serious first (contrast failures, then unbound values, then
   unused parts), one row each: screen, node id and path, what was found, the
   values (ratio and required ratio, hex and the nearest token), and the fix.
3. **Not checked**: pages skipped, results cut and not followed, `unmeasured`
   texts, `undecoded` fields, library components. Absence is part of the result.
4. **Pictures**: offer to show the screens with the worst findings via
   `render_node`.

If the person asks for fixes afterwards, that is a change: follow the `grafta`
skill, save to a new path, and run the same checks again on what you changed.

## On the file open in Figma

`check_contrast` works there, drawn by Figma; `summarize`, `search_nodes` and
`list_components` go a page at a time within a time budget, and say how far they
got under `omitted` or `pages_searched`. Report that coverage. Do not make any
change there during an audit: every call there is saved by Figma at once.
