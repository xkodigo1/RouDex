# Evidencias de aceptación · 003

Tareas ejecutadas fuera del coordinador (decisión D5). Entorno: Windows 11,
Rust estable MSVC, `codex app-server` de Codex Desktop.

| Escenario | Verificación | Resultado |
| --- | --- | --- |
| A01 aviso | `desktop_hook::tests::risky_prompt_on_a_lower_model_warns_without_blocking`, `hook_cli::risky_prompts_warn_and_are_recorded` | OK |
| A02 bloqueo | `desktop_hook::tests::blocking_option_blocks_risky_prompts_on_a_lower_model`, `hook_cli::the_blocking_option_blocks_risky_prompts` | OK |
| A03 modelo fuerte | `desktop_hook::tests::risky_prompt_on_the_strong_model_only_gets_context` | OK |
| A04 regla rápida | `desktop_hook::tests::fast_rules_recommend_the_economy_tier_quietly` | OK |
| A05 sin regla | `desktop_hook::tests::prompts_no_rule_settles_get_no_recommendation` | OK |
| A06 fallos | `hook_cli::failures_let_the_prompt_through` (stdin vacío, no JSON, sin sesión, `router.toml` inválido) | OK |
| A07 turnos internos | `hook_cli::roudex_internal_turns_are_ignored` | OK |
| A08 turno repetido | `desktop_hook::tests::a_repeated_turn_updates_one_row_and_the_summary_counts_it_once`, `hook_cli::a_repeated_turn_is_recorded_once` | OK |
| A09 instalar y desinstalar | `hook_cli::install_is_idempotent_and_uninstall_keeps_other_hooks`, `hook_cli::install_creates_hooks_json_and_uninstall_without_one_is_a_no_op`, `hook_cli::the_installed_command_runs_in_every_shell` | OK |
| A10 Resumen | Fixture en el navegador (con datos y vacío, 900 px) y ventana real de Tauri con datos aislados | OK |
| A11 Codex real | Pendiente: requiere instalar y confiar en el hook en el Codex del usuario | Pendiente |

## A10 · Detalle

- **Fixture** (`ui/tests/chat-fixture.html`, 2026-10-14):
  - Con datos: «148 mensajes · 9 hoy | 7 avisos de riesgo | 1 bloqueado | 62% en el modelo recomendado», barra por tier y «hook instalado».
  - `?desktop=empty`: «hook no instalado · router-cli hook-install» y el texto de ayuda.
  - A 900 px de ancho no hay desbordamiento horizontal.
- **Ventana real de Tauri** (build de depuración, `LOCALAPPDATA` y `CODEX_HOME` aislados, 7 llamadas al hook de release):
  - `desktop_hook_summary` por IPC devuelve 7 mensajes, 5 avisos, 1 sin regla y el comando instalado.
  - Resumen muestra «Codex Desktop | hook instalado | 7 mensajes · 7 hoy | 5 avisos de riesgo | 0 bloqueados».

## Rendimiento

`router-cli hook` en release: 62–82 ms por llamada, contando el arranque del
proceso (5 ejecuciones).
