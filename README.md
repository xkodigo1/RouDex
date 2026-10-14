# RouDex

RouDex es una aplicación local-first que enruta cada mensaje al modelo
adecuado: las tareas simples van a modelos económicos y las complejas o
arriesgadas a modelos fuertes. El objetivo es ahorrar cuota sin perder calidad.

Se ejecuta sobre `codex app-server` (stdio). Claude será el segundo runtime.

## Estado

El hito `v0.7.0` (integración con Codex Desktop) está en curso. La app de
escritorio tiene tres pantallas:

- **Resumen**: tokens del año y de hoy, calendario de actividad por día,
  tokens por modelo, turnos por tier, límites de uso, hilos recientes y los
  mensajes de Codex Desktop que vio el hook.
- **Hilos**: búsqueda y filtros, transcripción en estilo terminal con la
  decisión del router de cada turno, aprobaciones, renombrar y archivar.
- **Proyectos**: nuevo chat sobre un fondo ASCII animado; el router analiza el
  mensaje y puedes cambiar el tier antes de enviarlo.

El hook de Codex Desktop añade a cada mensaje el tier recomendado y avisa de los
mensajes de riesgo; consulta [docs/desktop-hook.md](docs/desktop-hook.md).

Consulta [ROADMAP.md](ROADMAP.md) y [docs/router.md](docs/router.md).

La interfaz usa IBM Plex Mono si está instalada; si no, Cascadia Mono o
Consolas. No descarga fuentes de internet.

## Ejecutar en desarrollo

```powershell
pnpm --dir ui dev
cargo run -p roudex-desktop
```

El primer comando sirve la interfaz en `http://localhost:5173`; el segundo
abre la ventana de escritorio. Sin Tauri, `ui/tests/chat-fixture.html`
muestra la interfaz con un backend simulado.

RouDex no requiere cuentas, sincronización remota ni credenciales dentro del
repositorio.

## Instalación

Consulta [docs/installation.md](docs/installation.md).
