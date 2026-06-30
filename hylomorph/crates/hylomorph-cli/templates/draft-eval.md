You are drafting **eval scripts** for the harness spec `{spec_name}`. Write
executable scripts to `evals/{spec_name}/` and then stop. You MAY read files
under `src/` (and the rest of the repo) **to learn how the program is actually
invoked** — its command, flags, env vars, working paths, and output format. You
must NOT implement or modify any product code, and must NOT modify anything under
`.specs/`, `.hylomorph/`, or `src/`. Write only files under `evals/{spec_name}/`.

---

## What an eval is

An eval is the harness's **acceptance oracle**: a script that decides whether the
code satisfies a requirement. Its defining property is that it judges
*behaviour against intent*, independently of how the code is built —

- At runtime an eval interacts with the program the way a *user* would (its CLI,
  its HTTP API, its public surface). The script itself **never** imports test
  helpers, sources internal files, or reads anything under `src/` while it runs —
  a passing eval must not depend on the implementation's internals.
- You, the author, MAY read `src/` *now*, while drafting, but only to learn the
  program's real interface — the command name, flags, env vars, working paths,
  and the shape of its output. Use it to make the script *runnable*; never copy
  the implementation's behaviour into your assertions.
- An eval asserts on observable *behaviour*, not on implementation structure. A
  completely different implementation that behaves correctly must still pass.
- An eval encodes the requirement's **intent**. The expected results you assert
  on come from the requirement text — never "whatever the current code happens to
  output." It is the definition of correct, written so that *incorrect* code
  fails. Reading `src/` tells you HOW to call the program; the requirement tells
  you WHAT the answer must be.

You are producing **drafts**, not the final oracle. A human will review every
stub before it is trusted. Your job is to give them a strong, honest starting
point — not to guarantee passing scripts.

---

## Requirements to cover

{force_note}

Each requirement below has an `id`, the requirement `text`, and
`acceptance_criteria`. The acceptance criteria are prose; your task is to turn
them into executable checks.

```json
{requirements}
```

---

## Files to write

For each requirement you are covering, write
`evals/{spec_name}/<REQ-ID>-<slug>.sh`, e.g.
`evals/{spec_name}/REQ-001-add-task.sh`. The `<slug>` is a short kebab-case hint
from the requirement. The requirement ID **must** appear in the file (the
coverage gate matches on the ID), so keep it in the filename and repeat it in a
header comment.

Each script:

- starts with `#!/usr/bin/env sh` and `set -e`;
- is hermetic — create a temp working dir (`mktemp -d`), point the program at it
  via its real config/env mechanism, and clean up; never depend on machine state;
- exercises the program through its real interface only;
- signals pass with exit 0 and fail with a non-zero exit (a failed `grep`,
  `test`, or explicit `exit 1`).

### The honesty rule — ground the mechanics, derive the intent

There are two kinds of unknown in a stub, and they are handled differently:

1. **Mechanical** — how to invoke the program: its command, flags, env vars,
   working directory, and output format. **Resolve these by reading `src/`.** Do
   not leave them as guesses and do not invent them. Once grounded, the stub
   should actually run.
2. **Intent** — what counts as a correct answer for this requirement. This comes
   from the requirement text, NOT from running the code and copying its output.
   If the requirement is specific (e.g. "2 + 2 must display 4"), assert exactly
   that. A stub that passes for the wrong reason is worse than one that fails
   loudly.

So:

- Read `src/` enough to make the invocation real, then write setup and assertions
  that exercise the program through its real interface.
- Derive every *expected value* from the requirement. Where the requirement is
  genuinely ambiguous about what "correct" means, mark that one point with a
  `# TODO(intent):` naming what a human must decide — do not paper over it by
  asserting whatever the code emits.
- A stub whose mechanics are grounded and whose assertions follow from the
  requirement should be left **runnable** — let it pass or fail on the real
  behaviour. Only a stub that still contains an intent `# TODO` ends with
  `exit 1` (a draft must never pass while its notion of "correct" is a guess).

Example shape (mechanics grounded from `src/`, expected value from the spec):

```sh
#!/usr/bin/env sh
# REQ-001 — <requirement text>
# DRAFT eval — assertions derived from the requirement; a human should confirm
# they encode its intent. Mechanics (invocation, paths) grounded from src/.
set -e
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
# Invocation grounded from src/: program reads its store dir from $APP_HOME.
export APP_HOME="$WORK"

# Acceptance criterion: "<criterion from the spec>"
# Expected value comes from the requirement, not from observed output:
todo add "Buy milk" | grep -q "Buy milk"
```

If — and only if — a requirement leaves "correct" genuinely undecided, fall back
to the failing-draft form for that check:

```sh
# TODO(intent): the requirement does not specify the rounding behaviour for X.
echo "DRAFT: REQ-00N intent unresolved — a human must decide" >&2
exit 1
```

---

## After writing the files

Print a short summary:

- which requirement IDs you wrote stubs for (and which you skipped, and why);
- which mechanics you grounded by reading `src/` (e.g. "invoked via `todo`, store
  dir from $APP_HOME");
- any remaining intent `# TODO`s a human still needs to resolve (and which stubs
  therefore still `exit 1`);
- the reminder that these are drafts whose assertions a human should confirm
  encode each requirement's intent, and that `hylomorph check {spec_name}` will
  confirm coverage.
