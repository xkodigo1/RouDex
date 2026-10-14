# Router

El router decide qué tier de modelo atiende cada mensaje. Nunca ejecuta la
tarea: solo devuelve la decisión, que la interfaz muestra y el usuario puede
cambiar antes de enviar.

## Orden de decisión

1. **Reglas de riesgo.** Si el mensaje contiene un término o una ruta de
   riesgo, el tier es `fuerte` y el clasificador no se consulta.
2. **Reglas rápidas.** Los mensajes cortos (hasta 30 palabras, sin bloques de
   código) con intención ligera —explicar, resumir, traducir, renombrar— van a
   `economico` sin llamar a ningún modelo.
3. **Clasificador.** El resto lo clasifica un modelo económico en un hilo
   efímero de solo lectura, que nunca pide aprobaciones.
4. **Respaldo.** Si el clasificador falla (límite de uso, runtime ausente,
   respuesta inválida), se usa `fallback_tier` y el motivo explica el error.

El escalado al tier `fuerte` es siempre manual.

## Configuración

`%LOCALAPPDATA%\RouDex\router.toml`. Si no existe se usan estos valores; si
existe y es inválido, el router falla en lugar de ignorarlo.

```toml
version = 1
fallback_tier = "estandar"

[tiers.economico]
model = "gpt-5.6-luna"
effort = "medium"

[tiers.estandar]
model = "gpt-6.1-sol"
effort = "medium"

[tiers.fuerte]
model = "gpt-6.1-sol"
effort = "high"

[classifier]
model = "gpt-5.6-luna"
effort = "high"
```

`router-cli models` compara esta configuración con el catálogo del runtime
(`model/list`) e informa de modelos o esfuerzos no disponibles.

## Reglas de riesgo por proyecto

`<proyecto>/.roudex/rules.toml` amplía las reglas por defecto (producción,
migraciones, credenciales, dinero, pagos, despliegues, borrados masivos…).
La comparación ignora mayúsculas y acentos y trabaja con palabras completas.

```toml
# extend_defaults = false  # para reemplazar las reglas por defecto
keywords = ["ledger", "settlement"]
paths = ["src/billing/"]
```

## Probar desde la terminal

```powershell
"Explícame qué es un trait en Rust" | cargo run -p router-cli -- route .
cargo run -p router-cli -- models .
```
