# T-006 CLI verification

Specification: `specs/001-task-orchestration/spec.md`, task T-006.

Execution is the existing Router worker, with the internal recursion guard
retained. The local store identified this dispatch as:

- route_id: `b7aefd6d-b13b-44f7-8ade-b5a998be84ab`
- run_id: `fc30becd-b90d-4116-826c-697068df92c6`

Implemented `spec-load`, `task-list`, and `task-dispatch` using existing manifest,
SQLite, and coordinator APIs. Existing `task-approve`, `task-status`,
`continue-run`, and `run-events` expose approval, status, continuation and event
evidence. `task-plan` remains compatible. Argument checks reject incomplete or
extra arguments before opening state. New mutating commands honor both worker
recursion markers. Manifest reload reports persisted readiness.

Technical choice: reuse the current Rust argument parser and local coordinator;
no framework, dependency, remote service or alternate task executor was added.

Verification: `cargo test --workspace` passed all 44 tests (14 CLI integration,
10 core unit, 1 App Server protocol, 19 coordinator integration). Coverage
includes invalid manifests, persisted status/approval/run evidence, dependency
and approval gates, runtime failure, completed-task reload, unknown spec,
argument errors, recursive command rejection, and continuation replay gates.
The CLI gate tests use isolated state and an unavailable runtime; successful
stdio dispatch and interrupted resume are covered by the core protocol suite.

The parent coordinator returned `completed` after executing the immutable
`cargo test --workspace` gate. This worker does not update its own task state or
create commits because the worker filesystem policy exposes `.git` read-only.
