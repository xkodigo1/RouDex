import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import NewChat from "./NewChat.jsx";
import Summary from "./Summary.jsx";
import TasksView from "./TasksView.jsx";
import Threads from "./Threads.jsx";
import { readStored, writeStored } from "./lib/format.js";
import { SessionProvider, useSession } from "./lib/session.jsx";
import "./styles.css";
import "./theme.css";

const VIEWS = [["resumen", "resumen"], ["hilos", "hilos"], ["proyectos", "proyectos"], ["tareas", "tareas"]];

function Notices() {
  const { notices, dismiss } = useSession();
  if (!notices.length) return null;
  return <div className="notices">
    {notices.map((notice) => <p key={notice.id} className="notice" role="status">{notice.text}
      <button type="button" aria-label="Descartar aviso" onClick={() => dismiss(notice.id)}>×</button></p>)}
  </div>;
}

function Shell() {
  const { rateLimits } = useSession();
  const [view, setView] = useState(() => {
    const stored = readStored("roudex.view", "resumen");
    return VIEWS.some(([id]) => id === stored) ? stored : "resumen";
  });
  const [project, setProject] = useState(() => readStored("roudex.project", ""));
  const [recent, setRecent] = useState(() => readStored("roudex.recentProjects", []));
  const [known, setKnown] = useState([]);
  const [thread, setThread] = useState(null);
  const [config, setConfig] = useState(null);

  useEffect(() => { invoke("router_config").then(setConfig).catch(() => setConfig(null)); }, []);
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
      <span className="muted small ellipsis">{project || "sin proyecto"}</span>
      <span className="grow" />
      {used !== undefined && <span className="badge-line">{rateLimits.planType ? `plan ${rateLimits.planType} · ` : ""}ventana {rateLimits.primary?.windowDurationMins ? Math.round(rateLimits.primary.windowDurationMins / 60) + " h" : ""} al {used}%</span>}
    </header>
    <div className="body">
      <nav className="sidenav" aria-label="Secciones">
        {VIEWS.map(([id, label]) => <button key={id} type="button" className="navlink" aria-current={view === id ? "page" : undefined}
          onClick={() => go(id)}><span className={view === id ? "accent" : "dim"}>❯</span> {label}</button>)}
        <span className="grow" />
        {config && <div className="tier-box small">
          <div>tiers activos</div>
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
        {view === "tareas" && <div className="legacy"><TasksView /></div>}
      </div>
    </div>
    <Notices />
  </main>;
}

createRoot(document.getElementById("root")).render(<SessionProvider><Shell /></SessionProvider>);
