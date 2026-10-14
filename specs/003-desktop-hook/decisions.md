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

## D6 · El hook vive fuera de AppData

*Sustituye la ubicación de D3 (la base de datos) y de D4 (el ejecutable).*

Codex Desktop es una app empaquetada (MSIX). Para los procesos que lanza,
Windows sustituye `%LOCALAPPDATA%` y `%APPDATA%` por una copia privada del
paquete. En la prueba real, la ruta del hook resolvía a un `router-cli` de
`v0.5.0` que quedó en esa copia: imprimía su ayuda, salía con 0 y Codex no
recibía contexto. Las escrituras también habrían ido a la copia.

- `hook-install` copia `router-cli` a `~/.roudex/bin` (`ROUDEX_HOME`) y apunta
  `hooks.json` ahí; rechaza rutas dentro de AppData.
- El hook registra en `~/.roudex/desktop-hook.sqlite3`; la app lee de ahí.
- `router.toml` se sigue leyendo de `%LOCALAPPDATA%\RouDex`: la lectura cae al
  archivo real mientras el paquete no tenga su propia copia.
- Descartado: mover todos los datos de RouDex fuera de AppData. Es un cambio
  mayor que no hace falta para el hook.

## D7 · El aviso lo repite el modelo

Codex Desktop no muestra el `systemMessage` de un hook. En modo aviso, el
contexto pide al modelo que empiece su respuesta con la línea `⚠ RouDex: …`.
`systemMessage` se mantiene para Codex CLI.

- Descartado: bloquear para que el aviso se vea. Contradice D2.

## D8 · Registro de fallos

Codex Desktop no muestra el stderr de los hooks. Cada fallo se escribe también
en `~/.roudex/hook.log` con el error y los nombres y tipos de los campos
recibidos, nunca sus valores. El archivo se reinicia al pasar de 64 KiB.
