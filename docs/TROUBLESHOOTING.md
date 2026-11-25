# Troubleshooting

By what you see. Each section says why it happens and what to do.

- [macOS says it cannot check grafta](#macos-says-it-cannot-check-grafta)
- [Your AI app does not show grafta's tools](#your-ai-app-does-not-show-graftas-tools)
- [Text looks wrong, or a result says a font is not installed](#text-looks-wrong-or-a-result-says-a-font-is-not-installed)
- [grafta's panel in Figma stays on "Looking for grafta"](#graftas-panel-in-figma-stays-on-looking-for-grafta)
- [grafta's panel in Figma says "Update needed"](#graftas-panel-in-figma-says-update-needed)
- [A result says the person skipped a change, or did not answer](#a-result-says-the-person-skipped-a-change-or-did-not-answer)
- [A result says a layer is protected](#a-result-says-a-layer-is-protected)
- [Your AI app does not see the file you have open](#your-ai-app-does-not-see-the-file-you-have-open)
- [A result says grafta is stopped in Figma's panel](#a-result-says-grafta-is-stopped-in-figmas-panel)
- [Figma refuses a change, saying it is limited](#figma-refuses-a-change-saying-it-is-limited)
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
[Install](INSTALL.md) has the line for each. A comma out of place
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
machine. The ChatGPT desktop app, with the Codex line in [Install](INSTALL.md), can.

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

## grafta's panel in Figma stays on "Looking for grafta"

You opened grafta's panel on a file in Figma's desktop app, and it keeps
saying *Looking for grafta on this computer…*.

**Why.** The panel looks for grafta on your computer every second, on five
ports, 47823 to 47827. grafta listens on them only while your AI app is
running it: grafta starts when the app starts its tools, and stops with it.

**What to do.**

1. Open your AI app, or start a new conversation in it, so that it starts
   grafta. In Claude Code, `/mcp` shows whether grafta is connected. The panel
   finds it within a second.
2. Check grafta is the same release as the panel: `grafta --version`, against
   the release you downloaded `grafta-figma-plugin.zip` from. A grafta from
   before the panel does not listen for it. Update it the way you installed it.
3. Read your AI app's log ([where](#your-ai-app-does-not-show-graftas-tools)).
   If it says *the bridge to Figma did not start*, the five ports are taken,
   for example by five other `grafta mcp` running: close the AI apps or
   conversations you are not using. grafta still works on files meanwhile.

Figma in a browser has not been tried with the panel; use Figma's desktop app.

## grafta's panel in Figma says "Update needed"

The panel found grafta, but says *grafta … and this panel do not match*.

**Why.** grafta and its panel speak to each other in messages whose shape
changes now and then. Each release says which shape it speaks, and the panel
works only with a grafta that speaks its own. The panel and grafta are
installed apart, so one can be updated and the other not.

**What to do.** The panel says which one to update.

- *grafta … is older than this panel*: update grafta the way you installed it
  ([installing](INSTALL.md)), then start your AI app or a new
  conversation in it again.
- *This panel is older than grafta …*: download `grafta-figma-plugin.zip`
  from the same release as your grafta (`grafta --version`), unzip it over the
  old folder, and close and open the panel in Figma.

## A result says the person skipped a change, or did not answer

Your AI says a change was skipped, or that you *did not answer in Figma's
panel in time*.

**Why.** grafta's panel is in *Ask me first*: each change waits for you to
click *Apply* or *Skip*, in the panel or on the notice at the bottom of the
canvas, for up to 110 seconds. Nothing is changed until you do.

**What to do.** Apply or skip each change as it comes, or click *Apply the
rest of this request without asking* to let the rest of one request through.
To stop being asked, choose *Apply directly* at the bottom of the panel.

## A result says a layer is protected

Your AI says a layer *is protected in grafta's panel in Figma*.

**Why.** You protected it, or a layer holding it, in the panel: grafta changes
nothing in it, and makes nothing inside it.

**What to do.** If the change is wanted, click *Unprotect* beside it in the
panel, under *Protected from the AI*, and ask again.

## Your AI app does not see the file you have open

The panel shows grafta connected, or showed it once, but your AI says it sees
no file open in Figma, or `list_open` lists none with `in: "Figma"`.

**Why.** When the panel finds grafta, it asks whether to allow it, and gives
it the file only once you click **Allow**. Click **Refuse**, and the panel
turns that grafta away, without asking again, until the panel is closed.
Closing the panel, or the file, takes the file away from every app.

**What to do.**

- If the panel is asking, click **Allow**. Figma remembers it on this
  computer, and the panel does not ask again for grafta installed there.
- If you refused it, close the panel and open it again on the file: it asks
  again.
- If the panel was closed, open it again on the file, then ask your AI to look
  again. A file reopened gets a new handle, so a call your AI makes with the
  old one is refused with *no file open in Figma has handle …*: it should call
  `list_open` again.

## A result says grafta is stopped in Figma's panel

Your AI says *grafta is stopped in Figma's panel … nothing was done*, and the
panel shows *Stopped: grafta can do nothing here*.

**Why.** Someone pressed **Stop** in the panel. While it is stopped, every
call from every app is refused, and nothing in the file is changed.

**What to do.** Press **Resume** in the panel, then ask again. Stop is there
for you to use whenever you want grafta to keep its hands off the file.

## Figma refuses a change, saying it is limited

A change on the file you have open comes back refused in Figma's own words,
such as *Limited to 1 modes only* for a second mode in a variable collection.

**Why.** The file's Figma plan does not allow it. Figma refuses it, not grafta:
grafta passes on what Figma said, changes nothing, and does not work around it.

**What to do.** Use a file in a team whose plan allows it, or ask your AI to
do without it. What each plan allows is Figma's to say, on its pricing page.

## Still stuck

[Open an issue](https://github.com/n0ss4/grafta/issues/new/choose) with what
you did, what you saw, your system, your AI app, and `grafta --version`. If it
is about a file, say what kind it is and which Figma made it, but attach it
only if you are allowed to share it.
