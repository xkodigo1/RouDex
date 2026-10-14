# Roadmap

RouDex enruta cada mensaje al modelo adecuado para ahorrar cuota sin perder
calidad. Primero es una herramienta personal en Windows; después, un proyecto
open source multiplataforma.

## Decisiones de producto

- **Router híbrido.** Reglas rápidas resuelven lo obvio. Un modelo económico
  clasifica los mensajes dudosos; el modelo y el esfuerzo son configurables.
- **Tiers, no modelos fijos.** `economico`, `estandar` y `fuerte` se definen en
  configuración y se validan contra `model/list` del runtime.
- **Regla de riesgo fija.** Lo que coincide con reglas de alto riesgo
  (producción, migraciones, credenciales, dinero, borrados, más las reglas del
  proyecto en `.roudex/rules.toml`) siempre va al tier `fuerte`.
- **Escalado manual.** Un botón re-ejecuta con el tier `fuerte`; no hay
  reintentos automáticos.
- **Permisos.** Escritura limitada a la carpeta del proyecto y aprobación de
  comandos y cambios desde la interfaz.
- **Runtimes.** Interfaz común; Codex primero, Claude después.

## Fases

| Versión | Fase | Resultado |
| --- | --- | --- |
| `v0.6.0` | Chat con router | Selector de proyecto, chat con streaming, decisión del router visible y editable antes de enviar, aprobaciones, historial de hilos, uso por modelo y estado de la ventana de límites. |
| `v0.7.0` | Integración con Codex Desktop | Hook que solo añade contexto (tier recomendado y riesgo). El bloqueo de tareas de alto riesgo es opcional; nunca se bloquea todo. |
| `v0.8.0` | Runtime Claude | Segundo adaptador sobre la misma interfaz. |
| `v0.9.0` | Orquestador SDD | Specs y grafos de tareas ejecutados con el mismo motor de routing. |
| `v1.0.0` | Open source | Instaladores multiplataforma, documentación y métrica de ahorro. |

Cada fase termina con un tag y una release. El trabajo avanza en ramas cortas
con PR hacia `main`.
