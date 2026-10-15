/**
 * Spanish and English for the interface (spec 005, D6). `t(key, values)`
 * looks the key up in the current language, falls back to Spanish, then to
 * the key itself, and fills `{name}` placeholders from `values`.
 */
import en from "./locales/en.js";
import es from "./locales/es.js";

export const LANGUAGES = ["es", "en"];

const DICTIONARIES = { es, en };

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

/** `key.one` or `key.other` by `count`, which is also a placeholder. */
export function tn(key, count, values = {}) {
  return t(`${key}.${count === 1 ? "one" : "other"}`, { count, ...values });
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

// Backend messages the app shows, as the backend writes them, with the key
// that says the same in the chosen language.
const BACKEND_ERRORS = [
  [/ya hay una ejecución en curso: (?<label>.+)$/, "error.busy"],
  [/la tarea (?<task>\S+) no existe$/, "error.noTask"],
  [/la ejecución (?<run>\S+) no existe$/, "error.noRun"],
  [/la spec (?<spec>\S+) no está cargada$/, "error.specNotLoaded"],
  [/no se puede cambiar el tier de (?<task>\S+): no existe, está en curso o ya terminó$/, "error.tierLocked"],
  [/(?<task>\S+) se está ejecutando; detenla en su lugar$/, "error.stillRunning"],
  [/la tarea (?<task>\S+) no se está ejecutando$/, "error.notRunning"],
  [/la tarea (?<task>\S+) aún no tiene un turno en curso$/, "error.noTurnYet"],
  [/Codex was not found\. Looked in: (?<places>.+)\. Install Codex or set codex_path in (?<config>.+) to its executable/, "error.codexMissing"],
  [/codex_path in (?<config>.+) points to (?<path>.+), which is not a file/, "error.codexPath"],
  [/this build has no bundled router-cli; run router-cli hook-install from a terminal/, "error.noBundledCli"],
];

/** A backend error in the chosen language when it is a known one. */
export function errorText(message) {
  const text = String(message);
  for (const [pattern, key] of BACKEND_ERRORS) {
    const match = text.match(pattern);
    if (match) return text.slice(0, match.index) + t(key, match.groups ?? {});
  }
  return text;
}
