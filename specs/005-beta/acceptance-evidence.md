# Evidencias de aceptación · 005

Tareas ejecutadas fuera del coordinador (decisión D8). Entorno: Windows 11 con
Rust estable MSVC y la app de Codex Desktop. CI en Windows y Linux; macOS en
`main` y en tags. Los escenarios pendientes se cierran en las tareas que los
cubren.

| Escenario | Verificación | Resultado |
| --- | --- | --- |
| A01 datos en `~/.roudex` | `paths::tests::*`, `task_cli::state_lives_in_dot_roudex_in_the_user_folder` (CI en los tres sistemas) | OK |
| A02 migración en Windows | `paths::tests::the_first_start_copies_the_store_and_the_config_once`, `existing_data_in_home_is_never_overwritten`, `task_cli::data_from_before_0_9_is_copied_once_and_left_in_place`; migración real en R005-T010 | OK (pruebas) |
| A03 `ROUDEX_HOME` | Todas las pruebas de `task_cli` y `hook_cli` aíslan con `ROUDEX_HOME`; el hook lee `router.toml` de ahí (`hook_cli::the_blocking_option_blocks_risky_prompts`) | OK |
| A04 encontrar Codex | | Pendiente (R005-T002) |
| A05 Codex ausente | | Pendiente (R005-T002) |
| A06 instalar el hook desde la app | | Pendiente (R005-T007) |
| A07 sin binario incluido | | Pendiente (R005-T007) |
| A08 workflow de release | | Pendiente (R005-T008) |
| A09 ahorro con precios | | Pendiente (R005-T004, R005-T006) |
| A10 sin precios | | Pendiente (R005-T004, R005-T006) |
| A11 coste del clasificador | | Pendiente (R005-T004) |
| A12 Codex Desktop activado | | Pendiente (R005-T005, R005-T006) |
| A13 Codex Desktop apagado | | Pendiente (R005-T005) |
| A14 cambiar de idioma | | Pendiente (R005-T003, R005-T009) |
| A15 idioma del sistema | | Pendiente (R005-T003) |
| A16 hook en el idioma elegido | | Pendiente (R005-T003) |
| A17 instalador de Windows con Codex real | | Pendiente (R005-T010) |
| A18 macOS y Linux en CI | | Pendiente (R005-T008, R005-T010) |

## A01–A03 · Detalle

- **Migración:**
  - La base se copia con `VACUUM INTO`, que incluye las filas que siguen en
    el `-wal`. La prueba deja una fila sin volcar y comprueba que llega a la
    copia.
  - La copia se escribe con nombre temporal y luego se renombra.
  - El original no cambia de fecha ni de contenido, y una segunda pasada no
    copia nada.
  - Un `router.sqlite3` existente en el destino bloquea la copia. Un
    `router.toml` propio se conserva aunque se copie la base.
- **CLI:**
  - Todas las órdenes, salvo `hook`, migran antes de abrir la base y lo dicen
    por stderr.
  - `hook` no migra: corre dentro de Codex Desktop, donde AppData es una
    copia privada.
- **App:**
  - Migra al arrancar, antes de abrir la base. Si la copia falla, no arranca,
    para no dejar una base vacía que impida reintentar.
  - Un aviso dice de dónde a dónde se copiaron los datos. Se puede ver con
    `?migrated=1` en el fixture.
- **Pruebas:**
  - Aíslan con `ROUDEX_HOME` y un `LOCALAPPDATA` vacío, así que ninguna puede
    copiar los datos reales.
