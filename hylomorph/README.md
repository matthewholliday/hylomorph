# Hylomorph

A project-agnostic CLI for running coding agents as a **Ralph loop** with
deterministic, blocking validation gates ("hooks"). Implements the `hylomorph`
design spec (draft v0.1).

The tool ships **no opinion** about your build system, test runner, or language.
You fill in per-project hook scripts, a guardrails policy, and a spec; `hylomorph`
drives a fresh-context agent over the task list one task at a time and decides
"done" by running your hooks — not by trusting the agent's claim.

## Build

The repository is a Cargo workspace with three crates:

- `hylomorph-core` — the shared data/logic library (config, state, specs, the loop, the snapshot read model).
- `hylomorph-cli` — the `hylomorph` command-line tool (CLI + terminal dashboard).
- `hylomorph-gui` — an optional desktop front-end (egui) for authoring a spec's five layers.

```sh
cargo build --release
# binary at target/release/hylomorph
```

### Desktop GUI

`hylomorph-gui` is a thin front-end over the same CLI. It shows every spec under
`.specs/` in a left column; selecting one opens an accordion of its five layers
(requirements → design → tasks → code → evals). Each layer can be generated once
its upstream layers exist — *Generate* opens a window for an optional prompt
(only `requirements` accepts one today, as `--brief`), and *Proceed* shells out
to `hylomorph …`, streaming output into a log pane. The evals layer also offers
*Run eval suite* (`hylomorph eval run <spec>`). Run it from a project root; it
finds the `hylomorph` binary via `HYLOMORPH_BIN`, a sibling next to itself, or `PATH`.

```sh
cargo run -p hylomorph-gui          # from your project root
```

## Quick start

```sh
cd your-project
git init                         # rollback boundary; recommended
hylomorph init                     # scaffold .hylomorph/
# edit .hylomorph/scripts/hooks/* to run your real build/test/lint
hylomorph gate check               # confirm the gate scripts are wired and executable
# author a spec under .specs/<name>/ (1-requirements.json, 2-design.md, 3-tasks.jsonl)
hylomorph check <name>             # spec well-formed + eval coverage + no drift
hylomorph build --dry-run --once   # preview task selection
hylomorph build                    # drive the loop
```

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

| Command | Purpose |
|---|---|
| `hylomorph init [--force]` | Scaffold `.hylomorph/` (config, prompts, guardrails, gate stubs, logs). |
| `hylomorph build [<spec> \| --all] [--once] [--max <N>] [--dry-run]` | Render code from a spec, task by task (incremental, non-destructive). The Ralph loop. |
| `hylomorph rebuild [<spec> \| --all] [--only <glob>] [--force]` | Burn a spec's owned files and re-render from the spec (destructive, eval-gated). |
| `hylomorph check [<spec> \| --all] [--reverse] [--determinism] [--accept]` | The invariant gate: spec well-formed + eval coverage + no manifest drift. Exits `2` on any problem. |
| `hylomorph spec ls` | List specs under `.specs/`. |
| `hylomorph spec new <name> [--brief <text> \| --from <file>]` | Draft a new spec from a brief (agent-assisted). |
| `hylomorph spec edit <name> [requirements\|design\|tasks]` | Open a spec file in `$EDITOR`, then check it. |
| `hylomorph spec show <name>` | Print a spec's resolved contents. |
| `hylomorph spec tasks <name> [--fix]` | Report requirement↔task coverage; `--fix` writes task stubs. |
| `hylomorph eval ls <spec>` / `hylomorph eval run <spec>` | List or run a spec's evals (the acceptance oracle). |
| `hylomorph eval draft <spec> [--force] [--use-code-agent]` | Draft reviewable eval stubs from the spec's acceptance criteria (independent reviewer model by default; stubs `exit 1` until a human fills in the TODOs). |
| `hylomorph gate ls` | List gate scripts. |
| `hylomorph gate check` | Verify every referenced gate exists and is executable (static preflight). Exits `2` if any is broken. |
| `hylomorph gate run <gate> [--task <id>]` | Run one gate manually. |
| `hylomorph explain <task> [--spec <name>] [--phase <name>]` | Preview the exact prompt the agent would receive for a task, without running it. |
| `hylomorph status` | Active spec/task and task counts. |
| `hylomorph watch` | Live terminal dashboard (TUI) that visualizes a run as it happens. |
| `hylomorph log [<n>] [--follow]` | List iteration records, show one by number, or `--follow` to stream them live. |
| `hylomorph doctor` | Validate config, gates, agent adapter, git. |

