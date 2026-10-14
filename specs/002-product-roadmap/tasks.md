# Specification 002 task graph

Version: 2. Date: 2026-10-13. Status: planned; H1 baseline completed.

`tasks.json` is the authoritative executable contract; `spec.md` defines the
approved roadmap. This document is their human-readable counterpart. There
are 50 tasks: R002-T001 is the completed historical baseline, R002-T010 is
completed, and the other 48 remain planned. Existing evidence and route/run fields are retained;
creating or aligning this plan grants no future execution or publication approval.
`decisions.md` is the historical version 1 decision record; version 2 of the
three current artifacts supersedes conflicting milestone numbering/scope.
The original `authoring_run` metadata is historical, not a new dispatch receipt.

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


## Approved milestones and release targets

| Milestone | Release target | Status | Exit criteria | Tasks | Release gate task |
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

Release progression: H1 -> H2 -> H3 -> H4 -> H5 -> H6 -> H7 -> H8 -> H9.
The listed gate depends transitively on all tasks in its milestone and earlier
gates. Gate readiness and actual publication are distinct. No feature task is
implemented or newly marked completed by this documentation revision.

## Dependency graph

| Task | Milestone | Release target | Status | Depends on | Profile |
| --- | --- | --- | --- | --- | --- |
| R002-T001 | H1 | v0.5.0 | completed | - | routine |
| R002-T002 | H2 | v0.6.0 | planned | R002-T001 | standard |
| R002-T003 | H2 | v0.6.0 | planned | R002-T002 | standard |
| R002-T004 | H2 | v0.6.0 | planned | R002-T003 | standard |
| R002-T005 | H2 | v0.6.0 | planned | R002-T004 | routine |
| R002-T026 | H3 | v0.7.0 | planned | R002-T005 | routine |
| R002-T034 | H3 | v0.7.0 | planned | R002-T005 | routine |
| R002-T029 | H3 | v0.7.0 | planned | R002-T026, R002-T034 | routine |
| R002-T035 | H3 | v0.7.0 | planned | R002-T026, R002-T029, R002-T034 | standard |
| R002-T036 | H3 | v0.7.0 | planned | R002-T035 | standard |
| R002-T037 | H3 | v0.7.0 | planned | R002-T026, R002-T029, R002-T036 | standard |
| R002-T010 | H4 | v0.8.0 | completed | R002-T001 | deep |
| R002-T011 | H4 | v0.8.0 | planned | R002-T010 | standard |
| R002-T012 | H4 | v0.8.0 | planned | R002-T010 | deep |
| R002-T013 | H4 | v0.8.0 | planned | R002-T011, R002-T012, R002-T003 | standard |
| R002-T014 | H4 | v0.8.0 | planned | R002-T001 | deep |
| R002-T015 | H4 | v0.8.0 | planned | R002-T014 | deep |
| R002-T016 | H4 | v0.8.0 | planned | R002-T015, R002-T004 | standard |
| R002-T017 | H4 | v0.8.0 | planned | R002-T015, R002-T016, R002-T013 | standard |
| R002-T022 | H4 | v0.8.0 | planned | R002-T001 | deep |
| R002-T023 | H4 | v0.8.0 | planned | R002-T022 | standard |
| R002-T024 | H4 | v0.8.0 | planned | R002-T023, R002-T004, R002-T017 | standard |
| R002-T050 | H4 | v0.8.0 | planned | R002-T013, R002-T017, R002-T024, R002-T037 | deep |
| R002-T006 | H5 | v0.9.0 | planned | R002-T050 | deep |
| R002-T007 | H5 | v0.9.0 | planned | R002-T006 | standard |
| R002-T008 | H5 | v0.9.0 | planned | R002-T007, R002-T003 | standard |
| R002-T009 | H5 | v0.9.0 | planned | R002-T006, R002-T004 | standard |
| R002-T038 | H5 | v0.9.0 | planned | R002-T007, R002-T050 | standard |
| R002-T039 | H5 | v0.9.0 | planned | R002-T008, R002-T009, R002-T038 | deep |
| R002-T040 | H6 | v0.10.0 | planned | R002-T039 | deep |
| R002-T041 | H6 | v0.10.0 | planned | R002-T040 | deep |
| R002-T042 | H6 | v0.10.0 | planned | R002-T040, R002-T041 | standard |
| R002-T043 | H7 | v0.11.0 | planned | R002-T042 | deep |
| R002-T044 | H7 | v0.11.0 | planned | R002-T043, R002-T017 | deep |
| R002-T045 | H7 | v0.11.0 | planned | R002-T043, R002-T044 | deep |
| R002-T018 | H8 | v0.12.0 | planned | R002-T017 | deep |
| R002-T019 | H8 | v0.12.0 | planned | R002-T018 | deep |
| R002-T020 | H8 | v0.12.0 | planned | R002-T019, R002-T012, R002-T014, R002-T045 | deep |
| R002-T021 | H8 | v0.12.0 | planned | R002-T020, R002-T017 | standard |
| R002-T025 | H8 | v0.12.0 | planned | R002-T022, R002-T045 | deep |
| R002-T027 | H8 | v0.12.0 | planned | R002-T022, R002-T045 | deep |
| R002-T028 | H8 | v0.12.0 | planned | R002-T027, R002-T020 | deep |
| R002-T046 | H8 | v0.12.0 | planned | R002-T042, R002-T044 | frontier |
| R002-T047 | H8 | v0.12.0 | planned | R002-T021, R002-T025, R002-T028, R002-T045, R002-T046 | deep |
| R002-T030 | H9 | v1.0.0 | planned | R002-T005, R002-T037, R002-T050, R002-T039, R002-T042, R002-T045, R002-T047 | frontier |
| R002-T031 | H9 | v1.0.0 | planned | R002-T030 | frontier |
| R002-T032 | H9 | v1.0.0 | planned | R002-T030 | standard |
| R002-T048 | H9 | v1.0.0 | planned | R002-T031, R002-T032 | standard |
| R002-T049 | H9 | v1.0.0 | planned | R002-T048 | frontier |
| R002-T033 | H9 | v1.0.0 | planned | R002-T031, R002-T032, R002-T049 | routine |

