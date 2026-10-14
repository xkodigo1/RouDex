# 004 · Orquestador SDD con el router

- **Versión:** 1
- **Fecha:** 2026-10-14
- **Estado:** en ejecución
- **Release:** `v0.8.0`
- Contrato ejecutable: [tasks.json](tasks.json) · Tareas: [tasks.md](tasks.md) ·
  Decisiones: [decisions.md](decisions.md)

## Problema

RouDex ya tiene un coordinador de tareas (importar un `tasks.json`, aprobar,
despachar, verificar, reanudar y reintentar), pero vive al margen del resto:

- enruta con una tabla fija de cinco perfiles y no usa `router.toml` ni las
  reglas de riesgo que usa el chat;
- ejecuta los turnos con `approvalPolicy: "never"`, sin streaming hacia la
  interfaz y sin forma de detenerlos;
- solo se maneja desde la CLI; la pestaña Tareas es de solo lectura;
- los manifiestos que escribimos hoy (por ejemplo, el de la spec 003) no se
  pueden cargar porque declaran un modelo que no coincide con el perfil.

## Resultado

Desde la app, sobre un proyecto:

1. **Cargar** un `tasks.json` (los de `specs/*/` del proyecto se listan solos)
   y ver sus hitos, tareas, dependencias, criterios de aceptación y comando de
   verificación.
2. **Enrutar por tier.** Cada tarea declara un tier (`economico`, `estandar`,
   `fuerte`); el modelo y el esfuerzo salen de `router.toml`. Las reglas de
   riesgo del proyecto fuerzan `fuerte`. El tier se puede cambiar a mano antes
   de ejecutar, y el cambio queda registrado.
3. **Aprobar o denegar** las tareas que lo requieren.
4. **Ejecutar** una tarea o todas las listas, en orden de dependencias, con la
   misma sesión que el chat: streaming de la respuesta, aprobaciones de
   comandos y cambios en la interfaz, y botón de detener.
5. **Verificar** con el comando de la tarea y ver su salida y el resultado.
6. **Reintentar, reanudar o recuperar** una tarea interrumpida o fallida.

La CLI sigue ofreciendo el mismo ciclo, con las mismas reglas.

## Invariantes

- Un turno de tarea nunca se ejecuta sin que sus dependencias estén
  completadas y, si la tarea lo exige, aprobada.
- El modelo y el esfuerzo de un turno salen siempre del tier resuelto; una
  tarea con riesgo nunca corre por debajo de `fuerte` salvo que el usuario lo
  cambie a mano, y ese cambio queda en la evidencia.
- La escritura se limita a la carpeta del proyecto (y a las raíces declaradas
  dentro de ella), sin red. `danger-full-access` no se acepta.
- Completar una tarea exige que su comando de verificación salga con 0.
- Reanudar no repite trabajo ya completado ni duplica eventos terminales.
- Los turnos que lanza el orquestador están marcados como internos: no pueden
  invocar el ciclo del coordinador ni disparan el hook de Codex Desktop.
- Local-first: sin red ni servicios remotos.

## Fuera de alcance

- Ejecutar tareas en paralelo (el orden es secuencial).
- Reintentos automáticos o escalado automático de tier.
- Operaciones de Git o PRs hechas por el orquestador.
- Editar el manifiesto desde la interfaz (se edita como archivo).
- Otros runtimes distintos de Codex.

## Hitos

| Hito | Release | Tareas |
| --- | --- | --- |
| H1 · Orquestador en la app | `v0.8.0` | R004-T001 … R004-T007 |
