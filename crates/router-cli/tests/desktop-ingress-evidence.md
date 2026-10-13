# Desktop task ingress evidence — T-007

Specification: `specs/001-task-orchestration/spec.md`, scenarios A9–A12.
Depends on T-006; assigned profile `deep`, model `gpt-6.1-sol`, effort `high`.
Route: `b9cd7413-d7cd-4125-9936-c7424e99cec1`.
Run: `2a54340d-f8fd-4c27-8b52-b514fbdb57db`.

Implementation and local checks are finished. The parent coordinator returned
`completed` after persisting the terminal turn and passing the independent
verification gate. No signed commit was created in this worker because
repository metadata is read-only here.

Validation: `cargo test --workspace` passed **51 tests** (19 CLI, 10 core unit,
1 App Server protocol, 21 coordinator integration). `cargo fmt --all -- --check`
and `git diff --check` passed.
Revalidated on 2026-10-13 after requiring a JSON object and a scalar string
decision at the PowerShell boundary, with regressions for array responses,
array decisions and a string-valued continuation flag.

- A9: CLI tests create one blocked draft, retain the same task on replay,
  reject changed input using the same session/turn, and serialize eight
  concurrent deliveries into one task and zero runs. The old `--record-only`
  flag has the same ingress-only semantics.
- A10: CLI tests update an explicit pending target, preserve scope/sandbox/
  assignment/verification, require renewed approval, reject a stale dispatch
  snapshot, and reject updates to active or previously executed tasks and
  tasks in a different cwd. Transaction failure rolls back the prompt,
  approval invalidation and delivery record together.
- A11: a coordinator integration test blocks a draft even after approval,
  then loads a reviewed contract and requires approval before invoking the
  fake App Server over stdio. Completion and replay yield exactly one run,
  one thread and one turn with the assigned model and effort. Existing
  coordinator tests continue to cover runtime failure and recovery.
- A12: actual Windows PowerShell entrypoint tests block a missing executable,
  nonzero exit, malformed or empty output, and an unexpected continuation.
  CLI tests block storage and validation failures. Internal workers pass
  without persistence even with malformed input; UTF-8 prompts round-trip.

Tests use isolated local stores and the protocol fixture. Installed Desktop
hook configuration was not changed or tested in a live Desktop conversation.
An operator must reinstall the hook to deploy the updated wrapper. Real
production prompts and installed routing policy were not used for testing.
