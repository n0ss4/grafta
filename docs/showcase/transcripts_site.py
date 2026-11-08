#!/usr/bin/env python3
"""Turn sessions recorded in real clients into the website's data.

    python3 docs/showcase/transcripts_site.py <recordings dir> <web dir>
    python3 docs/showcase/transcripts_site.py --list <recordings dir> <id> <client>
    python3 docs/showcase/transcripts_site.py --talk <recordings dir> <web dir>

<recordings dir> holds, per session id of web/src/content/sessions.json:

- <id>.code.jsonl: Claude Code's transcript of the session;
- <id>.mcp.jsonl: what grafta said to Claude Desktop, by scripts/record-mcp.py,
  and <id>.desktop.json: the conversation as Claude Desktop showed it
  {"clientVersion", "model", "recordedOn", "cwd", "turns":
  [{"kind": "user" | "reply", "text"} | {"kind": "tool", "name"?, "other"?}]},
  a "tool" turn where each call was made, in order: grafta's unless it says
  "other": true (the client's own tools, which the recorder does not see);
- theme-without.code.jsonl or theme-without.desktop.json: the theme prompt with
  grafta not installed (scripts/record-code.sh theme-without).

docs/showcase/beats.json (contracts/beats.schema.json) says which beat each run
of grafta calls reaches. --list prints a recording's grafta calls, numbered,
to write it.

Writes web/public/sessions/<id>.<client>.json (contracts/sessions.schema.json),
web/src/generated/session-index.json (the clients each session was recorded
in) and the
pictures to web/public/showcase/sessions/. Only four edits are made, and counted
The working directory dropped from paths, long results cut,
the client's own tools kept as collapsed rows, and a reply's link to a picture
on the recorder's disk pointed at the same picture on the site. Stops, naming the session, when
a recording does not match what the site claims.
"""
import base64
import datetime
import json
import pathlib
import shutil
import re
import struct
import sys

LIMIT = 600
ROOT = pathlib.Path(__file__).resolve().parents[2]
GRAFTA = "mcp__grafta__"
CLIENTS = ("claude-desktop", "claude-code")


class Stop(Exception):
    """A recording that does not match what the site would claim."""


# ---- reading the recordings --------------------------------------------------


def lines(path):
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


# Claude Code may keep a picture a tool returned in a file and leave this in the result instead.
SAVED_PICTURE = re.compile(r"\[Image: source: ([^\]]+\.png)\]")


def result_parts(content):
    """MCP content blocks → (text, png bytes or None).

    A picture Claude Code kept in a file on the recorder's disk is read from
    it, and its note in the text says only that a picture was there (R3)."""
    text, png = [], None
    for block in content if isinstance(content, list) else [{"type": "text", "text": str(content)}]:
        if block.get("type") == "text":
            saved = SAVED_PICTURE.search(block["text"])
            if saved and png is None and pathlib.Path(saved.group(1)).exists():
                png = pathlib.Path(saved.group(1)).read_bytes()
            text.append(SAVED_PICTURE.sub("[Image]", block["text"]))
        elif block.get("type") == "image":
            data = block.get("data") or block.get("source", {}).get("data")
            if data:
                png = base64.b64decode(data)
    return "\n".join(text), png


def read_code(path):
    """Claude Code's transcript → events: user, reply, call (tool, args, text, png)."""
    entries = [e for e in lines(path) if not e.get("isSidechain") and not e.get("isMeta")]
    results, events, meta = {}, [], {"model": None, "clientVersion": None, "recordedOn": None, "cwd": None}
    stamps = {}
    for e in entries:
        content = (e.get("message") or {}).get("content")
        if e.get("type") == "user" and isinstance(content, list):
            for block in content:
                if block.get("type") == "tool_result":
                    results[block["tool_use_id"]] = result_parts(block.get("content"))
                    stamps[block["tool_use_id"]] = e.get("timestamp")
    for e in entries:
        kind, message = e.get("type"), e.get("message") or {}
        if kind not in ("user", "assistant"):
            continue
        meta["cwd"] = meta["cwd"] or e.get("cwd")
        meta["clientVersion"] = meta["clientVersion"] or e.get("version")
        meta["recordedOn"] = meta["recordedOn"] or (e.get("timestamp") or "")[:10] or None
        content = message.get("content")
        if kind == "user":
            if isinstance(content, str):
                events.append({"kind": "user", "text": content})
            elif isinstance(content, list):
                texts = [b["text"] for b in content if b.get("type") == "text"]
                if texts:
                    events.append({"kind": "user", "text": "\n".join(texts)})
            continue
        meta["model"] = meta["model"] or message.get("model")
        for block in content or []:
            if block.get("type") == "text" and block["text"].strip():
                events.append({"kind": "reply", "text": block["text"]})
            elif block.get("type") == "tool_use":
                text, png = results.get(block["id"], ("", None))
                events.append({"kind": "call", "name": block["name"], "args": block.get("input", {}), "text": text, "png": png,
                               "at": millis(stamps.get(block["id"]))})
    return events, meta


