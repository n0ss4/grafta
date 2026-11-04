#!/usr/bin/env python3
"""Tests for docs/showcase/transcripts_site.py.  python3 docs/showcase/transcripts_site.test.py

The recordings are small hand-made ones, shaped as Claude Code's transcript and
scripts/record-mcp.py's file are.
"""
import base64
import importlib.util
import json
import pathlib
import struct
import sys
import tempfile
import unittest
import zlib

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("transcripts_site", HERE / "transcripts_site.py")
ts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ts)

SPECS = json.loads((HERE.parents[1] / "web" / "src" / "content" / "sessions.json").read_text(encoding="utf-8"))
THEME = next(s for s in SPECS if s["id"] == "theme")
CWD = "/Users/someone/rec"


def png(width=4, height=3):
    raw = b"".join(b"\0" + b"\0\0\0" * width for _ in range(height))
    chunk = lambda kind, data: struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


# The theme session as a model might make it: name, args, result text, picture.
CALLS = [
    ("open_file", {"path": f"{CWD}/order.fig"}, json.dumps({"handle": "h", "file": f"{CWD}/order.fig"}), None),
    ("set_variable_collection", {"handle": "h", "id": "1:10", "add_mode": "High contrast"}, '{"ok": true}', None),
    ("set_variable", {"handle": "h", "id": "1:13"}, json.dumps({"ok": True, "pad": "x" * 2000}), None),
    ("render_node", {"handle": "h", "id": "1:1"}, '{"name": "Order"}', png()),
    ("save_as", {"handle": "h", "path": f"{CWD}/order-themed.fig"}, '{"written": true}', None),
]
ROWS = [
    {"to": 1, "beat": "read", "label": "Read the file"},
    {"to": 2, "beat": "mode", "label": "Added a High contrast mode"},
    {"to": 3, "beat": "colours", "label": "Filled in its colours"},
    {"to": 4, "beat": "render", "label": "Rendered it"},
    {"to": 5, "beat": "save", "label": "Saved order-themed.fig"},
]


