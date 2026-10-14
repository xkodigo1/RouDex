# Specification 002 task graph

Version: 1. Date: 2026-10-13. Status: planned; H1 baseline completed.

`tasks.json` is the authoritative executable contract. The task details below are its human-readable counterpart. R002-T001 is a historical baseline reference; R002-T002 through R002-T033 are planned and have no dispatch receipts. Creating this plan grants no approval to execute them.

## Execution contract

- Run from the repository root through Codex Router; workers must not recursively approve, dispatch or modify their own lifecycle.
- Resolve profile, model, effort, cwd, sandbox and the latest approval before dispatch. Missing runtime or approval blocks execution; never substitute a model silently.
- Built-in mappings: routine = gpt-6-luna/medium; standard = gpt-6.1-sol/medium; deep = gpt-6.1-sol/high; frontier = gpt-6-astra/high.
- Each task produces one reviewable outcome and can be committed independently. Shared-file tasks must be scheduled without overlapping writes even if their dependency branches are independent.
- Logical scope is narrower than filesystem permission: restrict edits to scope. Writable roots are existing module directories so canonicalization can succeed; ordinary ignored build outputs are verification artifacts, not permission to change unrelated sources.
- The manifest keeps all future tasks planned. At dispatch, unsatisfied dependencies or approvals become concrete blocked/awaiting_approval states. Dispatch receipt, running state and completed state are different; completion needs an exit-zero verification result and the stated acceptance evidence.
- New named integration test targets are deliverables of their introducing tasks. Later tasks extend that same target. A missing target or zero relevant tests is a failure, not a passing verification.
- Commands below use the existing Windows coordinator shell (`cmd.exe`); `&&` stops on failure. When running from PowerShell, execute commands separately and check `$LASTEXITCODE`, or invoke the documented coordinator path.
- UI builds prove compilation only. UI/native tasks additionally require the named evidence artifact with date, environment, each action, expected/observed result and unresolved failures. File existence checks do not establish correctness. Missing manual evidence blocks completion.
- Verification that needs dependencies absent from the local cache blocks until provisioning is separately approved. Do not fetch packages implicitly.
- Runtime evidence lives in the local store; public summaries contain only sanitized relative references and task/run ids. Never copy operational data or environment files into this spec.

## Dependency graph

| Task | Milestone | Status | Depends on | Profile |
| --- | --- | --- | --- | --- |
| R002-T001 | H1 | completed | - | routine |
| R002-T002 | H2 | planned | R002-T001 | standard |
| R002-T003 | H2 | planned | R002-T002 | standard |
| R002-T004 | H2 | planned | R002-T003 | standard |
| R002-T005 | H2 | planned | R002-T004 | routine |
| R002-T006 | H3 | planned | R002-T001 | deep |
| R002-T007 | H3 | planned | R002-T006 | standard |
| R002-T008 | H3 | planned | R002-T007, R002-T003 | standard |
| R002-T009 | H3 | planned | R002-T006, R002-T004 | standard |
| R002-T010 | H4 | planned | R002-T001 | deep |
| R002-T011 | H4 | planned | R002-T010 | standard |
| R002-T012 | H4 | planned | R002-T010 | deep |
| R002-T013 | H4 | planned | R002-T011, R002-T012, R002-T003 | standard |
| R002-T014 | H5 | planned | R002-T006 | deep |
| R002-T015 | H5 | planned | R002-T014 | deep |
| R002-T016 | H5 | planned | R002-T015, R002-T004, R002-T008 | standard |
| R002-T017 | H5 | planned | R002-T015, R002-T016, R002-T009 | standard |
| R002-T018 | H6 | planned | R002-T001 | deep |
| R002-T019 | H6 | planned | R002-T018 | deep |
| R002-T020 | H6 | planned | R002-T019, R002-T012, R002-T014 | deep |
| R002-T021 | H6 | planned | R002-T020, R002-T017 | standard |
| R002-T022 | H7 | planned | R002-T001 | deep |
| R002-T023 | H7 | planned | R002-T022 | standard |
| R002-T024 | H7 | planned | R002-T023, R002-T004 | standard |
| R002-T025 | H7 | planned | R002-T022 | deep |
| R002-T026 | H8 | planned | R002-T001 | routine |
| R002-T027 | H8 | planned | R002-T022 | deep |
| R002-T028 | H8 | planned | R002-T027, R002-T020 | deep |
| R002-T029 | H8 | planned | R002-T026, R002-T028 | routine |
| R002-T030 | H9 | planned | R002-T005, R002-T009, R002-T013, R002-T017, R002-T021, R002-T024, R002-T025, R002-T029 | frontier |
| R002-T031 | H9 | planned | R002-T030 | frontier |
| R002-T032 | H9 | planned | R002-T030 | standard |
| R002-T033 | H9 | planned | R002-T031, R002-T032 | routine |