## Task details

### R002-T001 - Retain the completed orchestration baseline

- Milestone/status: H1 / completed.
- Release target: `v0.5.0`.
- Dependencies: none.
- Route: routine / gpt-6-luna / medium.
- Approval: `not_required`. No execution requested; this is a completed baseline reference, not a new approval.
- Scope: `specs/001-task-orchestration`.
- Cwd: `.`; sandbox: `readOnly`; writable roots: none.
- Scenarios: A01.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Historical completion reference only. Inspect specification 001 and local v0.5.0 metadata if auditing; do not redispatch or rewrite H1.

Acceptance:

- Specification 001 records H1 as published v0.5.0 with T-001 through T-009 completed.
- Historical assignment discrepancy and native smoke limitations remain visible.

Verification:

```text
git show v0.5.0:specs/001-task-orchestration/tasks.json
```

Evidence artifact: `specs/001-task-orchestration/acceptance-evidence.md`. Review its observed results before completion; existence alone is insufficient.

Retained evidence:

- specs/001-task-orchestration/spec.md
- specs/001-task-orchestration/tasks.json
- specs/001-task-orchestration/acceptance-evidence.md
- specs/001-task-orchestration/release-notes.md

### R002-T002 - Define the control-center read model

- Milestone/status: H2 / planned.
- Release target: `v0.6.0`.
- Dependencies: R002-T001.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src/lib.rs`, `src-tauri/tests/control_center.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`.
- Scenarios: A02, A10.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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
- Release target: `v0.6.0`.
- Dependencies: R002-T002.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/control-center-overview-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A02.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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
- Release target: `v0.6.0`.
- Dependencies: R002-T003.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/control-center-details-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A02.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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
- Release target: `v0.6.0`.
- Dependencies: R002-T004.
- Route: routine / gpt-6-luna / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/tests`, `src-tauri/tests/control_center.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`, `src-tauri`.
- Scenarios: A02.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T005 entry in tasks.md. Exercise all overview/detail fixture states and document keyboard, focus and viewport checks. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Keyboard navigation works with visible focus and labeled controls at 1280x720 and 800x600.
- Populated, empty, stale, missing-store and error fixtures have recorded observed results; fixture evidence is not labeled native evidence.

Verification:

```text
cargo test --offline -p roudex-desktop --test control_center && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/control-center-accessibility-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/control-center-accessibility-evidence.md`. Review its observed results before completion; existence alone is insufficient.

Retained evidence:

- ui/tests/control-center-accessibility-evidence.md
- Fixture overview: PASS 21 checks; details: PASS 20 checks; cargo test offline: PASS 5/5; fixture build: PASS (33 modules). Playwright browser fixture: 1280x720 scrollWidth=1265 and 800x600 scrollWidth=785; no horizontal overflow. At 800x600, Tab traversed controls and left visible focus on Actualizar. Temporary screenshots were not retained. Browser fixture evidence only; native evidence remains outstanding. Acceptance incomplete; status remains planned.

Actual verification record (2026-10-13):

- Existing browser fixture: overview checks PASS (21); detail checks PASS (20). State observations: populated (4 tasks/3 runs), empty (explicit empty snapshot), stale read error (error and stale snapshot retain selection), missing store (explicit), and event error/retry (explicit failure, successful read-only retry).
- Playwright browser fixture: at 1280x720, document.documentElement.scrollWidth=1265; at 800x600, scrollWidth=785; no horizontal overflow. At 800x600, Tab traversed controls and left visible focus on Actualizar. Temporary screenshots were not retained. Browser evidence is distinct from native evidence; native checks remain outstanding.
- `cargo test --offline -p roudex-desktop --test control_center`: PASS, exit 0, 5 passed / 0 failed.
- `pnpm --dir ui build --config tests/vite.config.js --configLoader runner`: PASS, exit 0, Vite 7.3.7, 33 modules.
- Evidence existence check: PASS, exit 0. `git diff --check`: PASS, exit 0 (Git line-ending notice only).
- Result: evidence added, acceptance incomplete; task remains planned and H2 is not complete.

### R002-T026 - Add local readiness diagnostics

- Milestone/status: H3 / planned.
- Release target: `v0.7.0`.
- Dependencies: R002-T005.
- Route: routine / gpt-6-luna / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-cli/src/main.rs`, `crates/router-cli/tests/local_diagnostics.rs`, `docs/installation.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-cli`, `docs`.
- Scenarios: A10.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T026 entry in tasks.md. Report local runtime, storage and hook readiness without reading authentication files or changing installation. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Missing runtime, unwritable data storage and absent hook have distinct actionable diagnostics.
- Diagnostics disclose no environment values or credentials and never install or dispatch anything.

Verification:

```text
cargo test --offline -p router-cli --test local_diagnostics
```

### R002-T034 - Define the multiplatform installer matrix

- Milestone/status: H3 / planned.
- Release target: `v0.7.0`.
- Dependencies: R002-T005.
- Route: routine / gpt-6-luna / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `docs/platform-matrix.md`, `docs/local-packaging.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `docs`.
- Scenarios: A13.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T034 entry in tasks.md. Define supported Windows, macOS and Linux targets, native Tauri bundle formats, architecture coverage, prerequisites and reproducible local packaging checks. Present any required dependency or platform-tool choice before acquiring it. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- The matrix names supported OS/architecture/runtime versions, native bundle formats and per-target verification owners; unsupported combinations are explicit.
- Each supported platform requires real installer and native launch evidence; cross-compilation or a browser build cannot replace it.
- Required signing/notarization prerequisites are recorded without credentials; unavailable prerequisites block the affected distribution gate.

Verification:

```text
git diff --check && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'docs/platform-matrix.md')) { exit 1 }"
```

