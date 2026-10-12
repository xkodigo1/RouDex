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

```powershell
.\scripts\install-hook.ps1 -Release
```

El script compila una copia estable del ejecutable en el almacenamiento local
del usuario y registra un handler `UserPromptSubmit` en la configuración de
Codex Desktop. El hook debe revisarse y confiarse desde la pantalla de hooks de
Codex Desktop antes de que el producto lo ejecute.

El adaptador devuelve `decision: block` si no puede registrar o enrutar un
mensaje. El worker interno se marca mediante una variable de proceso local para
evitar que el propio router se invoque recursivamente.

## Consultar estado

```powershell
& "$env:LOCALAPPDATA\RouDex\bin\router-cli.exe" status
```

Las ejecuciones y eventos se guardan en SQLite dentro del almacenamiento local
del usuario. El repositorio no contiene esa base de datos ni credenciales.
