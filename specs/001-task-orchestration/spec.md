# Especificación 001 — Orquestación local guiada por tareas

Estado: propuesta activa  
Última actualización: 2026-10-12

## Problema

El adaptador de escritorio ya recibe mensajes, clasifica una ruta y persiste
un recibo. Sin embargo, el modo administrado actual usa `record-only`: el turno
original de Codex continúa y RouDex todavía no es el coordinador que ejecuta
una tarea con el modelo asignado.

## Resultado deseado

RouDex será un orquestador local de tareas definido por especificaciones. Una
especificación produce un grafo de tasks ejecutables; cada task lista sus
dependencias, aprobación, perfil, modelo, esfuerzo y criterios de aceptación.
RouDex ejecuta las tasks listas mediante el App Server local, persiste su
estado y eventos, y permite continuar una ejecución mediante `runId`.

El hook global de Codex Desktop seguirá siendo la puerta de entrada. No se
considerará que una task fue ejecutada por RouDex hasta que exista evidencia de
que el coordinador inició el hilo y el turno del App Server con el modelo y
esfuerzo asignados.

## Alcance

- Especificaciones versionadas y un manifest JSON ejecutable.
- Tasks con ids estables, dependencias acíclicas y estados explícitos.
- Selección de perfil, modelo y esfuerzo antes de ejecutar.
- Persistencia SQLite de tasks, ejecuciones, aprobaciones y eventos.
- Coordinación local mediante App Server por `stdio`.
- Fallo cerrado cuando falta el router, una aprobación o el runtime.
- Continuación e idempotencia por `runId`.
- Evidencia de verificación por task y commits independientes por unidad.

## Fuera de alcance

- Cuentas, sincronización remota, servicios cloud o credenciales.
- Ejecución en otras máquinas o en ChatGPT Web.
- Cambiar silenciosamente el modelo seleccionado por la interfaz original.
- Un parser Markdown complejo antes de probar el contrato JSON ejecutable.
- Aprobar automáticamente acciones sensibles o irreversibles.

## Requisitos funcionales

- **R1 — Especificación:** cada milestone debe tener problema, resultado,
  alcance, no-alcance, escenarios y criterios medibles.
- **R2 — Grafo:** cada task debe tener un id estable, dependencias, prompt de
  ejecución, scope, cwd, sandbox, política de aprobación, acceptance y
  verificación.
- **R3 — Validación:** RouDex debe rechazar ids duplicados, dependencias
  inexistentes, ciclos y tasks con campos obligatorios ausentes.
- **R4 — Enrutamiento:** una task lista debe recibir un perfil, modelo y
  esfuerzo antes de crear el turno del App Server.
- **R5 — Dependencias:** una task bloqueada por una dependencia no se ejecuta;
  al completarse la dependencia puede pasar a `ready`.
- **R6 — Aprobaciones:** una task que lo requiera pasa por
  `awaiting_approval` y no inicia el runtime hasta contar con aprobación.
- **R7 — Ejecución:** el coordinador debe iniciar o reanudar el hilo local,
  enviar el turno con el modelo asignado y guardar deltas y eventos terminales.
- **R8 — Recuperación:** una ejecución interrumpida puede continuar por
  `runId` sin duplicar eventos terminales ya persistidos.
- **R9 — Aislamiento:** los workers internos no vuelven a entrar por el hook;
  una falla del coordinador bloquea la task y no habilita un bypass.
- **R10 — Auditoría:** una task solo es `completed` con evidencia de su
  verificación; el texto del modelo por sí solo no es evidencia suficiente.

## Contrato ejecutable de una task

El manifest es la fuente de intención de la task y debe resolver estos campos
antes del dispatch:

- `scope`: archivos, módulos o boundary que puede afectar la task.
- `cwd`: directorio de trabajo; no se infiere desde una ruta personal.
- `sandbox`: política y raíces escribibles explícitas.
- `acceptance`: resultados observables que deben comprobarse.
- `verify`: comando o artefacto que produce la evidencia.

