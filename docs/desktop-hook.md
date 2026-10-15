# Hook de Codex Desktop

El hook lleva el router de RouDex a Codex Desktop (y a Codex CLI). En cada
mensaje decide el tier con las [reglas del router](router.md) y:

- añade al turno un contexto breve con el tier recomendado y el riesgo;
- si el mensaje tiene riesgo y el chat no usa el modelo del tier `fuerte`,
  avisa con el modelo y el esfuerzo recomendados;
- registra la decisión para la tarjeta «Codex Desktop» del Resumen de la app.

El hook **no cambia el modelo del chat**: Codex no lo permite. Tampoco consulta
al clasificador, así que no gasta cuota y tarda unos 65 ms por mensaje. Los
mensajes que ninguna regla resuelve quedan como «sin regla».

## Instalar

```powershell
cargo build --release -p router-cli
.\target\release\router-cli.exe hook-install
```

`hook-install`:

1. Copia `router-cli` a `%USERPROFILE%\.roudex\bin\` (o `$ROUDEX_HOME\bin`).
2. Añade una entrada `UserPromptSubmit` a `%USERPROFILE%\.codex\hooks.json`
   (o `$CODEX_HOME\hooks.json`) que ejecuta esa copia.
3. Retira el hook `route-prompt` de `v0.3.x`–`v0.5.0` del `hooks.json` de
   usuario (`removedLegacy`) y avisa si sigue el hook administrado de `v0.5.0`
   (`legacyManagedHook`, ver más abajo).

Instalar dos veces deja una sola entrada y no toca los demás hooks. Para
actualizar el hook después de recompilar, vuelve a ejecutar `hook-install`.

Codex pide revisar y confiar en los hooks nuevos antes de ejecutarlos: en
Codex Desktop, en la pantalla de hooks; en Codex CLI, con `/hooks`. Hasta
entonces el hook no corre.

### Por qué fuera de AppData

Codex Desktop es una app empaquetada de Windows (MSIX). Para los procesos que
lanza, Windows sustituye `%LOCALAPPDATA%` y `%APPDATA%` por una copia privada
del paquete: un ejecutable o una base de datos ahí no serían los reales. Por
eso el hook, sus datos y su log viven en `%USERPROFILE%\.roudex\`, y
`hook-install` rechaza rutas dentro de AppData. La ruta tampoco puede tener
espacios: Codex no documenta qué shell ejecuta el comando.

## Qué ves en Codex

| Mensaje | Modelo del chat | Resultado |
| --- | --- | --- |
| Con riesgo (producción, migraciones, credenciales, dinero…) | distinto del de `fuerte` | Aviso «⚠ RouDex: mensaje de riesgo (…). Se recomienda el tier fuerte (…); este chat usa …». El mensaje se envía. |
| Con riesgo | el de `fuerte` | Solo contexto. |
| Corto de tipo explicar, resumir, traducir… | cualquiera | Solo contexto: recomienda `economico`. |
| Sin regla aplicable | cualquiera | Solo contexto: sin recomendación. |

Codex Desktop no muestra el `systemMessage` de los hooks, así que el contexto
le pide al modelo que empiece su respuesta con la línea del aviso. Codex CLI
muestra además el `systemMessage`.

Codex no envía el esfuerzo del chat al hook, así que la comparación es solo
por modelo.

## Bloquear mensajes de riesgo

Por defecto el hook solo avisa. Para bloquear los mensajes de riesgo enviados
con un modelo distinto del de `fuerte`, añade a `~/.roudex/router.toml`:

```toml
[desktop_hook]
block_high_risk = true
```

Codex no envía el mensaje y muestra a qué modelo y esfuerzo cambiar. Con el
modelo de `fuerte` el mensaje pasa. Nunca se bloquea nada más.

## Fallos

Si la entrada de Codex, `router.toml` o `.roudex/rules.toml` son inválidos, el
hook deja pasar el mensaje sin contexto. Si la base de datos no está
disponible, responde igual y no registra la decisión.

Codex Desktop no muestra los errores de los hooks, así que cada fallo queda
también en `%USERPROFILE%\.roudex\hook.log`: el error y los nombres de los
campos recibidos, nunca el texto del mensaje.

Los turnos que lanza la propia app de RouDex (chat y clasificador) no se
evalúan ni se registran.

## Qué se guarda

Una fila por turno de Codex en `%USERPROFILE%\.roudex\desktop-hook.sqlite3`:
sesión, turno, carpeta, modelo del chat, decisión (tier, modelo, esfuerzo,
fuente, riesgo) y acción (contexto, aviso o bloqueo). **El texto del mensaje no
se guarda.**

La tarjeta «Codex Desktop» del Resumen muestra los últimos 30 días: mensajes
por tier, avisos, bloqueos, cuántos usaron el modelo recomendado y si el hook
está instalado. Se recarga al volver a la ventana de RouDex y cada 30 s.

## Si venías de `v0.5.0`

- **Hook administrado.** `v0.5.0` podía instalar
  `%ProgramData%\OpenAI\Codex\requirements.toml`. Si solo contiene el bloque
  `# RouDex managed desktop routing hook`, renómbralo desde PowerShell como
  administrador; si contiene otras políticas, quita solo ese bloque:

  ```powershell
  Rename-Item "$env:ProgramData\OpenAI\Codex\requirements.toml" "requirements.toml.roudex-v0.5-old"
  ```

- **Copia privada antigua.** Lo que `v0.5.0` escribió en `%LOCALAPPDATA%\RouDex`
  desde Codex quedó en
  `%LOCALAPPDATA%\Packages\OpenAI.Codex_*\LocalCache\Local\RouDex`. El hook
  actual no la usa.

## Desinstalar

```powershell
& "$env:USERPROFILE\.roudex\bin\router-cli.exe" hook-uninstall
```

Quita solo las entradas de RouDex de `hooks.json`; el resto se conserva. La
copia de `router-cli` y los datos de `%USERPROFILE%\.roudex` quedan para que
puedas revisarlos o borrarlos.
