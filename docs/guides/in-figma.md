# In Figma

To work on the file you have open in Figma's desktop app, add grafta's panel to Figma once. grafta must be installed in your AI app ([Install](../INSTALL.md)), the same release as the panel.

1. Download [`grafta-figma-plugin.zip`](https://github.com/n0ss4/grafta/releases/latest/download/grafta-figma-plugin.zip) and unzip it.
2. In Figma's desktop app: *Plugins → Development → Import plugin from manifest…*, and choose the `manifest.json` in that folder.
3. Open a file, then *Plugins → Development → grafta*. The panel looks for grafta on your computer.
4. With your AI app running grafta, the panel finds it and asks whether to allow it. Click **Allow**: it does not ask again on this computer.

Each change shows in the file as it is made, and Figma saves it: each call grafta makes is one step of *Edit → Undo*. The panel gathers the calls of each request and shows each one before and after, with *Revert this request* to put it all back. Choose **Ask me first** to apply or skip each change yourself, protect the layers the AI must not touch, and **Stop** refuses every call until you press Resume. It talks to grafta on your computer and to nothing else. A few tools are not there yet, such as `export_pdf` and `check_contrast`: the [tool reference](../TOOLS.md) says, for each, what it does on the open file. Figma in a browser has not been tried.
