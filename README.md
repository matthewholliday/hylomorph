# Hylomorph

A project-agnostic CLI for running coding agents as a **Ralph loop** with
deterministic, blocking validation gates ("hooks"). Implements the `hylomorph`
design spec (draft v0.1).

The name is from **hylomorphism** — Aristotle's idea that every thing is matter
(*hyle*) shaped by form (*morphe*). That's the model here: your **spec is the
form**, the **code is the matter** rendered from it. You author the form;
Hylomorph renders the matter and keeps the two in sync — the agent never
hand-edits the code, and when the spec changes the code is regenerated, not
patched.

The tool ships **no opinion** about your build system, test runner, or language.
You fill in per-project hook scripts, a guardrails policy, and a spec; `hylomorph`
drives a fresh-context agent over the task list one task at a time and decides
"done" by running your hooks — not by trusting the agent's claim.

## Build

The CLI lives in the Cargo workspace under `hylomorph/`.

```sh
cd hylomorph
cargo build --release
# binary at hylomorph/target/release/hylomorph

# …or build + install `hylomorph` (and the optional `hylomorph-gui`) onto your PATH:
../scripts/build-and-reinstall.sh
```

Installing the CLI also puts a short **`hylo`** alias on your PATH — it's the
same binary, so `hylo build <name>` and `hylomorph build <name>` are
interchangeable. The rest of this README spells out `hylomorph`.

## Quick start

```sh
cd your-project
git init                         # rollback boundary; recommended
hylomorph init                     # scaffold .hylomorph/
# edit .hylomorph/scripts/hooks/* to run your real build/test/lint

# Build the spec one layer at a time — each step is gated on the one before it:
hylomorph spec requirements <name> --brief "what it should do"   # layer 1
hylomorph spec design <name>                                     # layer 2 (needs requirements)
hylomorph spec tasks <name>                                      # layer 3 (needs design)
# …or run all three in order at once:  hylomorph spec new <name> --brief "…"

hylomorph check <name>             # spec well-formed + eval coverage + drift
hylomorph build <name>             # layer 4: generate code (needs requirements+design+tasks)
hylomorph eval draft <name>        # layer 5: draft evals (needs code)
```

## The five-layer model

A spec is built as a **vertical slice** of five layers, produced strictly in
order. Each layer is the input to the next, and the harness makes it
*structurally impossible* to skip ahead — the gate is enforced before any agent
runs, not left to the agent's judgement:

```
requirements → design → tasks → code → evals
```

| layer | artifact | command | requires |
|---|---|---|---|
| 1 requirements | `.specs/<name>/1-requirements.json` | `hylomorph spec requirements` | — |
| 2 design | `.specs/<name>/2-design.md` | `hylomorph spec design` | requirements |
| 3 tasks | `.specs/<name>/3-tasks.jsonl` | `hylomorph spec tasks` | + design |
| 4 code | files matched by the spec's `owns` globs | `hylomorph build` | + tasks |
| 5 evals | `evals/<name>/*` | `hylomorph eval draft` | + code |

Run `hylomorph spec status <name>` to see which layers exist and the single next
allowed action. Each drafting command is also **write-scoped**: it may only
touch its own layer's file(s); anything it writes elsewhere is reverted.

## Guided setup (optional Claude Code agent)

`hylomorph init` installs a Claude Code subagent to
`.claude/agents/hylomorph-setup.md`. Instead of editing the hook stubs by hand,
you can let it configure them for you: it detects your project's
build/test/lint/docs commands, wires the `.hylomorph/scripts/hooks/*` scripts and
`hylomorph.toml`/`guardrails.toml` to match, runs each hook once, and finishes
with `hylomorph doctor`.

In Claude Code, just ask for it:

```
> use the hylomorph-setup agent to configure Hylomorph for this repo
```

It detects, **then confirms with you** before writing — it won't invent
commands that don't exist or overwrite hooks you've already filled in. The
canonical definition lives at
[`templates/hylomorph-setup.md`](templates/hylomorph-setup.md); `init` writes a copy
into each project (skipped if one already exists, unless `--force`).

## Commands

The CLI follows one grammar: top-level **verbs** for the lifecycle (`build`,
`rebuild`, `check`…), and **nouns** for managing source objects (`spec`, `eval`,
`gate`). Code is treated as a build artifact rendered from the spec.