Evidence artifact: `docs/platform-matrix.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T029 - Prepare reproducible local packaging instructions

- Milestone/status: H3 / planned.
- Release target: `v0.7.0`.
- Dependencies: R002-T026, R002-T034.
- Route: routine / gpt-6-luna / medium.
- Approval: `manual`. Approve the documentation/configuration contract. Installing hooks, changing machine-wide settings, deleting user data or acquiring new build dependencies needs a separate explicit action.
- Scope: `docs/installation.md`, `docs/local-packaging.md`, `src-tauri/tauri.conf.json`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `docs`, `src-tauri`.
- Scenarios: A10, A13.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T029 entry in tasks.md. Document and validate the existing desktop build and a local install/upgrade/uninstall checklist without adding a packaging framework. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Instructions use the existing Rust/Tauri/React stack and explicitly preserve local data on uninstall by default.
- The checklist covers missing prerequisites and recovery; no auto-update endpoint, remote listener or automatic hook activation is added.
- Document Windows, macOS and Linux build/bundle prerequisites and the platform matrix; packaging instructions alone do not prove installers work.

Verification:

```text
cargo build --workspace --offline && pnpm --dir ui build --configLoader runner && git diff --check && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'docs/local-packaging.md')) { exit 1 }"
```

Evidence artifact: `docs/local-packaging.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T035 - Build native multiplatform installers

- Milestone/status: H3 / planned.
- Release target: `v0.7.0`.
- Dependencies: R002-T026, R002-T029, R002-T034.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `src-tauri/tauri.conf.json`, `scripts`, `docs/local-packaging.md`, `specs/002-product-roadmap/installer-build-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`, `scripts`, `docs`, `specs/002-product-roadmap`.
- Scenarios: A13.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T035 entry in tasks.md. Configure reproducible native installers using the existing Tauri bundler for every supported platform in the reviewed matrix, with local data preservation and no automatic hook activation. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Windows, macOS and Linux installer artifacts have platform/architecture/version, build command, checksum and actual native build results recorded.
- Bundles use the established stack and exclude private state, prompts, environment files and authentication data; no remote listener or automatic updater is enabled.
- Uninstall preserves local state by default; installers never activate machine-wide hooks implicitly.

Verification:

```text
cargo build --workspace --offline && pnpm --dir ui build --configLoader runner && git diff --check && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/installer-build-evidence.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/installer-build-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T036 - Verify install and lifecycle on each platform

- Milestone/status: H3 / planned.
- Release target: `v0.7.0`.
- Dependencies: R002-T035.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve the native installer smoke contract and each disposable target installation. No operational data, machine-wide hook or existing user settings may be changed.
- Scope: `specs/002-product-roadmap/installer-smoke-evidence.md`, `docs/local-packaging.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `specs/002-product-roadmap`, `docs`.
- Scenarios: A13.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T036 entry in tasks.md. Exercise each native installer on disposable environments and record installation, first launch, upgrade and uninstall behavior for the full matrix. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Every supported Windows, macOS and Linux target has observed install/launch/upgrade/uninstall results and package provenance.
- Upgrade and uninstall preserve synthetic user data; failed or missing platform runs remain blocking and are never reported as passing.
- Missing runtime prerequisites give actionable local errors without implicit downloads or global configuration changes.

Verification:

```text
git diff --check && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/installer-smoke-evidence.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/installer-smoke-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T037 - Add and verify first-run onboarding

- Milestone/status: H3 / planned.
- Release target: `v0.7.0`.
- Dependencies: R002-T026, R002-T029, R002-T036.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `ui/src`, `src-tauri/src`, `src-tauri/tests/onboarding.rs`, `ui/tests/onboarding-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`, `src-tauri`.
- Scenarios: A13.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T037 entry in tasks.md. Guide the operator through local workspace selection, data location and runtime diagnostics without dispatching work or installing the global Desktop hook; verify onboarding using installed native packages. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- First-run and repeat-launch paths show local state ownership, prerequisites and the selected workspace; cancellation does not initialize unrelated paths.
- Missing Codex runtime blocks execution with remediation and never requests or embeds credentials; onboarding can be completed without running a task.
- Native onboarding passes on all supported installed platforms and H2 read-only checks remain passing.

Verification:

```text
cargo test --offline -p roudex-desktop --test onboarding && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/onboarding-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/onboarding-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T010 - Define validated local routing overrides

- Milestone/status: H4 / completed.
- Release target: `v0.8.0`.
- Dependencies: R002-T001.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/routing.rs`, `crates/router-core/src/lib.rs`, `crates/router-core/tests/roadmap_routing.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A04.
- Route/run ids: `698ee6be-683c-4ffd-821b-27fe0d307b88` / `39a46088-af40-445c-80ca-f456894f7ceb`.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T010 entry in tasks.md. Add a versioned local routing configuration contract while retaining all existing built-in defaults. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Invalid profile, model or effort values fail validation without overwriting the last valid configuration.
- Absent configuration retains the exact built-in mapping; configuration contains no authentication data.

Actual verification:

- `cargo test --offline -p router-core --test roadmap_routing`: PASS, 8 tests.
- `cargo test --offline -p router-core`: PASS, 40 tests.
- `git diff --check`: PASS; routed verification reported a clean worktree.
- Implementation published in signed commit `9467128`; no dependencies were installed and no credentials were added.

Verification:

```text
cargo test --offline -p router-core --test roadmap_routing
```

### R002-T011 - Preview a route and its resolution reason

- Milestone/status: H4 / planned.
- Release target: `v0.8.0`.
- Dependencies: R002-T010.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-cli/src/main.rs`, `crates/router-cli/tests/routing_preview.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-cli`.
- Scenarios: A04.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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
- Release target: `v0.8.0`.
- Dependencies: R002-T010.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/coordinator.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/tests/roadmap_routing.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A04, A07.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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
- Release target: `v0.8.0`.
- Dependencies: R002-T011, R002-T012, R002-T003.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src`, `src-tauri/tests/routing_commands.rs`, `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/routing-settings-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`, `ui`.
- Scenarios: A04.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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

- Milestone/status: H4 / planned.
- Release target: `v0.8.0`.
- Dependencies: R002-T001.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/domain.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/src/coordinator.rs`, `crates/router-core/tests/roadmap_approvals.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A05.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T014 entry in tasks.md. Extend explicit approval records with a contract revision so stale approvals cannot authorize changed tasks. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- A scope, prompt, verification, route or sandbox change requires a new decision.
- Denial remains blocking; repeated identical decisions do not cause duplicate dispatch.
- Use the existing task contract validator; SDD Studio import/editor work is not a prerequisite for execution review.

