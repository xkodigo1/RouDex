/**
 * Spanish and English for the interface (spec 005, D6). `t(key, values)`
 * looks the key up in the current language, falls back to Spanish, then to
 * the key itself, and fills `{name}` placeholders from `values`.
 */

export const LANGUAGES = ["es", "en"];

const DICTIONARIES = {
  es: {
    "nav.label": "Secciones",
    "nav.resumen": "resumen",
    "nav.hilos": "hilos",
    "nav.proyectos": "proyectos",
    "nav.tareas": "tareas",
    "shell.noProject": "sin proyecto",
    "shell.limits": "{plan}ventana {hours} al {used}%",
    "shell.plan": "plan {plan} · ",
    "shell.activeTiers": "tiers activos",
    "shell.running": "en curso: {label}",
    "shell.language": "Idioma",
    "notice.dismiss": "Descartar aviso",
    "notice.migrated": "Se copiaron tus datos de {from} a {to}; el original queda como respaldo.",
    "notice.sessionClosed": "La sesión con Codex se cerró: {reason}",
    "outside.needsDesktop": "Esta interfaz necesita la app de escritorio.",
    "outside.keepRunning": "Deja",
    "outside.openWith": "corriendo y abre la ventana con",
    "savings.title": "Ahorro estimado",
    "savings.window": "últimos {days} días",
    "savings.includeDesktop": "incluir Codex Desktop",
    "savings.loading": "Calculando…",
    "savings.cost": "coste estimado",
    "savings.baseline": "todo en fuerte",
    "savings.router": "coste del router",
    "savings.routerTurns": "{count} clasificaciones",
    "savings.savings": "ahorro",
    "savings.share": "{percent}% menos que en fuerte",
    "savings.source": "origen",
    "savings.turns": "turnos",
    "savings.tokens": "tokens",
    "savings.atFuerte": "en fuerte",
    "savings.source.chat": "chat",
    "savings.source.tasks": "tareas",
    "savings.source.codexDesktop": "Codex Desktop",
    "savings.unpriced": "{count} turnos sin precio no cuentan ({models}).",
    "savings.note": "Estimación con los precios de router.toml: con otro modelo los tokens habrían sido otros.",
    "savings.noPrices": "Sin precios no se estima dinero. Estos son los tokens por tier:",
    "savings.noTier": "sin tier",
    "savings.turnsCount": "{count} turnos",
    "savings.addPrices": "Añade el precio de cada modelo (USD por millón de tokens) en router.toml:",
  },
  en: {
    "nav.label": "Sections",
    "nav.resumen": "summary",
    "nav.hilos": "threads",
    "nav.proyectos": "projects",
    "nav.tareas": "tasks",
    "shell.noProject": "no project",
    "shell.limits": "{plan}{hours} window at {used}%",
    "shell.plan": "{plan} plan · ",
    "shell.activeTiers": "active tiers",
    "shell.running": "running: {label}",
    "shell.language": "Language",
    "notice.dismiss": "Dismiss notice",
    "notice.migrated": "Your data was copied from {from} to {to}; the original stays as a backup.",
    "notice.sessionClosed": "The Codex session closed: {reason}",
    "outside.needsDesktop": "This interface needs the desktop app.",
    "outside.keepRunning": "Keep",
    "outside.openWith": "running and open the window with",
    "savings.title": "Estimated savings",
    "savings.window": "last {days} days",
    "savings.includeDesktop": "include Codex Desktop",
    "savings.loading": "Calculating…",
    "savings.cost": "estimated cost",
    "savings.baseline": "all on fuerte",
    "savings.router": "router cost",
    "savings.routerTurns": "{count} classifications",
    "savings.savings": "savings",
    "savings.share": "{percent}% less than on fuerte",
    "savings.source": "source",
    "savings.turns": "turns",
    "savings.tokens": "tokens",
    "savings.atFuerte": "on fuerte",
    "savings.source.chat": "chat",
    "savings.source.tasks": "tasks",
    "savings.source.codexDesktop": "Codex Desktop",
    "savings.unpriced": "{count} turns without a price are left out ({models}).",
    "savings.note": "Estimated with the prices in router.toml: on another model the tokens would have differed.",
    "savings.noPrices": "Without prices no money is estimated. These are the tokens by tier:",
    "savings.noTier": "no tier",
    "savings.turnsCount": "{count} turns",
    "savings.addPrices": "Add each model's price (USD per million tokens) to router.toml:",
  },
};

let current = "es";

/** The language `t` uses; set by the language provider. */
export function setLanguage(language) {
  current = LANGUAGES.includes(language) ? language : "en";
  if (typeof document !== "undefined") document.documentElement.lang = current;
}

export function language() {
  return current;
}

/** Locale for `Intl` and `toLocale…` formatting. */
export function locale() {
  return current;
}

export function t(key, values = {}) {
  const text = DICTIONARIES[current]?.[key] ?? DICTIONARIES.es[key] ?? key;
  return text.replace(/\{(\w+)\}/g, (match, name) => (values[name] ?? match));
}

/** Every key with its text in both languages, for checks. */
export function dictionaries() {
  return DICTIONARIES;
}

/** Spanish for `es*` system languages, English otherwise. */
export function systemLanguage() {
  const system = typeof navigator === "undefined" ? "en" : (navigator.language || "en");
  return system.toLowerCase().startsWith("es") ? "es" : "en";
}
