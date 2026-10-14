# Specification 002 - Local-first product roadmap

Version: 1
Date: 2026-10-13
Status: planned; H1 completed, H2-H9 planned
Executable contract: [tasks.json](tasks.json)
Human task graph: [tasks.md](tasks.md)
Technical decisions: [decisions.md](decisions.md)

## Problem and outcome

RouDex has a working local orchestration foundation but needs a versioned path
from that foundation to a usable, auditable desktop product. A list of features
alone does not identify what can execute next, what must be approved, or which
evidence proves completion.

This specification defines that path as nine milestones and 33 bounded task
contracts: one completed historical reference and 32 future tasks. Its users
are the local operator who reviews and runs work, the developer who implements
one task at a time, and the reviewer who checks acceptance and release readiness.

The current change creates only `spec.md`, `decisions.md`, `tasks.md` and
`tasks.json` in this directory. It does not implement any H2-H9 feature,
including the visual control center, or authorize their execution, installation,
publication or dependency acquisition.

## Baseline and planning assumptions

H1 is **completed**, as requested and as recorded by
[specification 001](../001-task-orchestration/spec.md), its
[task manifest](../001-task-orchestration/tasks.json), and the local `v0.5.0`
tag. The baseline includes Rust domain/persistence, SQLite, the local Codex
App Server coordinator over stdio, CLI, desktop ingress and a read-only
Tauri/React state view.

The [H1 acceptance audit](../001-task-orchestration/acceptance-evidence.md)
records 58 passing tests, a historical T-001 assignment discrepancy and the
absence of native interactive UI smoke evidence. Its opening publication block
describes the earlier audit phase; the later task/spec records document
publication. This roadmap preserves those qualifications. A completed H1 is
not a claim that the future control center or native product acceptance exists.
No new remote publication check is claimed by this planning change.

The repository defines H1 but contains no established H2-H9 roadmap. Their
titles, grouping and acceptance below are explicit planning proposals, not
recovered requirements. The proposal reuses the current architecture and can
be revised before a milestone is approved. There are no promised dates,
performance SLAs, cloud features or additional platforms.

## Milestones

| Milestone | Status | Increment and exit condition | Tasks |
| --- | --- | --- | --- |
| H1 - Local task orchestration | completed | Retain the published orchestration baseline and its evidence limitations. | R002-T001 |
| H2 - Visual control center | planned | Read-only milestone/task overview, filters and run details; loading, empty, stale and failure states; keyboard and viewport evidence. | R002-T002-R002-T005 |
| H3 - Specification and task planning | planned | Validate and preview local manifests, import atomically, edit unexecuted drafts and expose dependency blockers without rewriting history. | R002-T006-R002-T009 |
| H4 - Routing configuration and preview | planned | Validate local overrides, preview the resolved route and bind each run to an immutable routing revision. | R002-T010-R002-T013 |
| H5 - Approval and execution controls | planned | Review exact contracts, approve or deny a revision, and explicitly dispatch eligible work through the coordinator once. | R002-T014-R002-T017 |
| H6 - Interruption and recovery | planned | Request interruption, observe its actual outcome and resume by existing run identity without duplicating completed work. | R002-T018-R002-T021 |
| H7 - Verification and evidence | planned | Persist and inspect verification results, distinguish runtime success from verified completion, and export a sanitized local summary. | R002-T022-R002-T025 |
| H8 - Local installation and data portability | planned | Diagnose readiness, back up and restore disposable/local stores safely, and document reproducible local packaging. | R002-T026-R002-T029 |
| H9 - Product acceptance and release readiness | planned | Complete integrated checks, boundary audit and native smoke; prepare a concrete release handoff with remaining publication authorization explicit. | R002-T030-R002-T033 |

Milestone numbers express product progression, not a mandatory global execution
sequence. The task graph contains the actual dependencies. Independent branches
can progress after H1; overlapping file scopes still require serialized edits.

## Scope and invariants

1. **Local-first:** SQLite remains authoritative on the local machine. Desktop
   communication uses Tauri IPC; runtime control uses the existing local App
   Server stdio boundary. No remote listener, web control endpoint, cloud sync,
   telemetry, account system or remote execution is added by default. Local-first
   does not claim model inference is offline: the installed Codex runtime has
   its own availability and authentication prerequisites, managed outside this
   repository. Tests use disposable data and synthetic runtimes unless a live
   run is separately authorized.
