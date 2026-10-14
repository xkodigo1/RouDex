# Decisiones · 004

## D1 · Las tareas declaran un tier, no un modelo

Cada tarea declara `tier`. El modelo y el esfuerzo se resuelven al ejecutar
con `router.toml`, igual que en el chat, y quedan registrados en la ejecución.

- **Compatibilidad:** una tarea sin `tier` pero con `profile` (manifiestos
  anteriores) se carga traduciendo el perfil: `micro` y `routine` →
  `economico`, `standard` → `estandar`, `deep` y `frontier` → `fuerte`. El
  `model` y el `effort` declarados se ignoran y la carga lo avisa. El formato
  se decide por tarea, no por el campo `version` del manifiesto, que ya es el
  número de revisión de la spec.
- **Riesgo:** las reglas de riesgo (las de RouDex y las de
  `.roudex/rules.toml`) se aplican al título y al prompt de la tarea; si
  coinciden, el tier pasa a `fuerte` y la razón queda visible.
- **Cambio manual:** el usuario puede cambiar el tier de una tarea antes de
  ejecutarla; se guarda como decisión manual, igual que en el chat.
- Descartado: clasificar cada tarea con modelo. El tier lo decide quien
  escribe la spec; el clasificador gastaría cuota en cada carga.
- Descartado: mantener la tabla fija de perfiles. Duplica `router.toml` y no
  sigue los cambios de modelos.

## D2 · Las tareas corren en la sesión del chat

Los turnos de tarea usan la misma `Session` interactiva que el chat:
streaming hacia la interfaz, aprobaciones `on-request` que el usuario
responde, y `turn/interrupt` para detener. Cada ejecución de tarea tiene su
propio proceso `codex app-server` (como hasta ahora el coordinador), porque
el registro del chat guarda los eventos de los turnos que no conoce: compartir
el proceso mezclaría ambos.

- Un registro de ejecuciones activas (`TaskRuns`) permite a la interfaz
  responder aprobaciones y detener el turno de una tarea.

- El coordinador conserva el ciclo de vida, las compuertas, la persistencia de
  eventos y la reanudación; cambia solo el runtime que ejecuta el turno.
- Desde la CLI, las aprobaciones se rechazan (como hoy en `router-cli chat`).
- La verificación sigue siendo `command/exec` en el mismo sandbox de la tarea,
  con `ROUDEX_ROUTER_INTERNAL` y `ROUDEX_ACTIVE_TASK_ID`.
- Descartado: mantener `approvalPolicy: "never"`. Cualquier petición del
  servidor aborta el turno y el usuario no puede intervenir.

## D3 · Sandbox limitado al proyecto

El sandbox de una tarea es `read-only` o `workspace-write` con raíces dentro
de la carpeta del proyecto, sin red. Un manifiesto con `danger-full-access` no
se carga.

- Coherente con la decisión de producto «escritura limitada a la carpeta del
  proyecto».

## D4 · La pestaña Tareas se rehace con el diseño actual

La vista heredada (solo lectura, sondeo cada 5 s, JSON crudo) se sustituye por
una pantalla con el estilo terminal del resto de la app: selector de spec, hitos
y tareas, detalle con acciones y la transcripción en vivo de la ejecución,
reutilizando los componentes de Hilos.

- Las escrituras pasan por comandos Tauri sobre el mismo `RouterStore` que usa
  el chat; la lectura de solo lectura para la vista heredada desaparece.

## D5 · Limpieza del código heredado

Se retiran `route-preview`, `RoutingConfig` y la tabla de perfiles (salvo la
traducción de D1), `RunStatus`, los métodos de `AppServerClient` que solo usa
un test, la vista heredada y su fixture. Se corrige el callejón sin salida de
una tarea bloqueada por `thread.resume.failed`, que hoy no se puede reintentar.

## D6 · Implementación fuera del coordinador

Como en la spec 003 (D5): `AGENTS.md` pide no usar RouDex para construir RouDex
hasta que sea estable, así que estas tareas se implementan directamente y se
registran con `route_id` y `run_id` nulos.