> Earlier verbs (`run`, `regen`, `hooks`, `logs`, `spec list/draft/validate/sync`,
> `manifest`) still work as hidden, deprecated aliases that print a migration note.

## Exit codes (top-level)

| Code | Meaning |
|---|---|
| `0` | Success / loop completed. |
| `1` | Usage or config error. |
| `2` | Loop stopped with blocked tasks remaining. |
| `3` | Agent adapter failure (in `--once` mode). |

## How the loop works

Each iteration:

1. Select the lowest-`priority` `todo` task whose `depends_on` are all `done`
   (across all in-scope specs).
2. Compose a prompt from the template + guardrails + the task + matching
   requirements + design excerpt + `progress.md`. If the task failed a previous
   attempt, the captured failure (failing gate output or agent error) is injected
   as a **"Previous attempt failed — fix this first"** section so the agent can
   diagnose and self-correct rather than blindly repeat the same approach.
3. Run the agent as a **fresh process** (`[agent].command`, with `{prompt_file}`
   substituted).
4. Run the task's blocking **gates** in order. First blocking failure
   short-circuits the iteration.
5. All blocking gates pass → mark `done`, clear the stored failure, optionally
   `git commit`. Otherwise capture the failure into the task's `last_failure`,
   increment `attempts`, and park as `blocked` once the global
   `[budgets].max_attempts_per_task` cap is reached.
6. Write a structured iteration record and update `state.json` / `progress.md`.

State lives entirely on disk (`.hylomorph/`, `.specs/`), so a run is safe to
Ctrl-C and resume — just re-run `hylomorph build` and it picks up the remaining
tasks. To preview what the agent will see for a given task before (or after) a
run, use `hylomorph explain <task>`.

## Watching a run live

`hylomorph watch` opens a read-only terminal dashboard that re-reads the on-disk
run state a few times a second and paints it. Because the loop already persists
everything as it goes (`state.json`, `iterations/*.json`, `progress.md`, and the
per-spec `3-tasks.jsonl`), the dashboard never touches the loop — run it in a
second terminal next to `hylomorph build`, or open it after a run to replay what
happened.

```sh
# terminal 1
hylomorph build

# terminal 2
hylomorph watch
```

For a lighter-weight, log-only view (no TUI), `hylomorph log --follow` streams a
one-line summary of each iteration as it completes — handy over SSH or when piping
to a file.

The `hylomorph watch` dashboard shows run status, a done/active/todo/blocked gauge,
the task table (in-progress first, with attempts and phase progress), a
task-detail pane (latest iteration's agent exit, per-hook pass/fail + timings,
commit sha, last failure), and a colorized `progress.md` tail. It auto-refreshes
~every 0.4s. Keys: `↑/↓` select · `g/G` jump to top/bottom · `r` force refresh ·
`q` quit.

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
hylomorph gate check               # confirm every referenced gate is present and executable
hylomorph gate run run_build       # smoke-test a single gate by hand
hylomorph check api                # spec well-formed + eval coverage + no drift
hylomorph build                    # drive the loop; each task is built/linted/tested before it counts as done
```

If `tsc` or Vitest fails, the iteration fails: `hylomorph` increments the task's
`attempts`, restores the working tree to the last clean commit (because
`reset_on_failure = true`), and either retries or parks the task as `blocked`
once the global `[budgets].max_attempts_per_task` cap is hit — so a broken type
or red test never gets recorded as done on the agent's say-so.

## Status

v0.1. Implemented: the full loop with prior-failure feedback into retries,
gates (with `gate check` preflight), spec authoring/coverage, the invariant
`check`, destructive `rebuild`, prompt preview (`explain`), live `watch` and
`log --follow`, and all CLI commands above. Not yet implemented: write-allowlist
sandboxing and cross-model review.
