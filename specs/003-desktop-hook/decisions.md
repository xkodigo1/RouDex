# Decisiones · 003

## D1 · Solo reglas en el hook

El hook decide con las reglas de riesgo y las reglas rápidas. Los mensajes que
ninguna regla resuelve quedan como «sin regla» con el tier `fallback_tier`.

- Descartado: llamar al clasificador. Añade unos 5 s y gasta cuota en cada
  mensaje de Codex Desktop, y el hook no puede cambiar el modelo de todas
  formas.
- Descartado: clasificador opcional. Se puede añadir después sin romper nada.

## D2 · Avisar por defecto, bloquear opcional

Con riesgo y un modelo distinto del de `fuerte`, el hook devuelve
`systemMessage` (visible) y `additionalContext`. Con
`[desktop_hook] block_high_risk = true` devuelve `decision: "block"` con el
modelo al que cambiar.

- El hook recibe el `model` del chat pero no el esfuerzo, así que la
  comparación es solo por modelo.
- Descartado: avisar siempre sin opción de bloqueo. El roadmap pide bloqueo
  opcional.

## D3 · Registro en SQLite sin el texto del mensaje

Tabla `desktop_prompts`, clave `(session_id, turn_id)`. Se guarda la decisión,
no el mensaje.

- Si Codex repite el hook para el mismo turno, la fila se actualiza en vez de
  duplicarse.

## D4 · Instalación en el `hooks.json` de usuario

`hook-install` escribe en `$CODEX_HOME/hooks.json` (por defecto
`~/.codex/hooks.json`) una entrada `UserPromptSubmit` que ejecuta el
`router-cli` instalado con el argumento `hook`.

- Codex pide revisar y confiar en los hooks no administrados antes de
  ejecutarlos; la documentación lo explica.
- Descartado: hook administrado. Requiere permisos de administrador y fue la
  causa de los problemas de `v0.5.0`.
- Descartado: botón de instalación en la interfaz. La app no sabe dónde está el
  `router-cli` instalado.

## D5 · Implementación fuera del coordinador

`AGENTS.md` pide no usar RouDex para construir RouDex hasta que sea estable.
Las tareas se implementan directamente y se registran con `route_id` y
`run_id` nulos.
