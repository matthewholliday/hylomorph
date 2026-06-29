# Contributing to harness

Thanks for your interest in contributing. This document covers how to set up,
build, test, and submit changes.

`harness` is a project-agnostic Rust CLI that runs a coding agent as a *Ralph loop*
behind deterministic validation gates. The canonical and only product is the Cargo
workspace under [`harness/`](harness/). See [AGENTS.md](AGENTS.md) for a quick
orientation and [README.md](README.md) for what the tool does.

> **Status: pre-1.0 (0.x).** The CLI surface and on-disk formats (`.specs/`,
> `.harness/`, the JSON/JSONL layer schemas) are not yet stable and may change
> between 0.x releases.

## Prerequisites

- A stable Rust toolchain (install via [rustup](https://rustup.rs)) with the
  `rustfmt` and `clippy` components.
- `git`.

```sh
rustup component add rustfmt clippy
```

## Build, test, lint

All commands run from the `harness/` workspace directory:

```sh
cd harness
cargo build --locked
cargo test  --locked
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
```

CI runs exactly these four checks (with `RUSTFLAGS=-D warnings`). **Please run them
locally before opening a PR** — a change that doesn't pass all four will fail CI.

To build and install `harness` (and the optional `harness-gui`) onto your PATH:

```sh
scripts/build-and-reinstall.sh            # both binaries
scripts/build-and-reinstall.sh --cli-only # CLI only
```

## Project layout

```
harness/crates/harness-core/   library: the loop, gates, spec layers, state, config
harness/crates/harness-cli/    the `harness` binary + embedded prompt templates
harness/crates/harness-gui/    optional egui desktop front-end
scripts/                       build/install helpers
```

`sandbox/` is a local dogfooding project and is git-ignored; don't commit it.

## Making changes

- **Keep changes focused.** Make the smallest change that solves the problem; avoid
  drive-by reformatting or unrelated refactors.
- **Match the surrounding style.** Keep `cargo fmt` and `clippy` clean; introduce no
  new warnings.
- **Prefer recoverable errors.** Use `anyhow` errors with context rather than
  `unwrap()/expect()/panic!` on any user-reachable path.
- **Add tests** for new behavior, especially around the loop, the layer-gating state
  machine, write-scoping/revert logic, and hook execution.
- **Flag format changes.** If you change an on-disk format or the CLI surface, say so
  explicitly in the PR and update `README.md` and `spec-as-source-dev-guide.md`.

## Commit and PR process

1. Branch off `main` (don't push directly to `main`).
2. Keep commits logical and write clear messages explaining the *why*.
3. Ensure `fmt`, `clippy`, `build`, and `test` all pass locally.
4. Open a PR against `main` with a description of the change and its motivation.
   Link any related issue.
5. A maintainer will review. Address feedback by pushing follow-up commits.

### Developer Certificate of Origin (DCO)

By contributing, you certify that you wrote the change or otherwise have the right to
submit it under the project's license (see [LICENSE](LICENSE)). Sign off your commits:

```sh
git commit -s -m "Your message"
```

This appends a `Signed-off-by:` line asserting the [DCO](https://developercertificate.org/).

## Reporting bugs and security issues

- **Bugs / features:** open a GitHub issue with steps to reproduce and the output of
  `harness --version`.
- **Security vulnerabilities:** please do **not** open a public issue. Until a
  `SECURITY.md` reporting channel is published, contact the maintainers privately.

## License

By contributing, you agree that your contributions are licensed under the same terms
as the project (see [LICENSE](LICENSE)).
