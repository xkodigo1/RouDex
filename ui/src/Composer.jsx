import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { ACCESS_MODES, TIERS, accessLabel, sourceLabel } from "./lib/format.js";
import { errorText, t } from "./lib/i18n.js";

/**
 * Prompt box with the router in the loop: the first Ctrl+Enter (or "analizar")
 * asks the router; the suggestion shows above the box with tier buttons; the
 * second Ctrl+Enter (or "enviar") sends with the chosen decision. With
 * `chooseAccess` it also offers the mode of the new thread (write or read-only).
 */
export default function Composer({ project, config, onSend, running, onStop, placeholder, compact = false, chooseAccess = false, children }) {
  const [draft, setDraft] = useState("");
  const [access, setAccess] = useState("write");
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
      setError(errorText(reason));
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
      reason: t("composer.manualReason", { tier: original.tier }) });
  }

  async function submit() {
    if (!decision) return analyze();
    setBusy("send");
    setError(null);
    try {
      await onSend(draft.trim(), decision, access);
      setDraft("");
      setDecision(null);
      setOriginal(null);
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(null);
    }
  }

  const loweredRisk = original?.source === "risk" && decision?.tier !== "fuerte";

  return <div className={`composer ${compact ? "compact" : ""}`}>
    {error && <p className="error-line" role="alert">{error}</p>}
    {decision && <div className="suggestion" aria-live="polite">
      <span className="muted">{t("composer.suggests")}</span>
      <span className={`tier-chip tier-${decision.tier}`}>{decision.tier} · {decision.model} {decision.effort}</span>
      <span className="muted">{sourceLabel(decision.source)}{seconds && decision.source === "model" ? t("composer.seconds", { seconds }) : ""}</span>
      <span className="grow" />
      <span className="muted">{t("composer.changeTier")}</span>
      {TIERS.map((tier) => <button key={tier} type="button" className={`tier-button tier-${tier}`}
        aria-pressed={decision.tier === tier} disabled={!config} onClick={() => choose(tier)}>{tier}</button>)}
      <p className="suggestion-reason">{decision.reason}</p>
      {decision.risk?.length > 0 && <p className="risk-line">{t("composer.risk", { list: decision.risk.map((r) => r.value).join(", ") })}</p>}
      {loweredRisk && <p className="error-line">{t("composer.lowered")}</p>}
    </div>}
    <label className="prompt">
      <span className="prompt-mark" aria-hidden="true">❯</span>
      <span className="visually-hidden">{t("composer.message")}</span>
      <textarea rows={compact ? 1 : 2} value={draft} placeholder={placeholder}
        onChange={(event) => { setDraft(event.target.value); setDecision(null); setOriginal(null); }}
        onKeyDown={(event) => { if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) { event.preventDefault(); submit(); } }} />
    </label>
    <div className="composer-actions">
      {children}
      {chooseAccess && <div className="access-modes" role="group" aria-label={t("composer.mode")} title={t("composer.modeHint")}>
        {ACCESS_MODES.map((mode) => <button key={mode} type="button" aria-pressed={access === mode}
          onClick={() => setAccess(mode)}>{accessLabel(mode)}</button>)}
      </div>}
      <span className="grow" />
      {running && <button type="button" className="ghost" onClick={onStop}>{t("composer.stop")}</button>}
      <button type="button" className="primary" disabled={!draft.trim() || !project || Boolean(busy) || running} onClick={submit}>
        {busy === "analyze" ? t("composer.analyzing") : busy === "send" ? t("composer.sending") : decision ? t("composer.send") : t("composer.analyze")}
      </button>
    </div>
  </div>;
}
