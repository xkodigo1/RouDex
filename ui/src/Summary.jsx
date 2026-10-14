import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { basename, percent, relativeTime, tokens } from "./lib/format.js";
import { useSession } from "./lib/session.jsx";

const MONTHS = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];
const TIER_COLOR = { economico: "var(--economico)", estandar: "var(--estandar)", fuerte: "var(--fuerte)" };
const SOURCE_SHORT = { risk: "riesgo", rule: "reglas", model: "modelo", fallback: "respaldo", manual: "manual" };

function dateKey(date) {
  const pad = (n) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** 53 Monday-first weeks ending today, each day with its activity level 0–4. */
function calendar(days) {
  const byDate = new Map(days.map((day) => [day.date, day]));
  const max = Math.max(1, ...days.map((day) => day.tokens));
  const today = new Date();
  today.setHours(12, 0, 0, 0);
  const start = new Date(today);
  start.setDate(today.getDate() - 364 - ((today.getDay() + 6) % 7));
  const weeks = [];
  const day = new Date(start);
  while (day <= today) {
    const week = { month: day.getDate() <= 7 ? MONTHS[day.getMonth()] : "", days: [] };
    for (let i = 0; i < 7; i++) {
      if (day > today) { week.days.push(null); continue; }
      const key = dateKey(day);
      const usage = byDate.get(key);
      const level = usage?.tokens ? Math.max(1, Math.min(4, Math.ceil((usage.tokens / max) * 4))) : 0;
      week.days.push({
        key,
        level,
        title: `${day.getDate()} ${MONTHS[day.getMonth()]} · ${usage ? `${tokens(usage.tokens)} tokens · ${usage.turns} turnos` : "sin actividad"}`,
      });
      day.setDate(day.getDate() + 1);
    }
    weeks.push(week);
  }
  return weeks;
}

function LimitBar({ label, window }) {
  if (!window) return null;
  const used = Math.max(0, Math.min(100, window.usedPercent ?? 0));
  const reset = window.resetsAt ? new Date(window.resetsAt * 1000) : null;
  return <div className="limit">
    <div className="row-between"><span>{label}</span><span>{used}%</span></div>
    <div className="bar" role="progressbar" aria-label={label} aria-valuenow={used} aria-valuemin={0} aria-valuemax={100}>
      <div className={`bar-fill ${used >= 85 ? "hot" : used >= 60 ? "warm" : "ok"}`} style={{ width: `${used}%` }} />
    </div>
    {reset && <span className="muted small">se reinicia {reset.toLocaleString("es", { weekday: "short", hour: "2-digit", minute: "2-digit" })}</span>}
  </div>;
}

/** Codex Desktop prompts seen by the RouDex hook, and whether it is installed. */
function DesktopCard({ desktop }) {
  if (!desktop) return null;
  const { summary, hook } = desktop;
  const counts = ["economico", "estandar", "fuerte"].map((tier) => [tier, summary[tier]]);
  return <section className="pane" aria-labelledby="desktop-title">
    <div className="row-wrap">
      <h2 id="desktop-title">Codex Desktop</h2>
      <span className="muted small">últimos {summary.days} días</span>
      <span className="grow" />
      {hook.command
        ? <span className="hook-status on" title={hook.command}>hook instalado</span>
        : <span className="hook-status" title={hook.error ?? hook.path}>hook no instalado · <code>router-cli hook-install</code></span>}
    </div>
    {summary.prompts ? <>
      <div className="desktop-stats">
        <span><strong>{summary.prompts.toLocaleString("es")}</strong> <span className="muted">mensajes · {summary.today} hoy</span></span>
        <span><strong className={summary.warned ? "warn-text" : ""}>{summary.warned}</strong> <span className="muted">{summary.warned === 1 ? "aviso" : "avisos"} de riesgo</span></span>
        <span><strong className={summary.blocked ? "warn-text" : ""}>{summary.blocked}</strong> <span className="muted">{summary.blocked === 1 ? "bloqueado" : "bloqueados"}</span></span>
        <span><strong>{percent(summary.on_recommended_model, summary.prompts)}</strong> <span className="muted">en el modelo recomendado</span></span>
      </div>
      <div className="stack-bar" aria-hidden="true">
        {counts.map(([tier, count]) => <div key={tier} style={{ width: percent(count, summary.prompts), background: TIER_COLOR[tier] }} />)}
      </div>
      <p className="muted small">
        {counts.map(([tier, count]) => <span key={tier}><span style={{ color: TIER_COLOR[tier] }}>● {tier}</span> {count} · </span>)}
        {summary.unruled} sin regla{summary.last_at ? ` · último ${relativeTime(summary.last_at)}` : ""}
      </p>
    </> : <p className="muted">{hook.command
      ? "Sin mensajes de Codex Desktop todavía."
      : "Instala el hook para ver aquí el tier recomendado y el riesgo de tus mensajes de Codex Desktop."}</p>}
  </section>;
}

export default function Summary({ project, onOpenThread, onOpenThreads }) {
  const { revision, rateLimits, setRateLimits } = useSession();
  const [data, setData] = useState(null);
  const [desktop, setDesktop] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    invoke("dashboard_summary").then(setData).catch((reason) => setError(String(reason)));
    invoke("desktop_hook_summary").then(setDesktop).catch((reason) => setError(String(reason)));
  }, [revision]);

  useEffect(() => {
    if (!project || rateLimits) return;
    invoke("usage_summary", { project })
      .then((summary) => summary.rateLimits && setRateLimits(summary.rateLimits))
      .catch(() => {});
  }, [project, rateLimits, setRateLimits]);

  const weeks = useMemo(() => calendar(data?.days ?? []), [data]);
  const activeDays = data?.days.length ?? 0;
  const yearTotal = data?.year.total ?? 0;
  const top = data?.models[0];
  const tierTotal = (data?.tiers ?? []).reduce((sum, tier) => sum + tier.turns, 0);

  return <div className="page">
    <div className="page-head">
      <span className="muted">$ roudex resumen --periodo 12m</span>
      <h1>Resumen</h1>
    </div>
    {error && <p className="error-line" role="alert">{error}</p>}

    <section className="stats" aria-label="Totales">
      <div className="pane stat"><span className="label">tokens · 12 meses</span><strong>{tokens(yearTotal)}</strong>
        <span className="muted small">{percent(data?.year.cached_input ?? 0, data?.year.input ?? 0)} servidos desde caché</span></div>
      <div className="pane stat"><span className="label">hoy</span><strong>{tokens(data?.today_tokens ?? 0)}</strong>
        <span className="muted small">{data?.today_turns ?? 0} turnos · {data?.today_threads ?? 0} hilos</span></div>
      <div className="pane stat"><span className="label">turnos</span><strong>{(data?.total_turns ?? 0).toLocaleString("es")}</strong>
        <span className="muted small">racha actual: {data?.streak_days ?? 0} días</span></div>
      <div className="pane stat"><span className="label">modelo más usado</span><strong className="stat-model">{top?.model ?? "—"}</strong>
        <span className="muted small">{top ? `${percent(top.tokens.total, (data?.models ?? []).reduce((s, m) => s + m.tokens.total, 0))} de los tokens` : "sin turnos todavía"}</span></div>
    </section>

    <section className="pane" aria-labelledby="heat-title">
      <div className="row-wrap">
        <h2 id="heat-title">Actividad · tokens por día</h2>
        <span className="muted small">{activeDays} días con actividad en el último año</span>
        <span className="grow" />
        <span className="legend muted small">menos {[0, 1, 2, 3, 4].map((level) => <span key={level} className={`cell l${level}`} />)} más</span>
      </div>
      <div className="heatmap-scroll">
        <div className="heatmap">
          <div className="heat-days"><span /><span>lun</span><span /><span>mié</span><span /><span>vie</span><span /><span /></div>
          {weeks.map((week, index) => <div key={index} className="heat-week">
            <span className="heat-month">{week.month}</span>
            {week.days.map((day, d) => day ? <span key={day.key} className={`cell l${day.level}`} title={day.title} /> : <span key={`x${d}`} className="cell empty" />)}
          </div>)}
        </div>
      </div>
    </section>

    <div className="grid-2">
      <section className="pane" aria-labelledby="models-title">
        <h2 id="models-title">Tokens por modelo</h2>
        {data?.models.length ? data.models.map((model) => <div key={model.model} className="meter">
          <div className="row-between"><span>{model.model}</span>
            <span className="muted">{tokens(model.tokens.total)} · {percent(model.tokens.total, data.models.reduce((s, m) => s + m.tokens.total, 0))}</span></div>
          <div className="bar"><div className="bar-fill model" style={{ width: percent(model.tokens.total, data.models.reduce((s, m) => s + m.tokens.total, 0)) }} /></div>
        </div>) : <p className="muted">Aún no hay turnos registrados.</p>}
      </section>

      <section className="pane" aria-labelledby="tiers-title">
        <h2 id="tiers-title">Turnos por tier</h2>
        {tierTotal ? <>
          <div className="stack-bar">
            {data.tiers.map((tier) => <div key={tier.tier} style={{ width: percent(tier.turns, tierTotal), background: TIER_COLOR[tier.tier] }} />)}
          </div>
          <div className="tier-grid">
            {["economico", "estandar", "fuerte"].map((name) => {
              const tier = data.tiers.find((t) => t.tier === name);
              return <div key={name} className="tier-stat">
                <span style={{ color: TIER_COLOR[name] }}>● {name}</span>
                <strong>{percent(tier?.turns ?? 0, tierTotal)}</strong>
                <span className="muted small">{tier ? tier.by_source.map(([source, count]) => `${SOURCE_SHORT[source] ?? source} ${percent(count, tierTotal)}`).join(" · ") : "sin turnos"}</span>
              </div>;
            })}
          </div>
        </> : <p className="muted">Aún no hay turnos registrados.</p>}
      </section>

      <section className="pane" aria-labelledby="limits-title">
        <h2 id="limits-title">Límites de uso</h2>
        {rateLimits ? <>
          <LimitBar label={`ventana ${rateLimits.primary?.windowDurationMins ? Math.round(rateLimits.primary.windowDurationMins / 60) + " h" : ""}`} window={rateLimits.primary} />
          <LimitBar label="semanal" window={rateLimits.secondary} />
        </> : <p className="muted">{project ? "Leyendo límites…" : "Abre un proyecto para leer los límites de Codex."}</p>}
      </section>

      <section className="pane" aria-labelledby="recent-title">
        <div className="row-between">
          <h2 id="recent-title">Hilos recientes</h2>
          <button type="button" className="link" onClick={onOpenThreads}>ver todos →</button>
        </div>
        {data?.recent.length ? data.recent.map((thread) => <button key={thread.thread_id} type="button" className="recent-thread" onClick={() => onOpenThread(thread)}>
          <span className="ellipsis">{thread.title || "Sin título"}</span>
          <span className={`tier-text tier-${thread.last_tier}`}>{thread.last_tier ?? ""}</span>
          <span className="muted small">{basename(thread.cwd)} · {relativeTime(thread.updated_at)}</span>
          <span className="muted small">{tokens(thread.tokens)}</span>
        </button>) : <p className="muted">Sin hilos todavía.</p>}
      </section>
    </div>

    <DesktopCard desktop={desktop} />
  </div>;
}