def millis(stamp):
    """An ISO timestamp as milliseconds since the epoch, or None."""
    if not stamp:
        return None
    return int(datetime.datetime.fromisoformat(stamp.replace("Z", "+00:00")).timestamp() * 1000)


# Claude Code saves a picture a tool returned as tool-results/<server>-blob-<ms>-<id>.png and a
# reply may link it from the recorder's disk.
LOCAL_PICTURE = re.compile(r"!\[([^\]]*)\]\(([^)\s]*-blob-(\d{13})-[A-Za-z0-9]+\.png)\)")


def relink(text, shown):
    """A reply's links to pictures on the recorder's disk pointed at the same pictures on the site (R3).

    `shown` is [(milliseconds the call answered, site path)]: Claude Code names the saved picture
    by the time it was saved, within a second of the call's answer. Returns the text and how many
    links changed; a link that matches no picture is named as left out, never kept."""
    count = 0

    def swap(match):
        nonlocal count
        count += 1
        alt, saved = match.group(1), int(match.group(3))
        near = [(abs(saved - at), src) for at, src in shown if at is not None and abs(saved - at) < 2000]
        if near:
            return f"![{alt}]({min(near)[1]})"
        return f"[a picture on the recording's machine, not kept: {alt or 'untitled'}]"

    return LOCAL_PICTURE.sub(swap, text), count


def read_desktop(conversation, recorder):
    """Claude Desktop's conversation joined with what grafta said, by order (R2)."""
    convo = json.loads(conversation.read_text(encoding="utf-8"))
    calls = []
    if recorder is not None and recorder.exists():
        answers = {e["msg"]["id"]: e["msg"] for e in lines(recorder) if e["dir"] == "out" and isinstance(e["msg"], dict) and "id" in e["msg"]}
        for e in lines(recorder):
            msg = e["msg"]
            if e["dir"] == "in" and isinstance(msg, dict) and msg.get("method") == "tools/call":
                answer = answers.get(msg["id"], {})
                text, png = result_parts((answer.get("result") or {}).get("content") or answer.get("error", {}).get("message", ""))
                calls.append({"kind": "call", "name": GRAFTA + msg["params"]["name"], "args": msg["params"].get("arguments", {}), "text": text, "png": png})
    tools = [t for t in convo["turns"] if t["kind"] == "tool"]
    grafta_tools = [t for t in tools if not t.get("other")]
    if len(grafta_tools) != len(calls):
        raise Stop(f"{conversation.name}: the conversation shows {len(grafta_tools)} grafta calls, grafta answered {len(calls)}")
    events, pending = [], iter(calls)
    for turn in convo["turns"]:
        if turn["kind"] in ("user", "reply"):
            events.append({"kind": turn["kind"], "text": turn["text"]})
        elif not turn.get("other"):
            call = next(pending)
            if turn.get("name") and turn["name"].removeprefix(GRAFTA) != call["name"][len(GRAFTA):]:
                raise Stop(f"{conversation.name}: the conversation calls {turn['name']} where grafta answered {call['name']}")
            events.append(call)
        else:
            events.append({"kind": "call", "name": turn.get("name", "another tool"), "args": {}, "text": "", "png": None})
    meta = {k: convo.get(k) for k in ("model", "clientVersion", "recordedOn", "cwd")}
    return events, meta


# ---- the three edits ----------------------------------------------------


