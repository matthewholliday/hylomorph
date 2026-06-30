# Production-Readiness Plan — Path to an Enterprise Open-Source Release

Scope: what it would take to distribute `hylomorph` (the Rust spec-as-source
coding-agent harness, the canonical product) as a credible open-source tool that an
enterprise security/platform team would adopt.

Severity legend: **P0** = blocks a credible release / will fail enterprise review ·
**P1** = expected of any serious OSS tool · **P2** = polish & maturity.

> **Decisions taken (canonicalization pass):** the Rust `hylomorph/` workspace is the
> single canonical product. The Python `orchestrations/spec-dev/` system, the stale
> root `src/` crate (+ root `Cargo.toml`/`Cargo.lock`), the loose MAPS/orchestration
> design docs, the root `templates/` copies, and the orchestration-oriented `AGENTS.md`
> have been **removed**. Items struck through below are done.

---

## 0. Repository hygiene & "one source of truth" (P0)

- ~~**Two parallel, divergent codebases** (stale root `src/` vs. `hylomorph/crates`).~~
  **Done** — root crate, `Cargo.toml`, `Cargo.lock` removed; `hylomorph/` is canonical.
- ~~**README build instruction built the wrong binary.**~~ **Done** — Build section now
  points at the `hylomorph/` workspace and `scripts/build-and-reinstall.sh`.
- ~~**Two `Cargo.lock` files.**~~ **Done** — only `hylomorph/Cargo.lock` remains.
- ~~**Orchestration/MAPS design `.txt` files + stale `AGENTS.md` at root.**~~ **Removed.**
- **Remaining loose docs:** `spec-as-source-dev-guide.md` and this `upgrades.md` still
  sit at root. Move narrative docs into a `docs/` tree (see §7).
- **`.gitignore`:** add a global `.DS_Store` ignore (currently `/target`, `sandbox/`
  only). Decide deliberately whether `.vscode/` belongs in the public repo.
- ~~**Write a new, Rust-focused `AGENTS.md`/`CONTRIBUTING.md`.**~~ **Done** — both
  rewritten for the canonical Rust workspace; `CONTRIBUTING.md` includes a DCO sign-off.

## 1. Identity, scope & versioning (P0)

- ~~**Dual-product confusion** (Rust harness vs. Python orchestration).~~ **Done** — the
  Python orchestration is removed; the Rust harness is the sole product.
- ~~**Honest versioning** (`1.0.0` while the spec is "draft v0.1").~~ **Done** — dropped
  to `0.1.0` in `[workspace.package]`, lockfile refreshed, README Status notes the
  pre-1.0 format-instability caveat. Keep `0.x` until the CLI surface and on-disk formats
  (`.specs/`, `.hylomorph/`, the JSON/JSONL layer schemas) are committed to.
- **Stability contract:** document which CLI flags, exit codes, and on-disk schemas are
  stable vs. experimental — enterprises script against these.
- **Vendor-neutral agent runner.** The harness shells out to a generic agent command
  (good — no vendor lock-in now that `cursor-agent` is gone). Make the runner interface
  an explicit, documented contract and ship example adapters.

## 2. Security & trust model (P0 — the gating concern for enterprise)

The tool autonomously runs an agent loop that **edits source and executes shell hooks**
(`sh -c <cmd>` / `cmd /C` in `hylomorph-cli/src/main.rs`).

- **Threat model & trust boundaries doc.** State plainly: the agent can modify any file
  in `owns` globs, and the harness executes arbitrary shell from hook scripts and eval
  commands. Distinguish trusted (user-authored hooks) from untrusted (model output).
- **Command-injection surface review.** Audit every `Command::new("sh").arg("-c")` path
  for how `cmd_str` is built and whether any model-/spec-derived string reaches a shell
  unquoted. Prefer argv-vector execution over `sh -c` where a shell isn't required.
- **Sandboxing / least privilege.** Support and document running under a sandbox
  (container, seccomp/bubblewrap on Linux, Seatbelt on macOS), restricted network
  egress, and a read-only filesystem outside the project root.
- **Secrets handling.** Document where agent API keys come from; guarantee they are never
  written to specs/logs; add **log redaction** for token-shaped strings.
- **Autonomy controls** for the unattended loop: hard iteration cap, wall-clock and
  cost/token budgets, interactive approval/dry-run mode, and a kill switch.
- **`SECURITY.md`** with a private vulnerability-reporting channel and an SLA.
- **Supply-chain hardening in CI:** add `cargo audit` and `cargo deny` (advisories +
  licenses + bans), pin GitHub Actions to commit SHAs (currently floating `@v4`/`@v2`),
  set minimal `permissions:` on every workflow, enable Dependabot.
- **Provenance & SBOM:** generate an SBOM (`cargo cyclonedx`) and SLSA/artifact
  attestations for released binaries; publish SHA-256 checksums.

## 3. Release engineering & distribution (P0)

The current `release.yml` won't satisfy enterprise distribution:

- **macOS binary is unsigned & unnotarized** — Gatekeeper blocks it. Add Developer ID
  signing + notarization, or clearly document the quarantine workaround.
- **macOS-only**, despite Windows code paths in the source. Build a matrix:
  `x86_64`/`aarch64` × {linux-gnu, linux-musl, macOS, windows}.
