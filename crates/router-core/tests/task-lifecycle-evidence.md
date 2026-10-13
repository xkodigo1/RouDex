# T-005 coordinator lifecycle evidence

Specification: `specs/001-task-orchestration/spec.md`, R6/R8/R9/R10 and
acceptance scenarios A3/A4/A5/A6/A8. Depends on completed T-004. The requested
outcome is manual approval enforcement, fail-closed recovery, worker recursion
protection and idempotent continuation of task runs.

## Decisions and scope

Reuse the existing Rust coordinator, SQLite transactions and local stdio App
Server. No dependency, framework, remote service or permission expansion was
introduced.

- Store approval decisions and pending-task transitions atomically. Only
  `approved` and `denied` are accepted. Denied pending tasks remain blocked
  across repeated dispatches and reopening SQLite until explicitly approved.
  Active and terminal task states are preserved; continuation rechecks approval.
- Claim continuation atomically against the latest interrupted run, immutable
  approval policy, current decision and dependency gates. Persist the claim.
  Exactly one coordinator may claim the run.
- Commit task state, run state and outcome evidence together. Failed verification
  retains the command and its result. Database errors roll back the entire
  outcome instead of exposing partial completion.
- Missing immutable definitions, thread identities and turn identities block
  recovery durably. Later dispatch cannot substitute another thread. Invalid
  runtime identities and non-terminal responses never reach verification.
- Deduplicate both protocol and normalized terminal records by run, event type,
  thread and turn, including after reopening SQLite. Preserve the separate raw
  protocol and normalized audit representations.
- Route CLI continuation of task-owned runs through `resume_run`, retaining the
  immutable task and terminal replay checks. Keep legacy non-task continuation
  behavior. Task continuation accepts an empty stdin; supplied text does not
  replace the immutable task definition.
- Reject nested CLI lifecycle mutations when either worker marker is present,
  before opening state or starting the runtime. Either marker also skips hook
  ingress without creating a second run. Worker guards remain child-process
  environment values; no global environment setting is changed.

The CLI implementation is included because its previous `continue-run` path
could bypass the coordinator's lifecycle checks. These changes repair that
boundary; they do not implement the broader T-006 CLI milestone.

## Verification

- `cargo test --workspace`: 40 tests passed (19 coordinator integration,
  10 core unit, 10 CLI integration, 1 App Server protocol).
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- Strict Clippy still reports existing `result_large_err` and
  `too_many_arguments` categories. The baseline also documented
  `redundant_closure_call`.
- `cargo clippy --workspace --all-targets -- -D warnings -A clippy::result_large_err -A clippy::too_many_arguments -A clippy::redundant_closure_call`:
  passed. No source-level lint allowances were added.

Focused regression checks cover denial/reapproval, malformed approval decisions,
revocation before continuation, dependency and superseded-run claim checks,
competing SQLite connections, missing recovery identities, duplicate terminal
notifications, invalid runtime responses, CLI replay and worker recursion.
SQLite triggers inject failures in approval state updates and outcome audit
writes; both tests confirm complete rollback and safe recovery.

## Live dispatch evidence and handoff

The current worker markers and local SQLite dispatch were inspected read-only:

- Task: `T-005`
- Route: `29099428-fc64-45cf-ba0d-b3e1b2f3cf16`
- Run: `f3a93f3f-ed52-42d9-9cd1-bb0c9d530de4`
- Profile/model/effort: `deep` / `gpt-6.1-sol` / `high`
- Thread: `01a132c2-3722-7ee1-8097-423f110d9712`
- Turn: `01a132c2-37fa-7781-af77-61ea2afd7e55`

The worker implementation and local checks are validated. The parent coordinator
returned `completed` after executing the immutable `cargo test --workspace` gate
and persisted the terminal outcome. The worker does not finalize its own run or
change the specification manifest. A signed commit remains the parent's
responsibility because this worker has read-only access to `.git`.
