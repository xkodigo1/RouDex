import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { tokens } from "./lib/format.js";
import { locale, t } from "./lib/i18n.js";
import { useI18n } from "./lib/language.jsx";
import { useSession } from "./lib/session.jsx";

const TIER_COLOR = { economico: "var(--economico)", estandar: "var(--estandar)", fuerte: "var(--fuerte)" };

/** USD with cents, or more decimals for amounts under a cent. */
function money(amount) {
  const small = amount !== 0 && Math.abs(amount) < 0.01;
  return new Intl.NumberFormat(locale(), {
    style: "currency", currency: "USD", minimumFractionDigits: 2, maximumFractionDigits: small ? 4 : 2,
  }).format(amount);
}

/** The router.toml lines to add, one table per model still without a price. */
function priceSnippet(models) {
  return models.map((model) => `[prices."${model}"]\ninput = 0.0\ncached_input = 0.0\noutput = 0.0`).join("\n\n");
}

/**
 * Estimated cost and savings of the last 30 days (spec 005, D5): real cost
 * at the user's prices, the same tokens at the fuerte model, and what the
 * classifier spent. Without prices it shows tokens by tier and how to add them.
 */
export default function SavingsCard() {
  useI18n();
  const { revision } = useSession();
  const [summary, setSummary] = useState(null);
  const [withDesktop, setWithDesktop] = useState(false);
  const [error, setError] = useState(null);

  const load = useCallback(() => {
    invoke("router_config").then((config) => setWithDesktop(Boolean(config?.savings?.include_codex_desktop))).catch(() => {});
    invoke("savings_summary").then((value) => { setSummary(value); setError(null); }).catch((reason) => setError(String(reason)));
  }, []);
  useEffect(load, [load, revision]);

  async function toggleDesktop(event) {
    try {
      await invoke("settings_set_codex_desktop", { enabled: event.target.checked });
      load();
    } catch (reason) { setError(String(reason)); }
  }

  const priced = summary && summary.baselinePriced && summary.sources.some((source) => source.pricedTurns > 0);
  const missing = summary ? [...new Set([...(summary.baselinePriced ? [] : [summary.baselineModel]), ...summary.unpricedModels])] : [];
  const share = priced && summary.baseline > 0 ? Math.round((summary.savings / summary.baseline) * 100) : null;

  return <section className="pane" aria-labelledby="savings-title">
    <div className="row-wrap">
      <h2 id="savings-title">{t("savings.title")}</h2>
      <span className="muted small">{t("savings.window", { days: summary?.days ?? 30 })}</span>
      <span className="grow" />
      <label className="toggle small">
        <input type="checkbox" checked={withDesktop} onChange={toggleDesktop} />
        {t("savings.includeDesktop")}
      </label>
    </div>
    {error && <p className="error-line" role="alert">{error}</p>}
    {!summary ? <p className="muted">{t("savings.loading")}</p> : priced ? <>
      <div className="savings-stats">
        <div><span className="label">{t("savings.cost")}</span><strong>{money(summary.cost)}</strong></div>
        <div><span className="label">{t("savings.baseline")}</span><strong>{money(summary.baseline)}</strong>
          <span className="muted small">{summary.baselineModel}</span></div>
        <div><span className="label">{t("savings.router")}</span><strong>{money(summary.router.cost)}</strong>
          <span className="muted small">{t("savings.routerTurns", { count: summary.router.turns })}</span></div>
        <div><span className="label">{t("savings.savings")}</span>
          <strong className={summary.savings >= 0 ? "good" : "bad"}>{money(summary.savings)}</strong>
          {share !== null && <span className="muted small">{t("savings.share", { percent: share })}</span>}</div>
      </div>
      <table className="savings-sources">
        <thead><tr><th>{t("savings.source")}</th><th>{t("savings.turns")}</th><th>{t("savings.tokens")}</th><th>{t("savings.cost")}</th><th>{t("savings.atFuerte")}</th></tr></thead>
        <tbody>{summary.sources.map((source) => <tr key={source.source}>
          <td>{t(`savings.source.${source.source}`)}</td>
          <td>{source.pricedTurns === source.turns ? source.turns : `${source.pricedTurns}/${source.turns}`}</td>
          <td>{tokens(source.tokens.total)}</td>
          <td>{money(source.cost)}</td>
          <td>{money(source.baseline)}</td>
        </tr>)}</tbody>
      </table>
      {summary.unpricedTurns > 0 && <p className="warn-text small">{t("savings.unpriced", { count: summary.unpricedTurns, models: summary.unpricedModels.join(", ") })}</p>}
      <p className="muted small">{t("savings.note")}</p>
    </> : <>
      <p className="muted">{t("savings.noPrices")}</p>
      {summary.tiers.length > 0 && <div className="desktop-stats">
        {summary.tiers.map((tier) => <span key={tier.tier ?? "none"}>
          <span style={{ color: TIER_COLOR[tier.tier] ?? "var(--muted)" }}>● {tier.tier ?? t("savings.noTier")}</span> <strong>{tokens(tier.tokens.total)}</strong>
          <span className="muted"> · {t("savings.turnsCount", { count: tier.turns })}</span></span>)}
      </div>}
      {missing.length > 0 && <>
        <p className="muted small">{t("savings.addPrices")}</p>
        <pre className="snippet">{priceSnippet(missing)}</pre>
      </>}
    </>}
  </section>;
}
