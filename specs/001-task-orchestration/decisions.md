# Decisiones técnicas — Especificación 001

## D001 — Manifest ejecutable en JSON

La especificación humana se mantiene en Markdown y el grafo ejecutable en
`tasks.json`. RouDex ya usa `serde_json`, por lo que esta primera versión no
introduce un parser YAML/TOML ni una dependencia nueva. Se puede añadir otro
formato cuando exista una necesidad demostrada y una decisión registrada.

## D002 — El coordinador es distinto del hook

`UserPromptSubmit` es un adaptador de entrada. El coordinador crea la task,
resuelve su ruta y controla el App Server. El contexto devuelto por un hook no
se considera ejecución de la task ni puede sustituir el turno coordinado.

## D003 — Ejecución local por `stdio`

El App Server local existente se reutiliza como boundary de ejecución. RouDex
le entrega explícitamente el modelo, esfuerzo, cwd, política de aprobación y
sandbox, y persiste los eventos que permiten auditar el run.

## D004 — SQLite como fuente local de estado

Se amplía la persistencia SQLite existente para tasks, dependencias,
aprobaciones, runs y eventos. No se añade un servicio externo ni un segundo
almacén de estado para el milestone.

## D005 — Fallo cerrado y aprobación manual

Una dependencia no resuelta, aprobación ausente o runtime fallido bloquea la
task. Las acciones sensibles requieren aprobación explícita; no se habilita un
bypass para mejorar la apariencia de avance.

## D006 — Commits por task verificable

Cada unidad funcional validada se publica en un commit firmado independiente.
Los milestones se marcan con tag y release después de comprobar tests,
historial y ausencia de secretos. Los nombres de código y documentación se
mantienen neutrales.

## D007 — UI después del contrato de ejecución

La interfaz Tauri/React consumirá el estado del coordinador después de que el
contrato de tasks, persistencia y ejecución esté probado. No se añade UI para
ocultar una frontera de ejecución todavía incompleta.

## D008 — El perfil gobierna modelo y esfuerzo

El manifest declara el perfil; el registro de perfiles de RouDex resuelve el
modelo y el esfuerzo. Los campos explícitos opcionales solo sirven para
verificar la resolución y deben coincidir. Esto evita que una task cambie de
modelo por una discrepancia silenciosa entre documentación y ejecución.

## D009 — Scope y sandbox son obligatorios

Una task ejecutable debe declarar el scope lógico, cwd y sandbox antes del
dispatch. La ausencia de una raíz escribible o de una política de aprobación
es un error de validación, no una razón para ampliar permisos automáticamente.

## D010 — La evidencia es parte del estado

La respuesta del modelo, un `runId` o un evento de finalización no convierten
por sí solos una task en `completed`. El coordinador debe persistir el
resultado de `verify`; una task sin evidencia permanece pendiente, fallida o
bloqueada.

## D011 — Bootstrap explícito del almacenamiento

T-002 y T-003 son las units de bootstrap: crean el almacenamiento y la
validación que el propio coordinador necesita para poder registrar y ejecutar
tasks. Se validan con pruebas locales y quedan identificadas como `bootstrap`,
sin inventar un `run_id`. Desde T-004, las units de implementación se deben
despachar por RouDex y conservar su recibo y eventos.