Verification:

```text
cargo test --offline -p router-core --test roadmap_approvals
```

### R002-T015 - Add explicit native approval and dispatch commands

- Milestone/status: H4 / planned.
- Release target: `v0.8.0`.
- Dependencies: R002-T014.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src`, `src-tauri/tests/approval_commands.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`.
- Scenarios: A05, A06.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T015 entry in tasks.md. Expose separate decision and dispatch IPC commands that use the coordinator and reject stale contracts. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Approval alone never starts execution; dispatch enforces dependencies and the current approval.
- Double submission starts at most one run; IPC cannot execute arbitrary shell text or expand scope.

Verification:

```text
cargo test --offline -p roudex-desktop --test approval_commands
```

### R002-T016 - Build the review-and-approve interaction

- Milestone/status: H4 / planned.
- Release target: `v0.8.0`.
- Dependencies: R002-T015, R002-T004.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/approval-review-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A05.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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

- Milestone/status: H4 / planned.
- Release target: `v0.8.0`.
- Dependencies: R002-T015, R002-T016, R002-T013.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `src-tauri/tests/approval_commands.rs`, `ui/tests/dispatch-controls-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`, `src-tauri`.
- Scenarios: A06.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T017 entry in tasks.md. Add an explicit run action for an eligible task and display the coordinator receipt. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Rapid repeated clicks yield one run; blocked receipts are shown as blocked rather than successful.
- A completed task is not dispatched again; router failure never falls back to execution from the UI.
- Eligibility and dependency blockers come from the coordinator; execution does not depend on the later SDD graph editor.

Verification:

```text
cargo test --offline -p roudex-desktop --test approval_commands && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/dispatch-controls-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/dispatch-controls-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T022 - Persist structured verification evidence

- Milestone/status: H4 / planned.
- Release target: `v0.8.0`.
- Dependencies: R002-T001.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/domain.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/src/coordinator.rs`, `crates/router-core/tests/roadmap_evidence.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A08.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T022 entry in tasks.md. Attach verification command, exit status, artifact references and task/run identity to a durable evidence record. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Model success text without successful verification cannot complete a task.
- Failed or missing verification evidence stays failed/blocked; migration tests preserve H1 history and do not rewrite it.

Verification:

```text
cargo test --offline -p router-core --test roadmap_evidence
```

### R002-T023 - Add a bounded evidence read model

- Milestone/status: H4 / planned.
- Release target: `v0.8.0`.
- Dependencies: R002-T022.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src`, `src-tauri/tests/evidence_commands.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`.
- Scenarios: A08.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T023 entry in tasks.md. Read verification evidence and ordered lifecycle events with explicit pagination and run isolation. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Large synthetic histories are bounded and counts indicate omitted records.
- Switching run ids cannot return another run evidence; malformed payloads have an explicit error representation.

Verification:

```text
cargo test --offline -p roudex-desktop --test evidence_commands
```

### R002-T024 - Show verification and evidence in run details

- Milestone/status: H4 / planned.
- Release target: `v0.8.0`.
- Dependencies: R002-T023, R002-T004, R002-T017.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/evidence-details-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A08.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T024 entry in tasks.md. Extend details with command results, artifact references and the distinction between runtime completion and verified completion. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- The UI links each result to its task/run and displays nonzero exit codes and missing artifacts.
- Fixture-only, native and live-runtime evidence are visibly distinguishable.

Verification:

```text
pnpm --dir ui build --configLoader runner && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/evidence-details-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/evidence-details-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T050 - Add interactive run input and streaming session controls

- Milestone/status: H4 / planned.
- Release target: `v0.8.0`.
- Dependencies: R002-T013, R002-T017, R002-T024, R002-T037.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `crates/router-core/src`, `crates/router-core/tests/interactive_execution.rs`, `src-tauri/src`, `src-tauri/tests/interactive_execution.rs`, `ui/src`, `ui/tests/interactive-execution-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `src-tauri`, `ui`.
- Scenarios: A04, A05, A06, A08, A19.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T050 entry in tasks.md. Allow the operator to send and continue interactive turns in RouDex, inspect ordered streaming events and answer outstanding runtime approval requests through the coordinator-owned local App Server session. Verify the complete installed-native H4 workflow. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Initial and continued interactive input uses the existing approved contract and persisted run/thread identity; assigned model/effort are actual turn inputs.
- Streaming events are ordered and isolated by run; duplicate submissions, stale selections and disconnected clients cannot create duplicate turns or contaminate another session.
- Runtime approval replies match an outstanding request and its exact scope; denied/stale replies block, UI approval never widens coordinator permissions.
- Installed-native review, dispatch, interactive input, approval handling and verified completion have observed results; runtime success alone does not complete a task.

Verification:

```text
cargo test --offline -p router-core --test interactive_execution && cargo test --offline -p roudex-desktop --test interactive_execution && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/interactive-execution-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/interactive-execution-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T006 - Validate an import before persisting it

- Milestone/status: H5 / planned.
- Release target: `v0.9.0`.
- Dependencies: R002-T050.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/task_manifest.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/tests/roadmap_planning.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A03, A10.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T006 entry in tasks.md. Add a non-mutating manifest preview that validates contracts and rejects cross-spec task-id collisions before an atomic import. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Duplicate ids, missing dependencies, cycles, route conflicts and unsafe scopes are rejected before any write.
- A task id owned by another spec is rejected rather than reassigned.

Verification:

```text
cargo test --offline -p router-core --test roadmap_planning
```

### R002-T007 - Expose reviewed manifest and draft persistence

- Milestone/status: H5 / planned.
- Release target: `v0.9.0`.
- Dependencies: R002-T006.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src`, `src-tauri/tests/planning_commands.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/tests/roadmap_planning.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`, `crates/router-core`.
- Scenarios: A03.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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

- Milestone/status: H5 / planned.
- Release target: `v0.9.0`.
- Dependencies: R002-T007, R002-T003.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/task-editor-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A03, A05.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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

- Milestone/status: H5 / planned.
- Release target: `v0.9.0`.
- Dependencies: R002-T006, R002-T004.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/task-graph-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`.
- Scenarios: A03.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T009 entry in tasks.md. Render a dependency list and readiness reasons from the validated task graph without introducing a graph library. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Each blocked task identifies its actual unmet dependency or approval; independent tasks remain eligible.
- Cycles and missing dependencies render validation errors, never a ready badge.
- Create or change dependency edges only for unexecuted drafts; cyclic, missing and cross-spec edges are rejected atomically and review is invalidated.

