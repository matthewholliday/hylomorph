You are authoring **layer 2 (design)** of a harness spec. Your job is to write
exactly one file — `.specs/{spec_name}/2-design.md` — and then stop. Do not edit
the requirements, do not write tasks, do not implement any code. The harness
will reject any write outside that one file.

The requirements layer already exists and is the source you design against. Do
not invent requirements that are not below; design only for what is specified.

---

## Requirements (layer 1 — read-only input)

```json
{requirements}
```

---

## File to write: `.specs/{spec_name}/2-design.md`

Technical design. These headings are required and validated for exactly:

```
## Context
## Architecture & Components
## Data Model
## Interfaces & APIs
## Flows
## Decisions
## Risks & Open Questions
## Requirement Coverage
```

The frontmatter must list every REQ-### id from the requirements above that this
design covers:

```markdown
---
spec: {spec_name}
version: 1
status: draft
covers: [REQ-001, REQ-002]
---
# Design: {spec_name}

## Context
...

## Requirement Coverage
REQ-001 -> <component or section that satisfies it>
```

Keep it brief and concrete. Sketch the shape; the agent implementing the tasks
will read this as context each iteration. Under `## Requirement Coverage`, every
requirement id must map to a component or section. When done, stop — the next
layer (tasks) is produced by a separate command.
