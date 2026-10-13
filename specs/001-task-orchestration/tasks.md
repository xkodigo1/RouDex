# Tasks — Especificación 001

Las tasks de implementación deben despacharse mediante RouDex. T-001 es la
task de definición de la especificación y se validó mediante el runtime actual
como revisión de solo lectura. T-002 y T-003 son el bootstrap explícito del
almacenamiento y la validación que el coordinador necesita para existir; no se
les inventa un `run_id`. Desde T-004, las tasks requieren `route_id`, `run_id`,
eventos y evidencia de verificación del coordinador.

| ID | Estado | Depende de | Resultado |
| --- | --- | --- | --- |
| T-001 | completed | — | Crear spec, decisiones y manifest ejecutable |
| T-002 | completed | T-001 | Modelar tasks, dependencias, aprobaciones y estados en dominio/SQLite |
| T-003 | completed | T-002 | Validar y cargar el manifest JSON como grafo acíclico |
| T-004 | completed | T-002, T-003 | Coordinador ejecutado mediante App Server local con modelo/esfuerzo asignados, evidencia persistida y verificación de 28 pruebas |
| T-005 | completed | T-004 | Gates de aprobación, fallos fail-closed, recursión e idempotencia de `runId` implementados y verificados con 40 pruebas |
| T-006 | planned | T-004, T-005 | Exponer comandos CLI de plan, ejecución, aprobación, estado y continuación |
| T-007 | planned | T-006 | Integrar el adaptador de escritorio con creación/actualización de tasks |
| T-008 | planned | T-006 | Conectar la UI local al estado de tasks, runs, eventos y aprobaciones |
| T-009 | planned | T-005, T-006, T-007, T-008 | Ejecutar aceptación A1–A8 y publicar milestone |

## Registro de ejecución

| ID | route_id | run_id | evidencia | commit |
| --- | --- | --- | --- | --- |
| T-001 | `8f79c7d6-d072-4b54-b31a-758ab3a79041` | `a7845c37-bbfb-49f5-8132-024b5e64f911` | Revisión RouDex: grafo acíclico; detectó campos y gates faltantes | pendiente |
| T-002 | bootstrap | — | `cargo test -p router-core` — 5 tests passed | pendiente |
| T-003 | bootstrap | — | `cargo test --workspace` — 8 router-core tests passed | pendiente |
| T-004 | `9c51d6bb-e24a-4384-b420-44f6aa7e89d5` | `ab86fa49-4873-4863-9b69-2b9d6b2945cb` | App Server local completó el thread y el turn asignados; `cargo test --workspace` pasó con 28 pruebas; se conservaron eventos, modelo, esfuerzo y run_id | `6cd5669` |
| T-005 | `29099428-fc64-45cf-ba0d-b3e1b2f3cf16` | `f3a93f3f-ed52-42d9-9cd1-bb0c9d530de4` | App Server local ejecutó la task con aprobación explícita; `cargo test --workspace` pasó con 40 pruebas; se registraron gates y evidencia transaccional | `f19ede6` |
| T-006 | — | — | pendiente | pendiente |
| T-007 | — | — | pendiente | pendiente |
| T-008 | — | — | pendiente | pendiente |
| T-009 | — | — | pendiente | pendiente |

## Definition of Done por task

- El resultado está implementado dentro del alcance declarado.
- Las pruebas o verificaciones de la task pasan.
- La evidencia queda registrada en este archivo o en un artefacto enlazado.
- El estado de la task y sus dependencias se actualiza.
- Se crea un commit firmado que no incluye cambios ajenos.
