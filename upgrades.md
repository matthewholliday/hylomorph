# Production-Readiness Plan — Path to an Enterprise Open-Source Release

Scope: what it would take to distribute `harness` (the spec-as-source coding-agent
harness) as a credible open-source tool that an enterprise security/platform team
would adopt. Findings are grounded in the current tree as of this writing.

Severity legend: **P0** = blocks a credible release / will fail enterprise review ·
**P1** = expected of any serious OSS tool · **P2** = polish & maturity.

---

## 0. Repository hygiene & "one source of truth" (P0)

These undermine credibility on first inspection and must be fixed before anything else.

- **Two parallel, divergent codebases.** The root crate `src/` (`src/main.rs`, 848
  lines, `Cargo.toml` v1.0.0) is a *stale fork* of the real workspace at
  `harness/crates/harness-cli/src/main.rs` (2932 lines). They differ substantially.
  CI (`.github/workflows/ci.yml`) only ever builds/tests `harness/`. Decide which is
  canonical (clearly the `harness/` workspace), **delete the orphaned root crate**,
  and remove the duplicate root `Cargo.toml`/`Cargo.lock`.
- **README build instruction builds the wrong binary.** The Quick Start says
  `cargo build --release` from the repo root -> produces the *stale* root binary, not
  the documented `harness spec` CLI (which lives in the workspace). Fix paths so the
  documented build = the shipped build.
- **Loose design docs dumped at root.** `logging_system.txt`, `orchestration_design.txt`,
  `spec-as-source-dev-guide.md`, this `upgrades.md`, `AGENTS.md` — move into a `docs/`
  tree (or a docs site, see §7). `.txt` design notes read as scratch files in a repo
  that wants to be trusted.
- **Two `Cargo.lock` files** (root + `harness/`). Keep exactly one, committed, for the
  canonical workspace.
- **Editor/tool config committed** (`.cursor/`, `.vscode/`, `.DS_Store`). Add `.DS_Store`
  to `.gitignore` globally (currently only ignored at root via `sandbox/` + `/target`);
  decide deliberately whether `.cursor/` and `.vscode/` belong in the public repo.
- **Single, accurate top-level README** that matches the actual workspace layout and
  the single supported entry point.

## 1. Identity, scope & versioning (P0)

- **Resolve the dual-product confusion.** The repo ships *two* agent systems with an
  unclear relationship: the Rust `harness` (project-agnostic Ralph-loop CLI, shells out
  to a generic agent command) and the Python `orchestrations/spec-dev/` system (drives
  the `cursor-agent` binary via `subprocess`). An evaluator can't tell which is "the
  product." Either (a) make one canonical and move the other to `examples/` or a
  separate repo, or (b) document the architecture that ties them together explicitly.
- **Honest versioning.** Crates are tagged `1.0.0` while the README calls the design
  spec "draft v0.1" and the tool is clearly pre-stable. Ship `0.x` until the CLI surface
  and on-disk formats (`.specs/`, `.harness/`, JSON/JSONL schemas) are committed to.
  A `1.0.0` that changes its file formats next week destroys trust faster than `0.4`.
- **Document a stability contract**: which CLI flags, exit codes, and on-disk schemas
  are stable vs. experimental. Enterprises script against these.
- **Pin a vendor-neutral agent abstraction.** Hard-coding `cursor-agent` (Python) ties
  the tool to one vendor. Define a documented "agent runner" interface so users can plug
  in their own model/CLI; ship adapters rather than a baked-in default.

## 2. Security & trust model (P0 — the gating concern for enterprise)

This tool autonomously runs an agent loop that **edits source and executes shell hooks**
(`sh -c <cmd>` / `cmd /C` in `harness-cli/src/main.rs`). Enterprise adoption demands an
explicit, documented security posture:

- **Threat model & trust boundaries doc.** State plainly: the agent can modify any file
  in `owns` globs and the harness will execute arbitrary shell from hook scripts and
  eval commands. Document what is trusted (local hook scripts authored by the user) vs.
  untrusted (model output).
