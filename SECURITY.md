# Security

## Reporting a vulnerability

Please do not open a public issue. Report it privately through GitHub:
**Security → Report a vulnerability** on
[n0ss4/grafta](https://github.com/n0ss4/grafta/security/advisories/new).
Include the grafta version (`grafta --version`), your platform, and the
smallest file or tool call that shows the problem. A file you cannot share can
be described instead.

The report is acknowledged and fixed in private, released as soon as the fix
is ready, and credited to you unless you ask otherwise.

## Supported versions

Only the latest release gets security fixes.

## What grafta trusts

grafta reads files a person names, and those files may come from anyone. It is
in scope when a file or a tool call can make grafta:

- run code, or read or write a path the call did not name;
- overwrite a file (`save_as` and `new_file` refuse an existing path);
- hang or exhaust memory beyond what the file's size explains;
- make a network call (it makes none).

grafta keeps unsaved edits in `~/.grafta/journals`, with the permissions your
system gives new files. What a model reads through grafta is
sent wherever its MCP client sends the conversation; that is the client's
policy, not grafta's.
