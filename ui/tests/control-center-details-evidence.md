# R002-T004 task and run detail verification

Date: 2026-10-13. Scope: `ui/src`, synthetic `ui/tests/ipc-fixture.html` and this evidence artifact. Scenario: A02.

## Execution provenance

- Active coordinator worker: `R002-T004`; `ROUDEX_ROUTER_INTERNAL` recursion guard present.
- Read-only SQLite inspection observed dependency `R002-T003` as `completed`, the latest existing manual approval as `approved`, and this task as `running`.
- Assigned and persisted route: `standard / gpt-6.1-sol / medium`.
- Run id: `df160fa7-64aa-40ee-bdee-31874d8c2ac9`.
- Route id: `d70508e8-6556-4da3-bc48-57cb5043bf8b`.
- Observed ordered lifecycle event identities: `111242 task.started`, `111243 thread.started`, `111244 turn/started`, `111245 thread/started`. Only event ids/types and allowlisted task/run fields were inspected; operational prompt/event bodies were not read into this artifact.
- The parent coordinator owns terminal state and independent verification after this worker returns. No approval, import, dispatch, continuation or lifecycle mutation was performed by the worker.

## Resulting behavior

Selecting a task shows its dependency links and statuses, profile/model/effort, approval policy and latest recorded decision/reason, and selected run/route identity. A task defaults to its newest registered run using the snapshot's existing descending persistence order. Selecting a historical run selects its owning task and preserves that explicit run across refresh. Dependency buttons select the target task by its stable selection key, including when the target is outside the active status filter. Missing dependency records are labeled unavailable. No approval capability is exposed.

Event state belongs to a component keyed by selected task/run identity. Switching identity immediately replaces its state; effect cleanup ignores late successes and failures. Responses with another run identity are rejected. Events render in ascending persisted event-id order. Missing run identity, loading, empty history and read failure are explicit; a failed read can be retried through the same read-only IPC command. No new persistence, framework, dependency, remote service or execution control was added. The only application IPC calls remain `local_snapshot` and `local_run_events`.

## Actual verification results

| Check | Observed result |
| --- | --- |
| `pnpm --dir ui build --configLoader runner` | PASS, exit 0; Vite 7.3.7, 30 modules. |
| `pnpm --dir ui build --config tests/vite.config.js --configLoader runner` | PASS, exit 0; 33 modules. |
| Built fixture in Chrome: `Run detail checks` | PASS, all 20 checks below. |
| Built fixture in Chrome: `Run overview checks` | PASS, all 21 existing regression checks, including filtering, retained selection, stale/error, loading, removed task, empty and missing store states. |
| Browser screenshot inspection | PASS: selected task assignment, denied approval/reason, dependency button, run/route identity and ascending events 1/2/3 visible. This is fixture evidence only. |
| Evidence artifact existence check | PASS, exit 0, after this artifact was written. |
| `git diff --check` | PASS, exit 0; Git line-ending notices only. |

Both builds used already available local dependencies; nothing was installed. The built synthetic fixture was temporarily served only on loopback:

```text
pnpm --dir ui preview --configLoader runner --outDir dist/fixture --host 127.0.0.1 --port 5178 --strictPort
```

Open `/tests/ipc-fixture.html` and click `Run detail checks`. The suite acts on the rendered React UI. It introduces a 1200 ms prior-run delay and switches to a 50 ms run to exercise success/error races; it also supplies deliberately unordered events and a wrong-run response. Fixture data and approval decisions are synthetic, not writes to the operational store.

Observed detail output:

```text
PASS: Selected task exposes route assignment
PASS: Approval decision and reason visible
PASS: Latest task run and route identity visible
PASS: Events ordered by persisted id
PASS: Dependency link selects exact task outside filter
PASS: Approved state visible
PASS: Switch clears prior-run loading and identity
PASS: Late prior-run success cannot contaminate selected task
PASS: Event loading explicit
PASS: Late prior-run failure cannot contaminate selected task
PASS: Missing run identity explicit, prior events cleared
PASS: Approval not required and no dependencies explicit
PASS: Unreviewed task approval remains pending
PASS: Event-read failure explicit
PASS: Read-only event retry recovers
PASS: Historical run navigation retains task and changes run
PASS: Refresh preserves explicit historical run
PASS: Empty event history explicit
PASS: Wrong-run IPC response rejected
PASS: Read-only IPC commands only
20 detail checks passed. Synthetic browser evidence only; no native runtime claim.
```

## Evidence limits

These results verify browser fixture behavior and compilation, not native Tauri smoke, comprehensive accessibility/viewport acceptance, or H2 completion. Those remain later task gates. The existing approval record is reported without granting an approval. Worker results do not assert their own terminal coordinator status. No publication was performed.