The H2, H3, H4, H6, H7 and H8 entry tasks can begin after H1 once explicitly approved. H5 depends on H3 contract validation. Cross-milestone edges express real prerequisites; H9 aggregates every terminal branch. There is no requirement to finish an entire numbered milestone before starting an independent branch.

## Task details

### R002-T001 - Retain the completed orchestration baseline

- Milestone/status: H1 / completed.
- Dependencies: none.
- Route: routine / gpt-6-luna / medium.
- Approval: `not_required`. No execution requested; this is a completed baseline reference, not a new approval.
- Scope: `specs/001-task-orchestration`.
- Cwd: `.`; sandbox: `readOnly`; writable roots: none.
- Scenarios: A01.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Historical completion reference only. Inspect specification 001 and local v0.5.0 metadata if auditing; do not redispatch or rewrite H1.

Acceptance:

- Specification 001 records H1 as published v0.5.0 with T-001 through T-009 completed.
- Historical assignment discrepancy and native smoke limitations remain visible.

Verification:

```text
git show v0.5.0:specs/001-task-orchestration/tasks.json
```

Evidence artifact: `specs/001-task-orchestration/acceptance-evidence.md`. Review its observed results before completion; existence alone is insufficient.

This route is metadata for a possible read-only audit, not a claim that H1 ran under one model. Original implementation receipts stay in specification 001.

### R002-T002 - Define the control-center read model

- Milestone/status: H2 / planned.
- Dependencies: R002-T001.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src/lib.rs`, `src-tauri/tests/control_center.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`.
- Scenarios: A02, A10.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T002 entry in tasks.md. Extend local read-only snapshots with milestone totals, status counts and stable task/run selection keys. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Synthetic data yields correct totals without counting retries as tasks.
- Missing, unreadable and incompatible stores are distinct; reading does not create or migrate a store.

Verification:

```text
cargo test --offline -p roudex-desktop --test control_center
```

### R002-T003 - Build the milestone and task overview

- Milestone/status: H2 / planned.
- Dependencies: R002-T002.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/control-center-overview-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A02.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T003 entry in tasks.md. Create a local overview with milestone progress, status filters and stable selected-task state using the existing React stack. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Counts and filters match the snapshot; selection survives refresh when the task still exists.
- Loading, empty, stale and error states are visible; no mutation action is introduced.

Verification:

```text
pnpm --dir ui build --configLoader runner && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/control-center-overview-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/control-center-overview-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T004 - Add task and run detail navigation

- Milestone/status: H2 / planned.
- Dependencies: R002-T003.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/control-center-details-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A02.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T004 entry in tasks.md. Expose dependencies, profile/model/effort, approval state, run identity and ordered events for the selected task. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Switching tasks cannot display a late response from the prior run.
- Missing run identity and event-read failure have explicit states; dependency links select the correct task.

Verification:

```text
pnpm --dir ui build --configLoader runner && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/control-center-details-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/control-center-details-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T005 - Verify control-center accessibility and states

- Milestone/status: H2 / planned.
- Dependencies: R002-T004.
- Route: routine / gpt-6-luna / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/tests`, `src-tauri/tests/control_center.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`, `src-tauri`.
- Scenarios: A02.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T005 entry in tasks.md. Exercise all overview/detail fixture states and document keyboard, focus and viewport checks. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Keyboard navigation works with visible focus and labeled controls at 1280x720 and 800x600.
- Populated, empty, stale, missing-store and error fixtures have recorded observed results; fixture evidence is not labeled native evidence.

