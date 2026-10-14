# 003 · Integración con Codex Desktop

- **Versión:** 2
- **Fecha:** 2026-10-14
- **Estado:** en ejecución
- **Release:** `v0.7.0`
- Contrato ejecutable: [tasks.json](tasks.json) · Tareas: [tasks.md](tasks.md) ·
  Decisiones: [decisions.md](decisions.md)

## Problema

El router de RouDex solo actúa en el chat de RouDex. Cuando escribes en Codex
Desktop no sabes qué tier recomendaría el router ni si el mensaje toca algo de
riesgo, y puedes mandar una migración o un cambio de producción con un modelo
económico.

## Resultado

Un hook `UserPromptSubmit` de Codex que, en cada mensaje de Codex Desktop:

1. Decide el tier con las **reglas de riesgo y las reglas rápidas** del router
   (sin clasificador: sin latencia ni cuota).
2. Añade al turno un contexto breve con el tier recomendado y el riesgo.
3. Si el mensaje tiene riesgo y el chat no usa el modelo del tier `fuerte`,
   **avisa** al usuario. Con `block_high_risk = true` en `router.toml`, en vez
   de avisar **bloquea** ese mensaje y explica a qué modelo cambiar.
4. Registra la decisión en `~/.roudex/desktop-hook.sqlite3`. El Resumen de la app
   muestra los mensajes de Codex Desktop por tier, los avisos, los bloqueos y si
   el hook está instalado.

`router-cli hook-install` copia `router-cli` a `~/.roudex/bin` y lo instala en
el `hooks.json` de usuario de Codex; `hook-uninstall` lo retira; la instalación también retira el hook
bloqueante de `v0.3.x`–`v0.5.0` si sigue ahí.

## Invariantes

- El hook nunca cambia el modelo del chat: Codex no lo permite y RouDex no lo
  intenta.
- Nunca se bloquea todo. Sin riesgo, o con el modelo de `fuerte`, el mensaje
  pasa siempre. Solo bloquea si `block_high_risk = true`.
- Cualquier fallo del hook (entrada inválida, configuración inválida, base de
  datos no disponible) deja pasar el mensaje y escribe el motivo en stderr.
- Los turnos que lanza RouDex (`ROUDEX_ROUTER_INTERNAL`) no se evalúan ni se
  registran.
- No se guarda el texto del mensaje: solo la decisión (tier, fuente, riesgo,
  modelo del chat, acción).
- Instalar dos veces deja una sola entrada; desinstalar solo quita entradas de
  RouDex y conserva el resto de `hooks.json`.
- Nada de lo que ejecuta o escribe el hook vive en AppData: Codex Desktop es
  una app empaquetada y vería una copia privada (decisión D6).
- Local-first: sin red y sin servicios remotos.

## Fuera de alcance

- Clasificar con modelo los mensajes dudosos desde el hook.
- Cambiar el modelo o el esfuerzo del chat de Codex Desktop.
- Hooks administrados (`requirements.toml`) o de proyecto.
- Instalar el hook desde la interfaz.

## Hitos

| Hito | Release | Tareas |
| --- | --- | --- |
| H1 · Hook de contexto | `v0.7.0` | R003-T001 … R003-T005 |
