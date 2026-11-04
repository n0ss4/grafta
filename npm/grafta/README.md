# grafta

Figma's file format for your agent: an MCP server that lets a model read and
write `.fig` files, exactly and offline.

```bash
npx -y grafta mcp                          # start the server on stdio
claude mcp add grafta -- npx -y grafta mcp # register it with Claude Code
```

This package runs the prebuilt binary for your platform (macOS, Linux, Windows
on x64; macOS and Linux on arm64), installed as an optional dependency. Nothing
is downloaded at run time.

Documentation: https://github.com/n0ss4/grafta