def strip_paths(value, cwd):
    """The working directory dropped from paths, and the repository's (where the
    showcase's inputs were made, as pricing.json's snapshot names its page);
    any other path under the recorder's home folder written from ~."""
    if isinstance(value, dict):
        return {k: strip_paths(v, cwd) for k, v in value.items()}
    if isinstance(value, list):
        return [strip_paths(v, cwd) for v in value]
    if isinstance(value, str):
        for prefix in filter(None, (cwd, str(ROOT))):
            value = value.replace("file://" + prefix.rstrip("/") + "/", "").replace(prefix.rstrip("/") + "/", "")
            value = value.replace(prefix.rstrip("/"), ".")
        # The folders above the working directory, where a model wrote beside it
        # or looked around, as relative paths: three levels, the session's folder
        # and the work folder scripts/record-code.sh makes.
        if cwd:
            home = pathlib.PurePosixPath(pathlib.Path.home())
            for depth, parent in enumerate(list(pathlib.PurePosixPath(cwd).parents)[:3], 1):
                # Never a home folder, one of its parents, or a folder two levels
                # from the root (/Users/someone): a path there would hide, not drop.
                if len(parent.parts) <= 3 or parent == home or parent in home.parents:
                    break
                up = "/".join([".."] * depth)
                value = value.replace(str(parent) + "/", up + "/").replace(str(parent), up)
        # What is left under the recorder's home folder (a parent of the working directory) as ~.
        return value.replace(str(pathlib.Path.home()), "~")
    return value


def leftover_paths(recording):
    """Paths from the recorder's machine still in a recording, which the site must not show."""
    text = json.dumps(recording, ensure_ascii=False)
    user = pathlib.Path.home().name
    named = [f"the user name {user!r}"] if user and re.search(rf"\b{re.escape(user)}\b", text) else []
    return named + sorted({m for m in re.findall(r"(?:/Users/|/home/|/private/|[A-Z]:\\\\Users)[^\s\"'\]]{0,40}", text)})


def excerpt(text):
    """A result as the site shows it: compact, cut at LIMIT, grafta's omitted kept."""
    omitted = None
    try:
        parsed = json.loads(text)
        omitted = parsed.get("omitted") if isinstance(parsed, dict) else None
        compact = json.dumps(parsed, ensure_ascii=False, separators=(", ", ": "))
    except ValueError:
        compact = text
    step, notes = {"result": compact[:LIMIT]}, []
    if omitted:
        notes.append(f"grafta: {json.dumps(omitted, ensure_ascii=False)}")
    if len(compact) > LIMIT:
        notes.append(f"…cut here: {len(compact) - LIMIT} more characters")
    if notes:
        step["omitted"] = "; ".join(notes)
    return step, len(compact) > LIMIT


def png_size(png):
    return struct.unpack(">II", png[16:24])


# ---- building turns ----------------------------------------------------------


def grafta_calls(events):
    return [e for e in events if e["kind"] == "call" and e["name"].startswith(GRAFTA)]


