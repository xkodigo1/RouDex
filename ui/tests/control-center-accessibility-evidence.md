# R002-T005 accessibility and state evidence

Date: 2026-10-13. Sources: existing synthetic `ui/tests/ipc-fixture.html` and local browser fixture check.

## Provenance and limits

- Browser fixture: `http://127.0.0.1:5188/tests/ipc-fixture.html`, served on loopback from the existing fixture build. No operational store or native Tauri window was used.
- A subsequent Playwright check used viewports 1280×720 and 800×600. At those sizes `document.documentElement.scrollWidth` was respectively 1265 and 785, both below viewport width; no horizontal overflow was observed.
- At 800×600, Tab traversed the controls and left visible focus on the `Actualizar` button. This is browser fixture evidence, not a native Tauri keyboard check.
- Screenshots were temporary inspection artifacts and were not retained in the repository. No native evidence or H2 completion is claimed.

## Observed fixture results

| State | Existing fixture mode / observed result |
| --- | --- |
| Populated | `populated`: 4 tasks and 3 runs including a retry; totals, milestone progress, unassigned task and status filters passed. |
| Empty | `empty`: explicit “No hay tasks registradas.” and no selected task; no progress indicators. |
| Stale | `read-error`: database-unavailable alert and stale-data message visible; prior selection retained. |
| Missing store | `missing`: explicit “Aún no existe estado local” message. |
| Error | `read-error` on overview: error and stale snapshot; `event-error` on details: event-read failure explicit, empty history and read-only retry recovered. |

## Keyboard and visible focus

- Earlier browser observation: Tab focused `Fixture: populated` with a visible light-blue outline.
- Playwright browser fixture check: at 800×600, Tab traversed the controls and left visible focus on `Actualizar`.
- These observations cover the local browser fixture only; native Tauri keyboard traversal and focus remain unverified.

## Checks run

| Declared check | Result |
| --- | --- |
| `cargo test --offline -p roudex-desktop --test control_center` | PASS, exit 0; 5 passed, 0 failed. |
| `pnpm --dir ui build --config tests/vite.config.js --configLoader runner` | PASS, exit 0; Vite 7.3.7, 33 modules transformed. |
| Existing fixture `Run overview checks` | PASS, 21 checks, including populated, filters, stale/error, loading, empty and missing-store states; read-only IPC only. |
| Existing fixture `Run detail checks` | PASS, 20 checks, including event error/retry, missing run, empty events and response isolation; read-only IPC only. |
| Evidence artifact existence check | PASS, exit 0 after writing this file. |
| `git diff --check` | PASS, exit 0; Git reported only the tasks.md LF-to-CRLF notice. |

Browser viewport checks are recorded for both requested sizes; keyboard focus was observed at 800×600 only, and native evidence remains outstanding. R002-T005 stays `planned`; its scope and H2 status are unchanged.
