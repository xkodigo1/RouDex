# Specification 002 - Approved local-first product roadmap

Version: 2
Date: 2026-10-13
Status: planned; H1 completed, H2-H9 planned
Executable contract: [tasks.json](tasks.json)
Human task graph: [tasks.md](tasks.md)
Historical technical decisions: [decisions.md](decisions.md) (version 1)

## Problem and outcome

Align the product roadmap exactly with the approved RouDex master plan, from
completed local coordination to public beta and stable v1.0.0. The local operator
needs reviewable execution contracts, the developer needs explicit dependencies,
and the reviewer needs observable acceptance and release evidence.

This revision changes only `spec.md`, `tasks.md` and `tasks.json`. It defines
50 task contracts: one completed historical baseline and 49 planned tasks.
It does not implement features, execute future tasks, install dependencies,
change machine settings or publish releases. Existing task ids, outcomes,
statuses, receipts and evidence are retained. New task ids R002-T034 through
R002-T050 cover missing approved capabilities; none represents executed work.

## Approved baseline and evidence

H1 is **completed local coordination in v0.5.0**, recorded by
[specification 001](../001-task-orchestration/spec.md), its
[task manifest](../001-task-orchestration/tasks.json) and the local tag.
It includes Rust/SQLite persistence, the local Codex App Server coordinator over
stdio, CLI, Desktop ingress and a read-only Tauri/React state view.
The [historical acceptance audit](../001-task-orchestration/acceptance-evidence.md)
retains 58 passing tests, an assignment discrepancy and missing native interactive
smoke qualifications. No new remote publication verification is claimed.

H2 remains **visual control center targeting v0.6.0**, with its existing partial
evidence preserved. R002-T005 records 21 passing overview fixture checks, 20
detail checks, 5 passing control-center tests and a 33-module fixture build.
Only initial fixture keyboard focus was observed. Complete application keyboard
traversal and 1280x720 / 800x600 viewports remain unverified; no native evidence
is claimed. Acceptance is incomplete and neither H2 nor its tasks is newly
marked completed. The full actual verification record is retained under
R002-T005 in `tasks.md` and its evidence array remains intact in `tasks.json`.

The user's approved master plan fixes the titles, order and release targets
below. It supersedes the version 1 proposed grouping, not the implementation
history. `decisions.md` remains an unchanged historical version 1 record;
conflicting milestone references there (notably D001/D002/D004) are superseded
by these three version 2 artifacts. The existing stack and safety decisions
remain in force. There are no invented release dates or performance SLAs.

## Milestones and release targets

| Milestone | Release target | Status | Increment and exit condition | Tasks | Release gate task |
| --- | --- | --- | --- | --- | --- |
| H1 - Local coordination | v0.5.0 | completed | Retain the completed local coordinator baseline and its historical audit qualifications. | R002-T001 | R002-T001 |
| H2 - Visual control center | v0.6.0 | planned | Read-only overview, filters and task/run details pass state, keyboard and viewport checks; preserve partial H2 evidence and unresolved gaps. | R002-T002, R002-T003, R002-T004, R002-T005 | R002-T005 |
| H3 - Multiplatform installers and onboarding | v0.7.0 | planned | Native Tauri installers for Windows, macOS and Linux and first-run onboarding have per-platform install/launch/upgrade/uninstall evidence; missing platform evidence blocks release. | R002-T026, R002-T034, R002-T029, R002-T035, R002-T036, R002-T037 | R002-T037 |
| H4 - Interactive execution from RouDex | v0.8.0 | planned | Review routes and concrete contracts, approve/deny, dispatch once, stream events and send interactive turns through Tauri IPC and the local App Server stdio coordinator; verified completion is distinct from runtime success. | R002-T010, R002-T011, R002-T012, R002-T013, R002-T014, R002-T015, R002-T016, R002-T017, R002-T022, R002-T023, R002-T024, R002-T050 | R002-T050 |
| H5 - SDD Studio for specs and task graphs | v0.9.0 | planned | Author and review versioned local specs, map acceptance to tasks, validate/import manifests and edit draft dependency graphs atomically without rewriting execution history. | R002-T006, R002-T007, R002-T008, R002-T009, R002-T038, R002-T039 | R002-T039 |
| H6 - Global Codex Desktop integration | v0.10.0 | planned | Global local-machine Desktop ingress is explicitly installed and reversible, preserves other configuration and routes each delivery to the coordinator once with native evidence. | R002-T040, R002-T041, R002-T042 | R002-T042 |
| H7 - Multi-project, queues and long-running executions | v0.11.0 | planned | Registered local projects remain isolated; persistent queues enforce eligibility and shared-write exclusion; long-running executions retain identity and bounded events across app reconnects. | R002-T043, R002-T044, R002-T045 | R002-T045 |
| H8 - Security, recovery and compatibility | v0.12.0 | planned | Audit permissions and sensitive-data boundaries, verify interruption/crash recovery and backup/restore, and prove data/runtime/platform compatibility without silent fallback. | R002-T018, R002-T019, R002-T020, R002-T021, R002-T025, R002-T027, R002-T028, R002-T046, R002-T047 | R002-T047 |
| H9 - Public beta and stable release | v1.0.0 | planned | Complete integrated and native acceptance, prepare the public beta, resolve blocking feedback and approve stable readiness with an auditable v1.0.0 release handoff; publication remains separately authorized. | R002-T030, R002-T031, R002-T032, R002-T048, R002-T049, R002-T033 | R002-T033 |

