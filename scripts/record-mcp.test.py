#!/usr/bin/env python3
"""Tests for scripts/record-mcp.py.  python3 scripts/record-mcp.test.py"""
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

RECORDER = pathlib.Path(__file__).with_name("record-mcp.py")
# A server that answers each request with its id, and says something on stderr.
ECHO = (
    "import json,sys\n"
    "sys.stderr.write('ready\\n')\n"
    "for line in sys.stdin:\n"
    "    req = json.loads(line)\n"
    "    print(json.dumps({'jsonrpc': '2.0', 'id': req['id'], 'result': {'content': [{'type': 'text', 'text': 'é'}]}}), flush=True)\n"
)


class RecordMcp(unittest.TestCase):
    def run_recorder(self, requests):
        with tempfile.TemporaryDirectory() as work:
            out = pathlib.Path(work) / "out.jsonl"
            done = subprocess.run(
                [sys.executable, str(RECORDER), str(out), "--", sys.executable, "-c", ECHO],
                input="".join(json.dumps(r) + "\n" for r in requests).encode(),
                capture_output=True,
                timeout=20,
            )
            return done, [json.loads(line) for line in out.read_text(encoding="utf-8").splitlines()]

    def test_passes_lines_through_unchanged_and_records_both_ways(self):
        call = {"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": "open_file", "arguments": {"path": "order.fig"}}}
        done, log = self.run_recorder([call])
        self.assertEqual(done.returncode, 0)
        answer = json.loads(done.stdout)
        self.assertEqual(answer["id"], 7)
        self.assertEqual(answer["result"]["content"][0]["text"], "é")
        self.assertEqual([e["dir"] for e in log], ["in", "out"])
        self.assertEqual(log[0]["msg"], call)
        self.assertEqual(log[1]["msg"], answer)
        self.assertTrue(all("t" in e for e in log))
        self.assertIn(b"ready", done.stderr)

    def test_keeps_order_over_several_calls(self):
        done, log = self.run_recorder([{"jsonrpc": "2.0", "id": i, "method": "ping"} for i in range(5)])
        self.assertEqual(done.returncode, 0)
        ins = [e["msg"]["id"] for e in log if e["dir"] == "in"]
        outs = [e["msg"]["id"] for e in log if e["dir"] == "out"]
        self.assertEqual(ins, list(range(5)))
        self.assertEqual(outs, list(range(5)))

    def test_refuses_without_a_command(self):
        done = subprocess.run([sys.executable, str(RECORDER), "x.jsonl"], capture_output=True)
        self.assertEqual(done.returncode, 2)


if __name__ == "__main__":
    unittest.main()