Verification:

```text
pnpm --dir ui build --configLoader runner && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/task-graph-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/task-graph-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T038 - Author and review local versioned specifications

- Milestone/status: H5 / planned.
- Release target: `v0.9.0`.
- Dependencies: R002-T007, R002-T050.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `ui/src`, `src-tauri/src`, `src-tauri/tests/sdd_studio.rs`, `ui/tests/sdd-studio-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `ui`, `src-tauri`.
- Scenarios: A14.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T038 entry in tasks.md. Add SDD Studio editing and review of local specifications, acceptance scenarios and decisions, with explicit saved revisions and bounded repository-relative Markdown artifacts. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Create, edit, review and reopen a versioned local spec containing problem, boundaries, decisions and measurable acceptance scenarios.
- Unsaved changes and invalid drafts are visible; cancel or failed save preserves the prior revision; paths cannot escape the selected project.
- Saving a spec does not dispatch tasks, broaden permissions or rewrite completed task/run contracts.

Verification:

```text
cargo test --offline -p roudex-desktop --test sdd_studio && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/sdd-studio-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/sdd-studio-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T039 - Trace acceptance to reviewed task graph revisions

- Milestone/status: H5 / planned.
- Release target: `v0.9.0`.
- Dependencies: R002-T008, R002-T009, R002-T038.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `crates/router-core/src`, `crates/router-core/tests/sdd_traceability.rs`, `src-tauri/src`, `src-tauri/tests/sdd_studio.rs`, `ui/src`, `ui/tests/sdd-traceability-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `src-tauri`, `ui`.
- Scenarios: A03, A14.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T039 entry in tasks.md. Link specification revisions and acceptance scenarios to reviewed draft tasks and dependency edges; validate graph edits, preview imports and expose missing coverage in SDD Studio. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Each acceptance scenario maps to a task and verification method; missing coverage is visibly blocking for review.
- Graph edits reject cycles, missing dependencies and cross-spec id collisions atomically; valid drafts save a new revision and invalidate stale approvals.
- Round-trip local spec/graph export and import preserve task ids, prior immutable contracts, run history and evidence without dispatching.

Verification:

```text
cargo test --offline -p router-core --test sdd_traceability && cargo test --offline -p roudex-desktop --test sdd_studio && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/sdd-traceability-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/sdd-traceability-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T040 - Define global Desktop ingress and coordinator handoff

- Milestone/status: H6 / planned.
- Release target: `v0.10.0`.
- Dependencies: R002-T039.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `crates/router-core/src`, `crates/router-core/tests/global_desktop.rs`, `crates/router-cli/src`, `crates/router-cli/tests/global_desktop.rs`, `scripts/route-prompt.ps1`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `crates/router-cli`, `scripts`.
- Scenarios: A15.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T040 entry in tasks.md. Extend and verify the local Desktop adapter contract for all supported chats and working directories, preserving delivery identity and the worker recursion guard; distinguish recorded ingress from coordinator execution. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- All supported local Desktop ingress surfaces carry stable session/turn/task identity and explicit cwd; replay records one task and changed-payload collisions block.
- Ingress never starts a worker; the explicit coordinator handoff uses the assigned App Server model/effort and persists route/run/thread/turn identity.
- Internal workers cannot recursively dispatch themselves; unsupported Desktop contracts and unavailable runtime versions block visibly.

Verification:

```text
cargo test --offline -p router-core --test global_desktop && cargo test --offline -p router-cli --test global_desktop
```

### R002-T041 - Manage opt-in global Desktop integration

- Milestone/status: H6 / planned.
- Release target: `v0.10.0`.
- Dependencies: R002-T040.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve the implementation contract; writing global/user Desktop settings, requesting administrative access and activating/removing a hook each require explicit action approval after the exact diff is reviewable.
- Scope: `crates/router-cli/src`, `crates/router-cli/tests/global_desktop.rs`, `scripts/install-hook.ps1`, `src-tauri/src`, `ui/src`, `docs/installation.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-cli`, `scripts`, `src-tauri`, `ui`, `docs`.
- Scenarios: A15.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T041 entry in tasks.md. Provide explicit install/status/repair/uninstall controls for the local global Desktop adapter, preserving unrelated managed/user settings and reporting required local permissions. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Global activation requires a concrete opt-in action and any OS permission; first-run onboarding never activates it by default.
- Install, repair and uninstall are idempotent, preserve unrelated configuration and avoid duplicate user/managed handlers; conflicting policy blocks without overwrite.
- Status reports the actual installed adapter/version and removal restores the prior local configuration without deleting task data.

Verification:

```text
cargo test --offline -p router-cli --test global_desktop && cargo build --offline -p roudex-desktop && pnpm --dir ui build --configLoader runner
```

### R002-T042 - Verify global Desktop delivery without duplicate execution

- Milestone/status: H6 / planned.
- Release target: `v0.10.0`.
- Dependencies: R002-T040, R002-T041.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve native Desktop smoke using disposable projects and each live model contract; global adapter activation or removal requires its own explicit machine-setting approval.
- Scope: `specs/002-product-roadmap/global-desktop-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `specs/002-product-roadmap`.
- Scenarios: A15.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T042 entry in tasks.md. Verify installed global integration in native Codex Desktop across supported chat/cwd surfaces and record actual coordinator handoff, replay, failure and uninstall results using disposable data. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Native Desktop delivery, repeated delivery, adapter outage and worker recursion cases have observed results for the supported platform/runtime matrix.
- A live approved handoff has route/run/thread/turn and verification receipts for one execution; additionalContext or fixture-only evidence cannot satisfy this gate.
- Missing Desktop access or platform evidence blocks completion; no operational project is inspected to perform the smoke run.

