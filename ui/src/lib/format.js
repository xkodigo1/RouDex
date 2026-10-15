import { errorText, locale, t } from "./i18n.js";

export const TIERS = ["economico", "estandar", "fuerte"];

/** How a router decision was made: "regla de riesgo", "risk rule"… */
export function sourceLabel(source) {
  return t(`source.${source}`);
}

export function taskStatusLabel(status) {
  return t(`taskStatus.${status}`);
}

/** Thread access modes, in the order the composer offers them. */
export const ACCESS_MODES = ["write", "readOnly"];

export function accessLabel(mode) {
  return t(`access.${mode ?? "write"}`);
}

// Reasons the coordinator gives when a task does not run (receipt text),
// as it writes them, and the key that says the same in either language.
const RECEIPT_REASON = {
  "explicit approval required": "approvalRequired",
  "task has incomplete dependencies": "incompleteDependencies",
  "manual approval denied": "approvalDenied",
  "task is not ready for dispatch": "notReady",
  "thread recovery is blocked; explicit recovery required": "recoveryBlocked",
  "resumed history does not match the persisted turn; explicit recovery required": "historyMismatch",
  "run has no persisted thread; explicit recovery required": "noThread",
  "run has no persisted turn identity; explicit recovery required": "noTurn",
  "run has no valid immutable dispatch definition; explicit recovery required": "noDefinition",
  "resume has incomplete dependencies; explicit recovery required": "resumeDependencies",
  "resume requires approval": "resumeApproval",
  "run is not interrupted": "notInterrupted",
  "run was superseded by a later dispatch; resume the latest run instead": "superseded",
  "task is not retryable": "notRetryable",
  "task is not running": "notRunning",
  "task requires scope, acceptance and verification before dispatch": "missingContract",
  "task verification failed or returned no exit code": "verificationFailed",
  "task ready for retry": "readyForRetry",
};

export function reasonText(text) {
  const key = RECEIPT_REASON[text];
  return key ? t(`reason.${key}`) : errorText(text);
}

/** One line for the end of a task run: "R1 completada", "R2 bloqueada: …". */
export function receiptLine(taskId, receipt) {
  if (receipt.status === "completed") return t("receipt.completed", { task: taskId });
  const status = taskStatusLabel(receipt.status);
  // An interrupted turn's text is its partial answer, not a reason.
  const text = receipt.text && receipt.status !== "interrupted" ? reasonText(receipt.text) : "";
  if (!text) return t("receipt.status", { task: taskId, status });
  return t("receipt.withReason", { task: taskId, status, reason: text.length > 200 ? `${text.slice(0, 200)}…` : text });
}

// How router reasons name their source, in either language.
const REASON_SOURCE = {
  risk: ["regla de riesgo", "risk rule"],
  rule: ["mensaje", "message"],
  manual: ["manual", "by hand"],
  spec: ["spec"],
  fallback: ["clasificador", "classifier", "regla", "rule"],
  model: [],
};

/** "regla de riesgo: x", without repeating the source the reason already names. */
export function routeReason(route) {
  const label = sourceLabel(route.source);
  const named = [label, ...(REASON_SOURCE[route.source] ?? [])].some((name) => route.reason.includes(name));
  return named ? route.reason : `${label}: ${route.reason}`;
}

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
  if (diff < 60) return t("time.now");
  if (diff < 3600) return t("time.minutes", { count: Math.floor(diff / 60) });
  if (diff < 86400) return t("time.hours", { count: Math.floor(diff / 3600) });
  if (diff < 172800) return t("time.yesterday");
  return new Date(seconds * 1000).toLocaleDateString(locale(), { day: "numeric", month: "short" });
}

export function clockTime(epochSeconds) {
  const seconds = Number(epochSeconds);
  if (!Number.isFinite(seconds)) return "";
  return new Date(seconds * 1000).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" });
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