def build(spec, client, events, meta, rows, pictures, prefix):
    """Events → the recording the site reads, checked."""
    sid = spec["id"]
    where = f"{sid} in {client}"
    users = [e for e in events if e["kind"] == "user"]
    if not users or users[0]["text"].strip() != spec["prompt"].strip():
        raise Stop(f"{where}: the first message is not the prompt in sessions.json")
    beat_ids = [b["id"] for b in spec.get("beats", [])]
    for row in rows:
        if row["beat"] not in beat_ids:
            raise Stop(f"{where}: beats.json names beat {row['beat']!r}, which the session does not have")
    cwd, cut, others, n, picture_n, relinked, shown = meta.get("cwd"), 0, 0, 0, 0, 0, []
    turns, last_beat = [], beat_ids[0] if beat_ids else ""

    def row_of(index):
        for row in rows:
            if index <= row["to"]:
                return row
        raise Stop(f"{where}: grafta call {index} is past the last row of beats.json")

    for event in events:
        if event["kind"] in ("user", "reply"):
            text, changed = relink(event["text"], shown)
            relinked += changed
            turns.append({"kind": event["kind"], "text": strip_paths(text, cwd)})
            continue
        if not event["name"].startswith(GRAFTA):
            # The client's own tools: the call shown, its result left out and said so: it is
            # not grafta's, and it carries the recorder's machine (a directory listing names its owner).
            others += 1
            step = {"tool": event["name"], "args": strip_paths(event["args"], cwd), "result": "",
                    "omitted": "the client's own tool: its result is not kept"}
            if turns and turns[-1]["kind"] == "tools" and turns[-1].get("other"):
                turns[-1]["calls"].append(step)
            else:
                turns.append({"kind": "tools", "label": "Used another tool", "beat": last_beat, "other": True,
                              "calls": [step]})
            continue
        n += 1
        row = row_of(n)
        step, was_cut = excerpt(strip_paths(event["text"], cwd))
        cut += was_cut
        step = {"tool": event["name"][len(GRAFTA):], "args": strip_paths(event["args"], cwd), **step}
        if event["png"]:
            picture_n += 1
            name = f"{prefix}-{picture_n}.png"
            pictures[name] = event["png"]
            width, height = png_size(event["png"])
            shown.append((event.get("at"), f"showcase/sessions/{name}"))
            step["picture"] = {"src": f"showcase/sessions/{name}", "width": width, "height": height,
                               "alt": f"{spec['title']}: what grafta drew at “{row['label']}”"}
        last = turns[-1] if turns else None
        if last and last["kind"] == "tools" and not last.get("other") and last["_row"] is row:
            last["calls"].append(step)
        else:
            turns.append({"kind": "tools", "label": row["label"], "beat": row["beat"], "calls": [step], "_row": row})
        last_beat = row["beat"]
    for t in turns:
        t.pop("_row", None)
    if turns and turns[-1]["kind"] != "reply":
        raise Stop(f"{where}: the conversation does not end with a reply")

    # Each beat reached, with the tools it needs.
    for beat in spec.get("beats", []):
        used = {c["tool"] for t in turns if t["kind"] == "tools" and t["beat"] == beat["id"] and not t.get("other") for c in t["calls"]}
        if not used:
            raise Stop(f"{where}: no grafta call reaches beat {beat['id']!r}")
        for need in beat["needs"]:
            if not used & set(need.split("|")):
                raise Stop(f"{where}: beat {beat['id']!r} needs {need}, its calls are {sorted(used)}")
    saves = [c for t in turns if t["kind"] == "tools" for c in t["calls"] if c["tool"] == "save_as"]
    wrote = saves[-1]["args"].get("path") if saves else None
    if spec.get("beats") and bool(wrote) != spec["outcome"]["writes"]:
        raise Stop(f"{where}: sessions.json says it {'writes' if spec['outcome']['writes'] else 'writes no'} file, "
                   f"the recording {'wrote ' + repr(wrote) if wrote else 'wrote none'}")

    recording = {
        "id": sid,
        "client": client,
        "clientVersion": meta.get("clientVersion") or "",
        "model": meta.get("model") or "",
        "recordedOn": meta.get("recordedOn") or datetime.date.today().isoformat(),
        "file": wrote,
        "edits": {"paths": bool(cwd), "cut": cut, "otherTools": others, "pictures": relinked},
        "turns": turns,
    }
    left = leftover_paths(recording)
    if left:
        raise Stop(f"{where}: paths from the recording's machine are left: {left[:3]}")
    return recording


# ---- entry points ------------------------------------------------------------


def recorded(rec, sid, client):
    """Claude Desktop is optional until it is recorded (maintainer, 2026-10-31)."""
    if client == "claude-code":
        return (rec / f"{sid}.code.jsonl").exists()
    return (rec / f"{sid}.desktop.json").exists()


def load(rec, sid, client):
    if client == "claude-code":
        return read_code(rec / f"{sid}.code.jsonl")
    return read_desktop(rec / f"{sid}.desktop.json", rec / f"{sid}.mcp.jsonl")


