# Fixtures

Real `.fig` files, for the tests that only a real file can exercise: the ZIP and
`fig-kiwi` container layers, the Kiwi schema Figma ships inside each file, and
the shape of `nodeChanges` in the wild.

The directory is empty in a fresh clone — these files are large, and most of them
are not ours to redistribute. Every fixture-backed test therefore **skips itself
when the file is absent**, so `cargo test --workspace` is green either way.

## Adding one

Drop any `.fig` file in here:

```
fixtures/
└── your-design.fig
```

The suites that iterate `testkit::fixture::all()` will pick it up on the next
run — there is no list to register it in. Nothing here is committed
(see `.gitignore`); a fixture is something you keep locally or fetch in CI.

To read them from somewhere else instead:

```bash
GRAFTA_FIXTURES=~/designs cargo test --workspace
```

## Control files

Tests that only exercise the file the goldens were written from cannot tell a
rule about design files from a habit of one designer. `crates/generate/ir` keeps
a set of controls for that: files from other design systems, in another
language, by other authors.

```
material3.fig  nuxtui.fig  gold-preview.fig
```

They come from [`open-pencil`](https://github.com/)'s own test fixtures
(`open-pencil/tests/fixtures/`). Like everything here they are not committed, and
the tests that use them skip when they are absent.

## Using one in a test

```rust
let Some((fig, graph)) = testkit::fixture::graph("your-design.fig") else { return };
```

`testkit::fixture` also exposes `bytes`, `parsed` and `all`, and treats an
unfetched Git LFS pointer the same as a missing file.
