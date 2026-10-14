# Instalación local

RouDex funciona de forma local y no requiere cuentas, tokens ni servicios
remotos propios.

## Dependencias

- Rust estable con el toolchain MSVC en Windows.
- Node.js y `pnpm` para la interfaz.
- Codex CLI con `codex app-server` disponible en el `PATH`, o la variable
  `ROUDEX_CODEX_BIN` apuntando al ejecutable.

## Preparar el workspace

```powershell
pnpm --dir ui install
cargo test --workspace
pnpm --dir ui build
```

## Estado local

Las ejecuciones y eventos se guardan en SQLite dentro del almacenamiento local
del usuario (`%LOCALAPPDATA%\RouDex\router.sqlite3`). El repositorio no
contiene esa base de datos ni credenciales.

```powershell
cargo run -p router-cli -- status
```

## Router

La configuración de tiers y las reglas de riesgo están en
[docs/router.md](router.md).

## Hook de versiones anteriores

Las versiones `v0.3.x`–`v0.5.0` instalaban un hook `UserPromptSubmit` que
bloqueaba los mensajes de Codex Desktop. Ese hook ya no forma parte de RouDex.
Si lo instalaste, retíralo:

- Hook de usuario: elimina la entrada de `route-prompt` de
  `%USERPROFILE%\.codex\hooks.json`.
- Hook administrado: desde PowerShell como administrador, renombra
  `%ProgramData%\OpenAI\Codex\requirements.toml` si solo contiene el bloque
  `# RouDex managed desktop routing hook`; si contiene otras políticas, quita
  únicamente ese bloque.

Reinicia Codex Desktop después del cambio.