def run(rec, web, beats_path):
    specs = json.loads((web / "src" / "content" / "sessions.json").read_text(encoding="utf-8"))
    beats = json.loads(beats_path.read_text(encoding="utf-8"))
    out_dir, pictures_dir = web / "public" / "sessions", web / "public" / "showcase" / "sessions"
    written, pictures = {}, {}
    for spec in specs:
        clients = [c for c in CLIENTS if recorded(rec, spec["id"], c)]
        if "claude-code" not in clients:
            raise Stop(f"{spec['id']}: not recorded in Claude Code ({spec['id']}.code.jsonl)")
        for client in clients:
            rows = (beats.get(spec["id"]) or {}).get(client)
            if not rows:
                raise Stop(f"{spec['id']} in {client}: no rows in beats.json (write them with --list)")
            events, meta = load(rec, spec["id"], client)
            written[f"{spec['id']}.{client}"] = build(spec, client, events, meta, rows, pictures, f"{spec['id']}-{client}")
    theme = next(s for s in specs if s["id"] == "theme")
    for client, path in (("claude-desktop", rec / "theme-without.desktop.json"), ("claude-code", rec / "theme-without.code.jsonl")):
        if not path.exists():
            continue
        events, meta = read_desktop(path, None) if client == "claude-desktop" else read_code(path)
        if grafta_calls(events):
            raise Stop(f"theme-without in {client}: grafta was called; it was meant to be recorded without grafta")
        written[f"theme-without.{client}"] = build({**theme, "id": "theme-without", "beats": []}, client, events, meta,
                                                   [], pictures, f"theme-without-{client}")
    # The hero's conversation (--talk) is written apart, and kept.
    if out_dir.exists():
        for old in out_dir.glob("*.json"):
            if not old.name.startswith(f"{TALK}."):
                old.unlink()
    out_dir.mkdir(parents=True, exist_ok=True)
    pictures_dir.mkdir(parents=True, exist_ok=True)
    for old in pictures_dir.glob("*.png"):
        if not old.name.startswith(f"{TALK}-"):
            old.unlink()
    for name, png in pictures.items():
        (pictures_dir / name).write_bytes(png)
    for name, recording in written.items():
        (out_dir / f"{name}.json").write_text(json.dumps(recording, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    index = {}
    for name in written:
        sid, client = name.rsplit(".", 1)
        index.setdefault(sid, []).append(client)
    (web / "src" / "generated").mkdir(parents=True, exist_ok=True)
    (web / "src" / "generated" / "session-index.json").write_text(json.dumps(index, indent=1) + "\n", encoding="utf-8")
    return written, pictures


TALK = "talk"


def run_talk(rec, web, beats_path):
    """The hero's conversation: one session, its prompt and follow-ups, recorded in
    Claude Code, written to web/public/sessions/talk.claude-code.json."""
    spec = json.loads((web / "src" / "content" / "talk.json").read_text(encoding="utf-8"))
    rows = (json.loads(beats_path.read_text(encoding="utf-8")).get(spec["id"]) or {}).get("claude-code")
    if not rows:
        raise Stop(f"{spec['id']}: no rows in beats.json (write them with --list)")
    events, meta = load(rec, spec["id"], "claude-code")
    said = [e["text"].strip() for e in events if e["kind"] == "user"]
    meant = [spec["prompt"].strip(), *(f.strip() for f in spec.get("followUps", []))]
    if said != meant:
        raise Stop(f"{spec['id']}: the messages sent are {said}, talk.json says {meant}")
    pictures = {}
    recording = build(spec, "claude-code", events, meta, rows, pictures, f"{spec['id']}-claude-code")
    out_dir, pictures_dir = web / "public" / "sessions", web / "public" / "showcase" / "sessions"
    out_dir.mkdir(parents=True, exist_ok=True)
    pictures_dir.mkdir(parents=True, exist_ok=True)
    for old in pictures_dir.glob(f"{TALK}-*.png"):
        old.unlink()
    for name, png in pictures.items():
        (pictures_dir / name).write_bytes(png)
    (out_dir / f"{spec['id']}.claude-code.json").write_text(
        json.dumps(recording, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    return recording, pictures


def main(argv):
    if len(argv) == 4 and argv[1] == "--talk":
        try:
            recording, pictures = run_talk(pathlib.Path(argv[2]), pathlib.Path(argv[3]), ROOT / "docs" / "showcase" / "beats.json")
        except Stop as stop:
            sys.stderr.write(f"stopped: {stop}\n")
            return 1
        print(f"talk: {len(recording['turns'])} turns, {len(pictures)} pictures")
        return 0
    if len(argv) == 5 and argv[1] == "--list":
        rec, sid, client = pathlib.Path(argv[2]), argv[3], argv[4]
        events, _ = load(rec, sid, client)
        for i, call in enumerate(grafta_calls(events), 1):
            print(i, call["name"][len(GRAFTA):], json.dumps(call["args"], ensure_ascii=False)[:100])
        return 0
    if len(argv) != 3:
        sys.stderr.write(__doc__)
        return 2
    try:
        written, pictures = run(pathlib.Path(argv[1]), pathlib.Path(argv[2]), ROOT / "docs" / "showcase" / "beats.json")
    except Stop as stop:
        sys.stderr.write(f"stopped: {stop}\n")
        return 1
    print(f"{len(written)} recordings, {len(pictures)} pictures")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
