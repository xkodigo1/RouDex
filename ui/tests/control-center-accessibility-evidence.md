# R002-T005 accessibility and state evidence

Date: 2026-10-13. Source: existing synthetic `ui/tests/ipc-fixture.html` only.

## Provenance and limits

- Browser fixture: `http://127.0.0.1:5188/tests/ipc-fixture.html`, served on loopback from the existing fixture build. No operational store or native Tauri window was used.
- Browser UI identified the first Tab stop as `Fixture: populated`; screenshot showed a light blue, 2 px outline with offset around the focused button. This confirms visible focus for that control only. The full application keyboard sequence was not established by this observation.
- This browser surface did not expose a viewport override, and the visible browser screenshot was 2560×1344. Neither requested viewport (1280×720 or 800×600) was actually set or observed. Results for those dimensions are therefore **not verified**; no responsive acceptance is claimed.
- Fixture evidence only; no native evidence or H2 completion is claimed.

## Observed fixture results

| State | Existing fixture mode / observed result |
| --- | --- |
| Populated | `populated`: 4 tasks and 3 runs including a retry; totals, milestone progress, unassigned task and status filters passed. |
| Empty | `empty`: explicit “No hay tasks registradas.” and no selected task; no progress indicators. |
| Stale | `read-error`: database-unavailable alert and stale-data message visible; prior selection retained. |
| Missing store | `missing`: explicit “Aún no existe estado local” message. |
| Error | `read-error` on overview: error and stale snapshot; `event-error` on details: event-read failure explicit, empty history and read-only retry recovered. |

## Keyboard and visible focus

- Sent Tab from the fixture page. Focus moved from the web area to the first fixture button, `Fixture: populated`.
- The screenshot showed a visible light-blue outline around that focused button. This is a direct observation at the available browser size, not either requested viewport.
- The existing fixture buttons and application controls appeared in the accessibility tree with names (for example, `Fixture: missing store`, `Actualizar`, and the labeled `Estado de task` selector). This is an accessibility-tree observation, not proof that every control is keyboard reachable.
- Keyboard navigation across the complete application and visible focus at 1280×720 and 800×600 remain unverified.

## Checks run

| Declared check | Result |
| --- | --- |
| `cargo test --offline -p roudex-desktop --test control_center` | PASS, exit 0; 5 passed, 0 failed. |
| `pnpm --dir ui build --config tests/vite.config.js --configLoader runner` | PASS, exit 0; Vite 7.3.7, 33 modules transformed. |
| Existing fixture `Run overview checks` | PASS, 21 checks, including populated, filters, stale/error, loading, empty and missing-store states; read-only IPC only. |
| Existing fixture `Run detail checks` | PASS, 20 checks, including event error/retry, missing run, empty events and response isolation; read-only IPC only. |
| Evidence artifact existence check | PASS, exit 0 after writing this file. |
| `git diff --check` | PASS, exit 0; Git reported only the tasks.md LF-to-CRLF notice. |

Viewport coverage remains an acceptance gap. R002-T005 stays `planned`; its scope and H2 status are unchanged.
