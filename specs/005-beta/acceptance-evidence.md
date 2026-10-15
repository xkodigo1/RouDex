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
| A06 instalar el hook desde la app | Instalador NSIS real en Windows, app aislada y pulsada por DevTools (abajo); `hook_cli::install_*` | OK |
| A07 sin binario incluido | Fixture sin `?bundled=1`; `desktop_hook_install` sin binario devuelve el error con el comando | OK |
| A08 workflow de release | | Pendiente (R005-T008) |
| A09 ahorro con precios | `savings::tests::*`, `tiers::tests::prices_are_read_by_model_and_must_not_be_negative`; tarjeta en el fixture | OK |
| A10 sin precios | `savings::tests::without_prices_no_money_is_estimated_but_tokens_by_tier_are_kept`; tarjeta en el fixture | OK |
| A11 coste del clasificador | `router_classifier::app_server_classifier_runs_an_ephemeral_read_only_turn`, `savings::tests::the_classifier_counts_against_the_savings`, `the_store_adds_chat_turns_task_runs_and_the_classifier` | OK |
| A12 Codex Desktop activado | `codex_sessions::tests::desktop_turns_count_once_without_keeping_any_text`, `lines_are_told_apart_by_their_start_only`; sesiones reales (conteos); tarjeta en el fixture | OK |
| A13 Codex Desktop apagado | `codex_sessions::tests::the_summary_reads_sessions_only_when_asked` | OK |
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

## A09–A11 · Detalle (R005-T004)

- **Fórmula:**
  - 1 M de entrada con 400 k en caché y 100 k de salida, a 2 / 0,5 / 10 USD
    por millón, cuestan 0,6 × 2 + 0,4 × 0,5 + 0,1 × 10 = 2,4 USD.
  - La entrada en caché nunca se cobra dos veces ni da negativa.
- **Base:**
  - Cada turno se compara con los mismos tokens al precio del modelo de
    `fuerte`.
  - Un turno de chat en `gpt-6-luna` (0,7) y uno de tareas en
    `gpt-6.1-sol` (3,0) dan un coste de 3,7, una base de 6,0 y un ahorro
    de 2,3.
- **Clasificador:**
  - El turno de clasificación del servidor falso informa 1230 tokens.
  - La decisión los lleva en `router_usage`, y el ahorro los resta.
- **Base local:**
  - Se cuentan los turnos de chat de los últimos 30 días con tokens; uno de
    hace 40 días y uno sin tokens quedan fuera.
  - Las ejecuciones de tareas cuentan con la última notificación de tokens
    de su evidencia.
  - Se suman los registros de `router_usage`.
- **Sin precios:**
  - Coste, base y ahorro valen 0.
  - Se cuentan los turnos sin precio y se listan los modelos sin precio; los
    tokens por tier siguen disponibles.

## A12–A13 · Detalle (R005-T005)

- **Qué se lee:**
  - Solo se analizan las líneas `turn_context` y `token_usage_record`; el
    tipo se lee del inicio de la línea, antes del `payload`.
  - Un mensaje cuyo texto contiene `"type":"token_usage_record"` sigue
    siendo `response_item` y no se analiza.
- **Privacidad:** la prueba mete un texto secreto y un `cwd` en las sesiones
  y comprueba que nada de eso llega a la base.
- **Conteo:**
  - Los hilos de RouDex (chats y ejecuciones de tareas) no se cuentan dos
    veces.
  - Las peticiones de un mismo turno se suman.
- **Índice incremental:** la segunda pasada no relee archivos sin cambios; un
  archivo modificado reemplaza sus filas sin duplicarlas.
- **Opción apagada:** el resumen no abre ningún archivo de sesión y no aparece
  Codex Desktop. Encendida, aparece como origen propio.
- **Sesiones reales del usuario, 2026-10-15:**
  - 112 archivos de los últimos 30 días leídos en 1,3 s; la segunda pasada
    tarda 12 ms y no relee ninguno.
  - 399 turnos de 5 modelos.
  - Se imprimieron solo conteos, con una prueba temporal que se retiró antes
    del commit.

## A09, A10 y A12 · Tarjeta de ahorro (R005-T006)

Fixture `?view=resumen`, 2026-10-15:

- **Con precios, en español:**
  - Coste estimado 3,15 US$, todo en fuerte 8,32 US$ (`gpt-6.1-sol`), coste
    del router 0,0042 US$ (18 clasificaciones) y ahorro 5,17 US$, un 62 %
    menos que en fuerte.
  - Hay una tabla por origen (chat y tareas) y la nota de estimación.
- **«incluir Codex Desktop»:**
  - Escribe la opción, añade la fila Codex Desktop (382/399 turnos) y avisa
    de «17 turnos sin precio no cuentan (gpt-6-astra)».
  - Los totales pasan a 24,55 / 67,07 / 42,52 US$.
- **En inglés:** los mismos importes como $24.55 / $67.07 / $42.52, con
  «Estimated savings», «all on fuerte» y «router cost».
- **Sin precios (`?savings=none`):** «Sin precios no se estima dinero», los
  tokens por tier y el fragmento con `[prices."gpt-6.1-sol"]` y
  `[prices."gpt-6-luna"]` listo para pegar.
- **Diseño:** a 1280×900 la tarjeta sigue el estilo de Resumen.

## A06–A07 · Instalador de Windows (R005-T007)

2026-10-15, Windows 11:

1. **Build:** `node scripts/bundle.mjs --bundles nsis` compila `router-cli`
   y la app en release, descarga NSIS 3.11 y produce
   `RouDex_0.8.0_x64-setup.exe` (3,43 MiB). La versión sube a 0.9.0 en
   R005-T010.
2. **Instalación silenciosa** en una carpeta temporal
   (`/S /D=…\install`): la carpeta queda con `roudex-desktop.exe`,
   `router-cli.exe` y `uninstall.exe`.
3. **App instalada** con `ROUDEX_HOME`, `CODEX_HOME` y `LOCALAPPDATA`
   temporales, manejada por el puerto de DevTools de WebView2:
   - Sirve la interfaz desde `tauri.localhost` y guarda
     `language = "es"` (idioma del sistema).
   - `desktop_hook_summary` informa `bundled: true`, y la tarjeta muestra
     «hook no instalado · Instalar hook».
   - Al pulsar el botón:
     - `router-cli.exe` (3 721 216 bytes, igual que el incluido) se copia
       a `home\bin`;
     - `hooks.json` queda con una sola entrada `UserPromptSubmit`;
     - la tarjeta pasa a «hook instalado · Reinstalar hook» con un aviso.
   - Una segunda instalación reemplaza esa entrada (`replaced: 1`).
4. **Desinstalación silenciosa:**
   - Borra la carpeta de instalación y conserva los datos y la copia del
     hook. Desinstalar la app no quita el hook; para eso está
     `router-cli hook-uninstall`.
   - Deja `HKCU\Software\roudex\RouDex` con la ruta de instalación, como
     hace Tauri salvo que se marque «borrar datos de la app». La clave de la
     prueba se borró después.
   - En una instalación real esa clave es la carpeta por defecto:
     `%LOCALAPPDATA%\RouDex`. El desinstalador solo borra sus propios
     archivos, con `RMDir` no recursivo, así que no toca la copia de
     respaldo de los datos de antes de 0.9.0.
5. **Sin binario incluido (fixture):** la tarjeta no ofrece el botón y
   muestra «Sin instalador, instálalo desde una terminal con
   `router-cli hook-install`».
