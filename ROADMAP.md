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
- **Runtime.** Solo Codex hasta `v1.0.0`. Otros runtimes quedan fuera de
  alcance hasta después de la versión estable.
- **Idioma.** Documentación pública en inglés; la interfaz, en español e
  inglés.

## Fases

| Versión | Fase | Resultado |
| --- | --- | --- |
| `v0.6.0` | Chat con router | Selector de proyecto, chat con streaming, decisión del router visible y editable antes de enviar, aprobaciones, historial de hilos, uso por modelo y estado de la ventana de límites. |
| `v0.7.0` | Integración con Codex Desktop | Hook que solo añade contexto (tier recomendado y riesgo). El bloqueo de tareas de alto riesgo es opcional; nunca se bloquea todo. |
| `v0.8.0` | Orquestador SDD | Specs y grafos de tareas ejecutados desde la app con los mismos tiers y reglas de riesgo que el chat: cargar `tasks.json`, aprobar, despachar, seguir la ejecución y reintentar. |
| `v0.9.0` | Beta | Windows, macOS y Linux; instaladores con `router-cli` incluido; métrica de ahorro estimado (incluido Codex Desktop, opcional); interfaz en español e inglés. |
| `v1.0.0` | Open source | Licencia y guías de contribución, documentación pública en inglés, auditoría de datos y dependencias, y release estable con instaladores. |

Cada fase termina con un tag y una release; antes del tag, la versión de
`Cargo.toml` (`[workspace.package]`) y de `ui/package.json` sube a la de la
fase. El trabajo avanza en ramas cortas con PR hacia `main`, y cada PR se
fusiona con el CI en verde.