Verification:

```text
cargo test --offline -p router-cli --test global_desktop && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/global-desktop-evidence.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/global-desktop-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T043 - Isolate multiple registered local projects

- Milestone/status: H7 / planned.
- Release target: `v0.11.0`.
- Dependencies: R002-T042.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `crates/router-core/src`, `crates/router-core/tests/multi_project.rs`, `src-tauri/src`, `ui/src`, `ui/tests/multi-project-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `src-tauri`, `ui`.
- Scenarios: A16.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T043 entry in tasks.md. Persist explicit local project registrations and project-scoped task/spec/run views; scope selection must remain separate from permission to execute in a project. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Switching projects isolates tasks, specs, queues and run histories; identical spec names never overwrite another project and globally stable task ids remain collision-safe.
- Canonical roots and sandbox scopes are checked per execution; no project is automatically discovered or granted access.
- Concurrent reads and failed registration preserve existing project state; migration of the single-project baseline preserves all history.

Verification:

```text
cargo test --offline -p router-core --test multi_project && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/multi-project-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/multi-project-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T044 - Persist queues and guarded scheduling

- Milestone/status: H7 / planned.
- Release target: `v0.11.0`.
- Dependencies: R002-T043, R002-T017.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `crates/router-core/src`, `crates/router-core/tests/task_queues.rs`, `src-tauri/src`, `ui/src`, `ui/tests/queue-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `src-tauri`, `ui`.
- Scenarios: A16.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T044 entry in tasks.md. Add explicit project queues, ordering and pause/resume controls with durable scheduling decisions through the coordinator; prevent overlapping writes and duplicate dispatch. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Enqueue/reorder/dequeue and project queue pause/resume survive restart; enqueue does not grant task approval or start work implicitly.
- Scheduler checks current approval, dependency completion, assigned runtime and per-project sandbox before each dispatch; ineligible work has a durable blocked reason.
- Repeated dispatch and concurrent schedulers create one run; canonical overlapping writable scopes are serialized even across project registrations.

Verification:

```text
cargo test --offline -p router-core --test task_queues && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/queue-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/queue-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T045 - Track durable long-running executions

- Milestone/status: H7 / planned.
- Release target: `v0.11.0`.
- Dependencies: R002-T043, R002-T044.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `crates/router-core/src`, `crates/router-core/tests/long_running.rs`, `src-tauri/src`, `ui/src`, `ui/tests/long-running-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `src-tauri`, `ui`.
- Scenarios: A16.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T045 entry in tasks.md. Keep long-running coordinator executions observable across UI disconnects and app reconnects with durable progress, bounded event pagination and explicit queue/run lifecycle distinctions. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- A synthetic long-running task survives UI disconnect/reconnect without duplicate execution or a new run identity; event reads are bounded and ordered.
- Queue pause is distinct from active-run interruption; ambiguous runtime identity blocks continuation instead of automatically replaying work.
- Progress, failure and verified terminal results are persisted independently of the open window; advanced crash/interrupt recovery remains the H8 hardening gate.

Verification:

```text
cargo test --offline -p router-core --test long_running && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/long-running-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/long-running-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T018 - Define guarded interruption transitions

- Milestone/status: H8 / planned.
- Release target: `v0.12.0`.
- Dependencies: R002-T017.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/domain.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/tests/roadmap_recovery.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A07.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T018 entry in tasks.md. Specify and persist an interruption request separately from the terminal result without treating cancellation as rollback. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Only an active run can receive an interruption request; terminal runs remain immutable.
- A request and terminal result racing each other produce one authoritative terminal state.

Verification:

```text
cargo test --offline -p router-core --test roadmap_recovery
```

### R002-T019 - Implement local App Server interruption

- Milestone/status: H8 / planned.
- Release target: `v0.12.0`.
- Dependencies: R002-T018.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/app_server.rs`, `crates/router-core/src/coordinator.rs`, `crates/router-core/tests/roadmap_recovery.rs`, `crates/router-core/tests/support/fake_app_server.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A07.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T019 entry in tasks.md. Implement interruption against the locally verified App Server protocol and preserve run/thread identity. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Supported interruption records the request and observed outcome; unsupported protocol blocks with an actionable reason.
- Disconnect or timeout is not reported as successful cancellation; no replacement thread starts implicitly.

Verification:

```text
cargo test --offline -p router-core --test roadmap_recovery
```

### R002-T020 - Validate crash recovery and idempotent resume

- Milestone/status: H8 / planned.
- Release target: `v0.12.0`.
- Dependencies: R002-T019, R002-T012, R002-T014, R002-T045.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/src/coordinator.rs`, `crates/router-core/src/persistence.rs`, `crates/router-core/tests/roadmap_recovery.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`.
- Scenarios: A07.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T020 entry in tasks.md. Harden existing continuation for crash windows using persisted identities, immutable assignments and current approval gates. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Restart before/after terminal persistence cannot duplicate completed work or terminal events.
- Missing or conflicting thread/turn identity remains blocked; resume preserves the route and run id.

Verification:

```text
cargo test --offline -p router-core --test roadmap_recovery
```

### R002-T021 - Expose interruption and resume controls

- Milestone/status: H8 / planned.
- Release target: `v0.12.0`.
- Dependencies: R002-T020, R002-T017.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `src-tauri/src`, `src-tauri/tests/recovery_commands.rs`, `ui/src`, `ui/tests/ipc-fixture.html`, `ui/tests/recovery-controls-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `src-tauri`, `ui`.
- Scenarios: A07.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T021 entry in tasks.md. Expose explicit interruption and continuation actions through the coordinator with visible state and reasons. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Interrupted tasks offer resume by the existing run id; completed runs cannot be restarted by this action.
- The UI shows that interruption does not undo filesystem changes; denied approvals and unsupported operations stay blocked.

Verification:

```text
cargo test --offline -p roudex-desktop --test recovery_commands && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'ui/tests/recovery-controls-evidence.md')) { exit 1 }"
```

Evidence artifact: `ui/tests/recovery-controls-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T025 - Export a sanitized local evidence bundle

