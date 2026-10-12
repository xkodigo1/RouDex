# Tasks — Especificación 001

Las tasks de implementación deben despacharse mediante RouDex. T-001 es la
task de definición de la especificación y se validó mediante el runtime actual
como revisión de solo lectura; las demás requieren `route_id`, `run_id`,
eventos y evidencia de verificación del coordinador de tasks.

| ID | Estado | Depende de | Resultado |
| --- | --- | --- | --- |
| T-001 | completed | — | Crear spec, decisiones y manifest ejecutable |
| T-002 | ready | T-001 | Modelar tasks, dependencias, aprobaciones y estados en dominio/SQLite |
| T-003 | pending | T-002 | Validar y cargar el manifest JSON como grafo acíclico |
| T-004 | pending | T-002, T-003 | Coordinar dispatch real por App Server con modelo y esfuerzo asignados |
| T-005 | pending | T-004 | Persistir eventos, fallos, recursión e idempotencia de resume |
| T-006 | pending | T-004, T-005 | Exponer comandos CLI de plan, ejecución, aprobación, estado y continuación |
| T-007 | pending | T-006 | Integrar el adaptador de escritorio con creación/actualización de tasks |
| T-008 | pending | T-006 | Conectar la UI local al estado de tasks, runs, eventos y aprobaciones |
| T-009 | pending | T-005, T-006, T-007, T-008 | Ejecutar aceptación A1–A8 y publicar milestone |

## Registro de ejecución

| ID | route_id | run_id | evidencia | commit |
| --- | --- | --- | --- | --- |
| T-001 | `8f79c7d6-d072-4b54-b31a-758ab3a79041` | `a7845c37-bbfb-49f5-8132-024b5e64f911` | Revisión RouDex: grafo acíclico; detectó campos y gates faltantes | pendiente |
| T-002 | — | — | pendiente | pendiente |
| T-003 | — | — | pendiente | pendiente |
| T-004 | — | — | pendiente | pendiente |
| T-005 | — | — | pendiente | pendiente |
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
