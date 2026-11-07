#!/bin/sh
# Which of CI's jobs a change needs, from the files it touches, one per line
# on standard input. Prints rust=, packaging= and web=, each true or false.
#
#   git diff --name-only "$BASE...$HEAD" | scripts/ci-changes.sh
#
# A file is named here by what reads it. One that no rule names runs
# everything, so a new kind of file is checked until it is listed. No file
# at all (what cannot be compared, a new branch's first push) runs
# everything too.
set -eu

rust=false packaging=false web=false all=false seen=false

while IFS= read -r file; do
  [ -n "$file" ] || continue
  seen=true
  case "$file" in
    # Prose nothing builds or checks.
    README.md|AGENTS.md|SECURITY.md|LICENSE|docs/ROADMAP.md|docs/CONTRIBUTING.md|docs/ARCHITECTURE.md|docs/TROUBLESHOOTING.md|.github/ISSUE_TEMPLATE/*|.github/pull_request_template.md|.github/workflows/conventions.yml) ;;
    # CI itself, and what decides it.
    .github/workflows/ci.yml|scripts/ci-changes.sh) all=true ;;
    # The Rust workspace. packaging builds the binary and checks its tool
    # list against docs/TOOLS.md and the site's copy.
    crates/*|Cargo.toml|Cargo.lock|clippy.toml|rust-toolchain*|fixtures/*) rust=true packaging=true ;;
    # The site reads the changelog, the README's pictures and brand, and
    # serves the install scripts.
    install.sh|install.ps1) packaging=true web=true ;;
    web/src/generated/*|web/src/content/*|web/src/components/*|web/src/routes/*) packaging=true web=true ;;
    web/*|docs/assets/*|docs/brand/*|docs/showcase/*|CHANGELOG.md|scripts/snapshot.*|.github/workflows/pages.yml) web=true ;;
    # What ships the binary: npm, the plugins, Homebrew, the registry, and
    # the versions every channel names.
    npm/*|plugin/*|.claude-plugin/*|.agents/*|packaging/*|server.json|docs/TOOLS.md|scripts/*|release-please-config.json|.release-please-manifest.json|.github/workflows/release.yml|.github/workflows/release-please.yml) packaging=true ;;
    *) echo "not named in scripts/ci-changes.sh, so everything runs: $file" >&2; all=true ;;
  esac
done

if [ "$all" = true ] || [ "$seen" = false ]; then
  rust=true packaging=true web=true
fi
printf 'rust=%s\npackaging=%s\nweb=%s\n' "$rust" "$packaging" "$web"
