# Specification 001 acceptance and publication audit

Audit date: 2026-10-13. Baseline: `d7a23d1d96dba744cc532c8be8f2e2264224755a`.
Automated acceptance: **58 passed, 0 failed, 0 ignored**.
Milestone publication: **blocked; no new commit, tag or release created**.

## Routed execution

This audit runs inside the existing coordinator worker for T-009. Its recursion
guards remain active; no nested dispatch, approval or lifecycle mutation was
performed. The local SQLite store was inspected using a read-only connection.

- Profile/model/effort: `frontier` / `gpt-6-astra` / `high`.
- Route: `0f464250-0e29-4374-a7fb-aeea3b252c9b`.
- Run: `6ad1dc47-1040-4dfd-b6d5-a9fc16e21df0`.
- Thread: `01a133b4-9f2f-7610-9f6b-c43f69c0e614`.
- Turn: `01a133b4-9feb-77a2-a60f-d4a358e9ea8b`.
- Observed state: `running`. Terminal evidence belongs to the enclosing
  coordinator after this worker exits; it is not fabricated here.

## Reproducible checks

| Command | Result |
| --- | --- |
| `cargo test --workspace` | 58 passed: 21 CLI, 21 coordinator, 10 core unit, 5 desktop read-model, 1 App Server protocol |
| `cargo fmt --all -- --check` | Passed |
| `git diff --check` | Passed |
| `cargo build --workspace` | Passed; includes the native desktop binary |
| `cargo clippy --workspace --all-targets -- -D warnings -A clippy::result_large_err -A clippy::too_many_arguments -A clippy::redundant_closure_call` | Passed using the three previously documented baseline exceptions |
| `pnpm --dir ui build --configLoader runner` | Passed, 30 modules |
| `pnpm --dir ui build --config tests/vite.config.js --configLoader runner` | Passed, 33 modules |

The initial suite passed 56 tests. A7 required three independent rejection cases;
the CLI check now separately tests duplicate ids, missing dependencies and
cycles, and asserts that rejection happens before creating the state database.
The final complete suite passed 58 tests. No runtime code, dependency, framework
or network service changed. Build products remain under ignored directories.

## Acceptance mapping

Test names below are executable evidence, not conclusions inferred from model
output. Coordinator tests live in `crates/router-core/tests/task_coordinator.rs`;
CLI tests live in `crates/router-cli/tests/hook_protocol.rs`.

| Scenario | Passing evidence |
| --- | --- |
| A1 ready task | `stdio_dispatch_persists_route_turn_deltas_and_independent_verification`; `app_server_stdio_receives_assigned_model_and_effort` in `app_server_protocol.rs`; real local App Server receipts for T-004 through T-008 inspected below |
| A2 dependency gate | `missing_dependency_prevents_runtime_and_route_creation`; `batch_rechecks_dependencies_and_never_constructs_unapproved_runtime` |
| A3 approval | `approval_unblocks_dispatch_without_starting_an_early_worker`; `denial_stays_blocked_until_an_explicit_approval`; core `manual_task_waits_for_approval_without_runtime_call` |
| A4 continuation | `resume_reuses_run_and_thread_and_is_idempotent_after_completion`; `resume_uses_the_immutable_assignment_when_the_manifest_changes`; `durable_terminal_evidence_prevents_replay_after_a_crash` |
| A5 fail closed | `runtime_factory_failure_is_persisted_and_does_not_abort_independent_tasks`; `protocol_errors_and_verification_failure_never_complete_tasks`; `missing_recovery_identity_is_durably_blocked_without_starting_runtime` |
| A6 recursion | `internal_worker_bypasses_the_hook_once`; `active_task_marker_alone_skips_ingress_without_persisting_a_run`; `either_worker_marker_prevents_nested_lifecycle_commands` |
| A7 invalid graph | `spec_load_rejects_duplicate_ids_before_importing_any_tasks`; `spec_load_rejects_missing_dependency_before_importing_any_tasks`; `spec_load_rejects_cycle_before_importing_any_tasks` |
| A8 independent verification | `protocol_errors_and_verification_failure_never_complete_tasks`; `invalid_runtime_identity_or_status_fails_without_verification`; `terminal_state_and_audit_event_roll_back_together_on_database_failure` |
| A9 desktop replay | `desktop_delivery_creates_a_blocked_task_and_replays_without_execution`; `concurrent_desktop_deliveries_persist_one_task`; `delivery_identity_conflicts_and_invalid_messages_fail_closed` |
| A10 pending update | `desktop_update_preserves_contract_revokes_approval_and_rejects_stale_dispatch`; `ingress_cannot_overwrite_active_or_terminal_work_or_cross_working_directories`; `desktop_delivery_failure_rolls_back_task_and_approval_changes` |
| A11 reviewed contract | `desktop_ingress_reaches_stdio_only_after_contract_and_approval_review` |
| A12 entrypoint errors | `powershell_boundary_blocks_missing_failed_and_malformed_routers`; `router_storage_failure_blocks_the_prompt`; CLI recursion and Unicode cases |