Milestone dependencies are H1 -> H2 -> H3 -> H4 -> H5 -> H6 -> H7 -> H8 -> H9.
The task graph encodes the corresponding release-gate prerequisites without
serializing independent work inside a milestone. Each gate transitively covers
all its milestone tasks and earlier gates. Shared writable scopes still require
serialized edits. A gate task verifies readiness; it never automatically creates
a tag, publishes a package or completes other tasks.

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

## Approved capability boundaries

- H3 owns native Tauri installers and onboarding for Windows, macOS and Linux.
  Formats, architectures and supported runtime versions are reviewed in the
  platform matrix before packaging. Each supported target needs actual native
  install/launch/upgrade/uninstall evidence; missing prerequisites stay blocked.
  Installation preserves local data and does not activate global hooks by default.
- H4 owns route configuration, contract approvals, single-task dispatch, interactive
  input/continuation, streaming events and verification/evidence in RouDex. Existing
  contracts can execute before SDD Studio; approvals and dependency eligibility
  are enforced by the coordinator. Advanced interruption/crash recovery belongs
  to H8; H4 reuses existing coordinator behavior without promising that hardening.
- H5 owns SDD Studio: local versioned specs, scenarios, decisions, task drafts,
  dependency graph editing, validation/import and acceptance-to-verification
  traceability. Only unexecuted drafts can change, with stale review invalidated.
- H6 owns global Codex Desktop integration on the local machine, with explicit
  activation/removal, configuration preservation and native delivery evidence.
  H1 ingress is retained as a baseline; it is not proof of the complete global
  product workflow. Global integration grants no automatic project permissions.
- H7 owns explicit multi-project registration/isolation, persistent queues and
  long-running execution visibility across UI disconnect/reconnect. Scheduling
  must honor approvals, dependencies and shared writable-scope exclusion.
- H8 owns security hardening, interruption/crash recovery, private local backups,
  guarded restore, sanitized export and platform/runtime/schema compatibility.
  It retains original identities and approvals without replaying completed work.
- H9 owns public beta preparation, observed feedback and blocker disposition,
  stable v1.0.0 readiness and the final release handoff. A beta prerelease label
  is a stage within H9; its exact label is selected in that task. Publication
  requires a separate explicit action approval and actual publication evidence.

## Non-goals for this alignment

- Implementing any roadmap feature or executing a planned task now.
- Adding frameworks, dependencies, graph libraries or remote access by default.
- Reading authentication configuration, environment values or unrelated projects.
- Automatically approving, replaying ambiguous work, installing global hooks,
  uploading evidence, committing, tagging or publishing packages.
- Treating builds, fixtures, routing receipts or model prose as native/live
  acceptance or marking unexecuted tasks completed.

## Acceptance scenarios and traceability

A01 is historical. All other scenarios are future gates. Named test targets and
new evidence artifacts are planned deliverables, not claims they exist or pass.
R002-T030 aggregates A02-A19 after all H2-H8 gate tasks pass. Individual task
criteria and native/platform observations remain mandatory.