| Command | Purpose |
|---|---|
| `hylomorph init [--force]` | Scaffold `.hylomorph/`, `evals/`, and the pre-commit gate. |
| `hylomorph build <spec \| --all> [--once] [--max <N>] [--dry-run]` | Render code from a spec, task by task (incremental, non-destructive). |
| `hylomorph rebuild <spec \| --all> [--only <glob>] [--force]` | Burn the spec's owned files and re-render from scratch (destructive, eval-gated). |
| `hylomorph check [<spec> \| --all]` | The invariant gate: spec well-formed + eval coverage + no drift. |
| `hylomorph check <spec> --reverse` | Reconstruct the spec from code and report convergence (advisory). |
| `hylomorph check <spec> --determinism` | Rebuild twice and compare eval results (spec-tightness probe). |
| `hylomorph check <spec> --accept` | Accept the current code as the spec's baseline (escape hatch). |
| `hylomorph spec requirements <name> [--brief "…" \| --from <file> \| -]` | Layer 1: draft requirements from a brief. |
| `hylomorph spec design <name>` | Layer 2: draft a design from the requirements (gated on layer 1). |
| `hylomorph spec tasks <name>` | Layer 3: draft tasks from the design (gated on layers 1–2). |
| `hylomorph spec new <name> [--brief "…" \| --from <file> \| -]` | Run layers 1→3 in order, each gated (convenience wrapper). |
| `hylomorph spec status <name>` | Show the five-layer ladder and the next allowed action. |
| `hylomorph spec edit <name> [requirements\|design\|tasks]` | Open a spec file in `$EDITOR`, then check it. |
| `hylomorph spec show <name>` | Print a spec's resolved contents. |
| `hylomorph spec ls` | List specs under `.specs/`. |
| `hylomorph spec coverage <name> [--fix]` | Report requirement↔task coverage; `--fix` writes task stubs. |
| `hylomorph eval ls <spec>` | List eval scripts for a spec. |
| `hylomorph eval run <spec>` | Run a spec's evals against the current code. |
| `hylomorph eval draft <spec> [--force] [--use-code-agent]` | Draft reviewable eval stubs from the spec's acceptance criteria. Uses the independent reviewer model by default. |
| `hylomorph gate ls` | List gate (validation hook) scripts. |
| `hylomorph gate check` | Verify every referenced gate exists and is executable (static preflight; exits 2 if broken). |
| `hylomorph gate run <name> [--task <id>]` | Run one gate manually. |
| `hylomorph explain <task> [--spec <name>] [--phase <name>]` | Preview the exact prompt the agent would receive for a task, without running it. |
| `hylomorph status` | Active spec/task and task counts. |
| `hylomorph watch` | Live terminal dashboard. |
| `hylomorph log [<n>] [-f\|--follow]` | List iteration records, show one by number, or stream them live (`--follow`). |
| `hylomorph doctor` | Validate environment and config (agent adapter, gates, git). |

Older verbs (`run`, `regen`, `manifest`, `hooks`, `logs`, `spec list/draft/validate/sync`)
remain as hidden deprecated aliases that print a migration note and dispatch to
the new command for one release.

## Exit codes (top-level)

| Code | Meaning |
|---|---|
| `0` | Success / consistent. |
| `1` | Usage or config error (you typed it wrong). |
| `2` | Invariant violation — drift, failed gate/eval, or blocked tasks remaining. |
| `3` | Agent adapter failure (in `--once` mode). |

## How the loop works

Each iteration:

1. Select the lowest-`priority` `todo` task whose `depends_on` are all `done`
   (across all in-scope specs).
2. Compose a prompt from the template + guardrails + the task + matching
   requirements + design excerpt + `progress.md`. On a retry, the prior attempt's
   captured failure (failing gate output or agent error) is injected so the agent
   can fix the root cause instead of repeating it.
3. Run the agent as a **fresh process** (`[agent].command`, with `{prompt_file}`
   substituted).
4. Run the task's blocking **gates** in order. First blocking failure
   short-circuits the iteration.
5. All blocking gates pass → mark `done`, clear the stored failure, optionally
   `git commit`. Otherwise capture the failure, increment `attempts`, and park as
   `blocked` once the global `[budgets].max_attempts_per_task` cap is reached.
6. Write a structured iteration record and update `state.json` / `progress.md`.

State lives entirely on disk (`.hylomorph/`, `.specs/`), so a run is safe to
Ctrl-C and resume (re-run `hylomorph build`). Preview any task's prompt with
`hylomorph explain <task>`, and stream iterations live with `hylomorph log --follow`.

## Hook contract

A hook is any executable in `.hylomorph/scripts/hooks/`. It receives:

- **Env:** `HYLOMORPH_HOOK`, `HYLOMORPH_TASK_ID`, `HYLOMORPH_SPEC`, `HYLOMORPH_ITERATION`,
  `HYLOMORPH_ATTEMPT`, `HYLOMORPH_ROOT`.
- **Stdin:** the task object as JSON.
- **Exit code:** `0` = pass; non-zero = fail/block.

Full output is captured to `.hylomorph/logs/hooks/`; a head+tail excerpt lands in
the iteration record. By convention all hooks block except `run_e2e_tests`
(non-blocking by default); override per hook in `guardrails.toml`.

## Example: a TypeScript project driven by Claude Code