A4 performs an actual coordinator `resume_run(run_id, ...)` through a fake App
Server over local stdio: one original thread, one resume request, the same
run/route, a new turn for interrupted history, and unchanged events on replay.
This satisfies the specified fake-runtime verification. No live-model
continuation is claimed: the inspected operational store had no resume events.

## Task evidence and signed history

T-004 through T-008 each have a completed run, matching profile/model/effort,
`task.started`, `thread.started`, protocol turn events, one normalized terminal
event and one `task.verified` with command `cargo test --workspace` and
`result.exitCode = 0`. Thread and terminal turn identities were read from events;
the legacy `runs.thread_id` column is null and `runs.turn_id` holds the task id.
These columns alone must not be treated as runtime identity evidence.

| Task | Commit | Audit result |
| --- | --- | --- |
| T-001 | `94ca875` | Four SDD artifacts exist; historical pre-coordinator review, with assignment discrepancy described below |
| T-002 | `7c6018e` | Explicit bootstrap; persistence checks pass; no invented run id |
| T-003 | `7029163` | Explicit bootstrap; manifest checks pass; no invented run id |
| T-004 | `6cd5669` | Completed coordinator run and successful independent verification |
| T-005 | `f19ede6` | Completed coordinator run and successful independent verification |
| T-006 | `00ed728` | Completed coordinator run and successful independent verification |
| T-007 | `7ee89e3` | Completed coordinator run and successful independent verification |
| T-008 | `d21242a` | Completed coordinator run and successful independent verification; native interactive smoke remains unverified |
| T-009 | Not committed | Acceptance audit prepared; publication blocked |

T-001's manifest declares `routine / gpt-6-luna / medium`, but its historical
route/run records report `standard / gpt-6.1-sol / medium`. The legacy run remains
`accepted`, with thread `01a131f6-73d6-7af0-a2f7-8a1452bb46d7` and a completed
turn `01a131f6-747e-7550-b276-e82f0f95fbe2`; it has no `task.verified` event.
It is evidence of a legacy specification review, not model-enforced execution
under the current manifest. This discrepancy is preserved for the enclosing
coordinator to reconcile before declaring the task audit fully clear.

All 22 commits in `3f6377f..d7a23d1` passed `git verify-commit` against the
configured public signing key. Verification used a temporary allowed-signers
file in ignored build output because the configured trust file is inaccessible
to this worker. This proves signature integrity against that configured key;
GitHub's verification status was not available.

The initial worktree was clean. Tracked files and reachable historical blobs
were scanned for common private-key/token patterns and local-state filenames;
see the publication handoff for the final scan count. Such a heuristic scan
does not certify that every possible secret format is absent. No raw database,
worker transcript, credential, personal machine path or build output was added
to the publication artifacts.

## Remaining publication gates

1. Reconcile T-001's declared versus historical assignment explicitly; do not
   rewrite its old receipt or claim a verification event that does not exist.
2. Exercise the native Tauri window and record its IPC/display result, or
   explicitly retain that unverified limitation in the release decision. The
   desktop binary builds, five read-model tests pass, and the earlier browser
   fixture evidence is in `src-tauri/tests/local-ui-evidence.md`. Interactive
   native control is unavailable in this worker, and those earlier browser
   interactions were not rerun here.
3. Let the enclosing coordinator collect this run's terminal and independent
   verification evidence. A passing immutable `cargo test --workspace` command
   alone does not satisfy T-009's publication acceptance criteria.
4. Create and verify scoped signed commits and a signed annotated tag, push
   them, publish the release, and verify the remote references/metadata.

The worker's repository metadata is read-only, so it cannot stage, commit or
tag. GitHub release inspection also failed because outbound socket access is
denied. These are environment restrictions, not missing user authorization;
publication was already requested. No attempt was made to bypass either
restriction. The milestone must remain open until the publication evidence
exists. See [publication-handoff.md](publication-handoff.md).