Verification:

```text
cargo test --offline -p roudex-desktop --test control_center && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/control-center-accessibility-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/control-center-accessibility-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T006 - Validate an import before persisting it

- Milestone/status: H3 / planned.
- Dependencies: R002-T001.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/task_manifest.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/tests/roadmap_planning.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A03, A10.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T006 entry in tasks.md. Add a non-mutating manifest preview that validates contracts and rejects cross-spec task-id collisions before an atomic import. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Duplicate ids, missing dependencies, cycles, route conflicts and unsafe scopes are rejected before any write.
- A task id owned by another spec is rejected rather than reassigned.

Verification:

```text
cargo test --offline -p router-core --test roadmap_planning
```

### R002-T007 - Expose reviewed manifest and draft persistence

- Milestone/status: H3 / planned.
- Dependencies: R002-T006.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src`, `src-tauri/tests/planning_commands.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/tests/roadmap_planning.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`, `crates/router-core`.
- Scenarios: A03.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T007 entry in tasks.md. Expose native commands for selecting a local manifest, previewing changes, importing a valid preview and saving only unexecuted drafts through the core boundary. Add the bounded persistence operation that revokes approval whenever a draft contract changes; reject updates to running or completed tasks. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Preview causes no task or approval mutation; import is an explicit separate action.
- Reimport is idempotent and preserves existing run history; changed input invalidates the preview.

- Draft save is atomic, revokes prior approval and refuses running/completed task updates; a failed save preserves the prior definition.

Verification:

```text
cargo test --offline -p roudex-desktop --test planning_commands && cargo test --offline -p router-core --test roadmap_planning
```

### R002-T008 - Add a bounded task draft editor

- Milestone/status: H3 / planned.
- Dependencies: R002-T007, R002-T003.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/task-editor-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A03, A05.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T008 entry in tasks.md. Add editing for unexecuted drafts with required contract fields and a preview-before-save flow. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Missing acceptance, verification, scope or routing fields disable submission with field errors.
- Running/completed tasks cannot be rewritten; editing an approved draft invalidates the prior approval.

Verification:

```text
pnpm --dir ui build --configLoader runner && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/task-editor-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/task-editor-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T009 - Show graph readiness and blockers

- Milestone/status: H3 / planned.
- Dependencies: R002-T006, R002-T004.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/task-graph-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A03.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T009 entry in tasks.md. Render a dependency list and readiness reasons from the validated task graph without introducing a graph library. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Each blocked task identifies its actual unmet dependency or approval; independent tasks remain eligible.
- Cycles and missing dependencies render validation errors, never a ready badge.

Verification:

```text
pnpm --dir ui build --configLoader runner && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/task-graph-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/task-graph-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T010 - Define validated local routing overrides

- Milestone/status: H4 / planned.
- Dependencies: R002-T001.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/routing.rs`, `crates/router-core/src/lib.rs`, `crates/router-core/tests/roadmap_routing.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A04.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T010 entry in tasks.md. Add a versioned local routing configuration contract while retaining all existing built-in defaults. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Invalid profile, model or effort values fail validation without overwriting the last valid configuration.
- Absent configuration retains the exact built-in mapping; configuration contains no authentication data.

Verification:

```text
cargo test --offline -p router-core --test roadmap_routing
```

### R002-T011 - Preview a route and its resolution reason

