import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef, useState } from "react";
import Composer from "./Composer.jsx";
import { SOURCE_LABEL, basename, clockTime, decisionFromTurn, tokens } from "./lib/format.js";
import { useSession } from "./lib/session.jsx";

const FILTERS = [["todos", "todos"], ["curso", "en curso"], ["aprobacion", "aprobación"], ["fuerte", "fuerte"], ["archivados", "archivados"]];
const STATUS_LABEL = { running: "en curso", completed: "completado", interrupted: "interrumpido", failed: "falló" };
const APPROVAL_OPTIONS = [["accept", "aceptar"], ["acceptForSession", "aceptar en la sesión"], ["decline", "rechazar"], ["cancel", "cancelar turno"]];

// The reason often already names its source ("regla de riesgo: …").
function routerReason(turn) {
  const label = SOURCE_LABEL[turn.source] ?? turn.source;
  return turn.reason.startsWith(label) ? turn.reason : `${label}: ${turn.reason}`;
}

function TurnLog({ turn, live, onEscalate, disabled }) {
  const running = turn.status === "running" && !(live?.done);
  const text = live?.text || turn.response;
  return <div className="log-turn">
    <span className="log-time">{clockTime(turn.created_at)}</span>
    <span className="log-prompt"><span className="accent">❯</span> {turn.prompt}</span>
    <span />
    <span className="log-router">router  <span className={`tier-text tier-${turn.tier}`}>{turn.tier}</span> · {turn.model} {turn.effort} · {routerReason(turn)}</span>
    <span />
    <span className="log-answer">{text || (running ? "" : "sin respuesta")}{running && <span className="caret" aria-hidden="true">▍</span>}</span>
    {live?.commands?.map((command) => <FragmentCommand key={command.id} command={command} />)}
    {turn.error && <><span /><span className="log-error">{turn.error}</span></>}
    {!running && <><span />
      <span className="log-foot">─ turno {STATUS_LABEL[turn.status] ?? turn.status} · {tokens(turn.tokens.total)} tokens
        {turn.tier !== "fuerte" && <button type="button" className="link small" disabled={disabled} onClick={() => onEscalate(turn)}>re-ejecutar con fuerte</button>}
      </span></>}
  </div>;
}

function FragmentCommand({ command }) {
  const ok = command.exitCode === 0 || command.status === "completed";
  const failed = command.status === "failed" || (command.exitCode !== null && command.exitCode !== 0) || command.status === "declined";
  return <>
    <span />
    <span className="log-command">$ {command.command ?? command.type}
      <span className={failed ? "bad" : ok ? "good" : "accent"}>   {command.status === "en curso" ? "en curso" : failed ? `✗ ${command.status}` : "✓"}</span>
      {command.output && <pre>{command.output}</pre>}
    </span>
  </>;
}

