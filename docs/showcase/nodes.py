#!/usr/bin/env python3
"""The home page's "Every node" section: what read_node answers for the order
card's layers, as grafta answers it.

    python3 docs/showcase/nodes.py batch > nodes.json     # in the sessions' work dir
    grafta tool --batch nodes.json > nodes.out
    python3 docs/showcase/nodes.py site nodes.out web/src/generated/nodes.json

The batch opens order.fig (made by scripts/sessions.sh's setup), finds each
layer by name and reads it alone (depth 0).
"""
import json
import sys

LAYERS = [  # (name as search_nodes finds it, type, what the site calls it)
    ("Order", "FRAME", "Order"),
    ("My order", "TEXT", "Your order"),
    ("Card", "FRAME", "Card"),
    ("Package (Lucide)", "VECTOR", "Package"),
    ("Done", "FRAME", "Progress"),
    ("Button", "FRAME", "Button"),
    ("Track order", "TEXT", "Track order"),
]


def batch():
    calls = [{"tool": "open_file", "args": {"path": "order.fig"}}]
    for name, type_, _ in LAYERS:
        calls.append({"tool": "search_nodes", "args": {"handle": "$0.handle", "name": name, "type": type_}})
        calls.append({"tool": "read_node", "args": {"handle": "$0.handle", "id": f"${len(calls) - 1}.results.0.id", "depth": 0}})
    print(json.dumps(calls))


def site(out, dest):
    results = [json.loads(line.split(" → ", 1)[1]) for line in open(out) if " → " in line]
    reads = results[2::2]
    nodes = [{"layer": label, "node": node} for (_, _, label), node in zip(LAYERS, reads)]
    with open(dest, "w") as f:
        json.dump(nodes, f, indent=1, ensure_ascii=False)
        f.write("\n")


if __name__ == "__main__":
    batch() if sys.argv[1] == "batch" else site(sys.argv[2], sys.argv[3])