- Milestone/status: H4 / planned.
- Dependencies: R002-T010.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-cli/src/main.rs`, `crates/router-cli/tests/routing_preview.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-cli`.
- Scenarios: A04.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T011 entry in tasks.md. Expose a read-only route preview using the same resolver used by dispatch. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- The preview returns profile, model, effort and whether a built-in or explicit rule won.
- Preview neither creates a run nor starts an App Server turn; conflicting task overrides are rejected.

Verification:

```text
cargo test --offline -p router-cli --test routing_preview
```

### R002-T012 - Bind dispatch to an immutable routing revision

- Milestone/status: H4 / planned.
- Dependencies: R002-T010.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/coordinator.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/tests/roadmap_routing.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A04, A07.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T012 entry in tasks.md. Persist the validated routing revision with the dispatch definition and enforce it during continuation. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- A later configuration change cannot alter an already dispatched run.
- An unavailable assigned model produces a blocked reason with no silent fallback; synthetic capability fixtures cover it.

Verification:

```text
cargo test --offline -p router-core --test roadmap_routing
```

### R002-T013 - Expose routing settings and preview locally

- Milestone/status: H4 / planned.
- Dependencies: R002-T011, R002-T012, R002-T003.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src`, `src-tauri/tests/routing_commands.rs`, `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/routing-settings-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`, `ui`.
- Scenarios: A04.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T013 entry in tasks.md. Add local IPC and settings controls for validating, previewing and explicitly saving routing changes. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- The UI shows the resolved mapping before save and invalid values cannot be persisted.
- Save affects future dispatch only; the selected historical run retains its original mapping.

Verification:

```text
cargo test --offline -p roudex-desktop --test routing_commands && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/routing-settings-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/routing-settings-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T014 - Bind approvals to a task revision

- Milestone/status: H5 / planned.
- Dependencies: R002-T006.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/domain.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/src/coordinator.rs`, `crates/router-core/tests/roadmap_approvals.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A05.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T014 entry in tasks.md. Extend explicit approval records with a contract revision so stale approvals cannot authorize changed tasks. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- A scope, prompt, verification, route or sandbox change requires a new decision.
- Denial remains blocking; repeated identical decisions do not cause duplicate dispatch.

Verification:

```text
cargo test --offline -p router-core --test roadmap_approvals
```

### R002-T015 - Add explicit native approval and dispatch commands

- Milestone/status: H5 / planned.
- Dependencies: R002-T014.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src`, `src-tauri/tests/approval_commands.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`.
- Scenarios: A05, A06.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T015 entry in tasks.md. Expose separate decision and dispatch IPC commands that use the coordinator and reject stale contracts. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Approval alone never starts execution; dispatch enforces dependencies and the current approval.
- Double submission starts at most one run; IPC cannot execute arbitrary shell text or expand scope.

Verification:

```text
cargo test --offline -p roudex-desktop --test approval_commands
```

### R002-T016 - Build the review-and-approve interaction

- Milestone/status: H5 / planned.
- Dependencies: R002-T015, R002-T004, R002-T008.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/approval-review-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A05.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T016 entry in tasks.md. Show the concrete task contract and provide explicit approve/deny controls with pending and stale-response handling. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Scope, writable roots, model, effort and verification are visible before the decision.
- Denied, changed and unapproved tasks cannot be dispatched; failed decisions remain retryable without a false success state.

Verification:

```text
pnpm --dir ui build --configLoader runner && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/approval-review-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/approval-review-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T017 - Add single-task execution controls

- Milestone/status: H5 / planned.
- Dependencies: R002-T015, R002-T016, R002-T009.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `src-tauri/tests/approval_commands.rs`, `ui/tests/dispatch-controls-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`, `src-tauri`.
- Scenarios: A06.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T017 entry in tasks.md. Add an explicit run action for an eligible task and display the coordinator receipt. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Rapid repeated clicks yield one run; blocked receipts are shown as blocked rather than successful.
- A completed task is not dispatched again; router failure never falls back to execution from the UI.

Verification:

```text
cargo test --offline -p roudex-desktop --test approval_commands && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/dispatch-controls-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/dispatch-controls-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T018 - Define guarded interruption transitions

- Milestone/status: H6 / planned.
- Dependencies: R002-T001.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/domain.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/tests/roadmap_recovery.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A07.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T018 entry in tasks.md. Specify and persist an interruption request separately from the terminal result without treating cancellation as rollback. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Only an active run can receive an interruption request; terminal runs remain immutable.
- A request and terminal result racing each other produce one authoritative terminal state.

Verification:

```text
cargo test --offline -p router-core --test roadmap_recovery
```

### R002-T019 - Implement local App Server interruption