- **Command-injection surface review.** Audit every `Command::new("sh").arg("-c")` call
  path for how `cmd_str` is constructed and whether any model- or spec-derived string
  reaches a shell unquoted. Prefer argv-vector execution over `sh -c` where a shell
  isn't required.
- **Sandboxing / least privilege story.** Document and ideally support running the loop
  under a sandbox (container, seccomp/bubblewrap on Linux, Seatbelt on macOS), network
  egress restrictions, and a read-only filesystem outside the project root.
- **Secrets handling.** The agent backends need API keys. Document where keys come from,
  that they are never written to specs/logs, and add **log redaction** for token-shaped
  strings. No secrets in the repo, in `.harness/`, or in committed config.
- **Autonomy controls for the unattended loop:** a hard iteration cap, wall-clock and
  cost/token budgets, an interactive approval/dry-run mode, and a kill switch — so a
  runaway loop can't churn a repo or a billing account.
- **`SECURITY.md`** with a private vulnerability-reporting channel and an SLA.
- **Supply-chain hardening in CI:** add `cargo audit` and `cargo deny` (licenses +
  advisories + bans), pin all GitHub Actions to commit SHAs (currently `@v4`/`@v2`
  floating tags), set minimal `permissions:` on every workflow, and enable Dependabot.
- **Build provenance & SBOM:** generate an SBOM (e.g. `cargo cyclonedx`) and SLSA/GitHub
  artifact attestations for released binaries; publish SHA-256 checksums.

## 3. Release engineering & distribution (P0)

The current `release.yml` will not satisfy enterprise distribution:

- **macOS binary is unsigned & unnotarized.** A downloaded unsigned `.dmg` is blocked by
  Gatekeeper — enterprise users can't run it without disabling protections. Add Apple
  Developer ID signing + notarization, or clearly document the quarantine workaround.
- **macOS-only.** No Linux or Windows artifacts, despite the code having `cmd /C`
  Windows branches. Enterprises are overwhelmingly Linux for CI/servers. Build a matrix:
  `x86_64`/`aarch64` × {linux-gnu, linux-musl, macOS, windows}.
- **Releases fire on every push to `main`**, tagged `vX.Y.Z+sha`. That's not a release
  process — it's a build-artifact stream. Move to **tag-driven releases** with semver
  tags, signed checksums, and human-curated release notes.
- **Distribution channels.** Provide reproducible install paths: Homebrew tap, a
  `cargo install` path (publish to crates.io), prebuilt binaries with checksums, and
  ideally a container image. An `install.sh` piped from a `.dmg` is not enough.
- **Reproducible builds:** declare an **MSRV** and commit a `rust-toolchain.toml` so
  builds are pinned; keep building with `--locked` (CI already does — good).

## 4. Testing & quality assurance (P1)

- **Thin coverage.** ~10–21 test attributes across ~7.5k lines of Rust. The core loop,
  layer-gating state machine, write-scoping/revert logic, and hook execution are exactly
  the parts that must not regress. Add unit + integration tests around them.
- **End-to-end tests** that drive a fixture project through `init -> spec -> build ->
  check` with a stubbed agent runner (no live model), asserting on on-disk artifacts and
  exit codes.
- **Golden/snapshot tests** for the generated `1-requirements.json`, `3-tasks.jsonl`,
  template rendering, and the `maps` reply templates.
- **Robustness:** audit the 21 `unwrap()/expect()/panic!` sites in `harness/crates`;
  user-facing failures should be `anyhow` errors with context, not panics.
- **Coverage reporting** (`cargo llvm-cov`) wired into CI with a floor, and a badge.
- **Python orchestration**: it has tests, but pin deps, add lint (`ruff`)/format
  (`black`)/type-check (`mypy`), and run them in CI — currently CI only covers the Rust
  workspace, so the Python system is untested on PRs.
- **Cross-platform CI matrix**: tests run only on `ubuntu-latest`; the Windows code
  paths are never exercised.

