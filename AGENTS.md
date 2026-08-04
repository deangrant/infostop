# Agent and contributor guidance

Structured conventions for AI agents and humans working in this repository. For
fuller context, see [README.md](README.md).

## Rules

- [`.agents/rules/`](.agents/rules/) (symlinked from [`.cursor/rules`](.cursor/rules))
- [`.agents/rules/infostop-crate.mdc`](.agents/rules/infostop-crate.mdc) — always-on crate policy (deps, API, CI, git)

## Skills

- [`.agents/skills/`](.agents/skills/) (symlinked from [`.cursor/skills`](.cursor/skills))
- [`.agents/skills/infostop-pipeline/`](.agents/skills/infostop-pipeline/) — algorithm pipeline, modules, errors, tests
- [`.agents/skills/solid-rust/`](.agents/skills/solid-rust/) — SOLID design in Rust

## Commands

- [`.agents/commands/`](.agents/commands/) (symlinked from [`.cursor/commands`](.cursor/commands))
- `/verify` — local CI checklist (fmt, clippy, test, examples)

## Hooks

- Config: [`.cursor/hooks.json`](.cursor/hooks.json)
- `afterFileEdit` → [`.agents/hooks/rustfmt.sh`](.agents/hooks/rustfmt.sh) formats edited `*.rs` files
