export const TIERS = ["economico", "estandar", "fuerte"];

export const SOURCE_LABEL = {
  risk: "regla de riesgo",
  rule: "regla rápida",
  model: "clasificador",
  fallback: "respaldo",
  manual: "manual",
};

/** Thread access modes, in the order the composer offers them. */
export const ACCESS_LABEL = {
  write: "escritura",
  readOnly: "solo lectura",
};

export function tokens(count) {
  if (!count) return "0";
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${Math.round(count / 1_000)}k`;
  return String(count);
}

export function percent(part, total) {
  return total ? `${Math.round((part / total) * 100)}%` : "0%";
}

export function basename(path) {
  return String(path ?? "").split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/** Epoch seconds (stored as text) → "hace 12 min", "ayer", "3 oct". */
export function relativeTime(epochSeconds) {
  const seconds = Number(epochSeconds);
  if (!Number.isFinite(seconds)) return "";
  const diff = Date.now() / 1000 - seconds;
  if (diff < 60) return "ahora";
  if (diff < 3600) return `hace ${Math.floor(diff / 60)} min`;
  if (diff < 86400) return `hace ${Math.floor(diff / 3600)} h`;
  if (diff < 172800) return "ayer";
  return new Date(seconds * 1000).toLocaleDateString("es", { day: "numeric", month: "short" });
}

export function clockTime(epochSeconds) {
  const seconds = Number(epochSeconds);
  if (!Number.isFinite(seconds)) return "";
  return new Date(seconds * 1000).toLocaleTimeString("es", { hour: "2-digit", minute: "2-digit" });
}

export function readStored(key, fallback) {
  try {
    const value = localStorage.getItem(key);
    return value === null ? fallback : JSON.parse(value);
  } catch {
    return fallback;
  }
}

export function writeStored(key, value) {
  try { localStorage.setItem(key, JSON.stringify(value)); } catch { /* optional convenience */ }
}

/** Rebuild a RouteDecision from a stored turn (for manual escalation). */
export function decisionFromTurn(turn) {
  return { tier: turn.tier, model: turn.model, effort: turn.effort, source: turn.source, reason: turn.reason, risk: [] };
}