def code_transcript(path, calls=CALLS, prompt=THEME["prompt"], other=False):
    entries = [{"type": "user", "cwd": CWD, "version": "2.9.0", "timestamp": "2026-11-02T10:00:00Z",
                "message": {"role": "user", "content": prompt}}]
    entries.append({"type": "assistant", "message": {"model": "claude-opus-5-5", "content": [{"type": "thinking", "thinking": "hidden"}]}})
    entries.append({"type": "assistant", "message": {"model": "claude-opus-5-5", "content": [{"type": "text", "text": "I'll open the file."}]}})
    if other:
        entries.append({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": "o1", "name": "Bash", "input": {"command": f"ls {CWD}"}}]}})
        entries.append({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": "o1", "content": "order.fig"}]}})
    for i, (name, args, text, picture) in enumerate(calls):
        entries.append({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": f"t{i}", "name": ts.GRAFTA + name, "input": args}]}})
        content = [{"type": "text", "text": text}]
        if picture:
            content.append({"type": "image", "source": {"type": "base64", "data": base64.b64encode(picture).decode()}})
        entries.append({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": f"t{i}", "content": content}]}})
    entries.append({"type": "assistant", "message": {"content": [{"type": "text", "text": f"Saved {CWD}/order-themed.fig."}]}})
    path.write_text("".join(json.dumps(e) + "\n" for e in entries))


def desktop_recording(rec, sid="theme", calls=CALLS, drop_one=False):
    turns = [{"kind": "user", "text": THEME["prompt"]}, {"kind": "reply", "text": "Opening it."}]
    turns += [{"kind": "tool", "name": name} for name, *_ in calls]
    turns.append({"kind": "reply", "text": "Done: order-themed.fig."})
    (rec / f"{sid}.desktop.json").write_text(json.dumps({"clientVersion": "1.4", "model": "Opus 5.5", "recordedOn": "2026-11-02", "cwd": CWD, "turns": turns}))
    log = []
    for i, (name, args, text, picture) in enumerate(calls[:-1] if drop_one else calls):
        log.append({"t": "x", "dir": "in", "msg": {"jsonrpc": "2.0", "id": i, "method": "tools/call", "params": {"name": name, "arguments": args}}})
        content = [{"type": "text", "text": text}]
        if picture:
            content.append({"type": "image", "data": base64.b64encode(picture).decode(), "mimeType": "image/png"})
        log.append({"t": "x", "dir": "out", "msg": {"jsonrpc": "2.0", "id": i, "result": {"content": content}}})
    (rec / f"{sid}.mcp.jsonl").write_text("".join(json.dumps(e) + "\n" for e in log))


class Import(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.rec = pathlib.Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def code(self, **kw):
        code_transcript(self.rec / "theme.code.jsonl", **kw)
        events, meta = ts.read_code(self.rec / "theme.code.jsonl")
        return ts.build(THEME, "claude-code", events, meta, ROWS, {}, "theme-code")

    def test_a_clean_code_session_imports(self):
        got = self.code()
        self.assertEqual(got["model"], "claude-opus-5-5")
        self.assertEqual(got["clientVersion"], "2.9.0")
        self.assertEqual(got["recordedOn"], "2026-11-02")
        kinds = [t["kind"] for t in got["turns"]]
        self.assertEqual(kinds[0], "user")
        self.assertEqual(kinds[-1], "reply")
        rows = [t for t in got["turns"] if t["kind"] == "tools"]
        self.assertEqual([r["beat"] for r in rows], ["read", "mode", "colours", "render", "save"])
        self.assertNotIn("hidden", json.dumps(got))  # thinking is not shown

    def test_paths_lose_the_working_directory(self):
        got = self.code()
        text = json.dumps(got)
        self.assertNotIn(CWD, text)
        self.assertIn('"path": "order-themed.fig"', text)
        self.assertTrue(got["edits"]["paths"])

    def test_a_long_result_is_cut_and_counted(self):
        got = self.code()
        colours = next(t for t in got["turns"] if t.get("beat") == "colours")["calls"][0]
        self.assertEqual(len(colours["result"]), ts.LIMIT)
        self.assertIn("…cut here", colours["omitted"])
        self.assertEqual(got["edits"]["cut"], 1)

    def test_a_picture_is_kept_with_its_size(self):
        pictures = {}
        code_transcript(self.rec / "theme.code.jsonl")
        events, meta = ts.read_code(self.rec / "theme.code.jsonl")
        got = ts.build(THEME, "claude-code", events, meta, ROWS, pictures, "theme-code")
        render = next(t for t in got["turns"] if t.get("beat") == "render")["calls"][0]
        self.assertEqual(render["picture"]["width"], 4)
        self.assertEqual(render["picture"]["src"], "showcase/sessions/theme-code-1.png")
        self.assertIn("theme-code-1.png", pictures)

    def test_another_tool_is_a_collapsed_row_never_dropped(self):
        got = self.code(other=True)
        other = [t for t in got["turns"] if t.get("other")]
        self.assertEqual(len(other), 1)
        self.assertEqual(other[0]["label"], "Used another tool")
        self.assertEqual(other[0]["calls"][0]["tool"], "Bash")
        self.assertEqual(got["edits"]["otherTools"], 1)

    def test_a_reply_linking_a_saved_picture_points_at_the_site(self):
        shown = [(1793450294150, "showcase/sessions/contrast-claude-code-1.png")]
        text, n = ts.relink("**Before:**\n![before](/Users/x/.claude/projects/p/s/tool-results/mcp-grafta-blob-1793450294141-gft12e.png)", shown)
        self.assertEqual(n, 1)
        self.assertIn("![before](showcase/sessions/contrast-claude-code-1.png)", text)
        self.assertNotIn("/Users/", text)

    def test_a_saved_picture_with_no_match_is_named_not_kept(self):
        text, n = ts.relink("![after](/Users/x/tool-results/mcp-grafta-blob-1793450399999-abc123.png)", [(1793450294150, "p.png")])
        self.assertEqual(n, 1)
        self.assertNotIn("/Users/", text)
        self.assertIn("not kept: after", text)

    def test_a_picture_claude_code_kept_in_a_file_is_read_and_its_note_shortened(self):
        saved = self.rec / "mcp-grafta-blob-1793450294141-abc.png"
        saved.write_bytes(png(6, 5))
        text, picture = ts.result_parts([{"type": "text", "text": f'{{"name": "Order"}}\n[Image: source: {saved}]'}])
        self.assertEqual(ts.png_size(picture), (6, 5))
        self.assertNotIn(str(self.rec), text)
        self.assertIn("[Image]", text)

    def test_stops_when_a_path_from_the_recording_machine_is_left(self):
        calls = [("open_file", {"path": "/Users/elsewhere/order.fig"}, "{}", None)] + CALLS[1:]
        with self.assertRaisesRegex(ts.Stop, "paths from the recording's machine"):
            self.code(calls=calls)

    def test_stops_when_the_first_message_is_not_the_prompt(self):
        with self.assertRaisesRegex(ts.Stop, "not the prompt"):
            self.code(prompt="Something else")

    def test_stops_when_a_beat_lacks_what_it_needs(self):
        calls = [c if c[0] != "set_variable_collection" else ("read_node", {"id": "1:10"}, "{}", None) for c in CALLS]
        with self.assertRaisesRegex(ts.Stop, "needs set_variable_collection"):
            self.code(calls=calls)

    def test_the_file_is_the_last_save_as(self):
        calls = CALLS[:-1] + [("save_as", {"path": f"{CWD}/order-themes.fig"}, "{}", None)]
        self.assertEqual(self.code(calls=calls)["file"], "order-themes.fig")

    def test_stops_when_a_session_meant_to_write_a_file_wrote_none(self):
        calls = [c for c in CALLS if c[0] != "save_as"]
        rows = [r for r in ROWS if r["beat"] != "save"]
        code_transcript(self.rec / "theme.code.jsonl", calls=calls)
        events, meta = ts.read_code(self.rec / "theme.code.jsonl")
        theme = {**THEME, "beats": [b for b in THEME["beats"] if b["id"] != "save"]}
        with self.assertRaisesRegex(ts.Stop, "wrote none"):
            ts.build(theme, "claude-code", events, meta, rows, {}, "theme-code")

    def test_a_desktop_session_joins_calls_by_order(self):
        desktop_recording(self.rec)
        events, meta = ts.read_desktop(self.rec / "theme.desktop.json", self.rec / "theme.mcp.jsonl")
        got = ts.build(THEME, "claude-desktop", events, meta, ROWS, {}, "theme-desktop")
        self.assertEqual(got["model"], "Opus 5.5")
        self.assertEqual(sum(len(t["calls"]) for t in got["turns"] if t["kind"] == "tools"), 5)
        render = next(t for t in got["turns"] if t.get("beat") == "render")["calls"][0]
        self.assertIn("picture", render)

    def test_stops_when_desktop_counts_differ(self):
        desktop_recording(self.rec, drop_one=True)
        with self.assertRaisesRegex(ts.Stop, "shows 5 grafta calls, grafta answered 4"):
            ts.read_desktop(self.rec / "theme.desktop.json", self.rec / "theme.mcp.jsonl")

    def test_run_writes_files_that_follow_the_contract(self):
        web = self.rec / "web"
        (web / "src" / "content").mkdir(parents=True)
        (web / "src" / "content" / "sessions.json").write_text(json.dumps([THEME]))
        code_transcript(self.rec / "theme.code.jsonl")
        desktop_recording(self.rec)
        beats = self.rec / "beats.json"
        beats.write_text(json.dumps({"theme": {"claude-code": ROWS, "claude-desktop": ROWS}}))
        written, pictures = ts.run(self.rec, web, beats)
        self.assertEqual(sorted(written), ["theme.claude-code", "theme.claude-desktop"])
        for path in (web / "public" / "sessions").glob("*.json"):
            recording = json.loads(path.read_text())
            for key in ["id", "client", "clientVersion", "model", "recordedOn", "file", "turns", "edits"]:
                self.assertIn(key, recording, path.name)
            for turn in recording["turns"]:
                self.assertIn(turn["kind"], ("user", "reply", "tools"))
        self.assertEqual(len(list((web / "public" / "showcase" / "sessions").iterdir())), 2)

    def test_run_takes_claude_code_alone_while_desktop_is_not_recorded(self):
        web = self.rec / "web"
        (web / "src" / "content").mkdir(parents=True)
        (web / "src" / "content" / "sessions.json").write_text(json.dumps([THEME]))
        code_transcript(self.rec / "theme.code.jsonl")
        beats = self.rec / "beats.json"
        beats.write_text(json.dumps({"theme": {"claude-code": ROWS}}))
        written, _ = ts.run(self.rec, web, beats)
        self.assertEqual(sorted(written), ["theme.claude-code"])


if __name__ == "__main__":
    unittest.main()
