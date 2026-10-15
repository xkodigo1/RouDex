import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import SavingsCard from "./SavingsCard.jsx";
import { basename, percent, relativeTime, tokens } from "./lib/format.js";
import { errorText, locale, t, tn } from "./lib/i18n.js";
import { useSession } from "./lib/session.jsx";

const TIER_COLOR = { economico: "var(--economico)", estandar: "var(--estandar)", fuerte: "var(--fuerte)" };

function dateKey(date) {
  const pad = (n) => String(n).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

const monthName = (date) => date.toLocaleDateString(locale(), { month: "short" }).replace(".", "");
const dayName = (date) => date.toLocaleDateString(locale(), { day: "numeric", month: "short" });

/** Monday-first weekday names, short, for the calendar's row labels. */
function weekdays() {
  const monday = new Date(2024, 0, 1);
  return Array.from({ length: 7 }, (_, index) => {
    const day = new Date(monday);
    day.setDate(monday.getDate() + index);
    return day.toLocaleDateString(locale(), { weekday: "short" }).replace(".", "");
  });
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
    const week = { month: day.getDate() <= 7 ? monthName(day) : "", days: [] };
    for (let i = 0; i < 7; i++) {
      if (day > today) { week.days.push(null); continue; }
      const key = dateKey(day);
      const usage = byDate.get(key);
      const level = usage?.tokens ? Math.max(1, Math.min(4, Math.ceil((usage.tokens / max) * 4))) : 0;
      week.days.push({
        key,
        level,
        title: usage
          ? t("summary.dayTitle", { date: dayName(day), tokens: tokens(usage.tokens), turns: usage.turns })
          : t("summary.dayIdle", { date: dayName(day) }),
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
    {reset && <span className="muted small">{t("summary.resets", { when: reset.toLocaleString(locale(), { weekday: "short", hour: "2-digit", minute: "2-digit" }) })}</span>}
  </div>;
}

/**
 * Install (or reinstall) the Codex hook from the router-cli bundled with an
 * installed app; running from source, point to the CLI instead.
 */
function HookInstall({ hook, onInstalled }) {
  const { notify } = useSession();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(null);
  if (!hook.bundled) {
    return <span className="muted small">{t("hook.fromSource")} <code>router-cli hook-install</code></span>;
  }
  async function install() {
    setBusy(true);
    setError(null);
    try {
      const installed = await invoke("desktop_hook_install");
      notify(t("hook.installed", { path: installed.hooksPath }));
      onInstalled();
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(false);
    }
  }
  return <>
    <button type="button" className={hook.command ? "ghost" : "primary"} disabled={busy} onClick={install}>
      {busy ? t("hook.installing") : hook.command ? t("hook.reinstall") : t("hook.install")}</button>
    {error && <span className="error-line small">{error}</span>}
  </>;
}

/** Codex Desktop prompts seen by the RouDex hook, and whether it is installed. */
function DesktopCard({ desktop, onInstalled }) {
  if (!desktop) return null;
  const { summary, hook } = desktop;
  const counts = ["economico", "estandar", "fuerte"].map((tier) => [tier, summary[tier]]);
  return <section className="pane" aria-labelledby="desktop-title">
    <div className="row-wrap">
      <h2 id="desktop-title">Codex Desktop</h2>
      <span className="muted small">{t("desktop.window", { days: summary.days })}</span>
      <span className="grow" />
      {hook.command
        ? <span className="hook-status on" title={hook.command}>{t("desktop.hookOn")}</span>
        : <span className="hook-status" title={hook.error ?? hook.path}>{t("desktop.hookOff")}</span>}
      <HookInstall hook={hook} onInstalled={onInstalled} />
    </div>
    {summary.prompts ? <>
      <div className="desktop-stats">
        <span><strong>{summary.prompts.toLocaleString(locale())}</strong> <span className="muted">{t("desktop.messages", { today: summary.today })}</span></span>
        <span><strong className={summary.warned ? "warn-text" : ""}>{summary.warned}</strong> <span className="muted">{tn("desktop.warnings", summary.warned)}</span></span>
        <span><strong className={summary.blocked ? "warn-text" : ""}>{summary.blocked}</strong> <span className="muted">{tn("desktop.blocked", summary.blocked)}</span></span>
        <span><strong>{percent(summary.on_recommended_model, summary.prompts)}</strong> <span className="muted">{t("desktop.onRecommended")}</span></span>
      </div>
      <div className="stack-bar" aria-hidden="true">
        {counts.map(([tier, count]) => <div key={tier} style={{ width: percent(count, summary.prompts), background: TIER_COLOR[tier] }} />)}
      </div>
      <p className="muted small">
        {counts.map(([tier, count]) => <span key={tier}><span style={{ color: TIER_COLOR[tier] }}>● {tier}</span> {count} · </span>)}
        {t("desktop.unruled", { count: summary.unruled })}{summary.last_at ? t("desktop.last", { when: relativeTime(summary.last_at) }) : ""}
      </p>
    </> : <p className="muted">{hook.command ? t("desktop.empty") : t("desktop.install")}</p>}
  </section>;
}

export default function Summary({ project, onOpenThread, onOpenThreads }) {
  const { revision, rateLimits, setRateLimits } = useSession();
  const [data, setData] = useState(null);
  const [desktop, setDesktop] = useState(null);
  const [error, setError] = useState(null);
  const [installs, setInstalls] = useState(0);

  useEffect(() => {
    invoke("dashboard_summary").then(setData).catch((reason) => setError(errorText(reason)));
  }, [revision]);

  // Codex Desktop prompts arrive from another app: reload when the window
  // comes back to the front and every 30 s while the summary is open.
  useEffect(() => {
    const load = () => invoke("desktop_hook_summary").then(setDesktop).catch((reason) => setError(errorText(reason)));
    const onVisible = () => { if (document.visibilityState === "visible") load(); };
    load();
    const timer = setInterval(load, 30_000);
    window.addEventListener("focus", load);
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      clearInterval(timer);
      window.removeEventListener("focus", load);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [revision, installs]);

  useEffect(() => {
    if (!project || rateLimits) return;
    invoke("usage_summary", { project })
      .then((summary) => summary.rateLimits && setRateLimits(summary.rateLimits))
      .catch(() => {});
  }, [project, rateLimits, setRateLimits]);

  const language = locale();
  const weeks = useMemo(() => calendar(data?.days ?? []), [data, language]);
  const dayLabels = useMemo(weekdays, [language]);
  const activeDays = data?.days.length ?? 0;
  const yearTotal = data?.year.total ?? 0;
  const top = data?.models[0];
  const modelTotal = (data?.models ?? []).reduce((sum, model) => sum + model.tokens.total, 0);
  const tierTotal = (data?.tiers ?? []).reduce((sum, tier) => sum + tier.turns, 0);
  const windowHours = rateLimits?.primary?.windowDurationMins ? `${Math.round(rateLimits.primary.windowDurationMins / 60)} h` : "";

  return <div className="page">
    <div className="page-head">
      <span className="muted">{t("summary.command")}</span>
      <h1>{t("summary.title")}</h1>
    </div>
    {error && <p className="error-line" role="alert">{error}</p>}

    <section className="stats" aria-label={t("summary.totals")}>
      <div className="pane stat"><span className="label">{t("summary.tokensYear")}</span><strong>{tokens(yearTotal)}</strong>
        <span className="muted small">{t("summary.fromCache", { percent: percent(data?.year.cached_input ?? 0, data?.year.input ?? 0) })}</span></div>
      <div className="pane stat"><span className="label">{t("summary.today")}</span><strong>{tokens(data?.today_tokens ?? 0)}</strong>
        <span className="muted small">{t("summary.todayDetail", { turns: data?.today_turns ?? 0, threads: data?.today_threads ?? 0 })}</span></div>
      <div className="pane stat"><span className="label">{t("summary.turns")}</span><strong>{(data?.total_turns ?? 0).toLocaleString(locale())}</strong>
        <span className="muted small">{t("summary.streak", { days: data?.streak_days ?? 0 })}</span></div>
      <div className="pane stat"><span className="label">{t("summary.topModel")}</span><strong className="stat-model">{top?.model ?? "—"}</strong>
        <span className="muted small">{top ? t("summary.topShare", { percent: percent(top.tokens.total, modelTotal) }) : t("summary.noTurnsYet")}</span></div>
    </section>

    <SavingsCard />

    <section className="pane" aria-labelledby="heat-title">
      <div className="row-wrap">
        <h2 id="heat-title">{t("summary.activity")}</h2>
        <span className="muted small">{t("summary.activeDays", { days: activeDays })}</span>
        <span className="grow" />
        <span className="legend muted small">{t("summary.less")} {[0, 1, 2, 3, 4].map((level) => <span key={level} className={`cell l${level}`} />)} {t("summary.more")}</span>
      </div>
      <div className="heatmap-scroll">
        <div className="heatmap">
          <div className="heat-days"><span />{dayLabels.map((name, index) => <span key={index}>{index % 2 === 0 && index < 6 ? name : ""}</span>)}</div>
          {weeks.map((week, index) => <div key={index} className="heat-week">
            <span className="heat-month">{week.month}</span>
            {week.days.map((day, d) => day ? <span key={day.key} className={`cell l${day.level}`} title={day.title} /> : <span key={`x${d}`} className="cell empty" />)}
          </div>)}
        </div>
      </div>
    </section>

    <div className="grid-2">
      <section className="pane" aria-labelledby="models-title">
        <h2 id="models-title">{t("summary.byModel")}</h2>
        {data?.models.length ? data.models.map((model) => <div key={model.model} className="meter">
          <div className="row-between"><span>{model.model}</span>
            <span className="muted">{tokens(model.tokens.total)} · {percent(model.tokens.total, modelTotal)}</span></div>
          <div className="bar"><div className="bar-fill model" style={{ width: percent(model.tokens.total, modelTotal) }} /></div>
        </div>) : <p className="muted">{t("summary.noTurnsRecorded")}</p>}
      </section>

      <section className="pane" aria-labelledby="tiers-title">
        <h2 id="tiers-title">{t("summary.byTier")}</h2>
        {tierTotal ? <>
          <div className="stack-bar">
            {data.tiers.map((tier) => <div key={tier.tier} style={{ width: percent(tier.turns, tierTotal), background: TIER_COLOR[tier.tier] }} />)}
          </div>
          <div className="tier-grid">
            {["economico", "estandar", "fuerte"].map((name) => {
              const tier = data.tiers.find((item) => item.tier === name);
              return <div key={name} className="tier-stat">
                <span style={{ color: TIER_COLOR[name] }}>● {name}</span>
                <strong>{percent(tier?.turns ?? 0, tierTotal)}</strong>
                <span className="muted small">{tier ? tier.by_source.map(([source, count]) => `${t(`sourceShort.${source}`)} ${percent(count, tierTotal)}`).join(" · ") : t("summary.noTurns")}</span>
              </div>;
            })}
          </div>
        </> : <p className="muted">{t("summary.noTurnsRecorded")}</p>}
      </section>

      <section className="pane" aria-labelledby="limits-title">
        <h2 id="limits-title">{t("summary.limits")}</h2>
        {rateLimits ? <>
          <LimitBar label={t("summary.window", { hours: windowHours })} window={rateLimits.primary} />
          <LimitBar label={t("summary.weekly")} window={rateLimits.secondary} />
        </> : <p className="muted">{project ? t("summary.readingLimits") : t("summary.openProject")}</p>}
      </section>

      <section className="pane" aria-labelledby="recent-title">
        <div className="row-between">
          <h2 id="recent-title">{t("summary.recent")}</h2>
          <button type="button" className="link" onClick={onOpenThreads}>{t("summary.seeAll")}</button>
        </div>
        {data?.recent.length ? data.recent.map((thread) => <button key={thread.thread_id} type="button" className="recent-thread" onClick={() => onOpenThread(thread)}>
          <span className="ellipsis">{thread.title || t("threads.untitled")}</span>
          <span className={`tier-text tier-${thread.last_tier}`}>{thread.last_tier ?? ""}</span>
          <span className="muted small">{basename(thread.cwd)} · {relativeTime(thread.updated_at)}</span>
          <span className="muted small">{tokens(thread.tokens)}</span>
        </button>) : <p className="muted">{t("summary.noThreads")}</p>}
      </section>
    </div>

    <DesktopCard desktop={desktop} onInstalled={() => setInstalls((n) => n + 1)} />
  </div>;
}
