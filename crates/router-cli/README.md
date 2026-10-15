# router-cli

Run from the workspace root using `cargo run -p router-cli -- <command>`.
Commands use the local SQLite store; no remote service is added.

```text
hook                          (Codex UserPromptSubmit input on stdin)
hook-install
hook-uninstall
chat <projectDir>             (prompt on stdin)
route <projectDir>            (prompt on stdin)
models [projectDir]
spec-load <manifest.json>
task-list <specId>
task-status <taskId>
task-approve <taskId> [approved|denied]
task-run <taskId>
task-dispatch <specId>
task-recover <taskId> [reason]
task-retry <taskId>
continue-run <runId>
run-events <runId>
status
```

`chat` routes the prompt, sends it in a new thread of the project's
`codex app-server` session (workspace-write limited to the project, approvals
on request) and prints every session event as JSON until the turn ends.
Approval requests are declined; the desktop app is where they are answered.
The thread, turn, response and token usage are stored in the local SQLite.

`route` prints the router decision for a prompt (tier, model, effort, source,
reason and risk matches) using `router.toml` and the project's
`.roudex/rules.toml`. It starts `codex app-server` only when the classifier is
needed and never executes the prompt. `models` prints the runtime catalog and
any configured tier that the catalog does not support. See
[docs/router.md](../../docs/router.md).

`hook` is the Codex `UserPromptSubmit` hook: it routes the prompt with the risk
and fast rules only, prints the hook response (context, a warning, or a block
when `[desktop_hook] block_high_risk = true`), records the decision without the
prompt text in `~/.roudex/desktop-hook.sqlite3`, logs failures to
`~/.roudex/hook.log`, and always exits 0. `hook-install` copies `router-cli` to
`~/.roudex/bin` (outside AppData, which Codex Desktop sees as a private copy)
and points the RouDex entry in `$CODEX_HOME/hooks.json` at it;
`hook-uninstall` removes that entry. See
[docs/desktop-hook.md](../../docs/desktop-hook.md).

The task commands drive the spec-driven task coordinator. Each task declares a
tier (`economico`, `estandar` or `fuerte`); at dispatch the coordinator turns it
into a model and effort with `router.toml`, and a risk rule match forces
`fuerte`. Manifests written before tiers still load: their `profile` maps to a
tier and `spec-load` reports it in `warnings`.

- `spec-load` validates a JSON task manifest before importing it and returns
  `specId`, `taskCount`, `readyTaskIds` and `warnings`. `task-plan` remains an alias.
- `task-approve` records an approval or denial (default `approved`).
- `task-run` / `task-dispatch` execute eligible tasks through `codex app-server`.
  Approval and dependency gates stay active; dispatch never grants approval.
  Nobody can answer Codex's command or file-change approvals here, so the CLI
  declines them and keeps the requests as evidence; the desktop app lets you
  answer them.
- `continue-run` resumes a task run from its persisted thread; completed runs
  are returned without replay.
- `task-retry` makes an interrupted or failed task ready for a new run, and
  also one blocked because its thread could not be resumed.
  `task-recover` marks a task left running by a closed process as interrupted.
- `run-events` returns the persisted events of a run.

Errors go to stderr and exit nonzero. Internal workers cannot load specs, grant
approvals, dispatch tasks or continue runs recursively.
