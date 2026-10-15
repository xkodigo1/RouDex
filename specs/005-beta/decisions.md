# Decisiones · 005

## D1 · Todo en `~/.roudex`, en los tres sistemas

`router.toml`, `router.sqlite3`, la base del hook, la copia de `router-cli` y
el registro de fallos viven en `~/.roudex`, o en `$ROUDEX_HOME` si está
definido. Codex hace lo mismo con `~/.codex`. Decisión del usuario
(2026-10-14).

- **Migración en Windows:** al abrir la base o la configuración, si
  `~/.roudex/router.sqlite3` no existe y `%LOCALAPPDATA%\RouDex\router.sqlite3`
  sí:
  - la base se copia con la API de copia de SQLite, que es consistente
    aunque haya `-wal`;
  - también se copia `router.toml`;
  - el original queda como respaldo y la operación se registra.
  - Si `~/.roudex` ya tiene datos, no se copia nada.
- Al vivir fuera de AppData, Codex Desktop (MSIX) lee el `router.toml` real y
  no una copia privada.
- Las pruebas aíslan el estado con `ROUDEX_HOME` en lugar de `LOCALAPPDATA`.
- **Descartado: la convención de cada sistema** (`%LOCALAPPDATA%`,
  `~/Library/Application Support`, XDG). Serían tres ubicaciones que
  documentar, el hook seguiría aparte y en Windows persistiría la copia
  privada de `router.toml`.

## D2 · Encontrar Codex en cada sistema

Orden de búsqueda:

