# AGENTS.md

Guidance for AI coding agents (and humans) working in this repository.

## What this repo is

`harness` is a **project-agnostic Rust CLI** that drives a coding agent as a
*Ralph loop* with deterministic, blocking validation gates ("hooks"). It enforces a
gated five-layer spec pipeline: `requirements → design → tasks → code → evals`.

The **canonical and only product is the Cargo workspace under `harness/`.** There is
no Python component and no second crate — earlier orchestration code was removed.

## Layout

```
harness/                      # the Cargo workspace — all real code lives here
  crates/harness-core/        # library: config, hooks, layers, loop_runner,
                              #   manifest, prompt, scope, snapshot, spec, state, util
  crates/harness-cli/         # the `harness` binary (main.rs, tui.rs, templates/)
  crates/harness-gui/         # optional egui desktop front-end (`harness-gui`)
scripts/                      # build-and-reinstall.sh, reinstall-harness.sh
new-spec.sh                   # convenience wrapper around `harness spec new`
spec-as-source-dev-guide.md   # narrative guide to the spec-as-source workflow
README.md                     # user-facing docs
upgrades.md                   # production-readiness roadmap
```

`sandbox/` is a local dogfooding project and is git-ignored — never commit it.

## Build / test / lint (run from `harness/`)

```sh
cd harness
cargo build --locked            # build the workspace
cargo test  --locked            # run tests
cargo fmt --all --check         # formatting gate (CI enforces this)
cargo clippy --all-targets --locked -- -D warnings   # lint gate (CI enforces this)
```

CI (`.github/workflows/ci.yml`) runs exactly these four checks with
`RUSTFLAGS=-D warnings`, working-directory `harness`. **A change that doesn't pass
`fmt`, `clippy -D warnings`, `build`, and `test` will fail CI** — run them locally first.

To build + install the binaries onto your PATH: `scripts/build-and-reinstall.sh`.

## Conventions

- **Edit only the canonical workspace** under `harness/`. Do not reintroduce a
  top-level crate, a duplicate `Cargo.toml`/`Cargo.lock`, or a non-Rust product.
- The version is set once in `harness/Cargo.toml` `[workspace.package]` and inherited
  by all crates. We are **pre-1.0 (0.x)**: CLI flags and on-disk formats may change.
  If you change an on-disk format (`.specs/`, `.harness/`, the JSON/JSONL layers),
  call it out explicitly and update the README + `spec-as-source-dev-guide.md`.
- Prefer `anyhow` errors with context over `unwrap()/expect()/panic!` on any
  user-reachable path.
- Keep `cargo fmt` and `clippy` clean; no new warnings.
- CLI prompt/template text lives in `harness/crates/harness-cli/templates/` and is
  embedded via `include_str!`.

## Scope discipline

Make the smallest change that satisfies the task. Don't reformat unrelated code,
bump dependencies opportunistically, or restructure modules unless that *is* the task.
When in doubt about an architectural change, propose it rather than applying it.

See `CONTRIBUTING.md` for the human-facing contribution process.