2. **Project isolation:** operate only within the declared RouDex scope. Do not
   inspect, modify or link unrelated project directories. Repository artifacts
   contain relative paths, not personal machine paths.
3. **No sensitive content:** do not read or embed environment-file values,
   credentials, authentication configuration or real operational prompt/event
   bodies in specifications, fixtures, exports or release artifacts. Backups of
   operational local state are private data, never release attachments.
4. **Existing stack:** reuse Rust, SQLite, Tauri, React and Vite. Present scope,
   alternatives and the technical choice before adding a framework or dependency.
   No such addition is part of this change. Missing cached dependencies block
   verification rather than silently permitting a network install.
5. **Coordinator authority:** every future implementation task executes through
   Codex Router and its local App Server path. Hook ingestion, route selection,
   an approval click and model prose are not execution evidence. Internal
   workers cannot approve, dispatch or change their own lifecycle recursively.
6. **Explicit permissions:** `scope`, `cwd` and `sandbox` are resolved before
   dispatch. Every future task has a manual contract approval. A plan is not
   permission to run it. Later restore/overwrite, hook installation and release
   operations retain their specific action gates. UI controls never grant
   broader permissions than the coordinator accepts.
7. **Stable identity:** task ids are globally namespaced for this spec because
   the current store keys tasks by task id. Existing run definitions, approvals
   and verification history must survive import, configuration changes and
   upgrades. H1 records are not overwritten or redispatched.
8. **Fail closed:** missing dependencies, stale approvals, invalid configuration,
   unavailable assigned models, runtime failure or unsafe paths produce a
   concrete blocked/failed reason. No fallback model, direct UI execution or
   silent widening of writable scope is allowed.
9. **Idempotency and recovery:** repeated input, dispatch, approval and continuation
   cannot duplicate work. Interruption is a request with an observed result, not
   an assertion that filesystem changes were undone. Resume uses persisted
   run/thread/turn identity and the immutable execution contract.
10. **Evidence-backed completion:** a task requires verification exit status,
    acceptance results and any manual artifact. A runtime terminal message alone
    is insufficient. Fixture, automated, native and live-runtime results are
    distinct; missing evidence remains visible and blocking where required.

## Non-goals

- Implementing the visual control center or any other future roadmap task now.
- Replacing the current stack, writing a general Markdown parser, introducing
  a graph rendering library or implementing remote collaboration.
- Changing the host Codex application's model selection or bypassing its runtime
  permissions through a hook.
- Automatic approval, automatic replay after an ambiguous crash, destructive
  cancellation rollback, unattended installation or automatic updates.
- Committing, pushing, tagging, creating releases, uploading exports or altering
  an operational database as part of this specification authoring task.

## Acceptance scenarios and traceability

The following are future product acceptance gates except A01, which records
the historical baseline. Commands are defined per task in `tasks.md` and
`tasks.json`. New named test targets are planned deliverables, not existing
tests or evidence of completion.

