# RouDex v0.5.0 — Local task orchestration milestone

This milestone delivers the first local task-orchestration slice driven by a
versioned executable specification. The publication audit is recorded in
[acceptance-evidence.md](acceptance-evidence.md).

RouDex coordinates local Codex tasks from an executable JSON specification.
It validates dependency graphs, resolves model and effort before dispatch,
requires explicit approvals, and stores execution and verification evidence in
SQLite. Interrupted runs can resume by run id with replay protection.

Desktop ingress records and deduplicates prompts without starting a worker or
duplicating the original turn. The CLI exposes task review, approval, dispatch,
status and continuation. The local Tauri/React interface displays tasks, runs,
events, approvals and failures without offering mutation actions.

Validation: 58 passing tests cover acceptance scenarios A1–A12, including
PowerShell boundary checks and a stdio App Server fixture. Workspace build,
format checks and production/fixture frontend builds pass. Clippy passes with
the three documented baseline lint exceptions.

Limits: native interactive Tauri smoke testing remains unverified. The live
installed Desktop hook was not exercised in this audit. Continuation was tested
with the local protocol fixture, not a live model. The audit retains the
historical T-001 assignment discrepancy. No installer or binary artifact is
included.
