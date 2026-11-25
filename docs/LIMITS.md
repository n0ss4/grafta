# Limits

- **On a file, it is not live.** You work on a saved copy and open the new file it gives back. For the file you have open, use [the panel](guides/in-figma.md), or [Figma's own server](https://developers.figma.com/docs/figma-mcp-server/).
- **It works on files**, and with its panel on the file open in Figma's desktop app, not Figma's cloud. `new_file` takes Figma's format from a `.fig` you have; grafta does not ship it.
- **Pictures are close, not Figma's.** They use the fonts installed on your machine: with the design's font installed, most lines lay out within 2% of Figma's width; without it, a stand-in is used and the caption says so.
- **An open file costs memory**, far more than its size: on one Mac, a 57 MB file took 1.3 GB while open and an 86 MB one 5.5 GB. Close what you are done with.
- **Not yet:** auto-layout grids, component slots, boolean operations, directional transitions and mouse enter and leave in a prototype written (they are read), Dev Mode annotations, and component properties on variants.
- **Only grafta's own variables and styles can be changed.** The file's existing ones are read and bound to, not edited.