| ID | Precondition | Action | Observable result | Tasks and verification |
| --- | --- | --- | --- | --- |
| A01 | Specification 001 and local H1 release metadata exist. | Inspect completion records and qualifications. | H1 is completed without implying H2 or native smoke has passed. | R002-T001; historical manifest, audit, local tag. |
| A02 | Synthetic snapshots contain tasks, retries, missing runs and errors. | Open overview, filter, select details, refresh, switch selections and navigate by keyboard. | Correct task counts, stable selection, ordered run events, no stale-response contamination; empty/loading/stale/error states and visible focus. No writes occur. | R002-T002-T005; `control_center` tests, fixture builds and three UI evidence artifacts. |
| A03 | Valid, cyclic, duplicate, cross-spec-collision and changed manifests are available. | Preview, import, reimport and edit an unexecuted draft. | Invalid input produces zero writes; valid import is atomic; reimport preserves history; draft changes invalidate review; running/completed contracts remain immutable. | R002-T006-T009; `roadmap_planning`, `planning_commands` and task-editor/graph evidence. |
| A04 | Built-in routing and a valid/invalid override revision exist. | Preview, save, dispatch, change settings and continue the original run. | Preview matches dispatch; invalid values cannot replace settings; resume retains the original mapping; unavailable models block without fallback. | R002-T010-T013; `roadmap_routing`, `routing_preview`, `routing_commands` and UI evidence. |
| A05 | A task is unapproved, denied, approved or changed after approval. | Review its concrete contract and approve/deny, then request dispatch. | Approval alone starts nothing; stale or denied approval blocks; only the approved revision is eligible. | R002-T014-T016 and R002-T008; `roadmap_approvals`, `approval_commands` and review/editor evidence. |
| A06 | An eligible task, an unmet dependency and a failed runtime are available. | Submit dispatch twice and then retry the blocked/failing cases. | At most one eligible run starts with assigned model/effort; dependency/runtime failure has a durable reason; no UI bypass. | R002-T015, R002-T017; `approval_commands` and dispatch-control evidence. |
| A07 | A run is active, interrupted, terminal or missing reliable identity. | Interrupt, simulate a crash at terminal persistence, restart and resume twice. | One terminal result; immutable assignment and original run identity retained; completed work does not replay; unsupported interruption and ambiguous identity block. | R002-T018-T021 and R002-T012; `roadmap_recovery`, `recovery_commands`, routing tests and recovery UI evidence. |
| A08 | Runs have successful, failed, absent and malformed verification records. | Complete a runtime turn and inspect/paginate its evidence. | Only independently verified success completes the task; history is bounded, run-isolated and explicit about missing/failed evidence. | R002-T022-T024; `roadmap_evidence`, `evidence_commands` and evidence UI artifact. |
| A09 | Synthetic records contain sensitive markers and an existing export destination. | Export an allowlisted local summary and inspect the candidate release package. | No raw prompts/event bodies, authentication data or absolute personal paths; overwrite requires explicit confirmation; nothing is sent externally. | R002-T025, R002-T031; `evidence_export` and boundary audit. |
| A10 | Runtime/configuration/storage are absent, incompatible or inaccessible. | Run diagnostics and read local state; inspect exposed interfaces and permissions. | Actionable errors, no accidental database creation/migration, installation, dispatch, new listener or expanded path scope. | R002-T002, R002-T006, R002-T026, R002-T029, R002-T031; focused tests, packaging checklist and boundary audit. |
| A11 | Disposable stores include concurrent writes, corruption and incompatible versions. | Back up, preview restore, confirm restore and inject replacement failure. | Backup is consistent; invalid/busy restore changes nothing; rollback preserves the previous store; restore cannot execute tasks. | R002-T027-T028; `local_backup`, `local_restore`, with synthetic source/destination evidence. |
| A12 | Feature tasks and integrated checks pass and a local native runtime is available. | Build, launch the native app, follow the smoke workflow and prepare the release handoff. | Actual native actions and runtime provenance are recorded; unresolved findings block readiness; handoff distinguishes verified local readiness from unapproved publication. | R002-T029-T033; workspace tests/builds, native evidence, boundary audit and release handoff. |

R002-T030 aggregates A02-A12. R002-T033 closes the milestone record only after
all prior tasks, automated checks and required human observations are complete.
Each task's narrower acceptance criteria remain mandatory; this matrix does
not replace them.

## Verification and delivery for this planning change

- Verify the four files, JSON parse, required fields, unique ids, acyclic and
  complete dependency graph, matching built-in routes and milestone coverage.
- Verify all acceptance scenarios map to task records, all future tasks are
  planned/manual, and logical scopes plus writable roots stay repository-relative.
- Parse the manifest with the existing Rust `TaskManifest` validator without
  importing it into operational storage or dispatching future tasks.
- Run the enclosing authoring task's required `cargo test --workspace` check
  and inspect the whitespace diff and changed-file list.
- Inspect for personal paths, unrelated project paths, credentials and
  environment values. No implementation or dependency file should change.

Current authoring is already inside the coordinator worker. Its receipt is
recorded in `tasks.json` under `authoring_run`; it does not belong to a future
product task. The worker cannot assert its own terminal completion. The parent
coordinator records that after the authoring turn and independent verification.

## Versioning and completion policy

Keep `spec_id` stable and increment `version` for a material scope, task-contract
or acceptance change. Update all four files together; retain existing ids and
record supersession rather than reusing ids for a different outcome. A changed
approved contract must pass review again before dispatch.

H1's completed status is inherited. H2-H9 remain planned until explicitly
authorized for execution and become completed only with all listed task and
milestone evidence. Local readiness is independent of publication: a later
approved publishing workflow owns signed commits, tags and release metadata.
