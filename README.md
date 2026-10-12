# RouDex

RouDex es una aplicación local-first para supervisar y ejecutar tareas de
Codex mediante perfiles de enrutamiento y Codex App Server.

## Hito 1

El primer hito establece una base local para:

- recibir mensajes desde el adaptador de escritorio;
- clasificar tareas por perfil, modelo y esfuerzo;
- persistir tareas, ejecuciones y eventos en SQLite;
- validar dependencias y aprobaciones;
- iniciar y continuar ejecuciones mediante `runId`;
- observar el estado desde una interfaz Tauri/React.

La ejecución no requiere cuentas, sincronización remota ni credenciales dentro
del repositorio.

## Instalación

Consulta [docs/installation.md](docs/installation.md) para preparar el
workspace y activar el adaptador local de mensajes.