- Milestone/status: H6 / planned.
- Dependencies: R002-T018.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/app_server.rs`, `crates/router-core/src/coordinator.rs`, `crates/router-core/tests/roadmap_recovery.rs`, `crates/router-core/tests/support/fake_app_server.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A07.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T019 entry in tasks.md. Implement interruption against the locally verified App Server protocol and preserve run/thread identity. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Supported interruption records the request and observed outcome; unsupported protocol blocks with an actionable reason.
- Disconnect or timeout is not reported as successful cancellation; no replacement thread starts implicitly.

Verification:

```text
cargo test --offline -p router-core --test roadmap_recovery
```

### R002-T020 - Validate crash recovery and idempotent resume

- Milestone/status: H6 / planned.
- Dependencies: R002-T019, R002-T012, R002-T014.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/coordinator.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/tests/roadmap_recovery.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A07.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T020 entry in tasks.md. Harden existing continuation for crash windows using persisted identities, immutable assignments and current approval gates. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Restart before/after terminal persistence cannot duplicate completed work or terminal events.
- Missing or conflicting thread/turn identity remains blocked; resume preserves the route and run id.

Verification:

```text
cargo test --offline -p router-core --test roadmap_recovery
```

### R002-T021 - Expose interruption and resume controls

- Milestone/status: H6 / planned.
- Dependencies: R002-T020, R002-T017.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src`, `src-tauri/tests/recovery_commands.rs`, `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/recovery-controls-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`, `ui`.
- Scenarios: A07.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T021 entry in tasks.md. Expose explicit interruption and continuation actions through the coordinator with visible state and reasons. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Interrupted tasks offer resume by the existing run id; completed runs cannot be restarted by this action.
- The UI shows that interruption does not undo filesystem changes; denied approvals and unsupported operations stay blocked.

Verification:

```text
cargo test --offline -p roudex-desktop --test recovery_commands && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/recovery-controls-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/recovery-controls-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T022 - Persist structured verification evidence

- Milestone/status: H7 / planned.
- Dependencies: R002-T001.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/domain.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/src/coordinator.rs`, `crates/router-core/tests/roadmap_evidence.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A08.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T022 entry in tasks.md. Attach verification command, exit status, artifact references and task/run identity to a durable evidence record. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Model success text without successful verification cannot complete a task.
- Failed or missing verification evidence stays failed/blocked; migration tests preserve H1 history and do not rewrite it.

Verification:

```text
cargo test --offline -p router-core --test roadmap_evidence
```

### R002-T023 - Add a bounded evidence read model

- Milestone/status: H7 / planned.
- Dependencies: R002-T022.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src`, `src-tauri/tests/evidence_commands.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`.
- Scenarios: A08.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T023 entry in tasks.md. Read verification evidence and ordered lifecycle events with explicit pagination and run isolation. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Large synthetic histories are bounded and counts indicate omitted records.
- Switching run ids cannot return another run evidence; malformed payloads have an explicit error representation.

Verification:

```text
cargo test --offline -p roudex-desktop --test evidence_commands
```

### R002-T024 - Show verification and evidence in run details

- Milestone/status: H7 / planned.
- Dependencies: R002-T023, R002-T004.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/evidence-details-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A08.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T024 entry in tasks.md. Extend details with command results, artifact references and the distinction between runtime completion and verified completion. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- The UI links each result to its task/run and displays nonzero exit codes and missing artifacts.
- Fixture-only, native and live-runtime evidence are visibly distinguishable.

Verification:

```text
pnpm --dir ui build --configLoader runner && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/evidence-details-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/evidence-details-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T025 - Export a sanitized local evidence bundle

- Milestone/status: H7 / planned.
- Dependencies: R002-T022.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve the implementation contract; each later export requires a selected local destination and separate explicit overwrite confirmation. No external send is authorized.
- Scope: `crates/router-cli/src/main.rs`, `crates/router-core/src`, `crates/router-cli/tests/evidence_export.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-cli`, `crates/router-core`.
- Scenarios: A09.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T025 entry in tasks.md. Add explicit local export of an allowlisted evidence summary without raw prompts, raw event bodies or authentication material. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Export uses only reviewed fields and relative artifact references; synthetic sensitive markers and absolute paths are omitted.
- Export creates no network connection and refuses to overwrite an existing file unless explicitly confirmed.

Verification:

