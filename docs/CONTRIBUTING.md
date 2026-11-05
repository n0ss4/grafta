# Working on grafta

Start with [ARCHITECTURE.md](ARCHITECTURE.md) for how the code is laid out, and
[ROADMAP.md](ROADMAP.md) for where it is going.

## Build and test

```bash
cargo build --release                         # target/release/grafta
cargo test --workspace
cargo clippy --workspace --all-targets
```

Both must pass before a change is proposed, on a clone with no fixtures. CI runs
them on Linux, macOS and Windows.

Unit tests sit beside what they test as sidecar `tests.rs` files, so no source
file is half implementation and half test. Integration tests are in
`crates/*/tests/`. Shared scene graphs live in `testkit`.

## Fixtures

Tests that need a real `.fig` read it from `fixtures/` and **skip themselves
when it is empty**, which it is in a fresh clone: the files are large, and most
are not ours to redistribute. Drop any `.fig` in, or point elsewhere:

```bash
GRAFTA_FIXTURES=~/designs cargo test --workspace
```

## Ladders

A write is proven the way Figma decides: a ladder of files, opened by a person.
A ladder is a `grafta tool --batch` file whose rungs are written to `ladder/`
(not committed):

```bash
grafta tool --batch my-ladder.json
```

A batch is a JSON array of `{ "tool", "args", "out"? }`; `"$N.field"` in an
argument is call N's result field. `grafta tool` and batches keep no journal,
so a ladder produces the same file every time.

## The showcase and the brand

The README's pictures and the site's social card are drawn by grafta:
`docs/showcase/showcase.py` and `docs/showcase/card.py` write batches, and

```bash
scripts/showcase.sh path/to/any.fig
```

runs them and writes `docs/assets/showcase-*.png` and `docs/assets/card.png`
(and `ladder/showcase.fig` and `ladder/card.fig`, to open in Figma).
The `.fig` lends only its format. Inter and Inter Display must be installed.

## The website and the tool reference

`web/` is the website: React and TypeScript, every page pre-rendered to HTML
at build time, published by `.github/workflows/pages.yml`. How it is laid out
and the commands to work on it are in [web/README.md](../web/README.md).

What grafta produces is generated, never typed in, and checked:

```bash
python3 scripts/tool-reference.py target/release/grafta   # docs/TOOLS.md and web/src/generated/tools.json; CI checks them
scripts/sessions.sh --inputs <dir> path/to/any.fig         # the files the site's sessions are recorded on (needs Chrome)
scripts/record-code.sh <work dir> docs/showcase/recordings/075   # the sessions, recorded in Claude Code
python3 docs/showcase/transcripts_site.py docs/showcase/recordings/075 web   # the recordings into the site
```

## Branches and commits

```
feat/… fix/… docs/… → develop → main → v* tag
```

Work goes on a branch named for its type, with a pull request into `develop`, the
default branch. `main` is what is released: it moves only when `develop` is merged
into it, and by the release pull request.

Commit messages and pull request titles follow
[Conventional Commits](https://www.conventionalcommits.org/). A merge commit, not a
squash, keeps each commit's type for the version. `conventions.yml` checks all of this.

| Type | Next version | CI |
|---|---|---|
| `feat` | minor: 0.1.0 → 0.2.0 | runs |
| `fix`, `perf` | patch: 0.1.0 → 0.1.1 | runs |
| `feat!`, or a `BREAKING CHANGE:` footer | minor before 1.0, major after | runs |
| `docs`, `chore`, `ci`, `refactor`, `test`, `build`, `style` | none | runs, unless only prose changed |

A commit that touches nothing but `web/` or `docs/` makes no release, whatever its
type: the website deploys from `main` by itself. CI skips a change to nothing but
the prose listed in `ci.yml`; `ci` is the one check to require.

## Releasing

One version names every channel; `Cargo.toml`'s `[workspace.package]` is the source.

Each push to `main` runs `.github/workflows/release-please.yml`. If it holds a `feat`
or a `fix`, the workflow opens or updates a release pull request into `main`, with
the next version in every channel's file (`scripts/set-version.sh`), `Cargo.lock` and
`CHANGELOG.md`. Nothing is released until that pull request is merged. To force a
version, add a `Release-As: 1.0.0` footer to a commit.

It needs the secret `RELEASE_TOKEN`: a fine-grained token of a repository admin, with
Contents and Pull requests read and write on this repository. Only admins can create
release tags, and what the default token does starts no other workflow. `main` is
merged back into `develop` after every push, by the same workflow.

Merging it tags the version and leaves a draft release. `.github/workflows/release.yml`
then checks the version against the tag, builds
the five binaries, publishes the GitHub release with `SHA256SUMS`, and runs each
channel whose secret is set:

| Secret | Channel |
|---|---|
| `NPM_TOKEN` | npm: `grafta` and `grafta-<os>-<cpu>` |
| `HOMEBREW_TAP_TOKEN` | a token that can push to `n0ss4/homebrew-tap` |
| `CARGO_REGISTRY_TOKEN` | crates.io: `grafta-kiwi` … `grafta` |

The install scripts and the Claude Code plugin need no step: they read the
latest release and the repository.

The release also attaches `grafta.mcpb`, the Claude Desktop bundle: a
universal macOS binary (`lipo` of both CPUs) and the Windows one. Its version
and tools are asked of the binary itself, so nothing is kept in step by hand.
To build one locally, with any binary that runs here standing in for both:

```bash
python3 scripts/mcpb.py target/release/grafta target/release/grafta grafta.mcpb
```

## Code that was removed

The semantic IR, the spec bundle and its code generators, `audit`, the TUI and
the developer commands (`synth`, `roundtrip`, `info`, `tree`, `variables`,
`tokens`, `targets`) were removed in 042. They are under the tag
`legacy-pre-toolkit`:

```bash
git worktree add ../grafta-legacy legacy-pre-toolkit
```
