# Evidencias de aceptación · 004

Tareas ejecutadas fuera del coordinador (decisión D6). Entorno: Windows 11,
Rust estable MSVC; CI en Windows y Linux. Los escenarios marcados como
pendientes se cierran en las tareas que los cubren.

| Escenario | Verificación | Resultado |
| --- | --- | --- |
| A01 cargar specs | `task_manifest::tests::accepts_valid_graph_and_finds_ready_dependents`, `the_repository_specs_load`, `task_service::project_specs_are_listed_and_loaded_with_their_warnings`; pantalla Tareas en el fixture | OK |
| A02 perfiles heredados | `task_manifest::tests::legacy_profiles_map_to_tiers_with_a_warning`, `task_cli::spec_load_translates_legacy_profiles_and_reports_it` | OK |
| A03 manifiestos inválidos | `task_manifest::tests::rejects_duplicate_ids_and_cycles`, `unknown_tiers_profiles_and_missing_tiers_are_rejected`, `sandboxes_stay_inside_the_project`, `task_cli::spec_load_rejects_*` | OK |
| A04 modelo de `router.toml` | `task_coordinator::runs_record_the_model_router_toml_had_at_dispatch` | OK |
| A05 riesgo y tier manual | `task_coordinator::risk_forces_fuerte_and_a_manual_tier_wins_on_the_record`, `task_service::the_board_shows_the_route_a_dispatch_would_take`; fixture | OK |
| A06 compuertas | `task_coordinator::batch_rechecks_dependencies_and_never_constructs_unapproved_runtime`, `denial_stays_blocked_until_an_explicit_approval`, `approval_unblocks_dispatch_without_starting_an_early_worker` | OK |
| A07 ejecución en la app | `task_coordinator::app_runs_wait_for_the_ui_to_answer_approvals`, `stopping_a_running_task_interrupts_its_turn`, `task_service::runs_stream_to_the_app_and_only_one_runs_at_a_time`; fixture | OK (fixture); app real en R004-T007 |
| A08 verificación | `task_coordinator::stdio_dispatch_persists_route_turn_deltas_and_independent_verification`, `protocol_errors_and_verification_failure_never_complete_tasks` | OK |
| A09 ejecutar listas | `task_coordinator::batch_rechecks_dependencies_and_never_constructs_unapproved_runtime`, `task_service::ready_tasks_run_in_dependency_order` | OK |
| A10 reanudar y reintentar | `task_coordinator::resume_reuses_run_and_thread_and_is_idempotent_after_completion`; reintento tras `thread.resume.failed` | Pendiente (R004-T006) |
| A11 CLI | `task_cli::*` | Pendiente (R004-T006) |
| A12 turnos internos | `task_cli::either_worker_marker_prevents_nested_lifecycle_commands`, `app_server_protocol` (entorno de la verificación) | OK |
| A13 spec de ejemplo con Codex real | App real | Pendiente (R004-T007) |

## A01 y A07 · Pantalla Tareas en el fixture

`ui/tests/chat-fixture.html?view=tareas` (2026-10-14), con los comandos de
tareas simulados y los eventos de `roudex://task` emitidos como lo hace
`TaskService`. Comprobado en el navegador de la app:

- **Cargar (A01):**
  - El selector lista la spec cargada (`001-comisiones · 1/6`) y la del
    proyecto sin cargar (`002-conciliacion · 2 tareas`).
  - La spec rota aparece como error: «003-borrador: la tarea D3-T2 depende de
    D3-T9, que no existe».
  - Al elegir `002-conciliacion` se carga y avisa «cargada con 1 aviso»; el
    aviso del perfil heredado se ve bajo la spec.
  - Las tareas aparecen agrupadas por hito (`H1 · Cálculo 1/3`,
    `H2 · Entrega 0/3`, o «sin hito»), con su estado y su tier resuelto.
  - El detalle muestra:
    - prompt, aceptación y verificación;
    - dependencias con su estado;
    - tier de la spec y ruta («regla de riesgo: produccion»);
    - riesgo, aprobación, alcance, sandbox y carpeta.
- **Ejecutar con aprobación (A07):**
  - `R1-T2` → «▶ ejecutar». El texto llega en vivo y el comando pendiente se
    muestra como «en curso».
  - La aprobación aparece en línea con las cuatro opciones; la lista muestra
    «esperando tu respuesta» y la barra lateral el indicador de ejecución.
  - Tras «aceptar», el comando termina con su salida. Durante la verificación
    se ve «verificación · $ cargo test -p reports en curso» y al final
    «verificación · salida 0» con la salida del comando.
  - Aviso final: «R1-T2 completada y verificada».
- **Detener (A07):**
  - `R1-T4` (interrumpida) → «reanudar». Se ve «hilo reanudado» y un turno
    nuevo en el mismo hilo.
  - «■ detener» lo termina como «turno interrumpido»; la tarea vuelve a
    «interrumpida» con «reanudar» y «reintentar».
- **Reintentar:**
  - `R1-T5` (falló con «salida 1» y la salida de la verificación) →
    «reintentar». Se crea una ejecución nueva, que termina verificada.
- **Compuertas y tier (A05, A06):**
  - `R1-T3` («Recalcula comisiones de producción…», `economico` en la spec)
    se resuelve a `fuerte` por riesgo.
  - «▶ ejecutar» está desactivado con «necesita aprobación».
  - Al bajarla a `economico` se ve el aviso de riesgo y la ruta «elegido
    manualmente pese al riesgo». «spec» la devuelve a `fuerte`, y «aprobar»
    deja la aprobación como «aprobada».
- **Ejecutar listas (A09):**
  - En `002-conciliacion`, «▶ ejecutar listas» corre `C2-T1` y luego `C2-T2`
    (que depende de la primera).
  - La pantalla sigue a la tarea en curso y muestra «en curso: spec
    002-conciliacion»; al final avisa de las dos completadas.
- **Hilos sin cambios:**
  - Tras compartir la transcripción y las aprobaciones con Tareas, un hilo
    sigue haciendo streaming, muestra la aprobación en línea («rechazar» →
    «✗ rechazado») y ofrece «re-ejecutar con fuerte».
- **Diseño:**
  - A 1280×720 no hay desbordamiento horizontal.
  - La lista de tareas y la transcripción hacen scroll por separado, y una
    tarea completada no muestra la barra de acciones vacía.