## 5. Observability & operability (P1)

- **Structured logging** with levels and a `--json` log mode; redact secrets. The
  existing logging design (`logging_system.txt`) should become a real, documented
  subsystem.
- **Audit trail** of every agent action, hook result, file mutation, and git checkpoint —
  enterprises need to answer "what did the agent change and why." Persist run records.
- **Cost/usage accounting** surfaced per run (tokens, wall-clock, iterations).
- **Deterministic, documented exit codes** for scripting in CI pipelines.
- **Telemetry, if any, must be opt-in** and documented (what's collected, where it goes).

## 6. Configuration & UX (P1)

- **Schema-validated config.** Publish JSON Schemas for `.harness/harness.toml`, the
  spec JSON/JSONL layers, and validate on load with actionable errors.
- **Config precedence** (flags > env > project file > user file) documented and tested.
- **`harness doctor`** command to diagnose the environment (git present, agent runner on
  PATH, hook scripts executable, versions) — reduces support load.
- **Helpful first run**: `harness init` should detect language/test runner and scaffold
  sensible default hooks rather than empty stubs.

## 7. Documentation (P1)

- **Restructure into `docs/`** (or an mdBook/Docusaurus site): Overview, Concepts
  (five-layer model, Ralph loop, hooks/gates), Install, Quick Start, CLI reference,
  Configuration reference, Security model, Troubleshooting, FAQ.
- **Architecture doc** with a diagram showing the Rust harness <-> agent runner <->
  Python orchestration relationship (see §1).
- **Auto-generated CLI reference** from `clap` so docs can't drift from flags.
- **Real-world example walkthrough** end to end on a sample repo.
- **Versioned docs** matching releases.

## 8. Open-source governance & community (P1)

Required files currently absent:

- **`CONTRIBUTING.md`** (dev setup, how to run tests/lint, branch/PR conventions).
- **`CODE_OF_CONDUCT.md`** (e.g. Contributor Covenant).
- **`SECURITY.md`** (see §2).
- **`CHANGELOG.md`** (Keep a Changelog) — the version-per-merge scheme has no human log.
- **Issue & PR templates** + `.github/CODEOWNERS`.
- **`SUPPORT.md`** / support expectations and a stated maintenance/release cadence.
- **Contributor licensing:** adopt **DCO** (`Signed-off-by`) or a CLA; enforce in CI.
- **License completeness:** MIT `LICENSE` exists (good). Add SPDX headers to source
  files, a `NOTICE`/`THIRD-PARTY-LICENSES` aggregation of dependency licenses
  (`cargo about`), and confirm deps are license-compatible (`cargo deny`).
- **Trademark/naming:** "harness" is generic and collides with an existing commercial
  product (Harness.io). Consider a distinctive name before publishing.

## 9. Maintainability & engineering (P2)

- **Decompose the 2.9k-line `harness-cli/src/main.rs`** into focused modules; it's the
  main maintenance liability.
- **Public API docs** (`cargo doc`) for `harness-core` if it's meant to be a reusable lib.
- **Enforce lint gates** beyond the CI flag, and add `#![warn(missing_docs)]` on core.
- **Encoding robustness:** ensure non-UTF-8 paths and non-ASCII spec/diff content are
  handled (relevant given prior "space-in-path" fixes).

---

## Suggested sequencing

1. **Make it coherent (P0, §0–1):** delete the stale root crate, fix the README build,
   pick one canonical product, drop to `0.x`. *Without this, nothing else matters.*
2. **Make it safe (P0, §2):** threat model, secrets redaction, autonomy caps,
   supply-chain CI.
3. **Make it shippable (P0, §3):** signed, cross-platform, tag-driven releases with
   checksums.
4. **Make it trustworthy (P1, §4–5):** real test coverage + audit logging.
5. **Make it adoptable (P1, §6–8):** docs, config schemas, governance files.
6. **Make it maintainable (P2, §9).**