- Milestone/status: H8 / planned.
- Release target: `v0.12.0`.
- Dependencies: R002-T022, R002-T045.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve the implementation contract; each later export requires a selected local destination and separate explicit overwrite confirmation. No external send is authorized.
- Scope: `crates/router-cli/src/main.rs`, `crates/router-core/src`, `crates/router-cli/tests/evidence_export.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-cli`, `crates/router-core`.
- Scenarios: A09.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T025 entry in tasks.md. Add explicit local export of an allowlisted evidence summary without raw prompts, raw event bodies or authentication material. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Export uses only reviewed fields and relative artifact references; synthetic sensitive markers and absolute paths are omitted.
- Export creates no network connection and refuses to overwrite an existing file unless explicitly confirmed.

Verification:

```text
cargo test --offline -p router-cli --test evidence_export
```

### R002-T027 - Create a consistent local backup

- Milestone/status: H8 / planned.
- Release target: `v0.12.0`.
- Dependencies: R002-T022, R002-T045.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve implementation and use synthetic stores for verification. Backing up operational data later requires a user-selected destination; backup data is private and never a publication artifact.
- Scope: `crates/router-core/src/persistence.rs`, `crates/router-cli/src/main.rs`, `crates/router-cli/tests/local_backup.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `crates/router-cli`.
- Scenarios: A11.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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
- Release target: `v0.12.0`.
- Dependencies: R002-T027, R002-T020.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve implementation using disposable databases only. An operational restore requires a separate explicit confirmation of source, destination and rollback copy before replacing any local data.
- Scope: `crates/router-core/src/persistence.rs`, `crates/router-cli/src/main.rs`, `crates/router-cli/tests/local_restore.rs`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `crates/router-cli`.
- Scenarios: A11.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T028 entry in tasks.md. Add preview/validate/restore with exclusive coordination, source compatibility checks and an automatic rollback copy. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Corrupt, incompatible and busy destinations are rejected before replacement.
- Injected replacement failure restores the previous state; repeated restore never dispatches tasks or resets completed history.

Verification:

```text
cargo test --offline -p router-cli --test local_restore
```

### R002-T046 - Harden local permissions and sensitive-data boundaries

- Milestone/status: H8 / planned.
- Release target: `v0.12.0`.
- Dependencies: R002-T042, R002-T044.
- Route: frontier / gpt-6-astra / high.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `crates/router-core/src`, `crates/router-core/tests/local_security.rs`, `crates/router-cli/src`, `src-tauri/src`, `src-tauri/tauri.conf.json`, `specs/002-product-roadmap/security-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `crates/router-cli`, `src-tauri`, `specs/002-product-roadmap`.
- Scenarios: A17.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T046 entry in tasks.md. Audit and harden coordinator, IPC, project paths, global adapter and queue boundaries with synthetic adversarial inputs; keep authentication and private operational data outside repository/export artifacts. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Path traversal, canonical-path aliasing, stale approval and cross-project write attempts are rejected with no unauthorized side effects.
- IPC and App Server transport remain local; no remote listener, telemetry or authentication artifacts are introduced.
- Logs/exports use allowlisted synthetic-safe fields; unresolved critical boundary findings block H8 and require scoped remediation.

Verification:

```text
cargo test --offline -p router-core --test local_security && git diff --check && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/security-evidence.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/security-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T047 - Verify platform, runtime and data compatibility

- Milestone/status: H8 / planned.
- Release target: `v0.12.0`.
- Dependencies: R002-T021, R002-T025, R002-T028, R002-T045, R002-T046.
- Route: deep / gpt-6.1-sol / high.
- Approval: `manual`. Approve this exact task contract before dispatch; dependency installation, machine configuration and publication require separate explicit action approval.
- Scope: `crates/router-core/src`, `crates/router-core/tests/compatibility.rs`, `src-tauri/src`, `docs/platform-matrix.md`, `specs/002-product-roadmap/compatibility-evidence.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `src-tauri`, `docs`, `specs/002-product-roadmap`.
- Scenarios: A07, A11, A17.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T047 entry in tasks.md. Define and test the supported OS/Tauri/Codex App Server/schema compatibility matrix, safe upgrades and rollback of disposable stores, including long-run recovery and global adapter compatibility. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Each supported platform/runtime/schema combination has observed compatibility evidence; unsupported versions block with an actionable reason and no model fallback.
- Upgrade/rollback tests retain project registrations, queues, immutable approvals, run identities and evidence; failed migration preserves the prior store.
- Interruption, crash recovery, backup/restore, sanitized export and permission checks pass without replaying completed work.

Verification:

```text
cargo test --offline -p router-core --test compatibility && cargo test --workspace --offline && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/compatibility-evidence.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/compatibility-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T030 - Run the integrated product acceptance matrix

- Milestone/status: H9 / planned.
- Release target: `v1.0.0`.
- Dependencies: R002-T005, R002-T037, R002-T050, R002-T039, R002-T042, R002-T045, R002-T047.
- Route: frontier / gpt-6-astra / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `crates/router-core/tests`, `crates/router-cli/tests`, `src-tauri/tests`, `ui/tests`, `specs/002-product-roadmap`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `crates/router-core`, `crates/router-cli`, `src-tauri`, `ui`, `specs/002-product-roadmap`.
- Scenarios: A02, A03, A04, A05, A06, A07, A08, A09, A10, A11, A12, A13, A14, A15, A16, A17, A18, A19.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T030 entry in tasks.md. Add integrated synthetic acceptance coverage and collect results for A02-A19 after every feature branch is complete. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Installers/onboarding, interactive execution, SDD Studio, global Desktop ingress, isolated project queues, long runs and security/recovery/compatibility form one verified local workflow.
- Every automated scenario maps to a named test; failure injection covers rejection paths and each result has actual output.

Verification:

```text
cargo test --workspace --offline && pnpm --dir ui build --configLoader runner && pnpm --dir ui build --config tests/vite.config.js --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/acceptance-evidence.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/acceptance-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T031 - Audit local boundaries and release contents