El `profile` del manifest es la autoridad de enrutamiento. RouDex resuelve
desde el registro de perfiles el `model` y `effort`; si el manifest incluye
valores explícitos que contradicen el perfil, la task se rechaza en vez de
seleccionar silenciosamente otro modelo. Un modelo no disponible también es un
rechazo accionable.

## Transiciones y recuperación

Las transiciones permitidas son:

```text
planned -> ready -> awaiting_approval -> running -> completed
   |         |             |              |
   |         |             v              +-> failed / interrupted
   |         +-----------> blocked
   +-------------------------------------> blocked
```

Una task sin dependencias pendientes pasa de `planned` a `ready`. Una task con
aprobación requerida pasa de `ready` a `awaiting_approval`; una aprobación la
lleva a `running` y una denegación a `blocked` con motivo. Una dependencia
fallida o inválida bloquea a sus descendientes. `failed` significa que el
runtime comenzó y devolvió error; `blocked` significa que el runtime no debe
comenzar; `interrupted` significa que el runtime comenzó pero no produjo un
terminal válido.

Para reanudar, RouDex usa la identidad `(task_id, run_id, thread_id)` y un
identificador de evento idempotente. Antes de crear un nuevo turno debe revisar
si ya existe un evento terminal para el mismo turno. Si el hilo persistido no
está disponible, la task queda `blocked` y requiere una decisión explícita de
recuperación; no se crea un hilo sustituto sin evidencia de que repetir el
trabajo sea seguro.

## Escenarios de aceptación

### A1 — Ejecutar una task lista

**Dado** un manifest válido con una task sin dependencias pendientes, **cuando**
RouDex la despacha, **entonces** crea un `runId`, selecciona el modelo y
esfuerzo declarados por el perfil, inicia un hilo local y persiste el evento de
inicio y el evento terminal.

Verificación: prueba de protocolo con App Server falso y una prueba local con
App Server por `stdio`.

### A2 — Bloquear una dependencia

**Dado** un grafo donde T2 depende de T1, **cuando** T1 no está completada,
**entonces** T2 permanece bloqueada y no se crea un turno para T2.

Verificación: prueba de validación y contador de llamadas del runtime falso.

### A3 — Requerir aprobación

**Dado** una task marcada como sensible, **cuando** aún no existe aprobación,
**entonces** su estado es `awaiting_approval` y no se inicia el App Server.

Verificación: prueba de transición y evento de aprobación.

### A4 — Continuar por runId

**Dado** un run interrumpido con hilo persistido, **cuando** se solicita
continuación, **entonces** RouDex reanuda el hilo, registra el nuevo turno y no
duplica la finalización anterior.

Verificación: prueba de resume con App Server falso.

### A5 — Fallo cerrado

**Dado** un runtime no disponible o un manifest inválido, **cuando** se intenta
despachar una task, **entonces** la task queda `blocked` o `failed` con una
razón accionable y no se ejecuta directamente en el chat.

Verificación: prueba de error y revisión del evento persistido.

### A6 — Recursión controlada

**Dado** un worker iniciado por RouDex, **cuando** el worker genera una acción
interna, **entonces** el guard de recursión evita que esa acción se registre como
una nueva task de usuario.

Verificación: prueba de hook interno y ausencia de una segunda ejecución.

### A7 — Rechazar un manifest inválido

**Dado** un manifest con un id duplicado, dependencia inexistente o ciclo,
**cuando** se valida, **entonces** no se crea ninguna task ejecutable y cada
error identifica el campo o relación que debe corregirse.

Verificación: tres casos de prueba independientes y ausencia de llamadas al
runtime.

### A8 — Exigir evidencia para completar

**Dado** un turno que responde correctamente pero cuya verificación no se ha
ejecutado o falla, **cuando** se actualiza el estado, **entonces** la task no
puede pasar a `completed` y conserva el motivo de verificación pendiente o
fallida.

Verificación: prueba de transición de estado y evento de auditoría.

## Criterio de finalización del milestone

El milestone termina cuando todas las tasks requeridas del manifest están
completadas, A1–A8 tienen evidencia, existe al menos una continuación por
`runId`, el repositorio conserva commits independientes firmados y el
workspace no contiene secretos ni estado local generado.
