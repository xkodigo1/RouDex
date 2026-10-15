import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import NewChat from "./NewChat.jsx";
import Summary from "./Summary.jsx";
import Tasks from "./Tasks.jsx";
import Threads from "./Threads.jsx";
import { readStored, writeStored } from "./lib/format.js";
import { LANGUAGES, t } from "./lib/i18n.js";
import { LanguageProvider, useI18n } from "./lib/language.jsx";
import { SessionProvider, useSession } from "./lib/session.jsx";
import { TaskProvider, useTasks } from "./lib/tasks.jsx";
import "./theme.css";

const VIEWS = ["resumen", "hilos", "proyectos", "tareas"];

function Notices() {
  const { notices, dismiss } = useSession();
  if (!notices.length) return null;
  return <div className="notices">
    {notices.map((notice) => <p key={notice.id} className="notice" role="status">{notice.text}
      <button type="button" aria-label={t("notice.dismiss")} onClick={() => dismiss(notice.id)}>×</button></p>)}
  </div>;
}

function LanguagePicker() {
  const { language, choose } = useI18n();
  return <div className="segmented language-picker" role="group" aria-label={t("shell.language")}>
    {LANGUAGES.map((id) => <button key={id} type="button" aria-pressed={language === id} lang={id}
      onClick={() => language !== id && choose(id)}>{id}</button>)}
  </div>;
}

function Shell() {
  useI18n();
  const { rateLimits, notify } = useSession();
  const tasks = useTasks();
  const [view, setView] = useState(() => {
    const stored = readStored("roudex.view", "resumen");
    return VIEWS.includes(stored) ? stored : "resumen";
  });
  const [project, setProject] = useState(() => readStored("roudex.project", ""));
  const [recent, setRecent] = useState(() => readStored("roudex.recentProjects", []));
  const [known, setKnown] = useState([]);
  const [thread, setThread] = useState(null);
  const [config, setConfig] = useState(null);

  useEffect(() => { invoke("router_config").then(setConfig).catch(() => setConfig(null)); }, []);
  // Data from before 0.9.0 is copied to ~/.roudex once; say so that start.
  useEffect(() => {
    invoke("startup_migration").then((migration) => {
      if (migration) notify(t("notice.migrated", { from: migration.from, to: migration.to }));
    }).catch(() => {});
  }, [notify]);
  useEffect(() => { invoke("chat_projects").then(setKnown).catch(() => setKnown([])); }, [view]);

  const go = useCallback((id) => {
    setView(id);
    writeStored("roudex.view", id);
  }, []);

  // Validates the folder through the backend before switching to it.
  const chooseProject = useCallback(async (path) => {
    await invoke("thread_list", { project: path });
    setProject(path);
    writeStored("roudex.project", path);
    setRecent((list) => {
      const next = [path, ...list.filter((item) => item !== path)].slice(0, 8);
      writeStored("roudex.recentProjects", next);
      return next;
    });
  }, []);

  const projects = [...new Set([...recent, ...known])];
  const used = rateLimits?.primary?.usedPercent;

  return <main className="app">
    <header className="topbar">
      <div className="brand"><span className="accent">&gt;_</span><span>roudex</span></div>
      <span className="muted small ellipsis">{project || t("shell.noProject")}</span>
      <span className="grow" />
      {used !== undefined && <span className="badge-line">{t("shell.limits", {
        plan: rateLimits.planType ? t("shell.plan", { plan: rateLimits.planType }) : "",
        hours: rateLimits.primary?.windowDurationMins ? `${Math.round(rateLimits.primary.windowDurationMins / 60)} h` : "",
        used,
      })}</span>}
      <LanguagePicker />
    </header>
    <div className="body">
      <nav className="sidenav" aria-label={t("nav.label")}>
        {VIEWS.map((id) => <button key={id} type="button" className="navlink" aria-current={view === id ? "page" : undefined}
          onClick={() => go(id)}><span className={view === id ? "accent" : "dim"}>❯</span> {t(`nav.${id}`)}
          {id === "tareas" && tasks.active && <span className="accent caret" title={t("shell.running", { label: tasks.active })}>●</span>}</button>)}
        <span className="grow" />
        {config && <div className="tier-box small">
          <div>{t("shell.activeTiers")}</div>
          <div className="muted">economico · {config.tiers.economico.model}</div>
          <div className="muted">estandar · {config.tiers.estandar.model}</div>
          <div className="muted">fuerte · {config.tiers.fuerte.model} {config.tiers.fuerte.effort}</div>
        </div>}
      </nav>
      <div className="content">
        {view === "resumen" && <Summary project={project}
          onOpenThread={(item) => { setThread(item.thread_id); go("hilos"); }} onOpenThreads={() => go("hilos")} />}
        {view === "hilos" && <Threads project={project} config={config} selected={thread} onSelect={setThread} onNewChat={() => go("proyectos")} />}
        {view === "proyectos" && <NewChat project={project} projects={projects} config={config} onProject={chooseProject}
          onStarted={(id) => { setThread(id); go("hilos"); }} />}
        {view === "tareas" && <Tasks project={project} />}
      </div>
    </div>
    <Notices />
  </main>;
}

// The UI talks to the Rust backend through Tauri IPC; a plain browser tab on
// the dev server has no backend, so explain that instead of failing on invoke.
function OutsideDesktop() {
  return <main className="app outside">
    <div className="placeholder">
      <h2><span className="accent">&gt;_</span> roudex</h2>
      <p className="muted">{t("outside.needsDesktop")}</p>
      <p className="muted small">{t("outside.keepRunning")} <code>pnpm --dir ui dev</code> {t("outside.openWith")} <code>cargo run -p roudex-desktop</code>.</p>
    </div>
  </main>;
}

const inDesktop = typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
createRoot(document.getElementById("root")).render(inDesktop
  ? <LanguageProvider><SessionProvider><TaskProvider><Shell /></TaskProvider></SessionProvider></LanguageProvider>
  : <OutsideDesktop />);
