# Local task commands

Run from the workspace root using `cargo run -p router-cli -- <command>`.
Commands use the local SQLite store; no remote service is added.

`route-prompt` reads a desktop JSON message from stdin (`prompt`, `session_id`,
`turn_id`, `cwd`, optional `permission_mode` and `task_id`). It persists a blocked
draft or updates an unexecuted task in the same working directory. Deliveries
are deduplicated by session and turn; conflicting replays fail closed. Updates
preserve the execution contract and require fresh approval. The response stops
the original Desktop turn, and ingress never starts an App Server worker.
`--record-only` is a compatibility alias with the same behavior. Only internal
coordinator workers receive `continue: true` without recording another task.

New drafts have no inferred scope, acceptance criteria or verification command.
Load a reviewed manifest using the returned task/spec ids, approve the task,
then execute with `task-run` or `task-dispatch`. Approval alone cannot execute
an incomplete draft. Route/run ids exist only after coordinator dispatch.
Installed hooks use a local PowerShell wrapper that also blocks when the router
is missing, fails, or returns invalid output. Reinstall the hook after upgrading
to deploy the wrapper; source changes do not alter the installed configuration.

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
