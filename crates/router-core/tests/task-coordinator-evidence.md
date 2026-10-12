# Task coordinator verification

Specification: `specs/001-task-orchestration/spec.md`, task `T-004`.

This change completes the existing coordinator's ready-task dispatch and
recovery boundaries without adding dependencies or a remote transport.

## Checks

- `cargo test --workspace`: 26 tests passed, including 9 new coordinator
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

## Live dispatch receipt

The implementation session itself was dispatched through the local coordinator:

- Route: `0b804590-4350-4706-935f-1e5fdb638c1a`
- Run: `7df4a224-db95-4e43-b5bd-eb9d8d05a949`
- Profile/model/effort: `deep` / `gpt-6.1-sol` / `high`
- Thread: `01a1321c-5a08-79b3-b9eb-03392c5a5ec0`
- Turn: `01a1321c-5ad2-7df2-a147-af5552305573`

The receipt, immutable task definition, thread start and turn start were read
back from local SQLite. This document does not assert live terminal completion:
the worker is still executing while writing it. The immutable dispatch definition
contains the descriptive verification value `Protocol test with a fake App Server
plus cargo test --workspace`, which is not an executable verification command.
The successful checks above do not override that recorded gate; the coordinator
must not mark the live task completed if its configured verification fails.

A signed commit could not be created because the worker has read-only access to
`.git`; staging failed with permission denied when creating `.git/index.lock`.