| ID | Precondition | Action | Observable result | Tasks and verification |
| --- | --- | --- | --- | --- |
| A01 | Specification 001 and local v0.5.0 metadata exist. | Inspect completed records and qualifications. | H1 remains completed; historical limitations do not imply H2/native acceptance. | R002-T001; Historical manifest, audit and local tag. |
| A02 | Synthetic overview/detail snapshots include retries, empty state and errors. | Open, filter, refresh, select details and traverse at both requested viewports by keyboard. | Correct counts/selection/events and loading/empty/stale/error/focus states, with no writes; incomplete keyboard/viewport evidence remains blocking. | R002-T002, R002-T003, R002-T004, R002-T005, R002-T030; control_center tests, fixture builds and UI evidence. |
| A03 | Valid, cyclic, duplicate and colliding manifests/drafts exist. | Preview, import, reimport and edit drafts and dependency edges. | Atomic validation and persistence; invalid input writes nothing; history survives reimport; changes invalidate review; completed contracts are immutable. | R002-T006, R002-T007, R002-T008, R002-T009, R002-T039, R002-T030; roadmap_planning, planning_commands, sdd_traceability and editor/graph evidence. |
| A04 | Built-in routes and valid/invalid local overrides exist. | Preview, save, dispatch and continue an existing run after changing settings. | Preview equals dispatch; invalid settings cannot replace a revision; existing runs retain model/effort; unavailable runtime blocks without fallback. | R002-T010, R002-T011, R002-T012, R002-T013, R002-T050, R002-T030; roadmap_routing, routing_preview, routing_commands and interactive_execution. |
| A05 | Tasks are unapproved, denied, approved or changed after review. | Review concrete contracts and approve/deny or edit a draft. | Approval starts nothing; stale/denied review blocks; only the exact approved revision is eligible. | R002-T014, R002-T015, R002-T016, R002-T050, R002-T008, R002-T030; roadmap_approvals, approval_commands and interactive/editor evidence. |
| A06 | Eligible, dependency-blocked and runtime-failing tasks exist. | Dispatch twice and retry blocked/failing requests. | One eligible run with assigned route; durable blocked/failed reasons; no UI execution bypass. | R002-T015, R002-T017, R002-T050, R002-T030; approval_commands and interactive dispatch evidence. |
| A07 | Runs are active, interrupted, terminal or missing reliable identity. | Interrupt, inject terminal-persistence failure, restart and resume twice. | One terminal result; original assignment/run/thread identity; no replay; unsupported/ambiguous recovery blocks. | R002-T012, R002-T018, R002-T019, R002-T020, R002-T021, R002-T047, R002-T030; roadmap_recovery, recovery_commands, routing and compatibility tests. |
| A08 | Verification is successful, failed, absent or malformed. | Complete a runtime turn and inspect/paginate evidence. | Only independently verified success completes a task; bounded, run-isolated evidence exposes missing/failed checks. | R002-T022, R002-T023, R002-T024, R002-T050, R002-T030; roadmap_evidence, evidence_commands and interactive evidence. |
| A09 | Synthetic records contain sensitive markers and an existing destination. | Export an allowlisted local summary and inspect release contents. | No raw prompts/events, authentication data or personal paths; overwrite needs confirmation; nothing is sent externally. | R002-T025, R002-T030, R002-T031; evidence_export and boundary audit. |
| A10 | Runtime/config/storage are unavailable, incompatible or inaccessible. | Read local state, validate manifests and run readiness/boundary checks. | Actionable errors without accidental writes, implicit installation, dispatch, new listeners or wider path permissions. | R002-T002, R002-T026, R002-T029, R002-T006, R002-T030, R002-T031; control_center, roadmap_planning, diagnostics, packaging checklist and boundary audit. |
| A11 | Disposable stores contain concurrent writes, corruption and incompatible schemas. | Back up, preview/confirm restore and inject replacement failure. | Consistent backup; invalid/busy restore writes nothing; rollback preserves prior state; restore never executes tasks. | R002-T027, R002-T028, R002-T047, R002-T030; local_backup, local_restore and compatibility evidence. |
| A12 | All milestone features and integration checks have evidence. | Run integrated/native acceptance and prepare final release handoff. | Actual native actions/runtime provenance recorded; findings block readiness; publication remains a separate explicit gate. | R002-T030, R002-T031, R002-T032, R002-T033; Workspace tests/builds, native smoke, audit and release handoff. |
| A13 | Windows/macOS/Linux targets and disposable install environments are reviewed. | Build native installers, install/launch/upgrade/uninstall and complete first-run onboarding. | Per-platform package/checksum and observed lifecycle/onboarding evidence; state preserved; missing target/prerequisite blocks v0.7.0. | R002-T034, R002-T029, R002-T035, R002-T036, R002-T037, R002-T030; Platform matrix, installer build/smoke artifacts, onboarding tests/native evidence. |
| A14 | A local versioned spec and draft task graph are available. | Edit/review/save/reopen the spec, link acceptance and round-trip graph changes. | Versioned scenarios/decisions trace to tasks/checks; invalid/uncovered drafts block; no execution or history rewrite. | R002-T038, R002-T039, R002-T030; sdd_studio, sdd_traceability and native/fixture evidence. |
| A15 | Supported native Desktop, disposable cwd surfaces and explicit hook action approval exist. | Install global integration, submit/replay messages, test failure/recursion and uninstall. | One ingress and one explicit approved coordinator execution; other configuration preserved; unsupported/absent runtime blocks. | R002-T040, R002-T041, R002-T042, R002-T030; global_desktop tests plus actual native Desktop route/run/thread/turn evidence. |
| A16 | Two disposable registered projects and persistent queues exist. | Switch projects, enqueue/reorder/pause, schedule competing scopes and reconnect to a long run. | Project isolation, durable queues, eligibility and shared-write exclusion; bounded ordered events; no duplicate run or silent replay. | R002-T043, R002-T044, R002-T045, R002-T030; multi_project, task_queues, long_running and UI evidence. |
| A17 | Synthetic adversarial paths, runtime versions and upgrade fixtures exist. | Probe permissions/data boundaries and upgrade/rollback supported combinations. | Unauthorized access rejected; safe local-only exports; identities/history preserved; incompatibility and unresolved security findings block. | R002-T046, R002-T047, R002-T030; local_security, compatibility and matrix/audit artifacts. |
| A18 | Integrated/native gates pass and beta publication/observation has separate authorization. | Prepare beta, assess actual feedback/blockers, validate stable promotion and handoff. | Traceable beta results and resolved blockers required for v1.0.0 readiness; target, readiness and actual publication never conflated. | R002-T030, R002-T048, R002-T049, R002-T033; beta-evidence, stable-readiness, release-handoff and regression results. |
| A19 | An approved task/session and pending runtime approval request exist. | Send/continue interactive input, stream events and answer the exact pending request. | Coordinator owns stdio turns/permissions; input and approval replay cannot duplicate turns; native execution and verification receipts remain separate. | R002-T050, R002-T030; Core/native interactive_execution tests and installed-native interactive evidence. |

