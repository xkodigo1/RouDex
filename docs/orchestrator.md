# Spec orchestrator

The orchestrator runs the tasks of a spec (`specs/<slug>/tasks.json`) through
Codex, one at a time and in dependency order. Every task goes through the
router: its tier becomes a model and an effort from `router.toml`, and risk
rules can raise it to `fuerte`. A task only completes when its verification
command exits with 0, and every step is kept as evidence in the local store.

You drive it from the **tareas** screen of the desktop app or from
`router-cli`. Both follow the same rules.

## The manifest

A spec is a folder under `specs/` in your project with a `tasks.json`:

```json
{
  "spec_id": "hello-001-notes",
  "title": "Notes file for the orchestrator example",
  "version": 1,
  "milestones": [{ "id": "H1", "title": "Notes", "task_ids": ["HELLO-T1"] }],
  "tasks": [
    {
      "id": "HELLO-T1",
      "milestone": "H1",
      "title": "Create the notes file",
      "prompt": "Create a file named NOTES.md …",
      "depends_on": [],
      "status": "planned",
      "tier": "economico",
      "approval": "not_required",
      "scope": ["NOTES.md"],
      "cwd": ".",
      "sandbox": { "type": "workspaceWrite", "writable_roots": ["."] },
      "acceptance": ["NOTES.md exists and starts with the line \"# Notes\"."],
      "verify": "git grep --no-index -q -e \"# Notes\" -- NOTES.md"
    }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `spec_id`, `title`, `version` | Identity of the spec. `version` is the spec's revision number. |
| `milestones` | Optional groups of tasks. Every listed task must exist and declare the same `milestone`. |
| `id`, `title`, `prompt` | The task and the prompt Codex receives. |
| `depends_on` | Tasks that must be completed first. Cycles and unknown ids are rejected. |
| `tier` | `economico`, `estandar` or `fuerte`. The model comes from `router.toml`, never from the manifest. |
| `approval` | `not_required`, or anything else (for example `manual`) to require an explicit approval before running. |
| `scope`, `acceptance` | What the task may touch and how it is judged. A task without them is not dispatched. |
| `cwd` | Working folder. A relative path is resolved against the project (the folder that contains `specs/`). |
| `sandbox` | `readOnly`, or `workspaceWrite` with `writable_roots` inside `cwd`. The roots must exist. Network is always off; `dangerFullAccess` is rejected. |
| `verify` | Command that must exit with 0 for the task to complete. |

Extra fields (for example the ones the spec-driven-development skill writes)
are kept in the file and ignored by the orchestrator.

**Older manifests.** A task without `tier` but with `profile` still loads: `micro`
and `routine` become `economico`, `standard` becomes `estandar`, and `deep` and
`frontier` become `fuerte`. A declared `model` and `effort` are ignored. Loading
reports each translation as a warning.

A manifest with an error (a cycle, a missing dependency, an unknown tier, an
unsafe sandbox, a milestone that does not match its tasks) is rejected as a
whole, and nothing is imported.

## Routing

When a task is dispatched:

1. **Manual tier.** If you chose a tier for the task, it wins. If the task
   matched a risk rule, the route says it was lowered on purpose.
2. **Risk rules.** Otherwise, if the title or the prompt matches a risk rule
   (RouDex's own and the project's `.roudex/rules.toml`), the tier is
   `fuerte`.
3. **Spec tier.** Otherwise the spec's tier is used.

The tier is then turned into a model and an effort with `router.toml`. The run
records the tier, the source of the decision, the model and the effort it got.
Resuming a run later uses that same model even if `router.toml` changed. The
board shows the route each task would take now, before you run it.

## Lifecycle

Statuses: `planned`, `ready`, `awaiting_approval`, `running`, `completed`,
`failed`, `blocked` and `interrupted`.

- **Gates.** A turn never starts while a dependency is not completed, or while
  a task that requires approval has none. A denied task stays blocked until it
  is approved.
- **Run.** Each run starts its own `codex app-server` process and a new thread,
  sends the prompt and streams the turn. In the app you can answer Codex's
  command and file-change approvals and stop the turn. The CLI declines those
  approvals, since nobody can answer them there, and keeps the requests as
  evidence.
- **Verify.** After a completed turn, `verify` runs in the task's sandbox with
  a 120 s limit: through `cmd.exe` on Windows and `sh -c` elsewhere. Exit 0
  completes the task; anything else fails it, and the output is kept. On
  Windows the command reaches `cmd.exe` as written, quotes included, but a
  `%VAR%` inside it is not expanded; put that kind of logic in a script and
  call the script.
- **Run all ready.** Runs every ready task of the spec, one after another, in
  dependency order. A failed task stops its dependents.
- **Resume.** An interrupted run continues on the same thread. A turn that had
  already completed is not repeated.
- **Retry.** Makes an interrupted or failed task ready for a new run. It is
  also the way out of a run whose thread could not be resumed.
- **Recover.** Marks a task left `running` by a closed app or process as
  interrupted, so it can be resumed or retried.

Only one run happens at a time in the app. Runs launched by the orchestrator
carry `ROUDEX_ROUTER_INTERNAL` and `ROUDEX_ACTIVE_TASK_ID`: the Codex Desktop
hook skips them, and coordinator commands refuse to run inside them.

## The tareas screen

- **Spec picker.** Specs you loaded, and the ones in the current project that
  are not loaded yet; picking one loads it. Manifests with errors are listed
  with the reason. **recargar** imports the file again and keeps each task's
  state.
- **Task list.** Tasks are grouped by milestone, with their status, the tier
  they would run with, and whether they wait for an approval.
- **detalle tab.** Prompt, acceptance, verification, dependencies, the route
  and its reason, risk matches, approval, scope, sandbox and folder.
- **Actions.** ▶ ejecutar, ▶ ejecutar listas, ■ detener, reanudar,
  reintentar, recuperar, aprobar and denegar. A tier control
  (`spec` / economico / estandar / fuerte) warns when a risky task goes below
  `fuerte`.
- **ejecución tab.** The latest run as a transcript: the router decision, the
  streamed answer, commands with their output, approvals to answer, and the
  verification with its exit code and output.

## CLI

```powershell
router-cli spec-load specs\001-notes\tasks.json   # validate and import
router-cli task-list hello-001-notes              # tasks and statuses
router-cli task-approve HELLO-T2 [approved|denied]
router-cli task-run HELLO-T1                      # one task
router-cli task-dispatch hello-001-notes          # every ready task
router-cli continue-run <runId>                   # resume an interrupted run
router-cli task-retry HELLO-T1
router-cli task-recover HELLO-T1
router-cli run-events <runId>                     # the evidence of a run
```

## Where things live

Specs stay in your project. Loaded tasks, runs, approvals and their events
are stored in `%LOCALAPPDATA%\RouDex\router.sqlite3`, and tier targets are in
`%LOCALAPPDATA%\RouDex\router.toml`. Nothing leaves your machine except the
turns Codex runs.

## Not included

Tasks never run in parallel, and nothing retries or escalates on its own. The
orchestrator never runs Git or opens pull requests, and manifests are edited as
files. Codex is the only runtime.

## Try it

[examples/hello-orchestrator](../examples/hello-orchestrator) has a two-task
spec that costs very little quota.