export default function Threads({ project, config, selected, onSelect, onNewChat }) {
  const session = useSession();
  const [threads, setThreads] = useState([]);
  const [turns, setTurns] = useState([]);
  const [filter, setFilter] = useState("todos");
  const [search, setSearch] = useState("");
  const [renaming, setRenaming] = useState(null);
  const [error, setError] = useState(null);
  const endRef = useRef(null);

  const archived = filter === "archivados";
  useEffect(() => {
    invoke("thread_list", { includeArchived: archived }).then(setThreads).catch((reason) => setError(String(reason)));
  }, [session.revision, archived]);

  useEffect(() => {
    if (!selected) { setTurns([]); return; }
    invoke("chat_turns", { threadId: selected }).then(setTurns).catch((reason) => setError(String(reason)));
  }, [selected, session.revision]);

  useEffect(() => { endRef.current?.scrollIntoView({ block: "end" }); }, [turns, session.live]);

  const waiting = new Set(session.approvals.map((item) => item.threadId));
  const visible = useMemo(() => threads.filter((thread) => {
    if (archived !== thread.archived) return false;
    if (filter === "curso" && thread.last_status !== "running") return false;
    if (filter === "aprobacion" && !waiting.has(thread.thread_id)) return false;
    if (filter === "fuerte" && thread.last_tier !== "fuerte") return false;
    const needle = search.trim().toLowerCase();
    return !needle || [thread.title, thread.cwd, thread.last_model].some((value) => String(value ?? "").toLowerCase().includes(needle));
  }), [threads, filter, search, archived, waiting]);

  const current = threads.find((thread) => thread.thread_id === selected);
  const runningTurn = turns.find((turn) => turn.status === "running" && !session.live[turn.turn_id]?.done);
  const approvals = session.approvals.filter((item) => item.threadId === selected);
  const count = (id) => threads.filter((t) => !t.archived && (id === "curso" ? t.last_status === "running"
    : id === "aprobacion" ? waiting.has(t.thread_id) : id === "fuerte" ? t.last_tier === "fuerte" : true)).length;

  async function send(prompt, decision) {
    const sent = await invoke("chat_send", { project: current.cwd, threadId: current.thread_id, prompt, decision });
    session.markStarted(sent.turnId);
    session.bump();
  }

  async function escalate(turn) {
    try {
      const decision = await invoke("router_escalate", { decision: decisionFromTurn(turn) });
      await send(turn.prompt, decision);
    } catch (reason) { setError(String(reason)); }
  }

  async function answer(item, decision) {
    try {
      await invoke("chat_approval", { project: current?.cwd ?? project, requestId: item.requestId, decision });
      session.resolveApproval(item);
    } catch (reason) { setError(String(reason)); }
  }

  async function stop() {
    if (!runningTurn) return;
    try { await invoke("chat_interrupt", { project: current.cwd, threadId: current.thread_id, turnId: runningTurn.turn_id }); }
    catch (reason) { setError(String(reason)); }
  }

  async function rename(event) {
    event.preventDefault();
    try {
      await invoke("thread_rename", { threadId: current.thread_id, title: renaming });
      setRenaming(null);
      session.bump();
    } catch (reason) { setError(String(reason)); }
  }

  async function archive() {
    try {
      await invoke("thread_archive", { threadId: current.thread_id, archived: !current.archived });
      onSelect(null);
      session.bump();
    } catch (reason) { setError(String(reason)); }
  }

  return <div className="threads">
    <section className="thread-list" aria-label="Lista de hilos">
      <div className="thread-list-head">
        <div className="row-between"><h1>Hilos</h1><button type="button" className="ghost" onClick={onNewChat}>+ nuevo</button></div>
        <label className="field">buscar
          <input type="search" placeholder="título, proyecto o modelo" value={search} onChange={(event) => setSearch(event.target.value)} />
        </label>
        <div className="chips" role="group" aria-label="Filtros">
          {FILTERS.map(([id, label]) => <button key={id} type="button" className="chip" aria-pressed={filter === id}
            onClick={() => setFilter(id)}>{label}{id !== "archivados" ? ` · ${count(id)}` : ""}</button>)}
        </div>
      </div>
      <div className="thread-items">
        {visible.length ? visible.map((thread) => <button key={thread.thread_id} type="button" className="thread-item"
          aria-pressed={thread.thread_id === selected} onClick={() => onSelect(thread.thread_id)}>
          <span className="row-start"><span className={`dot tier-bg-${thread.last_tier ?? "none"}`} /><span className="ellipsis">{thread.title || "Sin título"}</span></span>
          <span className="row-between muted small">
            <span>{basename(thread.cwd)} · {waiting.has(thread.thread_id) ? "esperando aprobación" : STATUS_LABEL[thread.last_status] ?? "sin turnos"}</span>
            <span>{tokens(thread.tokens)}</span>
          </span>
        </button>) : <p className="muted pad">{threads.length ? "Ningún hilo coincide." : "Sin hilos todavía."}</p>}
      </div>
    </section>

    <main className="thread-main">
      {!current ? <div className="placeholder"><h2>Elige un hilo</h2><p className="muted">O empieza uno nuevo desde proyectos.</p></div> : <>
        <div className="thread-head">
          {renaming !== null ? <form className="rename" onSubmit={rename}>
            <label className="visually-hidden" htmlFor="rename">Nuevo título</label>
            <input id="rename" value={renaming} autoFocus onChange={(event) => setRenaming(event.target.value)} />
            <button type="submit" className="ghost">guardar</button>
            <button type="button" className="ghost" onClick={() => setRenaming(null)}>cancelar</button>
          </form> : <div className="thread-title">
            <h2>{current.title || "Sin título"}</h2>
            <span className="muted small">{basename(current.cwd)} · {current.turns} {current.turns === 1 ? "turno" : "turnos"} · {tokens(current.tokens)} tokens{current.last_model ? ` · ${current.last_model}` : ""}</span>
          </div>}
          <button type="button" className="ghost" onClick={() => setRenaming(current.title)}>renombrar</button>
          <button type="button" className="ghost" onClick={archive}>{current.archived ? "desarchivar" : "archivar"}</button>
        </div>
        <div className="transcript" aria-label="Transcripción">
          {turns.map((turn) => <TurnLog key={turn.turn_id} turn={turn} live={session.live[turn.turn_id]}
            disabled={Boolean(runningTurn)} onEscalate={escalate} />)}
          {approvals.map((item) => <div key={JSON.stringify(item.requestId)} className="approval" role="alert">
            <span className="approval-title">aprobación requerida · {item.kind === "fileChange" ? "cambio de archivos" : "comando"}</span>
            {item.command && <code>{item.command}</code>}
            {item.reason && <span className="approval-reason">{item.reason}</span>}
            <div className="row-wrap">
              {APPROVAL_OPTIONS.map(([decision, label]) => <button key={decision} type="button"
                className={decision === "accept" ? "primary" : decision === "acceptForSession" ? "amber-ghost" : "ghost"}
                onClick={() => answer(item, decision)}>{label}</button>)}
            </div>
          </div>)}
          <div ref={endRef} />
        </div>
        {error && <p className="error-line pad" role="alert">{error}</p>}
        <div className="thread-composer">
          <Composer project={current.cwd} config={config} onSend={send} running={Boolean(runningTurn)} onStop={stop}
            placeholder="describe la tarea · Ctrl+Enter analiza y envía" />
        </div>
      </>}
    </main>
  </div>;
}
