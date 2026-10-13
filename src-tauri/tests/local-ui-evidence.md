# Local orchestration UI evidence — T-008

Specification: `specs/001-task-orchestration/spec.md`; task T-008.
Profile: `standard`; model: `gpt-6.1-sol`; effort: `medium`.
Route: `d1f95766-5e6f-4aae-b272-431846bc2d06`.
Run: `d53cfb52-2874-4a9e-90c9-ec743a7a4ca4`.

Implementation and worker verification are finished. The parent coordinator
returned `completed` after persisting the terminal turn and independent
verification result. The worker does not mutate its own lifecycle state. No
commit was created because repository metadata is read-only in this worker.

## Technical decision and scope

Reuse React, Tauri IPC and the workspace's existing bundled SQLite dependency.
The desktop crate now directly references the existing `rusqlite` workspace
dependency; the lockfile records that edge without adding a package or version.
The read model stays inside `src-tauri`; router-core and the CLI are unchanged.
A browser fixture lives under `ui/tests` and is excluded from the normal build.
No remote service, network client, credential, environment value or framework
was introduced. Production UI remains read-only: decisions and dispatch stay
behind the existing coordinator/CLI gates.

## Observable acceptance

- `local_snapshot` opens the same local store as the coordinator, in SQLite
  read-only mode. One read transaction returns tasks across specs, dependencies,
  runs with route ids, approval history/current decisions, and failure events.
- `local_run_events` reads only the requested run using bound parameters.
  It returns the latest 200 events in chronological order and the full count.
  Error history is also limited to 200 with an explicit full count.
- A missing database is an uninitialized empty state without creating a file.
  Existing unreadable, invalid or incompatible stores produce errors; the UI
  keeps the previous snapshot and explicitly marks it as potentially stale.
- React refreshes every five seconds or manually, exposes loading and empty
  states, shows local timestamps, and guards responses after selection changes
  or unmount. Event read failures are distinct from snapshot failures.
- Refreshing preserves expanded event payloads for the selected run. Changing
  to an empty snapshot clears the selection and its events. The UI provides no
  approval, dispatch or continuation mutation commands.

## Verification on 2026-10-13

- `cargo test --workspace`: **56 passed**, including five desktop integration
  tests. Coverage includes multiple specs, latest versus historical approvals,
  legacy/unassigned runs, nested protocol failures, interrupted terminal events,
  malformed payloads, event ordering/isolation/limits, SQL parameter binding,
  and observations that neither alter nor migrate the store.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.
- `pnpm --dir ui build --configLoader runner`: passed (30 modules).
  The ordinary build's default esbuild config loader could not traverse a
  sandbox-restricted ancestor directory. Vite's existing runner loader succeeds
  without changing dependencies or production configuration.
- Browser fixture build:
  `pnpm --dir ui build --config tests/vite.config.js --configLoader runner`.
  Preview on loopback with:
  `node node_modules/vite/bin/vite.js preview --config tests/vite.config.js --configLoader runner --host 127.0.0.1 --port 5173`
  from `ui`, then visit `/tests/ipc-fixture.html`.
- Chrome fixture checks passed for populated state, pending approval count,
  selected-run payload, failed snapshot/stale data, isolated event error,
  recovery, empty state, delayed event responses, readable timestamps and
  preservation of expanded details across polling. The fixture uses Tauri's
  existing IPC mock and synthetic data; it never opens the real state database.
  The temporary loopback preview and browser tab were closed after testing.

The native Tauri window was not exercised interactively. IPC command registration
compiled as part of the workspace tests; desktop read behavior was checked
against isolated databases and React behavior against the browser IPC fixture.
Milestone publication and signed commit handling remain with the parent workflow.
