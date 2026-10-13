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

El adaptador crea una task local y devuelve `continue: false` y
`decision: block`, también cuando el registro tiene éxito. El motivo incluye
`taskId` y `specId`: el turno original se detiene y el hook nunca inicia un
worker. La ejecución posterior pasa exclusivamente por el coordinador.
`--record-only` sigue aceptándose para hooks antiguos, con este mismo contrato.
Tras actualizar el binario, vuelve a ejecutar el instalador para actualizar
el wrapper de PowerShell de los hooks instalados.

La identidad `(session_id, turn_id)` evita duplicados, incluso entre procesos
simultáneos. Repetir exactamente el JSON devuelve la misma task y su estado
actual. Reutilizar esa identidad con otro contenido bloquea el mensaje.
Un nuevo turno crea otra task; para actualizar una task pendiente, el JSON debe
incluir su `task_id`. La actualización exige el mismo directorio de trabajo,
conserva perfil, scope, sandbox y dependencias, y revoca la aprobación anterior.
Una task activa o con un run previo requiere recuperación explícita o una nueva
task. Los marcadores internos del coordinador evitan recursión.

Las tasks nuevas son borradores bloqueados, con aprobación manual y sandbox de
solo lectura. El texto del mensaje y `permission_mode` no conceden permisos.
Antes de ejecutar, prepara un manifest JSON con el `taskId` y `specId`
devueltos, perfil/modelo/esfuerzo, scope, cwd, sandbox, acceptance y verify
revisados. Cárgalo y apruébalo desde una terminal local:

```powershell
cargo run -p router-cli -- spec-load <manifest.json>
cargo run -p router-cli -- task-approve <taskId> approved
cargo run -p router-cli -- task-run <taskId>
```

También puedes despachar el grafo con `task-dispatch <specId>`. Aprobar un
borrador sin scope, acceptance o verify no inicia el runtime. Un ejecutable
ausente, fallo de proceso, respuesta inválida o error de SQLite produce una
decisión de bloqueo; no habilita la ejecución en el chat original.

## Consultar estado

```powershell
& "$env:LOCALAPPDATA\RouDex\bin\router-cli.exe" status
```

Las ejecuciones y eventos se guardan en SQLite dentro del almacenamiento local
del usuario. El repositorio no contiene esa base de datos ni credenciales.