1. `ROUDEX_CODEX_BIN`, para pruebas y desarrollo.
2. `codex_path` en `router.toml`.
3. `codex` en el `PATH`.
4. Rutas conocidas de cada sistema:
   - **Windows:** la copia más reciente de Codex Desktop en
     `%LOCALAPPDATA%\OpenAI\Codex\bin\<hash>\`, como hoy.
   - **macOS:** la app de Codex en `/Applications` o `~/Applications`. El
     binario está, según la versión, en
     `Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex`,
     `Contents/Resources/codex-cli/bin/codex` o `Contents/Resources/codex`,
     y se prueba en ese orden. Después, `/opt/homebrew/bin`,
     `/usr/local/bin` y `~/.local/bin`, porque una app gráfica no hereda el
     `PATH` del terminal.
   - **Linux:** `~/.local/bin`, `/usr/local/bin` y `~/.npm-global/bin`.

Si no se encuentra, la app y la CLI dicen dónde buscaron y cómo fijar
`codex_path`, en lugar de fallar al lanzar el proceso.

- Las rutas de macOS salen de informes de terceros y cambian entre versiones
  de la app de Codex. Por eso existe `codex_path`.

## D3 · Instaladores con Tauri, sin firma

- **Herramienta:** `@tauri-apps/cli` como dependencia de desarrollo de `ui/`.
  El usuario la aprobó en el plan a `v1.0.0`.
- **Formatos (decisión del usuario):**
  - NSIS por usuario (`installMode: currentUser`).
  - `.dmg` universal (`universal-apple-darwin`), con firma ad hoc para que
    arranque en Apple Silicon.
  - `.deb` y `.AppImage`.
- **`router-cli` va dentro** como binario externo de Tauri (`externalBin`),
  compilado para cada destino; en macOS, universal con `lipo`.
- **Botón «Instalar hook»:** la app localiza `router-cli` junto a su propio
  ejecutable e instala el hook con la misma función que
  `router-cli hook-install`. No añade `tauri-plugin-shell`: no hace falta
  lanzar el binario.
- **En desarrollo,** sin binario incluido, el botón explica que se use
  `router-cli hook-install`.
- **Sin firma:** Windows muestra SmartScreen y macOS pide abrir desde
  «Configuración > Privacidad». Los pasos se documentan.
- **Descartado:** MSI y RPM, que son más formatos que probar, y firmar o
  notarizar, que requiere certificados de pago fuera de alcance.

## D4 · Release con GitHub Actions, sin acciones nuevas de terceros

- **Disparadores:** `release.yml` corre al publicar una release, y a mano
  (`workflow_dispatch`) para probar los instaladores antes del tag.
- **Compilación:** compila en `windows-latest` y `macos-latest`, y en Linux
  en la imagen de Ubuntu LTS más antigua que ofrezca GitHub. Así el
  `.AppImage` y el `.deb` enlazan con una glibc más antigua y funcionan en más
  distribuciones.
- **Publicación:** sube los instaladores a la release con `gh release upload`
  y el `GITHUB_TOKEN` (permiso `contents: write` solo en ese workflow). En
  una ejecución manual quedan como artefactos.
- **Descartado:** `tauri-apps/tauri-action`. Sería otra acción de terceros
  con permisos de escritura, y el CLI hace lo mismo.

## D5 · Ahorro estimado con precios del usuario

- **Precios (decisión del usuario):** se configuran en `router.toml` por
  modelo, en USD por millón de tokens:

  ```toml
  [prices."gpt-6.1-sol"]
  input = 0.0
  cached_input = 0.0
  output = 0.0
  ```

  RouDex no trae precios. La estimación propia de Codex
  (`account/usage/read`) devuelve vacío en una cuenta Plus, y los precios de
  la API cambian y no reflejan el consumo de un plan.
- **Coste de un turno:**
  `(entrada − en caché) × input + en caché × cached_input + salida × output`.
  - La entrada incluye la parte en caché y la salida incluye el
    razonamiento, como en los contadores de Codex. Una prueba lo fija.
- **Base:** los mismos tokens al precio del modelo que tiene hoy el tier
  `fuerte` en `router.toml`.
  - Ahorro = base − coste real − coste del clasificador.
  - Es una estimación: con otro modelo los tokens habrían sido otros. La
    tarjeta lo dice.
- **Qué entra:**
  - Turnos de chat y de tareas de los últimos 30 días.
  - El clasificador: sus tokens se registran desde esta versión.
  - Los turnos sin precio para su modelo o para el de `fuerte` se cuentan
    aparte y no suman.
- **Codex Desktop, opcional** (`[savings] include_codex_desktop`, apagado
  por defecto):
  - Se leen los `rollout-*.jsonl` de `$CODEX_HOME/sessions` de los últimos
    30 días.
  - Solo se parsean las líneas `turn_context` (turno → modelo) y
    `token_usage_record` (turno → tokens); el resto se descarta sin
    parsear.
  - Un índice en SQLite (ruta, tamaño, fecha) evita releer archivos que no
    cambiaron.
  - Se excluyen los hilos que RouDex ya conoce: todas las sesiones dicen
    `originator: "Codex Desktop"`, incluidas las de RouDex, así que se
    distinguen por id de hilo.
- **Descartado:**
  - Pesos por tier: solo darían un porcentaje sin unidades.
  - Precios de la API incluidos: habría que mantenerlos y engañan en un plan
    de suscripción.

## D6 · Español e inglés sin librería

- **Traducciones:**
  - Diccionarios en `ui/src/lib/i18n.js` y una función `t(clave, valores)`,
    sin dependencias.
  - Fechas y números con `Intl` en el idioma elegido.
- **Idioma elegido (decisión del usuario):**
  - Si `router.toml` no tiene `language`, la app usa el idioma del sistema
    (`es*` → español, si no inglés) y lo guarda.
  - El selector lo cambia en caliente.
  - La app escribe `router.toml` editando solo la línea `language` (o
    añadiéndola) y conserva el resto del archivo.
- **Mensajes del backend:** los que ve la app se traducen en la interfaz por
  su texto estable, como ya se hace con los motivos del coordinador.
- **Hook:** el aviso y el contexto que añade al turno siguen `language`; sin
  valor, en inglés.
- **Tiers:** `economico`, `estandar` y `fuerte` siguen siendo identificadores
  en los dos idiomas.
- **Descartado:**
  - `i18next` u otra librería: es una dependencia más para un centenar de
    textos.
  - Guardar el idioma solo en el navegador: el hook no lo vería.

## D7 · Verificación por sistema

- **Windows:** prueba manual de punta a punta en la máquina del usuario con
  el instalador generado por el workflow y Codex real.
- **macOS y Linux:** sin máquina disponible, se verifican en CI.
  - Las pruebas corren en los tres sistemas; macOS en `main` y en tags, como
    hoy.
  - Se generan los instaladores.
  - En Linux, la app compilada arranca bajo `xvfb` sin errores.
- La beta lo dice en sus notas. Una prueba en un Mac o un Linux reales, si
  aparece, se añade como evidencia.

## D8 · Implementación fuera del coordinador

Como en las specs 003 y 004: `AGENTS.md` pide no usar RouDex para construir
RouDex hasta que sea estable. Las tareas se implementan directamente y se
registran con `route_id` y `run_id` nulos.
