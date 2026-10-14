# Hook de Codex Desktop

El hook lleva el router de RouDex a Codex Desktop (y a Codex CLI). En cada
mensaje decide el tier con las [reglas del router](router.md) y:

- añade al turno un contexto breve con el tier recomendado y el riesgo;
- si el mensaje tiene riesgo y el chat no usa el modelo del tier `fuerte`,
  muestra un aviso con el modelo y el esfuerzo recomendados;
- registra la decisión en la base de datos local para el Resumen de la app.

El hook **no cambia el modelo del chat**: Codex no lo permite. Tampoco consulta
al clasificador, así que no añade latencia ni gasta cuota. Los mensajes que
ninguna regla resuelve quedan como «sin regla».

## Instalar

Compila `router-cli` y ejecútalo desde una ruta sin espacios:

```powershell
cargo build --release -p router-cli
.\target\release\router-cli.exe hook-install
```

`hook-install` añade una entrada `UserPromptSubmit` a
`%USERPROFILE%\.codex\hooks.json` (o `$CODEX_HOME\hooks.json`) con la ruta
absoluta de ese `router-cli`. Si mueves o recompilas el ejecutable en otra
carpeta, vuelve a ejecutarlo. Instalar dos veces deja una sola entrada, y los
demás hooks del archivo no se tocan.

Codex pide revisar y confiar en los hooks nuevos antes de ejecutarlos. En Codex
CLI se hace con `/hooks`; hasta entonces el hook no corre.

Si tenías el hook de `v0.3.x`–`v0.5.0` (`route-prompt`), `hook-install` lo
retira y lo indica en `removedLegacy`.

## Qué ves en Codex

| Mensaje | Modelo del chat | Resultado |
| --- | --- | --- |
| Con riesgo (producción, migraciones, credenciales, dinero…) | distinto del de `fuerte` | Aviso: «RouDex: mensaje de riesgo (…). Se recomienda el tier fuerte (…); este chat usa …». El mensaje se envía. |
| Con riesgo | el de `fuerte` | Solo contexto. |
| Corto de tipo explicar, resumir, traducir… | cualquiera | Solo contexto: recomienda `economico`. |
| Sin regla aplicable | cualquiera | Solo contexto: sin recomendación. |

Codex no envía el esfuerzo del chat al hook, así que la comparación es solo
por modelo.

## Bloquear mensajes de riesgo

Por defecto el hook solo avisa. Para bloquear los mensajes de riesgo enviados
con un modelo distinto del de `fuerte`, añade a
`%LOCALAPPDATA%\RouDex\router.toml`:

```toml
[desktop_hook]
block_high_risk = true
```

Codex no envía el mensaje y muestra a qué modelo y esfuerzo cambiar. Con el
modelo de `fuerte` el mensaje pasa. Nunca se bloquea nada más.

## Fallos

Si la entrada de Codex, `router.toml` o `.roudex/rules.toml` son inválidos, el
hook deja pasar el mensaje sin contexto y escribe el motivo en stderr. Si la
base de datos no está disponible, responde igual y no registra la decisión.

Los turnos que lanza la propia app de RouDex (chat y clasificador) no se
evalúan ni se registran.

## Qué se guarda

Una fila por turno de Codex en la tabla `desktop_prompts` de
`%LOCALAPPDATA%\RouDex\router.sqlite3`: sesión, turno, carpeta, modelo del
chat, decisión (tier, modelo, esfuerzo, fuente, riesgo) y acción (contexto,
aviso o bloqueo). **El texto del mensaje no se guarda.**

El Resumen de la app muestra los últimos 30 días: mensajes por tier, avisos,
bloqueos, cuántos usaron el modelo recomendado y si el hook está instalado.

## Desinstalar

```powershell
.\target\release\router-cli.exe hook-uninstall
```

Quita solo las entradas de RouDex de `hooks.json`; el resto se conserva.
