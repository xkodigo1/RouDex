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
route-preview <profile> [--config-stdin] [--model <model>] [--effort <effort>]
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
prompt text, and always exits 0. `hook-install` and `hook-uninstall` manage the
RouDex entry in `$CODEX_HOME/hooks.json`. See
[docs/desktop-hook.md](../../docs/desktop-hook.md).

`route-preview` resolves a legacy coordinator profile without opening the local
store or starting a runtime.

The task commands drive the spec-driven task coordinator, which is frozen until
the SDD phase of the [roadmap](../../ROADMAP.md):

- `spec-load` validates a JSON task manifest before importing it and returns
  `specId`, `taskCount` and `readyTaskIds`. `task-plan` remains an alias.
- `task-approve` records an approval or denial (default `approved`).
- `task-run` / `task-dispatch` execute eligible tasks through `codex app-server`.
  Approval and dependency gates stay active; dispatch never grants approval.
- `continue-run` resumes a task run from its persisted thread; completed runs
  are returned without replay.
- `run-events` returns the persisted events of a run.

Errors go to stderr and exit nonzero. Internal workers cannot load specs, grant
approvals, dispatch tasks or continue runs recursively.
