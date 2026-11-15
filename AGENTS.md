# grafta

Read and write Figma files, exactly and offline, as an MCP server in Rust, with a
website in `web/`. Layout: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). The fuller
guide this summarises: [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md).

## Before proposing a change

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Tests that need a real `.fig` skip themselves when `fixtures/` is empty, so a pass on
a clean clone does not cover them. If you have fixtures, also run with
`GRAFTA_FIXTURES=<dir>`.

If a tool's description or schema changes, regenerate what is kept from it:

```bash
GRAFTA_RECORD=1 cargo test -p grafta-toolkit --test golden the_tool_list_is_as_recorded
GRAFTA_RECORD=1 cargo test -p grafta-mcp the_plugin_s_contract_is_as_recorded   # figma-plugin/src/contract.json
python3 scripts/tool-reference.py target/release/grafta
```

The same for a tool's `live` (what it does on a file open in Figma). grafta's
panel in Figma is `figma-plugin/` (TypeScript, Node 22): `npm run typecheck`
and `npm run build` there; the build fails when a tool the contract says acts
on the open file has no function in the panel.

## Branches, commits and versions

- Branch from `develop` and open pull requests into `develop`. Never into `main`,
  except `develop` itself and the release pull request.
- Name branches `<type>/<short-description>`: `feat/…`, `fix/…`, `docs/…`.
- Write pull request titles as Conventional Commits. The version comes from them:
  `feat` is a minor release, `fix` and `perf` a patch, and the other types none.
- Squash pull requests into `develop`. Merge `develop` into `main` with a merge commit.
- Do not edit `CHANGELOG.md` or the version in any file by hand: the release pull
  request writes both.
- release-please skips commits that only touch `.claude/`, `docs/`, `video/` or
  `web/`: the website is not a release. It can skip folders only, so a pull request
  that also touches a file outside them (the README, a script) counts: title it
  `docs:` or `chore:` unless it changes grafta itself. To recount a merged one, put
  `BEGIN_COMMIT_OVERRIDE`, the new title and `END_COMMIT_OVERRIDE` in its
  description.