```text
cargo test --offline -p router-cli --test evidence_export
```

### R002-T026 - Add local readiness diagnostics

- Milestone/status: H8 / planned.
- Dependencies: R002-T001.
- Route: routine / gpt-6-luna / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-cli/src/main.rs`, `crates/router-cli/tests/local_diagnostics.rs`, `docs/installation.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-cli`, `docs`.
- Scenarios: A10.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T026 entry in tasks.md. Report local runtime, storage and hook readiness without reading authentication files or changing installation. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Missing runtime, unwritable data storage and absent hook have distinct actionable diagnostics.
- Diagnostics disclose no environment values or credentials and never install or dispatch anything.

Verification:

```text
cargo test --offline -p router-cli --test local_diagnostics
```

### R002-T027 - Create a consistent local backup

- Milestone/status: H8 / planned.
- Dependencies: R002-T022.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve implementation and use synthetic stores for verification. Backing up operational data later requires a user-selected destination; backup data is private and never a publication artifact.
- Scope: `crates/router-core/src/persistence.rs`, `crates/router-cli/src/main.rs`, `crates/router-cli/tests/local_backup.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `crates/router-cli`.
- Scenarios: A11.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T027 entry in tasks.md. Create a consistent SQLite backup at an explicitly selected local destination using the existing SQLite stack. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Backup made during synthetic writes opens successfully with consistent task/run/evidence references.
- Source remains unchanged; destination collision fails closed and the backup excludes authentication configuration.

Verification:

```text
cargo test --offline -p router-cli --test local_backup
```

### R002-T028 - Validate restore with rollback protection

- Milestone/status: H8 / planned.
- Dependencies: R002-T027, R002-T020.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve implementation using disposable databases only. An operational restore requires a separate explicit confirmation of source, destination and rollback copy before replacing any local data.
- Scope: `crates/router-core/src/persistence.rs`, `crates/router-cli/src/main.rs`, `crates/router-cli/tests/local_restore.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `crates/router-cli`.
- Scenarios: A11.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T028 entry in tasks.md. Add preview/validate/restore with exclusive coordination, source compatibility checks and an automatic rollback copy. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Corrupt, incompatible and busy destinations are rejected before replacement.
- Injected replacement failure restores the previous state; repeated restore never dispatches tasks or resets completed history.

Verification:

```text
cargo test --offline -p router-cli --test local_restore
```

### R002-T029 - Prepare reproducible local packaging instructions

- Milestone/status: H8 / planned.
- Dependencies: R002-T026, R002-T028.
- Route: routine / gpt-6-luna / medium.
- Approval: `manual`. Approve the documentation/configuration contract. Installing hooks, changing machine-wide settings, deleting user data or acquiring new build dependencies needs a separate explicit action.
- Scope: `docs/installation.md`, `docs/local-packaging.md`, `src-tauri/tauri.conf.json`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `docs`, `src-tauri`.
- Scenarios: A10, A12.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T029 entry in tasks.md. Document and validate the existing desktop build and a local install/upgrade/uninstall checklist without adding a packaging framework. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Instructions use the existing Rust/Tauri/React stack and explicitly preserve local data on uninstall by default.
- The checklist covers missing prerequisites and recovery; no auto-update endpoint, remote listener or automatic hook activation is added.

Verification:

```text
cargo build --workspace --offline && pnpm --dir ui build --configLoader runner && git diff --check && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'docs/local-packaging.md')) { exit 1 }"
```

Evidence artifact: `docs/local-packaging.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T030 - Run the integrated product acceptance matrix

- Milestone/status: H9 / planned.
- Dependencies: R002-T005, R002-T009, R002-T013, R002-T017, R002-T021, R002-T024, R002-T025, R002-T029.
- Route: frontier / gpt-6-astra / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/tests`, `crates/router-cli/tests`, `src-tauri/tests`, `ui/tests`, `specs/002-product-roadmap`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `crates/router-cli`, `src-tauri`, `ui`, `specs/002-product-roadmap`.
- Scenarios: A02, A03, A04, A05, A06, A07, A08, A09, A10, A11, A12.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T030 entry in tasks.md. Add integrated synthetic acceptance coverage and collect results for A02-A12 after every feature branch is complete. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Import, preview, approval, dispatch, interruption, resume, verification and export form one verified local workflow.
- Every automated scenario maps to a named test; failure injection covers rejection paths and each result has actual output.

Verification:

```text
cargo test --workspace --offline && pnpm --dir ui build --configLoader runner && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/acceptance-evidence.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/acceptance-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T031 - Audit local boundaries and release contents

