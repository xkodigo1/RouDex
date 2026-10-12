# Task coordinator verification

Specification: `specs/001-task-orchestration/spec.md`, task `T-004`.

This change completes the existing coordinator's ready-task dispatch and
recovery boundaries without adding dependencies or a remote transport.

## Checks

- `cargo test --workspace`: 28 tests passed, including 11 coordinator
  integration tests using a local fake App Server over real stdio pipes.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: fails on existing
  `result_large_err`, `too_many_arguments`, and `redundant_closure_call` lints.
- The same Clippy command with those three lint categories explicitly allowed
  passes. No lint allowances were added to source files.

## Coverage

The integration suite verifies:

- Profile/model/effort, route/run identity, lifecycle events, message deltas,
  and independent verification survive reopening SQLite.
- The App Server receives model and effort as protocol parameters, explicit
  sandbox roots with network access disabled, and worker recursion guards.
- Batch dispatch refreshes dependencies after completion and does not construct
  a runtime before approval or route validation.
- A runtime factory failure is recorded without aborting independent tasks.
- Missing dependencies, unavailable models, failed/interrupted turns, transport
  disconnects, and failed verification never produce a completed task.
- Resume retains run/thread identity, reuses completed server history or durable
  terminal evidence, and does not duplicate completed work on repeated calls.
- Missing or mismatched resumed history blocks recovery. Later batches cannot
  silently replace that blocked thread.
- Unrelated or malformed turn notifications are excluded from persisted output
  and terminal evidence.

## Earlier dispatch receipt

The earlier implementation session was dispatched through the local coordinator:

- Route: `0b804590-4350-4706-935f-1e5fdb638c1a`
- Run: `7df4a224-db95-4e43-b5bd-eb9d8d05a949`
- Profile/model/effort: `deep` / `gpt-6.1-sol` / `high`
- Thread: `01a1321c-5a08-79b3-b9eb-03392c5a5ec0`
- Turn: `01a1321c-5ad2-7df2-a147-af5552305573`

The receipt, immutable task definition, thread start, turn start and terminal
events were read back from local SQLite. The earlier run is preserved as a failed
verification because its manifest value was descriptive prose rather than an
executable command. The retry used the corrected `cargo test --workspace` gate
and completed successfully.

A signed commit could not be created because the worker has read-only access to
`.git`; staging failed with permission denied when creating `.git/index.lock`.

## Recovery follow-up for T-004

The existing profile selection, lazy stdio runtime, ready-task dispatch, SQLite
receipts, notification filtering, and verification gate passed the baseline
suite. This follow-up closes two recovery gaps within the task's declared scope.

### Decision and acceptance evidence

For R7/R8 and A4/A5, validate the latest resumed turn against the persisted
`turn/started` identity before allowing a new turn. Previously this comparison
only covered completed history: interrupted history returned no turn and bypassed
the comparison. A different interrupted turn, an empty history, or a missing
turn id now blocks recovery, persists one `thread.resume.failed` event, and starts
neither a replacement turn nor verification. Matching interrupted history still
permits continuation, and matching completed history goes directly to verification.

The runtime trait adds a compatible resume method carrying the expected turn id.
The local runtime validates both completed and interrupted server history. The
default implementation accepts a matching recovered turn and fails closed when
an older runtime cannot confirm the identity. Existing start/resume entry points
remain available. No framework, dependency, or transport was added.

A run superseded by an explicit retry is rejected before constructing a worker
or changing either run. This prevents an old interrupted run from claiming a task
whose latest run is also interrupted. Recovery continues to use the immutable
dispatch definition, preserving the assigned model, effort, prompt, and verify
command when the imported manifest changes.

Verification: `cargo test --workspace`, especially
`resume_reuses_run_and_thread_and_is_idempotent_after_completion`,
`superseded_interrupted_run_cannot_resume_or_change_the_current_run`, and
`resume_uses_the_immutable_assignment_when_the_manifest_changes`.

### Completed local dispatch receipt

The completed worker's recursion guards and local SQLite receipt were inspected
after the coordinator returned its terminal result:

- Task: `T-004`
- Route: `9c51d6bb-e24a-4384-b420-44f6aa7e89d5`
- Run: `ab86fa49-4873-4863-9b69-2b9d6b2945cb`
- Profile/model/effort: `deep` / `gpt-6.1-sol` / `high`
- Thread: `01a13225-9f4f-7460-96b7-8409746346fe`
- Turn: `01a13225-a00f-7ae3-a978-65eb898cdc46`
- Immutable verification command: `cargo test --workspace`

Implementation and the checks above are validated. The task returned `completed`
after the parent coordinator executed the immutable verification command. The
earlier failed run remains in SQLite as historical evidence. Spec status and
signed commit creation belong to the enclosing coordinator session because this
worker's scope is `router-core` and `.git` is read-only.

Protocol references: the installed App Server JSON schema was generated into
ignored local build output; its `ThreadResumeResponse` includes thread/turn ids,
history, and turn statuses. The official
[App Server documentation](https://learn.chatgpt.com/docs/app-server) describes
`thread/resume`, `turn/start`, and terminal notifications. No account settings or
credentials were read or modified for this check.
