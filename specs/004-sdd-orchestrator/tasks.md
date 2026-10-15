# Tareas · 004

`tasks.json` es el contrato; este archivo lo resume. Las tareas declaran
`tier` en lugar de `profile`, `model` y `effort` (decisión D1) y se implementan
fuera del coordinador (decisión D6).

## R004-T001 · Manifiestos por tier, hitos y validación

`tier` y `milestone` por tarea, lista de hitos, traducción de `profile` a tier
para manifiestos anteriores, y rechazo de tiers desconocidos y de
`danger-full-access`. Escenarios A01–A03.

## R004-T002 · Tiers con `router.toml` y reglas de riesgo

Al despachar, el tier se resuelve a modelo y esfuerzo con `router.toml`; las
reglas de riesgo fuerzan `fuerte`; el cambio manual queda guardado. La
ejecución registra tier, origen, modelo y esfuerzo. Escenarios A04 y A05.

## R004-T003 · Turnos en la sesión interactiva

Las tareas corren sobre la `Session` del chat: streaming, aprobaciones en la
interfaz, detener, verificación y reanudación, conservando las garantías del
coordinador. Escenarios A06–A10 y A12.

## R004-T004 · Comandos Tauri y ejecución en segundo plano

Comandos para listar y cargar specs, aprobar, cambiar tier, ejecutar, detener,
reintentar, reanudar y recuperar, con eventos en `roudex://task`.
Escenarios A01, A05–A07 y A09.

## R004-T005 · Pantalla Tareas

Pantalla nueva en el estilo de la app, con acciones y la transcripción en vivo
de la ejecución. Escenarios A01 y A07.

## R004-T006 · Limpieza y CLI por tiers

Se retira el código heredado, la CLI usa tiers y la sesión, y una tarea
bloqueada por `thread.resume.failed` se puede reintentar. Escenarios A10 y A11.

## R004-T007 · Documentación, spec de ejemplo y cierre

`docs/orchestrator.md`, una spec de ejemplo ejecutada de punta a punta en la
app contra Codex, evidencias de A01–A13 y versión `0.8.0`. Escenario A13.
