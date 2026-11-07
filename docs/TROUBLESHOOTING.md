# Troubleshooting

By what you see. Each section says why it happens and what to do.

- [macOS says it cannot check grafta](#macos-says-it-cannot-check-grafta)
- [Your AI app does not show grafta's tools](#your-ai-app-does-not-show-graftas-tools)
- [Text looks wrong, or a result says a font is not installed](#text-looks-wrong-or-a-result-says-a-font-is-not-installed)
- [Still stuck](#still-stuck)

## macOS says it cannot check grafta

You see something like *"grafta" Not Opened: Apple could not verify "grafta" is
free of malware*, the first time grafta runs.

**Why.** grafta's binaries are not signed with an Apple developer account, so
macOS cannot check who made them. It asks only about a file marked as
downloaded, which is what a browser, or an app that downloads for you, does.
The install script, Homebrew, npm and Cargo do not mark it, so with those you
should not see this.

**What to do.** Allow it once:

1. Click **Done** in the warning. Do not move grafta to the bin.
2. Open **System Settings → Privacy & Security**, and scroll down to
   *Security*.
3. Next to *"grafta" was blocked*, click **Open Anyway**, and confirm with
   your password or Touch ID.
4. Start your AI app again, or reconnect grafta in it.

macOS does not ask again for that copy. A new version may ask once more.

If you would rather use the terminal, removing the mark does the same:

```bash
xattr -d com.apple.quarantine "$(command -v grafta)"
```

Every release's files are listed with their checksums in `SHA256SUMS`, on the
[releases page](https://github.com/n0ss4/grafta/releases), and the install
script checks them before installing.

## Your AI app does not show grafta's tools

You installed it, and your app lists no grafta, shows it as failed, or the
model says it has no such tool.

**First, check grafta itself runs.** In a terminal:

```bash
grafta --version      # installed by the script, Homebrew or Cargo
npx -y grafta --version  # or through npm
```

If that prints a version, grafta is fine and the app is not finding it. If it
does not, see below.

`grafta mcp` on its own prints nothing and waits: that is right. It speaks to
your app through its input and output, and stops when the app closes it. Closed
before any app has spoken to it, with Ctrl-D, it says
`connection closed: initialize request`, which is expected too.

**Restart the app after changing its configuration.** Most read it only when
they start. In Claude Code, `/mcp` shows each server and lets you reconnect.

**`npx` not found.** Configurations that start grafta with `npx` need Node.js
18 or newer. Check with `node --version`, and install it from
[nodejs.org](https://nodejs.org) if it is missing or older. Or skip npm: with
grafta installed by the script, Homebrew or Cargo, use
`"command": "grafta", "args": ["mcp"]`.

**`grafta` not found, though it runs in your terminal.** An app opened from the
Dock or the Start menu may not see the folders your terminal adds to its
`PATH`, such as `~/.local/bin`, where the install script puts grafta. Give the
app the full path:

```bash
command -v grafta     # macOS, Linux: prints e.g. /Users/you/.local/bin/grafta
where grafta          # Windows
```

```json
{ "mcpServers": { "grafta": { "command": "/Users/you/.local/bin/grafta", "args": ["mcp"] } } }
```

The same goes for `npx`: `command -v npx` gives its full path.

**Check the file and the key.** Each app has its own file and its own shape:
VS Code uses `"servers"`, the others `"mcpServers"`. The
[README](../README.md#install) has the line for each. A comma out of place
stops the whole file from being read, so try it in a JSON checker if you
edited it by hand.

**Read the app's log.** It usually says what went wrong in its first lines.

| App | Where |
|---|---|
| Claude Desktop | macOS: `~/Library/Logs/Claude/mcp-server-grafta.log`. Windows: `%APPDATA%\Claude\logs\` |
| Claude Code | `claude --debug`, or `/mcp` |
| Cursor | *Output* panel, *MCP Logs* |
| VS Code | *Output* panel, the server's own entry under *MCP* |

**ChatGPT on the web cannot use grafta.** It does not run tools from your
machine. The ChatGPT desktop app, with the Codex line in the README, can.

## Text looks wrong, or a result says a font is not installed

A picture from `render_node` shows text in another typeface, wider or
narrower than in Figma, or a result says *a font is not installed here, so
measured in a stand-in*.

**Why.** A Figma file names its fonts but does not carry them. grafta draws
and measures text with the fonts installed on your machine. When the design's
font is missing, it uses a stand-in and says which.

This changes only what grafta draws and measures. Your file keeps the font it
names, and Figma uses that font when you open it.

**What to do.** Install the font, then restart grafta.

1. Find the font's name in the result, or ask your AI which fonts the file
   uses.
2. Install it the usual way for your system: double-click the file and choose
   *Install* on macOS and Windows; on Linux, copy it to `~/.local/share/fonts`
   and run `fc-cache -f`. Many fonts in Figma files, such as Inter or Roboto,
   are free from [Google Fonts](https://fonts.google.com).
3. Restart your AI app, or reconnect grafta in it. grafta reads the machine's
   fonts once, when it starts, so a font installed after that is not seen.

With the design's font installed, most lines lay out within about 2% of
Figma's width. They are still grafta's pictures, not Figma's: open the file in
Figma to see it exactly.

**A machine with no fonts at all,** such as a minimal Linux container, gives
results that say *this machine has no fonts*. Sizes are then estimated, not
measured. Install any font, for example `fonts-dejavu` on Debian or Ubuntu.

## Still stuck

[Open an issue](https://github.com/n0ss4/grafta/issues/new/choose) with what
you did, what you saw, your system, your AI app, and `grafta --version`. If it
is about a file, say what kind it is and which Figma made it, but attach it
only if you are allowed to share it.
