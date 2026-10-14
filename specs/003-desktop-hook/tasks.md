# Tareas · 003

`tasks.json` es el contrato; este archivo lo resume. Las tareas se implementan
fuera del coordinador (decisión D5), así que `route_id` y `run_id` quedan
nulos y cada evidencia lo indica.

## R003-T001 · Decisión del hook, configuración y registro

Módulo de `router-core` que convierte la entrada de `UserPromptSubmit` en una
decisión con reglas de riesgo y reglas rápidas, y en la respuesta para Codex
(contexto, aviso o bloqueo). Añade `[desktop_hook] block_high_risk` a
`router.toml`, la tabla `desktop_prompts` y el resumen para el panel.
Escenarios A01–A05, A08 y A10.

## R003-T002 · Comandos `hook`, `hook-install` y `hook-uninstall`

`router-cli hook` lee stdin, responde y deja pasar el mensaje ante cualquier
fallo; ignora los turnos internos de RouDex. La instalación es idempotente,
retira el hook antiguo y conserva los demás hooks. Escenarios A01, A02 y
A06–A09.

## R003-T003 · Tarjeta de Codex Desktop en Resumen

Comando Tauri con el resumen y el estado de instalación, y la tarjeta en
Resumen. Escenario A10.

## R003-T004 · Documentación, prueba real y cierre

`docs/desktop-hook.md`, prueba con el Codex del usuario, evidencias de A01–A11
y versión `0.7.0` en los manifiestos. Escenario A11.
