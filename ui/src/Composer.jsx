import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { SOURCE_LABEL, TIERS } from "./lib/format.js";

/**
 * Prompt box with the router in the loop: the first Ctrl+Enter (or "analizar")
 * asks the router; the suggestion shows above the box with tier buttons; the
 * second Ctrl+Enter (or "enviar") sends with the chosen decision.
 */
export default function Composer({ project, config, onSend, running, onStop, placeholder, compact = false, children }) {
  const [draft, setDraft] = useState("");
  const [decision, setDecision] = useState(null);
  const [original, setOriginal] = useState(null);
  const [busy, setBusy] = useState(null);
  const [error, setError] = useState(null);
  const [seconds, setSeconds] = useState(null);

  async function analyze() {
    if (!draft.trim() || !project) return;
    setBusy("analyze");
    setError(null);
    const started = performance.now();
    try {
      const result = await invoke("router_decide", { project, prompt: draft });
      setDecision(result);
      setOriginal(result);
      setSeconds(((performance.now() - started) / 1000).toFixed(1));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(null);
    }
  }

  function choose(tier) {
    if (!original) return;
    if (tier === original.tier) { setDecision(original); return; }
    const target = config?.tiers?.[tier];
    if (!target) return;
    setDecision({ ...original, tier, model: target.model, effort: target.effort, source: "manual",
      reason: `elegido manualmente (el router sugería ${original.tier})` });
  }

  async function submit() {
    if (!decision) return analyze();
    setBusy("send");
    setError(null);
    try {
      await onSend(draft.trim(), decision);
      setDraft("");
      setDecision(null);
      setOriginal(null);
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy(null);
    }
  }

  const loweredRisk = original?.source === "risk" && decision?.tier !== "fuerte";

  return <div className={`composer ${compact ? "compact" : ""}`}>
    {error && <p className="error-line" role="alert">{error}</p>}
    {decision && <div className="suggestion" aria-live="polite">
      <span className="muted">router sugiere</span>
      <span className={`tier-chip tier-${decision.tier}`}>{decision.tier} · {decision.model} {decision.effort}</span>
      <span className="muted">{SOURCE_LABEL[decision.source] ?? decision.source}{seconds && decision.source === "model" ? ` · ${seconds} s` : ""}</span>
      <span className="grow" />
      <span className="muted">cambiar tier:</span>
      {TIERS.map((tier) => <button key={tier} type="button" className={`tier-button tier-${tier}`}
        aria-pressed={decision.tier === tier} disabled={!config} onClick={() => choose(tier)}>{tier}</button>)}
      <p className="suggestion-reason">{decision.reason}</p>
      {decision.risk?.length > 0 && <p className="risk-line">riesgo: {decision.risk.map((r) => r.value).join(", ")}</p>}
      {loweredRisk && <p className="error-line">Estás bajando una tarea riesgosa por debajo del tier fuerte.</p>}
    </div>}
    <label className="prompt">
      <span className="prompt-mark" aria-hidden="true">❯</span>
      <span className="visually-hidden">Mensaje</span>
      <textarea rows={compact ? 1 : 2} value={draft} placeholder={placeholder}
        onChange={(event) => { setDraft(event.target.value); setDecision(null); setOriginal(null); }}
        onKeyDown={(event) => { if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) { event.preventDefault(); submit(); } }} />
    </label>
    <div className="composer-actions">
      {children}
      <span className="grow" />
      {running && <button type="button" className="ghost" onClick={onStop}>detener</button>}
      <button type="button" className="primary" disabled={!draft.trim() || !project || Boolean(busy) || running} onClick={submit}>
        {busy === "analyze" ? "analizando…" : busy === "send" ? "enviando…" : decision ? "enviar ⏎" : "analizar ⏎"}
      </button>
    </div>
  </div>;
}
