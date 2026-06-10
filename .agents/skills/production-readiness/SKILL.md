---
name: production-readiness
description: Final quality gate for Rust4D feature branches. Use before PRs, releases, or after broad engine changes. Covers formatting, tests, clippy, rustdoc, visual captures, docs, and scratchpad handoff.
---

# Production Readiness Gate

Run this before opening a PR or calling a feature branch done.

## Code gate

```bash
nix develop --command cargo fmt --all -- --check
nix develop --command cargo clippy --workspace --all-targets -- -D warnings
nix develop --command bash -c 'RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps'
nix develop --command cargo test --workspace
nix develop --command cargo test --test slice_invariant
```

No warnings. No ignored failures unless already documented.

## Visual gate

For rendering-affecting work:

```bash
nix develop --command cargo run --example shape_showcase .scratchpad/captures-gallery
nix develop --command cargo run --example headless_protocol .scratchpad/captures
```

Inspect at least one contact sheet or representative PNGs. Do not ask Willow to
verify visually.

## Docs gate

- README feature list updated
- `docs/README.md` links any new doc
- `docs/developer-guide.md` updated for workflows/architecture changes
- `docs/4d-math.md` updated for math/convention changes
- `docs/shapes.md` updated for primitive changes
- `.agents/skills/*` updated if a workflow changed
- Rustdoc has no broken intra-doc links (`RUSTDOCFLAGS=-Dwarnings` catches this)

## Scratchpad gate

- Update the active plan with completed waves and next steps
- Update `scratchpad/board.md` with correct `# ` column names
- Write a short report if ending a substantial session
- Commit and push scratchpad separately from repo code

## PR gate

PR body should include:
- Summary
- Why it matters / root cause if a fix
- Verification commands and visual evidence
- Known follow-ups

Mention exact test counts and capture counts where relevant.
