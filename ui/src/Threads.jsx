import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef, useState } from "react";
import Composer from "./Composer.jsx";
import { ApprovalCard, TurnLog } from "./Transcript.jsx";
import { accessLabel, basename, decisionFromTurn, tokens } from "./lib/format.js";
import { errorText, t, tn } from "./lib/i18n.js";
import { useSession } from "./lib/session.jsx";

const FILTERS = ["todos", "curso", "aprobacion", "fuerte", "archivados"];
const STATUS_KEY = { running: "turn.status.running", completed: "turn.status.completed", interrupted: "turn.status.interrupted", failed: "turn.status.failed" };

export default function Threads({ project, config, selected, onSelect, onNewChat }) {
  const session = useSession();
  const [threads, setThreads] = useState([]);
  const [turns, setTurns] = useState([]);
  const [filter, setFilter] = useState("todos");
  const [search, setSearch] = useState("");
  const [renaming, setRenaming] = useState(null);
  const [error, setError] = useState(null);
  const endRef = useRef(null);
  const fail = (reason) => setError(errorText(reason));

  const archived = filter === "archivados";
  useEffect(() => {
    invoke("thread_list", { includeArchived: archived }).then(setThreads).catch(fail);
  }, [session.revision, archived]);

  useEffect(() => {
    if (!selected) { setTurns([]); return; }
    invoke("chat_turns", { threadId: selected }).then(setTurns).catch(fail);
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
    } catch (reason) { fail(reason); }
  }

  async function answer(item, decision) {
    try {
      await invoke("chat_approval", { project: current?.cwd ?? project, requestId: item.requestId, decision });
      session.resolveApproval(item);
    } catch (reason) { fail(reason); }
  }

  async function stop() {
    if (!runningTurn) return;
    try { await invoke("chat_interrupt", { project: current.cwd, threadId: current.thread_id, turnId: runningTurn.turn_id }); }
    catch (reason) { fail(reason); }
  }

  async function rename(event) {
    event.preventDefault();
    try {
      await invoke("thread_rename", { threadId: current.thread_id, title: renaming });
      setRenaming(null);
      session.bump();
    } catch (reason) { fail(reason); }
  }

  async function archive() {
    try {
      await invoke("thread_archive", { threadId: current.thread_id, archived: !current.archived });
      onSelect(null);
      session.bump();
    } catch (reason) { fail(reason); }
  }

  const statusOf = (thread) => (waiting.has(thread.thread_id) ? t("threads.waiting")
    : STATUS_KEY[thread.last_status] ? t(STATUS_KEY[thread.last_status]) : t("threads.noTurns"));

  return <div className="threads">
    <section className="thread-list" aria-label={t("threads.listLabel")}>
      <div className="thread-list-head">
        <div className="row-between"><h1>{t("threads.title")}</h1><button type="button" className="ghost" onClick={onNewChat}>{t("threads.new")}</button></div>
        <label className="field">{t("threads.search")}
          <input type="search" placeholder={t("threads.searchPlaceholder")} value={search} onChange={(event) => setSearch(event.target.value)} />
        </label>
        <div className="chips" role="group" aria-label={t("threads.filters")}>
          {FILTERS.map((id) => <button key={id} type="button" className="chip" aria-pressed={filter === id}
            onClick={() => setFilter(id)}>{t(`threads.filter.${id}`)}{id !== "archivados" ? ` · ${count(id)}` : ""}</button>)}
        </div>
      </div>
      <div className="thread-items">
        {visible.length ? visible.map((thread) => <button key={thread.thread_id} type="button" className="thread-item"
          aria-pressed={thread.thread_id === selected} onClick={() => onSelect(thread.thread_id)}>
          <span className="row-start"><span className={`dot tier-bg-${thread.last_tier ?? "none"}`} /><span className="ellipsis">{thread.title || t("threads.untitled")}</span></span>
          <span className="row-between muted small">
            <span>{basename(thread.cwd)}{thread.access === "readOnly" ? ` · ${accessLabel("readOnly")}` : ""} · {statusOf(thread)}</span>
            <span>{tokens(thread.tokens)}</span>
          </span>
        </button>) : <p className="muted pad">{threads.length ? t("threads.noMatch") : t("threads.none")}</p>}
      </div>
    </section>

    <main className="thread-main">
      {!current ? <div className="placeholder"><h2>{t("threads.pick")}</h2><p className="muted">{t("threads.pickHint")}</p></div> : <>
        <div className="thread-head">
          {renaming !== null ? <form className="rename" onSubmit={rename}>
            <label className="visually-hidden" htmlFor="rename">{t("threads.newTitle")}</label>
            <input id="rename" value={renaming} autoFocus onChange={(event) => setRenaming(event.target.value)} />
            <button type="submit" className="ghost">{t("threads.save")}</button>
            <button type="button" className="ghost" onClick={() => setRenaming(null)}>{t("threads.cancel")}</button>
          </form> : <div className="thread-title">
            <h2>{current.title || t("threads.untitled")}</h2>
            <span className="muted small">{basename(current.cwd)} · {tn("threads.turns", current.turns)} · {t("threads.tokens", { count: tokens(current.tokens) })}{current.last_model ? ` · ${current.last_model}` : ""}</span>
          </div>}
          <span className={`access-badge access-${current.access ?? "write"}`}
            title={current.access === "readOnly" ? t("threads.readOnlyHint") : t("threads.writeHint")}>
            {accessLabel(current.access)}</span>
          <button type="button" className="ghost" onClick={() => setRenaming(current.title)}>{t("threads.rename")}</button>
          <button type="button" className="ghost" onClick={archive}>{current.archived ? t("threads.unarchive") : t("threads.archive")}</button>
        </div>
        <div className="transcript" aria-label={t("threads.transcript")}>
          {turns.map((turn) => <TurnLog key={turn.turn_id} turn={turn} live={session.live[turn.turn_id]}
            disabled={Boolean(runningTurn)} onEscalate={escalate} />)}
          {approvals.map((item) => <ApprovalCard key={JSON.stringify(item.requestId)} item={item} onAnswer={answer} />)}
          <div ref={endRef} />
        </div>
        {error && <p className="error-line pad" role="alert">{error}</p>}
        <div className="thread-composer">
          <Composer project={current.cwd} config={config} onSend={send} running={Boolean(runningTurn)} onStop={stop}
            placeholder={t("threads.composerPlaceholder")} />
        </div>
      </>}
    </main>
  </div>;
}
