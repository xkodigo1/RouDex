# Specification 002 decisions

Version: 1. Date: 2026-10-13. Scope: planning only.

## D001 - A proposed roadmap above the completed H1 baseline

Record H1 as completed from specification 001 and local release metadata,
preserving its documented audit limitations. Define H2-H9 as proposed increments
because their definitions are absent from the repository. Reject treating the
proposed features as already approved implementation or rewriting H1 to hide
its evidence limitations. The trade-off is that future milestone approval must
confirm the proposed product boundaries before execution.

## D002 - Reuse the established local stack

Retain Rust, SQLite, Tauri, React and Vite. The current read-only UI becomes the
future control center in H2; explicit mutation controls arrive through the
coordinator in H5. Reject a new framework, remote backend, graph library or
additional state service for this plan. Reuse reduces integration scope and
keeps local ownership clear. Any later dependency choice requires a presented
scope and technical decision first; no dependency is added now.

## D003 - Keep the manifest executable by today's validator

Use the current `TaskManifest`/`TaskDefinition` fields and supported statuses:
`planned`, `ready`, `awaiting_approval`, `running`, `completed`, `failed`,
`blocked`, `interrupted`. Do not invent a `pending` or `dispatched` enum value.
Dispatch is an event/receipt, distinct from running or completed status.

`milestone`, `scenario_ids`, `approval_requirements`, `verification_artifact`,
`evidence`, `route_id`, `run_id`, top-level milestones and `authoring_run` are
versioned planning/audit metadata. The current Serde reader tolerates but does
not persist these extension fields. Their presence must not be represented as
new runtime enforcement. The authoritative dispatch fields remain `profile`,
`model`, `effort`, `approval`, `scope`, `cwd`, `sandbox`, `acceptance`, `verify`
and `depends_on`. Reject implementing a schema engine as part of writing this
roadmap.

## D004 - Namespace globally stored task ids

Use `R002-T001` through `R002-T033`, because SQLite keys tasks by `task_id`
globally, not by `(spec_id, task_id)`. Reject reusing specification 001's
`T-001` ids, which could overwrite unrelated task definitions. The first record
is a completed, read-only historical reference so the dependency graph is
self-contained. Its null route/run ids deliberately do not invent new H1
execution evidence. H3 adds explicit cross-spec collision rejection.

## D005 - Route by existing profiles and immutable assignments

Use the mappings verified in `crates/router-core/src/routing.rs`:

| Profile | Model | Effort | Assignment rationale |
| --- | --- | --- | --- |
| routine | gpt-6-luna | medium | Bounded documentation, fixture checks and diagnostic work. |
| standard | gpt-6.1-sol | medium | UI, IPC and CLI integrations with established contracts. |
| deep | gpt-6.1-sol | high | Persistence, approval, runtime, recovery and data-integrity changes. |
| frontier | gpt-6-astra | high | Cross-milestone acceptance and boundary audit. |

These are repository routing assignments, not claims about globally available
models or pricing. The unused `micro` profile remains unchanged. H4 can add
validated local overrides for future runs; existing runs retain their original
assignment. Reject automatic fallback or silently changing a task's explicit
model/effort. Unavailable models block with an actionable reason.

## D006 - Approval is separate from planning and execution

All future tasks use the existing `manual` gate. The user authorized authoring
these four artifacts, not dispatching the roadmap. Additional human decisions
for restore, overwrite, operational backups, native live runs, hook changes
and publication are stated on the affected tasks. The approval detail metadata
does not independently enforce those decisions; implementation and acceptance
must enforce the actual action gates. Reject a blanket approval created by
importing the manifest or opening the UI.

## D007 - Small outcomes with explicit cross-milestone dependencies

Each task has one independently reviewable outcome and bounded logical scope.
Use actual prerequisites instead of chaining entire milestones. H2, H3, H4,
H6, H7 and diagnostics can start independently after H1; integrated acceptance
waits for all feature branches. Tasks sharing a source file must avoid concurrent
writes. Reject one implementation task per milestone and an unconditional
H1-to-H9 serial chain, both of which obscure executable units.

## D008 - Existing directory roots and narrow logical scope

Keep `cwd` as `.` and writable roots as existing repository-relative module
directories. The current runtime canonicalizes roots before dispatch, so a
not-yet-created source file or test directory cannot serve as a new root.
The narrower file list in `scope` still controls authorized edits. Reject
personal absolute paths, parent traversal, unrestricted repository writes and
`dangerFullAccess`. Normal compiler build outputs remain local and ignored;
they do not authorize unrelated source changes.

## D009 - Verification commands are task deliverables, not claimed results

Name new integration targets in the task that introduces them, then extend
those targets in dependent tasks. Full target commands fail if the target is
missing; do not count an empty test selection as success. UI compilation has
an additional recorded fixture/manual acceptance gate. Native and live-runtime
evidence have separate provenance. Reject claiming a successful build proves
interactivity or that artifact existence alone proves the documented actions.

Verification commands use the current Windows coordinator shell and `&&`
failure propagation. Dependencies must already be locally provisioned; missing
ones remain a blocker. Command execution is subject to the current 120-second
coordinator verification timeout. If a future cold build exceeds that limit,
record the failure and propose a separately scoped timeout/provisioning change;
do not remove checks or mark a timed-out task completed.

## D010 - Preserve private state and sanitize exported summaries

Use an allowlisted local evidence summary instead of exporting raw prompts,
event payloads, authentication configuration or the whole operational store.
Export destinations require a deliberate local choice and explicit overwrite
confirmation. SQLite backups are private recovery artifacts and are never
included in releases. Reject telemetry, automatic uploads and remote access.
Local-first controls orchestration and storage; it does not promise offline
model inference or embed model authentication in the project.

## D011 - Model interruption honestly and restore reversibly

Interruption requests and terminal outcomes are separate events. Recovery
reuses immutable task/run identities and stops when history is ambiguous.
Backup/restore development uses disposable databases, consistent snapshots,
validation, exclusive coordination and rollback protection. Reject automatic
reruns after an uncertain crash or claiming cancellation rolls back file edits.
Operational restore requires its own reviewed source/destination confirmation.

## D012 - Separate release readiness from publication

H9 assembles tested local artifacts, a boundary audit, native acceptance and a
release handoff. Publication is a later explicitly authorized workflow with
signed commits, tag and release metadata. Reject creating remote endpoints,
publishing a package or modifying Git history merely to make this planning
milestone look complete. No release version is reserved in advance.

## D013 - Preserve the current coordinator boundary

This authoring task is already a Codex Router worker with an active receipt.
Do not clear recursion guards, start nested workers, import this new graph into
operational state or grant future task approvals. Validate the manifest through
the pure Rust parser and inspect the existing store read-only for provenance.
The enclosing coordinator owns authoring terminal status; the metadata here
records only what was observed during authoring.
