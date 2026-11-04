#!/usr/bin/env python3
"""Record what an MCP client and server say to each other.

    python3 scripts/record-mcp.py <out.jsonl> -- grafta mcp

Runs the command after `--` as the server, passes every line from the client
to it and every line it answers back, unchanged, and appends each line to
<out.jsonl> as {"t": <ISO time>, "dir": "in" | "out", "msg": <the line,
parsed>}: "in" is client to server. The server's stderr passes through; the
exit code is the server's. Register it in a client in place of the server's
own command, for a recording only.
"""
import datetime
import json
import subprocess
import sys
import threading


def record(log, lock, direction, line):
    try:
        msg = json.loads(line)
    except ValueError:
        msg = line.decode("utf-8", "replace").rstrip("\n")
    entry = {"t": datetime.datetime.now(datetime.timezone.utc).isoformat(), "dir": direction, "msg": msg}
    with lock:
        log.write(json.dumps(entry, ensure_ascii=False) + "\n")
        log.flush()


def pump(source, sink, log, lock, direction):
    for line in iter(source.readline, b""):
        sink.write(line)
        sink.flush()
        if line.strip():
            record(log, lock, direction, line)
    try:
        sink.close()
    except OSError:
        pass


def main(argv):
    if len(argv) < 4 or argv[2] != "--":
        sys.stderr.write("usage: record-mcp.py <out.jsonl> -- <command...>\n")
        return 2
    out, command = argv[1], argv[3:]
    lock = threading.Lock()
    with open(out, "a", encoding="utf-8") as log:
        server = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        to_server = threading.Thread(
            target=pump, args=(sys.stdin.buffer, server.stdin, log, lock, "in"), daemon=True
        )
        to_server.start()
        pump(server.stdout, sys.stdout.buffer, log, lock, "out")
        return server.wait()


if __name__ == "__main__":
    sys.exit(main(sys.argv))
