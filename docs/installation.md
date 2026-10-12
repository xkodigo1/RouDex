# Instalación local

RouDex funciona de forma local y no requiere cuentas, tokens ni servicios
remotos.

## Dependencias

- Rust estable con el toolchain MSVC en Windows.
- Node.js y `pnpm` para la interfaz.
- Codex Desktop con soporte para `UserPromptSubmit` hooks.

## Preparar el workspace

```powershell
pnpm --dir ui install
cargo test --workspace
pnpm --dir ui build
```

## Activar el adaptador de mensajes

Desde la raíz del repositorio:

Para cubrir todos los chats locales de Codex Desktop, instala el hook como
política administrada del equipo:

```powershell
.\scripts\install-hook.ps1 -Release -Managed
```

El script compila una copia estable del ejecutable en el almacenamiento local
del usuario y registra un handler `UserPromptSubmit` en la política local
administrada de Codex Desktop (`%ProgramData%\OpenAI\Codex\requirements.toml`).
El handler usa un entrypoint local de PowerShell para entregar de forma estable
el JSON del hook al ejecutable del router en Windows.
Los hooks administrados se consideran confiables por política y no requieren
una aprobación manual en la interfaz. El instalador retira solamente el hook
de usuario de RouDex para evitar que el mismo mensaje se procese dos veces.

La instalación administrada requiere permisos para escribir en `%ProgramData%`.
Si ya existe una política administrada que no fue creada por RouDex, el
instalador se detiene sin sobrescribirla; debe integrarse el bloque generado
manualmente con el administrador de la máquina.

Para volver al estado sin la política administrada de RouDex:

```powershell
& "$env:LOCALAPPDATA\RouDex\bin\router-cli.exe" uninstall-managed-hook
```

La modalidad anterior, limitada a la configuración de usuario, sigue
disponible con `-Release` sin `-Managed`, pero depende de la revisión de
confianza de hooks y no es la modalidad global recomendada.

El adaptador devuelve `decision: block` si no puede registrar o enrutar un
mensaje. El hook de Desktop usa el modo de registro para que el turno original
se ejecute una sola vez; la respuesta de clasificación llega al mismo turno
mediante `additionalContext`. Las ejecuciones internas opcionales se marcan
mediante una variable de proceso local para evitar recursión.

## Consultar estado

```powershell
& "$env:LOCALAPPDATA\RouDex\bin\router-cli.exe" status
```

Las ejecuciones y eventos se guardan en SQLite dentro del almacenamiento local
del usuario. El repositorio no contiene esa base de datos ni credenciales.
