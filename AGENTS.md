# Repository Purpose

This repository exists to produce standalone orchestration bundles.

Each generated orchestration is expected to be distributable as a `.zip` file
to another team or machine, where the recipient unpacks and runs it in their
own Cursor repository/workspace.

## Distribution Model

- Build each orchestration as a self-contained folder under `orchestrations/`.
- Assume recipients do not have local context from this source repository.
- Include clear setup and usage instructions in each orchestration `README.md`.
- Prefer deterministic, scriptable setup for recipients (`setup_recipient.sh`)
  when practical, with manual fallback instructions in the README.

## README Expectations for Every Orchestration

Every generated orchestration README should include:

- Setup steps for a recipient Cursor repository (where files should live).
- Agent installation instructions into project-scoped `.cursor/agents/`
  or user-scoped `~/.cursor/agents/`.
- Dependency/setup commands required before first run.
- A first-run command and expected output shape.
- Troubleshooting for common recipient setup failures.