- Milestone/status: H9 / planned.
- Release target: `v1.0.0`.
- Dependencies: R002-T030.
- Route: frontier / gpt-6-astra / high.
- Approval: `manual`. Approve this exact task contract before dispatch; approval does not authorize changes outside scope, dependency installation or publication.
- Scope: `specs/002-product-roadmap`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `specs/002-product-roadmap`.
- Scenarios: A09, A10, A12.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

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
- Release target: `v1.0.0`.
- Dependencies: R002-T030.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve this native smoke run with disposable data. Any live model task needs a separately reviewed and approved contract; no operational task, existing hook or user database may be changed.
- Scope: `specs/002-product-roadmap`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `specs/002-product-roadmap`.
- Scenarios: A12.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T032 entry in tasks.md. Perform a native Tauri smoke test against disposable local data and record operator actions, runtime versions and observed results. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- Native installed packages exercise onboarding, overview, interactive input/approval/dispatch, SDD specs/graphs, global Desktop ingress, project queues, long-run reconnect and recovery/evidence on every supported platform.
- A browser fixture cannot satisfy native acceptance; live model execution requires its own route/run/thread/turn and verification receipt. Unavailable native/runtime access remains blocked.

Verification:

```text
cargo build --workspace --offline && pnpm --dir ui build --configLoader runner && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/native-smoke-evidence.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/native-smoke-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T048 - Prepare public beta and feedback gates

- Milestone/status: H9 / planned.
- Release target: `v1.0.0`.
- Dependencies: R002-T031, R002-T032.
- Route: standard / gpt-6.1-sol / medium.
- Approval: `manual`. Approve beta preparation only. Exposing packages publicly, requesting external feedback or publishing a prerelease requires separate explicit authorization and review of the concrete candidate.
- Scope: `specs/002-product-roadmap/beta-evidence.md`, `specs/002-product-roadmap/release-handoff.md`, `docs/local-packaging.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `specs/002-product-roadmap`, `docs`.
- Scenarios: A18.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T048 entry in tasks.md. Prepare the public beta candidate from the validated installer matrix, native acceptance and boundary audit, with known issues, feedback/reproduction guidance and explicit promotion blockers. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- The beta candidate references exact versioned packages/checksums, supported platforms, test evidence and known limitations; it does not claim a published beta before authorized publication evidence exists.
- Feedback uses sanitized, voluntarily supplied local reproduction evidence; no telemetry or automatic upload is enabled.
- Beta exit criteria classify blocking issues and require verified dispositions before stable promotion; unresolved reports remain visible.

Verification:

```text
git diff --check && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/beta-evidence.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/beta-evidence.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T049 - Validate stable v1.0.0 promotion readiness

- Milestone/status: H9 / planned.
- Release target: `v1.0.0`.
- Dependencies: R002-T048.
- Route: frontier / gpt-6-astra / high.
- Approval: `manual`. Approve stable readiness review only. Signed commits, tags, release creation and external uploads require separate explicit publication authorization after the exact candidate is reviewable.
- Scope: `specs/002-product-roadmap/stable-readiness.md`, `specs/002-product-roadmap/release-handoff.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `specs/002-product-roadmap`.
- Scenarios: A18.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read version 2 of specs/002-product-roadmap/spec.md, tasks.json and the R002-T049 entry in tasks.md. Review actual beta results and verified blocker dispositions, rerun the release regression matrix as needed and prepare the stable v1.0.0 promotion decision without publishing. Implement only the declared scope and record actual verification. Preserve Rust/SQLite/Tauri/React, Tauri IPC and coordinator-owned App Server stdio execution. Do not install dependencies, read environment files, access unrelated repositories, publish, or grant approvals. Missing runtime, platform, approval or cached prerequisites must remain explicitly blocked.

Acceptance:

- Actual authorized beta results, issue dispositions and supported-platform native evidence are traceable; missing beta observations or unresolved blockers prevent stable readiness.
- The stable candidate targets v1.0.0 with matching package metadata, checksums, upgrade/recovery instructions and reproducible regression results.
- Local readiness, publication authorization and observed public availability are separate states; no release is declared stable/published from planning or model prose.

Verification:

```text
cargo test --workspace --offline && git diff --check && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/stable-readiness.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/stable-readiness.md`. Review its observed results before completion; existence alone is insufficient.

### R002-T033 - Prepare the evidence-backed release handoff

- Milestone/status: H9 / planned.
- Release target: `v1.0.0`.
- Dependencies: R002-T031, R002-T032, R002-T049.
- Route: routine / gpt-6-luna / medium.
- Approval: `manual`. Approve the documentation task only. Git publication, tags, releases, uploads, remote access and machine installation remain separate explicitly authorized operations.
- Scope: `specs/002-product-roadmap`, `docs/local-packaging.md`.
- Cwd: `.`; sandbox: `workspaceWrite`; writable roots: `specs/002-product-roadmap`, `docs`.
- Scenarios: A12, A18.
- Route/run ids: none / none; retained historical fields are not evidence of a new dispatch.

Read specs/002-product-roadmap/spec.md, decisions.md and the R002-T033 entry in tasks.md. Prepare release notes, milestone/task evidence mapping and local packaging handoff only after acceptance and native gates pass. Implement only the declared scope, add the named verification coverage and record actual results. Preserve local-only state and coordinator-only execution. Do not install dependencies, access unrelated repositories, read environment files, publish, or grant approvals. Stop with a concrete blocked reason if a required approval, runtime or dependency is unavailable.

Acceptance:

- H2-H9 can be marked completed only when all required task verification, manual artifacts and blocker dispositions are present.
- The handoff distinguishes local readiness from publication and lists an unexecuted signed-commit/tag/release procedure with explicit publication approval.
- The handoff names v0.6.0 through v0.12.0 and stable v1.0.0, includes actual beta/stable readiness evidence and never treats planned targets as published releases.

Verification:

```text
git diff --check && powershell -NoProfile -Command "if (-not (Test-Path -LiteralPath 'specs/002-product-roadmap/release-handoff.md')) { exit 1 }"
```

Evidence artifact: `specs/002-product-roadmap/release-handoff.md`. Review its observed results before completion; existence alone is insufficient.

## Acceptance traceability

The matrix matches `spec.md` and `tasks.json`. A01 is historical;
R002-T030 aggregates every future scenario without completing its source tasks.

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
