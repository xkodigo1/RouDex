# 005 · Beta multiplataforma

- **Versión:** 1
- **Fecha:** 2026-10-14
- **Estado:** completada
- **Release:** `v0.9.0`
- Contrato ejecutable: [tasks.json](tasks.json) · Tareas: [tasks.md](tasks.md) ·
  Decisiones: [decisions.md](decisions.md)

## Problema

RouDex funciona hoy solo en Windows y solo desde el código fuente.

- **Datos:**
  - `router.toml` y la base local viven en `%LOCALAPPDATA%\RouDex`. En macOS
    y Linux caen en `./RouDex`, relativo a la carpeta desde la que se lance.
  - El hook guarda lo suyo en `~/.roudex`, así que hay dos lugares.
  - Codex Desktop (empaquetado MSIX) puede ver una copia privada de
    `router.toml`.
- **Codex:** fuera de Windows solo se busca en el `PATH`. Una app gráfica de
  macOS no hereda el `PATH` del terminal y la app de escritorio de Codex trae
  su propio binario.
- **Instalación:** no hay instaladores. Para usar RouDex hay que compilarlo e
  instalar el hook a mano desde la CLI.
- **Ahorro:** no se ve cuánto ahorra el router. Hay tokens por modelo y por
  tier, pero no lo que habrían costado sin enrutar.
- **Idioma:** la interfaz, el aviso del hook y muchos mensajes están solo en
  español.

## Resultado

1. **Tres sistemas.** RouDex corre en Windows, macOS y Linux.
   - Guarda todo en `~/.roudex` (o en `ROUDEX_HOME`). En Windows, la primera
     vez copia los datos de `%LOCALAPPDATA%\RouDex` sin tocar el original.
   - Encuentra Codex en el `PATH`, en las rutas habituales de cada sistema y
     dentro de la app de Codex, o donde diga `codex_path` en `router.toml`.
2. **Instaladores.** Cada release trae, sin firma de código:
   - Windows: instalador NSIS por usuario, sin permisos de administrador.
   - macOS: `.dmg` universal (Apple Silicon e Intel).
   - Linux: `.deb` y `.AppImage`.

   `router-cli` va dentro, y la app instala el hook de Codex con un botón.
3. **Ahorro estimado.** Resumen muestra cuánto costaron los turnos de los
   últimos 30 días y cuánto habrían costado en el tier `fuerte`.
   - El cálculo usa los precios por modelo que el usuario pone en
     `router.toml`.
   - Incluye el chat, las tareas y el coste del propio clasificador.
   - Opcionalmente suma el uso de Codex Desktop, leyendo de
     `~/.codex/sessions` solo el modelo y los tokens de cada turno.
4. **Español e inglés.**
   - La interfaz arranca en el idioma del sistema y se puede cambiar.
   - La elección se guarda en `router.toml` (`language`). La siguen la app,
     los mensajes del backend que muestra y el aviso del hook en Codex
     Desktop.

## Invariantes

- **Local-first:** sin cuentas, sin telemetría y sin red, salvo los turnos que
  ejecuta Codex.
- **Precios:** nunca se inventan. Un modelo sin precio no entra en el ahorro,
  y la tarjeta lo dice.
- **Sesiones de Codex:** solo se leen si el usuario lo activa, y solo el
  modelo y los contadores de tokens; nunca el texto de los mensajes. Los
  hilos que ya cuenta RouDex no se cuentan dos veces.
- **Migración:** nunca borra ni modifica los datos originales, y nunca pisa
  datos que ya existan en `~/.roudex`.
- **Hook:** sigue sin cambiar el modelo del chat y sin bloquear salvo lo que
  ya bloqueaba. Un fallo deja pasar el mensaje.
- **Instaladores:** no piden administrador en Windows. Los instaladores sin
  firma se documentan con los avisos que muestra cada sistema.
- **Compatibilidad:** un `router.toml` válido de `v0.8.0` sigue siendo válido.

## Fuera de alcance

- Firma de código, notarización y actualización automática.
- Tiendas y otros formatos: App Store, Microsoft Store, MSI, RPM, Snap,
  Flatpak y AUR.
- Builds para Windows ARM y Linux ARM.
- Renombrar los tiers. `economico`, `estandar` y `fuerte` siguen siendo los
  identificadores en los dos idiomas.
- Traducir la salida de la CLI; se revisa en `v1.0.0`.
- Precios por defecto o descargados de internet.
- Otros runtimes distintos de Codex.

## Hitos

| Hito | Release | Tareas |
| --- | --- | --- |
| H1 · Beta multiplataforma | `v0.9.0` | R005-T001 … R005-T010 |