This walks through wiring `hylomorph` into a typical TypeScript project that uses
`tsc`, ESLint, and Vitest, with [Claude Code](https://claude.com/claude-code)
as the agent.

### 1. Point the agent adapter at Claude Code

In `.hylomorph/hylomorph.toml`, set `[agent].command`. `hylomorph` writes the composed
prompt to a temp file each iteration and substitutes its path for
`{prompt_file}`; Claude Code reads it on stdin in non-interactive (`-p`) mode:

```toml
[agent]
# Fresh `claude` process per iteration, reading the prompt from stdin.
command = "claude -p --dangerously-skip-permissions < {prompt_file}"
working_dir = "."

[loop]
max_iterations = 50
commit_each_success = true
reset_on_failure = true        # restore the tree to last commit after a failed iteration

[hooks]
default = ["run_build", "run_lint", "run_unit_tests"]
default_timeout_secs = 600
```

> `--dangerously-skip-permissions` lets the agent edit files unattended. Pair it
> with the `[writes]` allowlist in `guardrails.toml` and the per-iteration git
> commit so every green state is recoverable.

### 2. Write the hook scripts

Each hook is just an executable in `.hylomorph/scripts/hooks/` that exits `0` on
pass. `hylomorph init` creates stubs; replace their bodies with the real commands.

`.hylomorph/scripts/hooks/run_build` — type-check with `tsc`:

```sh
#!/usr/bin/env sh
# Build = the TypeScript compiles with no errors.
npx tsc --noEmit
```

`.hylomorph/scripts/hooks/run_lint` — ESLint over the source:

```sh
#!/usr/bin/env sh
# Lint = ESLint passes (and Prettier, if you use it).
npx eslint . --max-warnings=0
```

`.hylomorph/scripts/hooks/run_unit_tests` — Vitest in run-once mode:

```sh
#!/usr/bin/env sh
# Unit tests = the Vitest suite is green. CI=true keeps it non-interactive.
CI=true npx vitest run
```

`.hylomorph/scripts/hooks/run_e2e_tests` — Playwright (non-blocking by default):

```sh
#!/usr/bin/env sh
# End-to-end = Playwright. Kept non-blocking locally (see guardrails below).
CI=true npx playwright test
```

Make them executable (the stubs `init` writes already are; new ones need it):

```sh
chmod +x .hylomorph/scripts/hooks/*
```

A hook can read the task as JSON on stdin if it wants to scope its work — for
example, only running tests for the files a task touched. With Node available
you can parse it inline:

```sh
#!/usr/bin/env sh
# Scope Vitest to the task's files_hint, if any were provided.
PATTERNS=$(node -e 'const t=JSON.parse(require("fs").readFileSync(0,"utf8")); process.stdout.write((t.files_hint||[]).join(" "))')
if [ -n "$PATTERNS" ]; then
  CI=true npx vitest run $PATTERNS
else
  CI=true npx vitest run
fi
```

### 3. Keep e2e advisory, scope writes

In `.hylomorph/guardrails/guardrails.toml`:

```toml
[writes]
allow = ["src/**", "tests/**", "docs/**", ".specs/**"]
deny  = [".hylomorph/guardrails/**", ".git/**", "**/.env*"]

[operations]
deny_destructive = true

# Playwright is slow and flaky without a server — record it but don't block on it.
[hooks.run_e2e_tests]
blocking = false
timeout_secs = 1800
```

### 4. Reference hooks per task

A task in `.specs/<name>/3-tasks.jsonl` can override the default hook set via its
`hooks` field — e.g. a pure type-model change that needs the compiler and linter
but no tests yet:

```json
{"id":"T-001","spec":"api","title":"Add User type and zod schema","requirements":["REQ-001"],"status":"todo","priority":1,"depends_on":[],"hooks":["run_build","run_lint"],"acceptance":["User schema parses a valid payload","tsc passes"],"files_hint":["src/models/user.ts"],"attempts":0,"notes":"","created_at":"2026-06-20T00:00:00Z","updated_at":"2026-06-20T00:00:00Z"}
```

Tasks that omit `hooks` fall back to `[hooks].default` from `hylomorph.toml`.

### 5. Verify and run

```sh
hylomorph doctor                   # confirms claude is callable, gates exist, git is present
hylomorph gate run run_build       # smoke-test a single gate by hand
hylomorph check api
hylomorph build api                # drive the loop; each task is built/linted/tested before it counts as done
```

If `tsc` or Vitest fails, the iteration fails: `hylomorph` increments the task's
`attempts`, restores the working tree to the last clean commit (because
`reset_on_failure = true`), and either retries or parks the task as `blocked`
once the global `[budgets].max_attempts_per_task` cap is hit — so a broken type
or red test never gets recorded as done on the agent's say-so.

## Status

v0.1 — **pre-1.0**. The CLI surface and on-disk formats (`.specs/`, `.hylomorph/`,
the JSON/JSONL layer schemas) are not yet stable and may change between 0.x
releases; pin a version if you script against them.

Implemented: the full loop, gates, the gated five-layer pipeline
(requirements → design → tasks → code → evals) with per-layer write scoping,
drift detection (`check`), burn-and-rebuild (`rebuild`), evals, logging, and all
CLI commands above. Not yet implemented: full deterministic task regeneration
from requirements (`spec coverage --fix` only writes stubs today).
