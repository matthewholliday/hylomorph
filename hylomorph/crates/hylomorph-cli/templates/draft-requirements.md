You are authoring **layer 1 (requirements)** of a harness spec. Your job is to
write exactly one file — `.specs/{spec_name}/1-requirements.json` — and then
stop. Do not write a design, do not write tasks, do not implement any code. The
harness will reject any write outside that one file.

---

## Brief

{brief}

---

## File to write: `.specs/{spec_name}/1-requirements.json`

EARS requirements. Rules:
- Every requirement uses `"shall"` in its `text` field.
- `type` must be one of: `ubiquitous`, `event`, `state`, `unwanted`, `optional`, `complex`.
- Each type has corresponding fields (`trigger` for event, `precondition` for
  state, `condition` for unwanted, `feature` for optional); set unused fields to `null`.
- Every requirement must have at least one entry in `acceptance_criteria` — these
  become the agent's done criteria downstream.
- `priority` is one of: `must`, `should`, `could`, `wont`.
- `status` is one of: `draft`, `approved`, `deprecated`. Use `"draft"` unless
  the brief is explicit.
- IDs are `REQ-001`, `REQ-002`, … — stable and unique within the spec.
- `owns` is a list of globs (relative to the project root) naming the files this
  spec's *code* layer will own. Fill it with your best guess from the brief; it
  can be refined later. Every spec must declare at least one `owns` glob.

EARS templates for reference:

| type | text template |
|---|---|
| `ubiquitous` | The `<system>` shall `<response>`. |
| `event` | When `<trigger>`, the `<system>` shall `<response>`. |
| `state` | While `<precondition>`, the `<system>` shall `<response>`. |
| `unwanted` | If `<condition>`, then the `<system>` shall `<response>`. |
| `optional` | Where `<feature>`, the `<system>` shall `<response>`. |

Write the JSON without comments (they are invalid JSON); the schema is:

```json
{
  "spec": "{spec_name}",
  "version": "1",
  "introduction": "<one paragraph framing the feature and its scope>",
  "glossary": { "<term>": "<definition>" },
  "owns": ["src/<area>/**"],
  "requirements": [
    {
      "id": "REQ-001",
      "type": "<type>",
      "system": "<system name>",
      "trigger": "<event clause or null>",
      "precondition": "<state clause or null>",
      "condition": "<unwanted clause or null>",
      "feature": "<optional clause or null>",
      "response": "<the shall clause>",
      "text": "<rendered sentence using the template above>",
      "rationale": "<why this requirement exists>",
      "priority": "<must|should|could|wont>",
      "acceptance_criteria": ["<concrete, testable assertion>"],
      "tags": [],
      "derived_from": [],
      "status": "draft"
    }
  ]
}
```

Derive content from the brief; fill any gaps with sensible defaults, marking
anything you assumed with a TODO note in the `rationale`. When done, stop — the
next layer (design) is produced by a separate command.
