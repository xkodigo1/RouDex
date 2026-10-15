# Tareas · 005

`tasks.json` es el contrato; este archivo lo resume. Las tareas declaran
`tier` (como en la spec 004) y se implementan fuera del coordinador (decisión
D8). Ninguna empieza hasta que el usuario apruebe la spec.

```text
T001 datos ─▶ T002 Codex ─▶ T003 idioma ─▶ T004 coste ─▶ T005 Codex Desktop
   │                           │               │               │
   │                           ├───────────────┴───────────────┴─▶ T006 tarjeta
   └───────────────────────────┴─▶ T007 instaladores ─▶ T008 workflow
T003 + T006 + T007 ─▶ T009 traducir pantallas
todas ─▶ T010 cierre
```

Las dependencias evitan que dos tareas toquen a la vez los mismos archivos:
`tiers.rs`, `persistence.rs`, `Summary.jsx` y los diccionarios.

## R005-T001 · Datos de RouDex en `~/.roudex`

Un solo directorio en los tres sistemas (`$ROUDEX_HOME` o `~/.roudex`). En
Windows se copia una vez desde `%LOCALAPPDATA%\RouDex` sin tocar el original
ni pisar datos existentes. Las pruebas pasan a aislar con `ROUDEX_HOME`.
Escenarios A01–A03.

## R005-T002 · Encontrar Codex en macOS y Linux

`ROUDEX_CODEX_BIN`, `codex_path`, `PATH` y las rutas conocidas de cada sistema,
incluida la app de Codex en macOS. Si no aparece, el error dice dónde se buscó.
Escenarios A04 y A05.

## R005-T003 · Idioma, base de traducciones y hook en el idioma elegido

`language` en `router.toml` (se edita solo esa línea), `t()` con diccionarios y
`Intl`, selector en la app y primer arranque con el idioma del sistema. El
aviso del hook sigue el idioma (inglés si no hay valor). Escenarios A14–A16.

## R005-T004 · Coste por turno y precios del usuario

`[prices.<modelo>]` en `router.toml`, coste de cada turno, base al precio del
tier `fuerte`, tokens de las tareas y del clasificador, y resumen de 30 días
por un comando Tauri. Sin precios por defecto. Escenarios A09–A11.

## R005-T005 · Uso de Codex Desktop desde `~/.codex/sessions` (opcional)

Con `include_codex_desktop`, se leen solo las líneas `turn_context` y
`token_usage_record` de los últimos 30 días, con un índice incremental y sin
contar los hilos de RouDex. Si está apagado, no se abre ningún archivo.
Escenarios A12 y A13.

## R005-T006 · Tarjeta de ahorro en Resumen

Coste estimado, coste en `fuerte`, ahorro y coste del router, por origen, con
la nota de que es una estimación. También el número de turnos sin precio, el
interruptor de Codex Desktop y, si no hay precios, el fragmento para
añadirlos. Comprobada en el fixture en los dos idiomas. Escenarios A09, A10
y A12.

## R005-T007 · Instaladores con `router-cli` y botón «Instalar hook»

- `@tauri-apps/cli` y la configuración de empaquetado: NSIS por usuario,
  `.dmg` universal con firma ad hoc, `.deb` y `.AppImage`.
- `router-cli` va dentro como binario externo.
- El botón instala el hook desde la copia incluida.
- Prueba del instalador en Windows.

Escenarios A06 y A07.

## R005-T008 · Workflow de release con los instaladores

- `release.yml` se lanza al publicar una release y a mano.
- Compila en Windows, macOS y la Ubuntu LTS más antigua disponible.
- Sube los instaladores con `gh release upload`.
- Arranca la app de Linux bajo `xvfb`.

Escenario A08.

## R005-T009 · Todas las pantallas en español e inglés

Resumen, Hilos, Proyectos, Tareas, la transcripción y las aprobaciones pasan a
los diccionarios, incluidas las tablas de traducción de mensajes del backend.
Una comprobación falla si queda texto sin traducir. Escenario A14.

## R005-T010 · Documentación, prueba de la beta y release `v0.9.0`

Instalación en los tres sistemas, precios y ahorro, Codex Desktop e idioma.
Prueba de punta a punta en Windows con el instalador del workflow y Codex
real, evidencias de CI de macOS y Linux, versión `0.9.0` y notas de la
release. Escenarios A17 y A18.
