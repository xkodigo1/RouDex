# Local task commands

Run from the workspace root using `cargo run -p router-cli -- <command>`.
Commands use the local SQLite store; no remote service is added.

```text
spec-load <manifest.json>
task-list <specId>
task-status <taskId>
task-approve <taskId> [approved|denied]
task-dispatch <specId>
continue-run <runId>
run-events <runId>
```

`spec-load` validates the executable JSON task manifest before importing it.
It returns `specId`, `taskCount`, and persisted `readyTaskIds`. Reloading a
manifest preserves existing lifecycle state. `task-plan` remains an alias.

`task-list` returns a `tasks` array for a spec, with each task's definition
(including status), latest approval, latest run, and `dependenciesCompleted`.
Unknown specs produce an empty list. `task-status` returns one stored task.

`task-approve` records an explicit approval or denial; its default is `approved`.
`task-dispatch` passes eligible tasks to the coordinator in dependency order,
refreshing readiness after each execution. Approval and dependency gates stay
active. Its `receipts` array contains task statuses and route/run identifiers;
blocked, failed, or awaiting-approval receipts do not mean successful execution.
An unknown spec is an error; a known spec with no eligible tasks returns an
empty array. Dispatch never grants approval automatically.

`continue-run` resumes a task through the coordinator using its immutable
dispatch definition and persisted thread. Task runs require no stdin text.
Completed runs are returned without replay; superseded runs cannot resume.
For legacy prompt runs, supply continuation text on stdin. This command retains
the existing hook-compatible JSON block response on errors. Other task command
errors go to stderr and exit nonzero. `run-events` exposes persisted run evidence.

Internal workers may read task state, but cannot load specs, grant approvals,
dispatch tasks, or continue runs recursively.

Verification: `cargo test --workspace` covers manifest loading, argument errors,
persisted listings, approval/dependency gates, runtime failures, recursion
protection, and continuation replay protection.