## Verification for this documentation alignment

- Parse JSON and validate required task fields, unique/stable ids, complete
  dependencies, route/profile assignments and acyclic task/milestone graphs.
- Check exact H1-H9 titles, versions, statuses, membership, release-gate ancestry,
  criteria/scenario mapping and human task details across all three artifacts.
- Compare against the pre-edit records: all prior statuses, route/run ids and
  evidence survive; H2 checks and unresolved gaps remain visible. Added tasks
  remain planned/manual with null execution receipts and empty evidence.
- Load the JSON with the existing Rust `TaskManifest` validator without importing
  it into operational storage or dispatching any task.
- Inspect whitespace, the changed-file list and boundary/sensitive-content scan.
  Only the three requested roadmap artifacts may change; no dependency or
  feature implementation is part of the alignment.

## Versioning, authoring receipts and completion

Keep `spec_id` and existing task identities stable. Increment version for
material contract/scope changes and update the three current artifacts together.
Changed approvals must be reviewed again. The original `authoring_run` object
is retained as historical authoring metadata; it does not describe this revision's
worker or prove product task completion. The enclosing Codex Router coordinator
owns this authoring worker's terminal state.

Only H1 is completed. H2-H9 and all unexecuted tasks remain planned until
authorized execution and independent verification. Release targets are fixed
by the approved plan; they are not assertions of available installers, passed
acceptance, publication or release dates. Closing a milestone requires every
task, its release gate and required manual/platform evidence; stable v1.0.0
also requires actual beta results and resolved blockers. Signed publication
and public availability remain separately authorized and evidenced.
