# RouDex

RouDex es una aplicación local-first que enruta cada mensaje al modelo
adecuado: las tareas simples van a modelos económicos y las complejas o
arriesgadas a modelos fuertes. El objetivo es ahorrar cuota sin perder calidad.

Se ejecuta sobre `codex app-server` (stdio). Claude será el segundo runtime.

## Estado

El repositorio contiene la base técnica: cliente de App Server, persistencia
SQLite, un coordinador de tareas y una interfaz Tauri/React de solo lectura.
El siguiente hito (`v0.6.0`) es el chat con router. Consulta
[ROADMAP.md](ROADMAP.md).

RouDex no requiere cuentas, sincronización remota ni credenciales dentro del
repositorio.

## Instalación

Consulta [docs/installation.md](docs/installation.md).
