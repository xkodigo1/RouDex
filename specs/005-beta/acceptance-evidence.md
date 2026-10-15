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
| A04 encontrar Codex | `codex::tests::windows_uses_the_path_then_the_newest_desktop_copy`, `macos_finds_the_codex_app_in_every_known_layout`, `linux_looks_in_the_usual_user_and_system_folders`, `the_override_and_codex_path_come_first` | OK |
| A05 Codex ausente | `codex::tests::a_missing_codex_lists_every_place_tried`, `task_cli::a_missing_codex_says_where_it_looked_and_how_to_set_codex_path` | OK |
| A06 instalar el hook desde la app | | Pendiente (R005-T007) |
| A07 sin binario incluido | | Pendiente (R005-T007) |
| A08 workflow de release | | Pendiente (R005-T008) |
| A09 ahorro con precios | | Pendiente (R005-T004, R005-T006) |
| A10 sin precios | | Pendiente (R005-T004, R005-T006) |
| A11 coste del clasificador | | Pendiente (R005-T004) |
| A12 Codex Desktop activado | | Pendiente (R005-T005, R005-T006) |
| A13 Codex Desktop apagado | | Pendiente (R005-T005) |
| A14 cambiar de idioma | Fixture (marco, navegación y avisos); pantallas en R005-T009 | En curso |
| A15 idioma del sistema | `tiers::tests::a_setting_changes_one_line_and_keeps_the_rest`, `a_setting_in_an_empty_or_missing_file_is_all_the_file_holds`; fixture | OK |
| A16 hook en el idioma elegido | `desktop_hook::tests::the_hook_writes_in_the_chosen_language_and_in_english_when_unset`, `hook_cli::the_hook_follows_the_language_in_router_toml` | OK |
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

## A04–A05 · Detalle

- La búsqueda recibe la raíz del sistema y la carpeta del usuario como datos,
  así que las pruebas montan en una carpeta temporal la disposición de cada
  sistema y corren en cualquiera.
- **macOS:** se prueban las tres disposiciones conocidas de `Codex.app`, con
  la más nueva ganando cuando conviven, la app en `~/Applications` y
  Homebrew antes que `~/.local/bin`.
- **Prioridad:** `ROUDEX_CODEX_BIN` gana a `codex_path`, y `codex_path` a
  la búsqueda. Un `codex_path` que no apunta a un archivo da un error que lo
  nombra.
- **Sin Codex:** con `PATH`, `HOME` y `LOCALAPPDATA` vacíos,
  `router-cli models` falla con «Codex was not found», la lista de lugares,
  `codex_path` y la ruta de `router.toml`. No se lanza ningún proceso.

## A14–A16 · Detalle (R005-T003)

- **`router.toml`:**
  - Acepta archivos parciales; las claves que faltan toman su valor por
    defecto.
  - Fijar `language` cambia o añade solo esa línea y conserva comentarios y
    tablas.
  - Una sección nueva va al final, y un resultado inválido no se escribe.
- **Hook:**
  - Sin `language` y con `en`, el aviso dice «this chat uses …» y pide
    «Start your reply with this line».
  - Con `es`, dice «este chat usa …» y «Empieza tu respuesta con esta
    línea».
  - El bloqueo y el contexto sin regla siguen el mismo idioma.
- **Fixture, 2026-10-14:**
  - Con el sistema en `es-419`, la app arranca en español y guarda
    `language = "es"`.
  - EN traduce al momento la navegación («summary, threads, projects,
    tasks»), los tiers activos y la barra de límites («plus plan · 5 h window
    at 37%»), y guarda `en`; ES lo devuelve.
  - `systemLanguage()` da `en` para en-US, fr-FR y pt-BR, y `es` para
    es-ES y es-419.