- **Releases fire on every push to `main`** (`vX.Y.Z+sha`) — a build-artifact stream, not
  a release process. Move to **tag-driven semver releases** with signed checksums and
  curated notes.
- **Distribution channels:** Homebrew tap, `cargo install` / crates.io, prebuilt binaries
  with checksums, and a container image.
- **Reproducible builds:** ~~commit `rust-toolchain.toml`~~ **Done** — pinned to
  `1.96.0` (rustfmt + clippy), with CI/release actions pinned to the same version so
  `fmt`/`clippy` are deterministic; `--locked` already in use. Still: declare and
  document an **MSRV** policy (the pin is exact today, which is stricter than an MSRV).

## 4. Testing & quality assurance (P1)

- **Thin coverage.** ~10–20 test attributes across ~7.5k lines of Rust; only ~2 tests run
  today. The core loop, layer-gating state machine, write-scoping/revert logic, and hook
  execution must be covered.
- **End-to-end tests** driving a fixture project `init -> spec -> build -> check` with a
  stubbed agent runner (no live model), asserting on-disk artifacts and exit codes.
- **Golden/snapshot tests** for generated `1-requirements.json`, `3-tasks.jsonl`, and
  template rendering.
- **Robustness:** audit the ~21 `unwrap()/expect()/panic!` sites; user-facing failures
  should be `anyhow` errors with context, not panics.
- **Coverage reporting** (`cargo llvm-cov`) in CI with a floor + badge.
- **Cross-platform CI matrix:** tests run only on `ubuntu-latest`; Windows paths are
  never exercised.

## 5. Observability & operability (P1)

- **Structured logging** with levels and a `--json` mode; redact secrets.
- **Audit trail** of every agent action, hook result, file mutation, and git checkpoint —
  "what did the agent change and why." Persist run records.
- **Cost/usage accounting** per run (tokens, wall-clock, iterations).
- **Deterministic, documented exit codes** for CI scripting.
- **Telemetry, if any, opt-in** and documented.

## 6. Configuration & UX (P1)

- **Schema-validated config.** Publish JSON Schemas for `.hylomorph/hylomorph.toml` and the
  spec layers; validate on load with actionable errors.
- **Config precedence** (flags > env > project file > user file) documented and tested.
- **`hylomorph doctor`** to diagnose the environment (git, agent runner on PATH, hook
  scripts executable, versions).
- **Helpful first run:** `hylomorph init` should detect language/test runner and scaffold
  sensible default hooks rather than empty stubs.

## 7. Documentation (P1)

- **Restructure into `docs/`** (or mdBook/Docusaurus): Overview, Concepts (five-layer
  model, Ralph loop, hooks/gates), Install, Quick Start, CLI reference, Configuration
  reference, Security model, Troubleshooting, FAQ.
- **Architecture doc** with a diagram (harness <-> agent runner <-> hooks/gates).
- **Auto-generated CLI reference** from `clap` so docs can't drift.
- **Real-world example walkthrough** end to end on a sample repo.
- **Versioned docs** matching releases.

## 8. Open-source governance & community (P1)

- ~~**`CONTRIBUTING.md`**~~ **added** (with DCO sign-off). Still absent:
  **`CODE_OF_CONDUCT.md`** (Contributor Covenant), **`SECURITY.md`**.
- **`CHANGELOG.md`** (Keep a Changelog) — the version-per-merge scheme has no human log.
- **Issue & PR templates** + `.github/CODEOWNERS`.
- **`SUPPORT.md`** and a stated maintenance/release cadence.
- **Contributor licensing:** adopt **DCO** (`Signed-off-by`) or a CLA; enforce in CI.
- **License completeness:** MIT `LICENSE` exists (good). Add SPDX headers, a
  `THIRD-PARTY-LICENSES`/`NOTICE` aggregation (`cargo about`), and confirm dependency
  license compatibility (`cargo deny`).
- ~~**Trademark/naming:** "harness" is generic and collides with Harness.io. Consider a
  distinctive name before publishing.~~ **Done** — the project was renamed to
  **Hylomorph** (CLI `hylomorph`, short alias `hylo`); "harness" is retained only as a
  generic noun for the agent-loop mechanism.

## 9. Maintainability & engineering (P2)

- **Decompose the ~2.9k-line `hylomorph-cli/src/lib.rs`** into focused modules.
- **Public API docs** (`cargo doc`) for `hylomorph-core` if it's a reusable lib;
  `#![warn(missing_docs)]` on core.
- **Encoding robustness:** ensure non-UTF-8 paths and non-ASCII spec/diff content are
  handled (relevant given prior "space-in-path" fixes).

---

## Suggested sequencing

1. ~~**Make it coherent** (§0–1): delete the stale crate, fix the README, drop the Python
   product, drop to `0.x`, add a Rust-focused `AGENTS.md`/`CONTRIBUTING.md`.~~ **Done.**
2. **Make it safe** (§2): threat model, secrets redaction, autonomy caps, supply-chain CI.
3. **Make it shippable** (§3): signed, cross-platform, tag-driven releases with checksums.
4. **Make it trustworthy** (§4–5): real test coverage + audit logging.
5. **Make it adoptable** (§6–8): docs, config schemas, governance files.
6. **Make it maintainable** (§9).
