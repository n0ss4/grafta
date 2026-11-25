# From a terminal

The same tools, without a model:

```bash
grafta tool                                     # list the tools
grafta tool read_node                           # what one takes, with an example
grafta tool open_file '{"path":"design.fig"}'   # call one: each call starts afresh
grafta tool --batch calls.json                  # a sequence on one open file
```

A batch is a JSON array of `{ "tool", "args", "out"? }`; `"$0.handle"` in an
argument is call 0's result field, and `"out"` writes a picture to a file. The
[showcase](../showcase/showcase.py) is one.