- Milestone/status: H9 / planned.
- Dependencies: R002-T030.
- Route: frontier / gpt-6-astra / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `specs/002-product-roadmap`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `specs/002-product-roadmap`.
- Scenarios: A09, A10, A12.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T031 entry in tasks.md. Audit the concrete release diff and package for unintended remote surfaces, unsafe scopes, sensitive data and missing evidence; record findings without fixing implementation in this task. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- No new listener, remote synchronization, telemetry or authentication artifact is present.
- Every finding has reproduction evidence and a disposition; unresolved critical findings block release readiness and require a separate scoped fix task.

Verification:

```text
git diff --check && cargo test --workspace --offline && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/boundary-audit.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/boundary-audit.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T032 - Verify the native local product workflow

- Milestone/status: H9 / planned.
- Dependencies: R002-T030.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this native smoke run with disposable data. Any live model task needs a separately reviewed and approved contract; no operational task, existing hook or user database may be changed.
- Scope: `specs/002-product-roadmap`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `specs/002-product-roadmap`.
- Scenarios: A12.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T032 entry in tasks.md. Perform a native Tauri smoke test against disposable local data and record operator actions, runtime versions and observed results. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- A native launch exercises overview, import, approval, dispatch, recovery and evidence using an approved local runtime or explicitly labeled fixture.
- A browser fixture cannot satisfy native acceptance; live model execution requires its own route/run/thread/turn and verification receipt. Unavailable native/runtime access remains blocked.

Verification:

```text
cargo build --workspace --offline && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/native-smoke-evidence.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/native-smoke-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T033 - Prepare the evidence-backed release handoff

- Milestone/status: H9 / planned.
- Dependencies: R002-T031, R002-T032.
- Route: routine / gpt-6-luna / medium.
- Approval: `manual`. Approve the documentation task only. Git publication, tags, releases, uploads, remote access and machine installation remain separate explicitly authorized operations.
- Scope: `specs/002-product-roadmap`, `docs/local-packaging.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `specs/002-product-roadmap`, `docs`.
- Scenarios: A12.
- Route/run ids: none for this roadmap task; populate only after actual dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T033 entry in tasks.md. Prepare release notes, milestone/task evidence mapping and local packaging handoff only after acceptance and native gates pass. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- H2-H9 can be marked completed only when all required task verification, manual artifacts and blocker dispositions are present.
- The handoff distinguishes local readiness from publication and lists an unexecuted signed-commit/tag/release procedure with explicit publication approval.

Verification:

```text
git diff --check && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/release-handoff.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/release-handoff.md`. Review its observed results before completion; existence alone is insufficient.

Actual verification record (2026-10-13):

- Existing browser fixture: overview checks PASS (21); detail checks PASS (20). State observations: populated (4 tasks/3 runs), empty (explicit empty snapshot), stale read error (error and stale snapshot retain selection), missing store (explicit), and event error/retry (explicit failure, successful read-only retry).
- Keyboard/focus: Tab focused the first fixture control; visible 2 px light-blue outline observed. Complete app keyboard navigation was not verified.
- Requested viewports 1280x720 and 800x600: not verified. Available browser surface showed 2560x1344 and exposed no viewport override. Evidence artifact records this gap; no native evidence is claimed.
- `cargo test --offline -p roudex-desktop --test control_center`: PASS, exit 0, 5 passed / 0 failed.
- `pnpm --dir ui build --config tests/vite.config.js --configLoader runner`: PASS, exit 0, Vite 7.3.7, 33 modules.
- Evidence existence check: PASS, exit 0. `git diff --check`: PASS, exit 0 (Git line-ending notice only).
- Result: evidence added, acceptance incomplete; task remains planned and H2 is not complete.

